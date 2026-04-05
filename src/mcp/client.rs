//! MCP (Model Context Protocol) client implementation
//!
//! Supports both stdio and SSE transports

use crate::mcp::config::load_mcp_server_configs;
use crate::mcp::types::*;
use crate::config::Settings;
use anyhow::{Result, anyhow, bail};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, mpsc};

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

/// MCP Client for a single server
pub struct McpClient {
    pub config: McpServerConfig,
    pub state: ClientState,
    pub server_info: Option<Implementation>,
    pub tools: Vec<McpTool>,
    pub resources: Vec<McpResource>,
    pub prompts: Vec<McpPrompt>,
    process: Option<Child>,
    request_id: i64,
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
            request_id: 0,
        }
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

        let mut process = cmd.spawn()
            .map_err(|e| anyhow!("Failed to spawn process '{}': {}", command, e))?;

        self.process = Some(process);
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

        let result = self.send_request("initialize", Some(json!(init_request))).await?;

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
        // In a full implementation, this would:
        // 1. Generate request ID
        // 2. Send JSON-RPC request to server
        // 3. Wait for response
        // 4. Parse and return result

        // For now, return a placeholder
        warn!("send_request not fully implemented: {}", method);
        Ok(json!({}))
    }

    /// Send a notification
    async fn send_notification(&self, method: &str, params: Option<serde_json::Value>) -> Result<()> {
        // Notifications don't expect a response
        warn!("send_notification not fully implemented: {}", method);
        Ok(())
    }

    /// Disconnect from the server
    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(ref mut process) = self.process {
            process.kill().await.ok();
        }
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
    pub async fn get_client(&self, _name: &str) -> Option<tokio::sync::MutexGuard<McpClient>> {
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

    /// Call a tool on any connected server
    pub async fn call_tool(&self, server_name: &str, tool_name: &str, arguments: serde_json::Value) -> Result<ToolCallResult> {
        let clients = self.clients.lock().await;
        let client = clients.get(server_name)
            .ok_or_else(|| anyhow!("Server not found: {}", server_name))?;

        client.call_tool(tool_name, arguments).await
    }
}

use tracing::{info, warn, error};

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
