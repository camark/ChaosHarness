//! ReadMcpResource tool - Read an MCP resource

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use crate::mcp::client::GLOBAL_MCP_MANAGER;
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

        let manager = GLOBAL_MCP_MANAGER.lock().await;

        match manager.read_resource(server, uri).await {
            Ok(content) => {
                let mut output = String::new();
                output.push_str(&format!("Resource: {}\n", content.uri));
                output.push_str(&format!("MIME Type: {}\n", content.mime_type));

                if let Some(text) = content.text {
                    output.push_str(&format!("\n{}", text));
                } else if let Some(blob) = content.blob {
                    output.push_str(&format!("\n[Binary data: {} bytes]", blob.len()));
                }

                Ok(ToolResult::success(output))
            }
            Err(e) => Ok(ToolResult::error(format!(
                "Failed to read resource '{}' from '{}': {}",
                uri, server, e
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_read_mcp_resource_no_server() {
        let tool = ReadMcpResourceTool;
        let input = serde_json::json!({
            "server": "nonexistent-server",
            "uri": "file:///test.txt"
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        // Should fail because no server is connected
        assert!(result.is_error);
        assert!(result.output.contains("Failed to read resource"));
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
