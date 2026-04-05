//! ListMcpResources tool - List MCP resources

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
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
        // In a full implementation, this would query the MCP manager
        // for all available resources from connected servers
        tracing::info!("Listing MCP resources");

        Ok(ToolResult::success("(no MCP resources)".to_string()))
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
