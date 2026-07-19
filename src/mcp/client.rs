//! MCP (Model Context Protocol) client implementation
//!
//! Supports both stdio and SSE transports

#![allow(dead_code)]

use crate::mcp::config::load_mcp_server_configs;
use crate::mcp::types::*;
use crate::config::Settings;
use anyhow::{Result, anyhow, bail};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

/// MCP Client state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClientState {
    Disconnected,
    Connecting,
    Connected,
    Initializing,
    Ready,
    Error,
}

/// Type alias for pending requests map
type PendingRequests = Arc<Mutex<HashMap<i64, tokio::sync::oneshot::Sender<(Option<serde_json::Value>, Option<String>)>>>>;

/// MCP Client for a single server
pub struct McpClient {
    pub config: McpServerConfig,
    pub state: ClientState,
    pub server_info: Option<Implementation>,
    pub tools: Vec<McpTool>,
    pub resources: Vec<McpResource>,
    pub prompts: Vec<McpPrompt>,
    process: Option<Arc<Mutex<Child>>>,
    request_id: Arc<Mutex<i64>>,
    /// Channel sender for sending requests to the reader task
    request_tx: Option<tokio::sync::mpsc::Sender<(i64, String)>>,
    // Shared map of pending requests: request_id -> Sender for response
    pending_requests: PendingRequests,
}

impl McpClient {
    pub fn new(config: McpServerConfig) -> Self {
        Self {
            config,
            state: ClientState::Disconnected,
            server_info: None,
            tools: Vec::new(),
            resources: Vec::new(),
            prompts: Vec::new(),
            process: None,
            request_id: Arc::new(Mutex::new(0)),
            request_tx: None,
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Start the background reader task for stdio transport
    async fn start_reader_task(
        mut child: Child,
        pending_requests: PendingRequests,
    ) -> Result<tokio::sync::mpsc::Sender<(i64, String)>> {
        let stdin = child.stdin.take().ok_or_else(|| anyhow!("Failed to open stdin"))?;
        let stdout = child.stdout.take().ok_or_else(|| anyhow!("Failed to open stdout"))?;

        let mut reader = BufReader::new(stdout).lines();
        let (request_tx, mut request_rx) = tokio::sync::mpsc::channel::<(i64, String)>(100);

        // Wrap stdin in Arc<Mutex> for shared access
        let stdin = Arc::new(Mutex::new(stdin));

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    // Read from stdout
                    result = reader.next_line() => {
                        match result {
                            Ok(Some(line)) => {
                                // Try to parse as JSON-RPC response
                                if let Ok(response) = serde_json::from_str::<JsonRpcResponse>(&line) {
                                    let id = response.id.and_then(|v| v.as_i64());
                                    if let Some(req_id) = id {
                                        let mut pending = pending_requests.lock().await;
                                        if let Some(tx) = pending.remove(&req_id) {
                                            let _ = tx.send((response.result, response.error.map(|e| e.message)));
                                        }
                                    }
                                }
                                // Also handle notifications (they don't have responses)
                            }
                            Ok(None) => {
                                // EOF - server exited
                                tracing::warn!("MCP server stdout closed");
                                break;
                            }
                            Err(e) => {
                                tracing::error!("Error reading from MCP server: {}", e);
                            }
                        }
                    }
                    // Write to stdin
                    Some((_id, msg)) = request_rx.recv() => {
                        let stdin_lock = stdin.lock().await;
                        let mut stdin_ref = stdin_lock;
                        let write_result = async {
                            stdin_ref.write_all(msg.as_bytes()).await?;
                            stdin_ref.write_all(b"\n").await?;
                            stdin_ref.flush().await
                        };
                        if let Err(e) = write_result.await {
                            tracing::error!("Failed to write to MCP server: {}", e);
                        }
                    }
                }
            }

            // Try to kill the process when done
            let _ = child.kill().await;
        });

        Ok(request_tx)
    }

    /// Connect to the MCP server
    pub async fn connect(&mut self) -> Result<()> {
        self.state = ClientState::Connecting;

        match self.config.transport.as_str() {
            "stdio" => self.connect_stdio().await?,
            "sse" => self.connect_sse().await?,
            other => bail!("Unsupported transport type: {}", other),
        }

        self.state = ClientState::Connected;
        Ok(())
    }

    /// Connect using stdio transport
    async fn connect_stdio(&mut self) -> Result<()> {
        let command = self.config.command.as_ref()
            .ok_or_else(|| anyhow!("No command specified for stdio transport"))?;

        let mut cmd = Command::new(command);
        if let Some(args) = &self.config.args {
            cmd.args(args);
        }
        cmd.kill_on_drop(true);
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let child = cmd.spawn()
            .map_err(|e| anyhow!("Failed to spawn process '{}': {}", command, e))?;

        let pending = self.pending_requests.clone();
        let request_tx = Self::start_reader_task(child, pending).await?;

        self.request_tx = Some(request_tx);
        self.process = None; // Process is now managed by the reader task

        Ok(())
    }

    /// Connect using SSE transport
    async fn connect_sse(&mut self) -> Result<()> {
        let url = self.config.url.as_ref()
            .ok_or_else(|| anyhow!("No URL specified for SSE transport"))?;

        // For SSE, we would use reqwest to connect
        // This is a simplified implementation
        info!("SSE connection to {} (not fully implemented)", url);
        Ok(())
    }

    /// Initialize the MCP connection
    pub async fn initialize(&mut self) -> Result<InitializeResponse> {
        self.state = ClientState::Initializing;

        let init_request = InitializeRequest {
            protocol_version: MCP_VERSION.to_string(),
            capabilities: ClientCapabilities {
                roots: Some(RootsCapability { list_changed: true }),
                sampling: None,
            },
            client_info: Implementation {
                name: "rust_harness".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };

        let params = serde_json::to_value(&init_request)
            .map_err(|e| anyhow!("Failed to serialize init request: {}", e))?;

        let result = self.send_request("initialize", Some(params)).await?;

        let response: InitializeResponse = serde_json::from_value(result)
            .map_err(|e| anyhow!("Failed to parse initialize response: {}", e))?;

        self.server_info = Some(response.server_info.clone());
        self.state = ClientState::Ready;

        // Send initialized notification
        self.send_notification("notifications/initialized", None).await?;

        Ok(response)
    }

    /// List available tools
    pub async fn list_tools(&mut self) -> Result<Vec<McpTool>> {
        let result = self.send_request("tools/list", None).await?;

        let tools: Vec<McpTool> = serde_json::from_value(
            result.get("tools").cloned().unwrap_or(json!([]))
        ).map_err(|e| anyhow!("Failed to parse tools: {}", e))?;

        self.tools = tools.clone();
        Ok(tools)
    }

    /// List available resources
    pub async fn list_resources(&mut self) -> Result<Vec<McpResource>> {
        let result = self.send_request("resources/list", None).await?;

        let resources: Vec<McpResource> = serde_json::from_value(
            result.get("resources").cloned().unwrap_or(json!([]))
        ).map_err(|e| anyhow!("Failed to parse resources: {}", e))?;

        self.resources = resources.clone();
        Ok(resources)
    }

    /// List available prompts
    pub async fn list_prompts(&mut self) -> Result<Vec<McpPrompt>> {
        let result = self.send_request("prompts/list", None).await?;

        let prompts: Vec<McpPrompt> = serde_json::from_value(
            result.get("prompts").cloned().unwrap_or(json!([]))
        ).map_err(|e| anyhow!("Failed to parse prompts: {}", e))?;

        self.prompts = prompts.clone();
        Ok(prompts)
    }

    /// Call a tool
    pub async fn call_tool(&self, name: &str, arguments: serde_json::Value) -> Result<ToolCallResult> {
        let params = json!({
            "name": name,
            "arguments": arguments
        });

        let result = self.send_request("tools/call", Some(params)).await?;

        let tool_result: ToolCallResult = serde_json::from_value(result)
            .map_err(|e| anyhow!("Failed to parse tool result: {}", e))?;

        Ok(tool_result)
    }

    /// Read a resource
    pub async fn read_resource(&self, uri: &str) -> Result<ResourceContent> {
        let params = json!({ "uri": uri });
        let result = self.send_request("resources/read", Some(params)).await?;

        let contents = result.get("contents")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .cloned()
            .unwrap_or(json!({}));

        let content: ResourceContent = serde_json::from_value(contents)
            .map_err(|e| anyhow!("Failed to parse resource: {}", e))?;

        Ok(content)
    }

    /// Get a prompt
    pub async fn get_prompt(&self, name: &str, arguments: Option<serde_json::Value>) -> Result<String> {
        let mut params = json!({ "name": name });
        if let Some(args) = arguments {
            params["arguments"] = args;
        }

        let result = self.send_request("prompts/get", Some(params)).await?;

        // Extract prompt messages and combine into text
        let empty_vec = vec![];
        let messages = result.get("messages")
            .and_then(|m| m.as_array())
            .unwrap_or(&empty_vec);

        let text = messages.iter()
            .filter_map(|m| m.get("content").and_then(|c| c.as_str()))
            .collect::<Vec<_>>()
            .join("\n");

        Ok(text)
    }

    /// Send a JSON-RPC request
    async fn send_request(&self, method: &str, params: Option<serde_json::Value>) -> Result<serde_json::Value> {
        let request_id = {
            let mut id = self.request_id.lock().await;
            *id += 1;
            *id
        };

        let request = JsonRpcRequest::new(request_id, method, params);
        let request_str = serde_json::to_string(&request)
            .map_err(|e| anyhow!("Failed to serialize request: {}", e))?;

        tracing::debug!("MCP request: {}", request_str);

        // Create a channel to receive the response
        let (tx, rx) = tokio::sync::oneshot::channel::<(Option<serde_json::Value>, Option<String>)>();

        // Register the pending request
        {
            let mut pending = self.pending_requests.lock().await;
            pending.insert(request_id, tx);
        }

        // Send the request
        let request_tx = self.request_tx.as_ref()
            .ok_or_else(|| anyhow!("MCP client not connected"))?;

        request_tx.send((request_id, request_str)).await
            .map_err(|e| anyhow!("Failed to send request: {}", e))?;

        // Wait for response with timeout
        let timeout = tokio::time::Duration::from_secs(self.config.timeout);
        let response = tokio::time::timeout(timeout, rx)
            .await
            .map_err(|_| anyhow!("Request '{}' timed out after {}s", method, self.config.timeout))?
            .map_err(|_| anyhow!("Response channel closed"))?;

        if let Some(error_msg) = response.1 {
            return Err(anyhow!("MCP error: {}", error_msg));
        }

        response.0.ok_or_else(|| anyhow!("Empty response from MCP server"))
    }

    /// Send a notification
    async fn send_notification(&self, method: &str, params: Option<serde_json::Value>) -> Result<()> {
        let request_id = {
            let mut id = self.request_id.lock().await;
            *id += 1;
            *id
        };

        // Notifications use null id
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: serde_json::Value::Null,
            method: method.to_string(),
            params,
        };

        let request_str = serde_json::to_string(&request)
            .map_err(|e| anyhow!("Failed to serialize notification: {}", e))?;

        tracing::debug!("MCP notification: {}", request_str);

        // Send the notification
        let request_tx = self.request_tx.as_ref()
            .ok_or_else(|| anyhow!("MCP client not connected"))?;

        request_tx.send((request_id, request_str)).await
            .map_err(|e| anyhow!("Failed to send notification: {}", e))?;

        Ok(())
    }

    /// Disconnect from the server
    pub async fn disconnect(&mut self) -> Result<()> {
        // Drop the request_tx channel - the reader task will exit when it detects EOF
        self.request_tx = None;
        self.process = None;
        self.state = ClientState::Disconnected;
        Ok(())
    }

    /// Check if client is ready
    pub fn is_ready(&self) -> bool {
        self.state == ClientState::Ready
    }
}

/// MCP Manager - manages multiple MCP server connections
pub struct McpManager {
    pub clients: Arc<Mutex<HashMap<String, McpClient>>>,
    settings: Settings,
}

impl McpManager {
    pub fn new(settings: Settings) -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
            settings,
        }
    }

    /// Initialize all configured MCP servers
    pub async fn initialize_all(&self) -> Result<Vec<String>> {
        let server_configs = load_mcp_server_configs(&self.settings);
        let mut connected = Vec::new();

        for (name, config) in server_configs {
            if !config.enabled {
                info!("Skipping disabled MCP server: {}", name);
                continue;
            }

            info!("Connecting to MCP server: {}", name);

            let mut client = McpClient::new(config);
            match client.connect().await {
                Ok(_) => {
                    match client.initialize().await {
                        Ok(init_response) => {
                            info!("Connected to {} v{}",
                                init_response.server_info.name,
                                init_response.server_info.version);

                            // List available tools
                            if let Ok(tools) = client.list_tools().await {
                                info!("  Tools: {}", tools.len());
                            }

                            // List available resources
                            if let Ok(resources) = client.list_resources().await {
                                info!("  Resources: {}", resources.len());
                            }

                            connected.push(name.clone());

                            let mut clients = self.clients.lock().await;
                            clients.insert(name, client);
                        }
                        Err(e) => {
                            error!("Failed to initialize {}: {}", name, e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to connect to {}: {}", name, e);
                }
            }
        }

        Ok(connected)
    }

    /// Get a client by name
    pub async fn get_client(&self, _name: &str) -> Option<tokio::sync::MutexGuard<'_, McpClient>> {
        // This is a simplified version - in production would need better handling
        None
    }

    /// List all connected servers
    pub async fn list_servers(&self) -> Vec<String> {
        let clients = self.clients.lock().await;
        clients.keys()
            .filter(|name| {
                clients.get(*name).map(|c| c.is_ready()).unwrap_or(false)
            })
            .cloned()
            .collect()
    }

    /// List tools for a specific server
    pub async fn list_tools_for_server(&self, server_name: &str) -> Option<Vec<McpTool>> {
        let clients = self.clients.lock().await;
        let client = clients.get(server_name)?;
        Some(client.tools.clone())
    }

    /// Get all tools from all connected servers
    pub async fn list_all_tools(&self) -> Vec<(String, McpTool)> {
        let clients = self.clients.lock().await;
        let mut all_tools = Vec::new();

        for (server_name, client) in clients.iter() {
            if client.is_ready() {
                for tool in &client.tools {
                    all_tools.push((server_name.clone(), tool.clone()));
                }
            }
        }

        all_tools
    }

    /// Call a tool on any connected server
    pub async fn call_tool(&self, server_name: &str, tool_name: &str, arguments: serde_json::Value) -> Result<ToolCallResult> {
        let clients = self.clients.lock().await;
        let client = clients.get(server_name)
            .ok_or_else(|| anyhow!("Server not found: {}", server_name))?;

        client.call_tool(tool_name, arguments).await
    }

    /// List resources for a specific server
    pub async fn list_resources_for_server(&self, server_name: &str) -> Result<Vec<McpResource>> {
        let mut clients = self.clients.lock().await;
        let client = clients.get_mut(server_name)
            .ok_or_else(|| anyhow!("Server not found: {}", server_name))?;

        if !client.is_ready() {
            bail!("Server {} is not ready", server_name);
        }

        client.list_resources().await
    }

    /// Read a resource from a specific server
    pub async fn read_resource(&self, server_name: &str, uri: &str) -> Result<ResourceContent> {
        let clients = self.clients.lock().await;
        let client = clients.get(server_name)
            .ok_or_else(|| anyhow!("Server not found: {}", server_name))?;

        if !client.is_ready() {
            bail!("Server {} is not ready", server_name);
        }

        client.read_resource(uri).await
    }

    /// Get list of all resources from all connected servers
    pub async fn list_all_resources(&self) -> Vec<(String, McpResource)> {
        let mut clients = self.clients.lock().await;
        let mut all_resources = Vec::new();

        for (server_name, client) in clients.iter_mut() {
            if client.is_ready() {
                if let Ok(resources) = client.list_resources().await {
                    for resource in resources {
                        all_resources.push((server_name.clone(), resource));
                    }
                }
            }
        }

        all_resources
    }
}

lazy_static::lazy_static! {
    /// Global MCP manager instance
    pub static ref GLOBAL_MCP_MANAGER: tokio::sync::Mutex<McpManager> = {
        let settings = crate::config::load_settings(None).unwrap_or_default();
        tokio::sync::Mutex::new(McpManager::new(settings))
    };
}

use tracing::{info, error};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_client_creation() {
        let config = McpServerConfig::default();
        let client = McpClient::new(config);
        assert_eq!(client.state, ClientState::Disconnected);
    }

    #[tokio::test]
    async fn test_mcp_manager_creation() {
        let settings = Settings::default();
        let manager = McpManager::new(settings);

        let servers = manager.list_servers().await;
        assert!(servers.is_empty());
    }
}
