//! MCP (Model Context Protocol) types and schemas

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// MCP Protocol version
pub const MCP_VERSION: &str = "2024-11-05";

/// JSON-RPC 2.0 request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

impl JsonRpcRequest {
    pub fn new(id: i64, method: &str, params: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: serde_json::Value::Number(id.into()),
            method: method.to_string(),
            params,
        }
    }
}

/// JSON-RPC 2.0 response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// MCP Initialize request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeRequest {
    pub protocol_version: String,
    pub capabilities: ClientCapabilities,
    pub client_info: Implementation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roots: Option<RootsCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RootsCapability {
    pub list_changed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Implementation {
    pub name: String,
    pub version: String,
}

/// MCP Initialize response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResponse {
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    pub server_info: Implementation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptsCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourcesCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptsCapability {
    pub list_changed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourcesCapability {
    pub subscribe: bool,
    pub list_changed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolsCapability {
    pub list_changed: bool,
}

/// MCP Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpTool {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
}

/// MCP Resource definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpResource {
    pub uri: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// MCP Prompt definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpPrompt {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub arguments: Vec<PromptArgument>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptArgument {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

/// Tool call result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallResult {
    pub content: Vec<ContentBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text { text: String },
    Image { data: String, mime_type: String },
    Resource { resource: ResourceContent },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceContent {
    pub uri: String,
    pub mime_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob: Option<String>,
}

/// Log level for MCP logging
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)]
pub enum LogLevel {
    Debug,
    Info,
    Notice,
    Warning,
    Error,
    Critical,
    Alert,
    Emergency,
}

/// MCP Server configuration with transport type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Server name/identifier
    #[serde(default)]
    pub name: String,
    /// Command to spawn the server (for stdio transport)
    #[serde(default)]
    pub command: Option<String>,
    /// Command arguments
    #[serde(default)]
    pub args: Option<Vec<String>>,
    /// Environment variables
    #[serde(default)]
    pub env: Option<HashMap<String, String>>,
    /// Transport type: "stdio" or "sse"
    #[serde(default = "default_transport")]
    pub transport: String,
    /// URL for SSE transport
    #[serde(default)]
    pub url: Option<String>,
    /// Connection timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    /// Whether the server is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_transport() -> String { "stdio".to_string() }
fn default_timeout() -> u64 { 30 }
fn default_true() -> bool { true }

impl Default for McpServerConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            command: None,
            args: None,
            env: None,
            transport: "stdio".to_string(),
            url: None,
            timeout: 30,
            enabled: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_rpc_request() {
        let req = JsonRpcRequest::new(1, "tools/list", None);
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.method, "tools/list");
        assert!(req.params.is_none());
    }

    #[test]
    fn test_json_rpc_request_with_params() {
        let params = serde_json::json!({"key": "value"});
        let req = JsonRpcRequest::new(42, "tools/call", Some(params));
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.method, "tools/call");
        assert!(req.params.is_some());
    }

    #[test]
    fn test_json_rpc_request_serialization() {
        let req = JsonRpcRequest::new(1, "initialize", None);
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"method\":\"initialize\""));
    }

    #[test]
    fn test_json_rpc_response_success() {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Value::Number(1.into())),
            result: Some(serde_json::json!({"status": "ok"})),
            error: None,
        };
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_json_rpc_response_error() {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Value::Number(1.into())),
            result: None,
            error: Some(JsonRpcError {
                code: -32600,
                message: "Invalid Request".to_string(),
                data: None,
            }),
        };
        assert!(response.result.is_none());
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap().code, -32600);
    }

    #[test]
    fn test_json_rpc_error_with_data() {
        let error = JsonRpcError {
            code: -32602,
            message: "Invalid params".to_string(),
            data: Some(serde_json::json!({"detail": "missing field"})),
        };
        assert!(error.data.is_some());
    }

    #[test]
    fn test_mcp_server_config_default() {
        let config = McpServerConfig::default();
        assert_eq!(config.transport, "stdio");
        assert!(config.enabled);
        assert_eq!(config.timeout, 30);
        assert!(config.command.is_none());
        assert!(config.url.is_none());
    }

    #[test]
    fn test_mcp_server_config_serialization() {
        let config = McpServerConfig {
            name: "test_server".to_string(),
            command: Some("node".to_string()),
            args: Some(vec!["server.js".to_string()]),
            transport: "stdio".to_string(),
            timeout: 60,
            enabled: true,
            ..Default::default()
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"name\":\"test_server\""));
        assert!(json.contains("\"command\":\"node\""));
    }

    #[test]
    fn test_mcp_server_config_deserialization() {
        let json = r#"{"name":"my_server","command":"python","args":["-m","server"],"transport":"stdio","timeout":30,"enabled":true}"#;
        let config: McpServerConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.name, "my_server");
        assert_eq!(config.command, Some("python".to_string()));
        assert_eq!(config.transport, "stdio");
    }

    #[test]
    fn test_mcp_tool() {
        let tool = McpTool {
            name: "read_file".to_string(),
            description: Some("Read a file".to_string()),
            input_schema: serde_json::json!({"type": "object", "properties": {"path": {"type": "string"}}}),
        };
        assert_eq!(tool.name, "read_file");
        assert!(tool.description.is_some());
    }

    #[test]
    fn test_mcp_tool_serialization() {
        let tool = McpTool {
            name: "test".to_string(),
            description: None,
            input_schema: serde_json::json!({"type": "object"}),
        };
        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("\"name\":\"test\""));
        assert!(json.contains("\"inputSchema\""));
    }

    #[test]
    fn test_mcp_resource() {
        let resource = McpResource {
            uri: "file:///tmp/test.txt".to_string(),
            name: "test.txt".to_string(),
            description: Some("A test file".to_string()),
            mime_type: Some("text/plain".to_string()),
        };
        assert_eq!(resource.uri, "file:///tmp/test.txt");
        assert_eq!(resource.mime_type, Some("text/plain".to_string()));
    }

    #[test]
    fn test_mcp_prompt() {
        let prompt = McpPrompt {
            name: "code_review".to_string(),
            description: Some("Review code".to_string()),
            arguments: vec![
                PromptArgument {
                    name: "code".to_string(),
                    description: Some("Code to review".to_string()),
                    required: Some(true),
                },
            ],
        };
        assert_eq!(prompt.name, "code_review");
        assert_eq!(prompt.arguments.len(), 1);
        assert_eq!(prompt.arguments[0].name, "code");
    }

    #[test]
    fn test_content_block_text() {
        let block = ContentBlock::Text {
            text: "Hello".to_string(),
        };
        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains("\"type\":\"text\""));
        assert!(json.contains("\"text\":\"Hello\""));
    }

    #[test]
    fn test_content_block_image() {
        let block = ContentBlock::Image {
            data: "base64data".to_string(),
            mime_type: "image/png".to_string(),
        };
        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains("\"type\":\"image\""));
    }

    #[test]
    fn test_tool_call_result() {
        let result = ToolCallResult {
            content: vec![ContentBlock::Text {
                text: "Success".to_string(),
            }],
            is_error: None,
        };
        assert_eq!(result.content.len(), 1);
        assert!(result.is_error.is_none());
    }

    #[test]
    fn test_tool_call_result_error() {
        let result = ToolCallResult {
            content: vec![ContentBlock::Text {
                text: "File not found".to_string(),
            }],
            is_error: Some(true),
        };
        assert!(result.is_error.unwrap());
    }

    #[test]
    fn test_mcp_version() {
        assert_eq!(MCP_VERSION, "2024-11-05");
    }

    #[test]
    fn test_initialize_request() {
        let req = InitializeRequest {
            protocol_version: MCP_VERSION.to_string(),
            capabilities: ClientCapabilities {
                roots: Some(RootsCapability { list_changed: true }),
                sampling: None,
            },
            client_info: Implementation {
                name: "test_client".to_string(),
                version: "1.0.0".to_string(),
            },
        };
        assert_eq!(req.protocol_version, MCP_VERSION);
        assert_eq!(req.client_info.name, "test_client");
    }

    #[test]
    fn test_initialize_response() {
        let resp = InitializeResponse {
            protocol_version: MCP_VERSION.to_string(),
            capabilities: ServerCapabilities {
                logging: None,
                prompts: Some(PromptsCapability { list_changed: false }),
                resources: Some(ResourcesCapability {
                    subscribe: true,
                    list_changed: true,
                }),
                tools: Some(ToolsCapability { list_changed: true }),
            },
            server_info: Implementation {
                name: "test_server".to_string(),
                version: "2.0.0".to_string(),
            },
            instructions: Some("Use these tools".to_string()),
        };
        assert!(resp.capabilities.tools.is_some());
        assert!(resp.instructions.is_some());
    }

    #[test]
    fn test_resource_content() {
        let content = ResourceContent {
            uri: "file:///test".to_string(),
            mime_type: "text/plain".to_string(),
            text: Some("content".to_string()),
            blob: None,
        };
        assert!(content.text.is_some());
        assert!(content.blob.is_none());
    }

    #[test]
    fn test_log_level_serialization() {
        let level = LogLevel::Warning;
        let json = serde_json::to_string(&level).unwrap();
        assert_eq!(json, "\"warning\"");
    }

    #[test]
    fn test_prompt_argument_optional() {
        let arg = PromptArgument {
            name: "optional_arg".to_string(),
            description: None,
            required: None,
        };
        let json = serde_json::to_string(&arg).unwrap();
        // Should not contain null/optional fields
        assert!(json.contains("\"name\":\"optional_arg\""));
    }
}
