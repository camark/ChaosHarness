//! ToolSearch tool - Search for available tools

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use crate::tools::init::init_tools;
use anyhow::Result;
use serde_json::Value;

/// Input schema for tool_search tool
pub fn tool_search_input_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "query": {
                "type": "string",
                "description": "Search query to filter tools"
            }
        }
    })
}

/// ToolSearch tool
pub struct ToolSearchTool;

#[async_trait::async_trait]
impl Tool for ToolSearchTool {
    fn name(&self) -> &'static str {
        "tool_search"
    }

    fn description(&self) -> &'static str {
        "Search for available tools by name or description."
    }

    fn input_schema(&self) -> Value {
        tool_search_input_schema()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        true
    }

    async fn execute(&self, input: Value, _context: ToolExecutionContext) -> Result<ToolResult> {
        let query = input["query"].as_str();

        // Initialize registry to get all tools
        let registry = init_tools().await;
        let tools = registry.list_tools().await;

        let mut matching_tools = Vec::new();

        for tool in tools {
            let matches = if let Some(q) = query {
                let q_lower = q.to_lowercase();
                tool.name().to_lowercase().contains(&q_lower)
                    || tool.description().to_lowercase().contains(&q_lower)
            } else {
                true
            };

            if matches {
                matching_tools.push(format!("  - {} - {}", tool.name(), tool.description()));
            }
        }

        if matching_tools.is_empty() {
            let msg = if query.is_some() {
                format!("No tools found matching '{}'", query.unwrap())
            } else {
                "No tools available".to_string()
            };
            Ok(ToolResult::success(msg))
        } else {
            let output = format!(
                "Found {} tool(s):\n\n{}",
                matching_tools.len(),
                matching_tools.join("\n")
            );
            Ok(ToolResult::success(output))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_tool_search_all() {
        let tool = ToolSearchTool;
        let input = serde_json::json!({});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("tool"));
    }

    #[tokio::test]
    async fn test_tool_search_file() {
        let tool = ToolSearchTool;
        let input = serde_json::json!({"query": "file"});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.to_lowercase().contains("file"));
    }
}
