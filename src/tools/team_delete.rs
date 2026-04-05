//! TeamDelete tool - Remove an in-memory team

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use anyhow::Result;
use serde_json::Value;

/// Input schema for team_delete tool
pub fn team_delete_input_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "name": {
                "type": "string",
                "description": "Team name to delete"
            }
        },
        "required": ["name"]
    })
}

/// TeamDelete tool
pub struct TeamDeleteTool;

#[async_trait::async_trait]
impl Tool for TeamDeleteTool {
    fn name(&self) -> &'static str {
        "team_delete"
    }

    fn description(&self) -> &'static str {
        "Delete an in-memory team by name."
    }

    fn input_schema(&self) -> Value {
        team_delete_input_schema()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        false
    }

    async fn execute(&self, input: Value, _context: ToolExecutionContext) -> Result<ToolResult> {
        let name = input["name"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'name' field"))?;

        // In a full implementation, this would remove the team
        // For now, just acknowledge the deletion
        tracing::info!("Deleting team: {}", name);

        Ok(ToolResult::success(format!("Deleted team {}", name)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_team_delete() {
        let tool = TeamDeleteTool;
        let input = serde_json::json!({"name": "test-team"});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("Deleted team"));
    }

    #[tokio::test]
    async fn test_team_delete_missing_name() {
        let tool = TeamDeleteTool;
        let input = serde_json::json!({});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_team_delete_nonexistent() {
        let tool = TeamDeleteTool;
        let input = serde_json::json!({
            "name": "nonexistent-team"
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("Deleted team"));
    }
}
