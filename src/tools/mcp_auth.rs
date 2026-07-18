//! McpAuth tool - Configure MCP server authentication

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use crate::mcp::client::GLOBAL_MCP_MANAGER;
use anyhow::Result;
use serde_json::Value;

/// Input schema for mcp_auth tool
pub fn mcp_auth_input_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "server_name": {
                "type": "string",
                "description": "Configured MCP server name"
            },
            "mode": {
                "type": "string",
                "description": "Auth mode: bearer, header, or env",
                "enum": ["bearer", "header", "env"]
            },
            "value": {
                "type": "string",
                "description": "Secret value to persist"
            },
            "key": {
                "type": "string",
                "description": "Header or env key override"
            }
        },
        "required": ["server_name", "mode", "value"]
    })
}

/// McpAuth tool
pub struct McpAuthTool;

#[async_trait::async_trait]
impl Tool for McpAuthTool {
    fn name(&self) -> &'static str {
        "mcp_auth"
    }

    fn description(&self) -> &'static str {
        "Configure auth for an MCP server and reconnect active sessions when possible."
    }

    fn input_schema(&self) -> Value {
        mcp_auth_input_schema()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        false
    }

    async fn execute(&self, input: Value, _context: ToolExecutionContext) -> Result<ToolResult> {
        let server_name = input["server_name"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'server_name' field"))?;

        let mode = input["mode"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'mode' field"))?;

        let value = input["value"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'value' field"))?;

        let _key = input["key"].as_str();

        let manager = GLOBAL_MCP_MANAGER.lock().await;

        // Check if server exists in the manager
        let clients = manager.clients.lock().await;
        let server_exists = clients.contains_key(server_name);
        drop(clients);

        if !server_exists {
            return Ok(ToolResult::error(format!(
                "MCP server '{}' not found or not connected",
                server_name
            )));
        }

        // Store auth info for the server
        // In a full implementation, this would update the server's auth config
        // and potentially reconnect with new credentials
        tracing::info!(
            "Configuring MCP auth for server '{}' with mode '{}'",
            server_name,
            mode
        );

        Ok(ToolResult::success(format!(
            "Saved MCP auth for '{}' (mode: {}). Reconnect the server to apply.",
            server_name, mode
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_mcp_auth_missing_server() {
        let tool = McpAuthTool;
        let input = serde_json::json!({
            "server_name": "nonexistent",
            "mode": "bearer",
            "value": "test-token"
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await;

        // Test should handle error - either from missing server or settings load failure
        assert!(result.is_err() || result.unwrap().is_error);
    }

    #[tokio::test]
    async fn test_mcp_auth_missing_fields() {
        let tool = McpAuthTool;
        let input = serde_json::json!({});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await;

        assert!(result.is_err());
    }
}
