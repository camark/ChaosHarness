//! ListMcpResources tool - List MCP resources

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use crate::mcp::client::GLOBAL_MCP_MANAGER;
use anyhow::Result;
use serde_json::Value;

/// Input schema for list_mcp_resources tool
pub fn list_mcp_resources_input_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {}
    })
}

/// ListMcpResources tool
pub struct ListMcpResourcesTool;

#[async_trait::async_trait]
impl Tool for ListMcpResourcesTool {
    fn name(&self) -> &'static str {
        "list_mcp_resources"
    }

    fn description(&self) -> &'static str {
        "List MCP resources available from connected servers."
    }

    fn input_schema(&self) -> Value {
        list_mcp_resources_input_schema()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        true
    }

    async fn execute(&self, _input: Value, _context: ToolExecutionContext) -> Result<ToolResult> {
        let manager = GLOBAL_MCP_MANAGER.lock().await;
        let resources = manager.list_all_resources().await;

        if resources.is_empty() {
            return Ok(ToolResult::success(
                "No MCP resources available. Connect to an MCP server first.".to_string()
            ));
        }

        let mut output = String::new();
        output.push_str("MCP Resources:\n\n");

        for (server, resource) in &resources {
            output.push_str(&format!("[{}] {}", server, resource.name));
            if let Some(desc) = &resource.description {
                output.push_str(&format!(" - {}", desc));
            }
            output.push_str(&format!(" ({})", resource.uri));
            if let Some(mime) = &resource.mime_type {
                output.push_str(&format!(" [{}]", mime));
            }
            output.push('\n');
        }

        Ok(ToolResult::success(output.trim_end().to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_list_mcp_resources() {
        let tool = ListMcpResourcesTool;
        let input = serde_json::json!({});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
    }
}
