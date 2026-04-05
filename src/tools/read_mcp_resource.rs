//! ReadMcpResource tool - Read an MCP resource

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use anyhow::Result;
use serde_json::Value;

/// Input schema for read_mcp_resource tool
pub fn read_mcp_resource_input_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "server": {
                "type": "string",
                "description": "MCP server name"
            },
            "uri": {
                "type": "string",
                "description": "Resource URI"
            }
        },
        "required": ["server", "uri"]
    })
}

/// ReadMcpResource tool
pub struct ReadMcpResourceTool;

#[async_trait::async_trait]
impl Tool for ReadMcpResourceTool {
    fn name(&self) -> &'static str {
        "read_mcp_resource"
    }

    fn description(&self) -> &'static str {
        "Read an MCP resource by server and URI."
    }

    fn input_schema(&self) -> Value {
        read_mcp_resource_input_schema()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        true
    }

    async fn execute(&self, input: Value, _context: ToolExecutionContext) -> Result<ToolResult> {
        let server = input["server"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'server' field"))?;

        let uri = input["uri"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'uri' field"))?;

        // In a full implementation, this would call the MCP client
        // to read the resource from the server
        tracing::info!("Reading MCP resource from server '{}' at URI '{}'", server, uri);

        Ok(ToolResult::success(format!(
            "(MCP resource reading not implemented - server: {}, uri: {})",
            server, uri
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_read_mcp_resource() {
        let tool = ReadMcpResourceTool;
        let input = serde_json::json!({
            "server": "test-server",
            "uri": "file:///test.txt"
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
    }

    #[tokio::test]
    async fn test_read_mcp_resource_missing_fields() {
        let tool = ReadMcpResourceTool;
        let input = serde_json::json!({});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await;

        assert!(result.is_err());
    }
}
