//! TeamCreate tool - Create an in-memory team

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use crate::services::team_manager::GLOBAL_TEAM_MANAGER;
use anyhow::Result;
use serde_json::Value;

/// Input schema for team_create tool
pub fn team_create_input_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "name": {
                "type": "string",
                "description": "Team name"
            },
            "description": {
                "type": "string",
                "description": "Team description",
                "default": ""
            }
        },
        "required": ["name"]
    })
}

/// TeamCreate tool
pub struct TeamCreateTool;

#[async_trait::async_trait]
impl Tool for TeamCreateTool {
    fn name(&self) -> &'static str {
        "team_create"
    }

    fn description(&self) -> &'static str {
        "Create a lightweight in-memory team for agent tasks."
    }

    fn input_schema(&self) -> Value {
        team_create_input_schema()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        false
    }

    async fn execute(&self, input: Value, _context: ToolExecutionContext) -> Result<ToolResult> {
        let name = input["name"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'name' field"))?;

        let description = input["description"].as_str().unwrap_or("");

        let created = GLOBAL_TEAM_MANAGER.create_team(name, description).await;

        if created {
            Ok(ToolResult::success(format!("Created team '{}'", name)))
        } else {
            Ok(ToolResult::error(format!("Team '{}' already exists", name)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_team_create() {
        let tool = TeamCreateTool;
        let input = serde_json::json!({
            "name": "create-test-team",
            "description": "Test team"
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("Created team"));

        // Verify team exists
        let team = GLOBAL_TEAM_MANAGER.get_team("create-test-team").await;
        assert!(team.is_some());
    }

    #[tokio::test]
    async fn test_team_create_missing_name() {
        let tool = TeamCreateTool;
        let input = serde_json::json!({});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_team_create_duplicate() {
        let tool = TeamCreateTool;
        let input = serde_json::json!({
            "name": "dup-team",
            "description": "First"
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));

        // First creation should succeed
        let result = tool.execute(input.clone(), context.clone()).await.unwrap();
        assert!(!result.is_error);

        // Second creation should fail
        let result = tool.execute(input, context).await.unwrap();
        assert!(result.is_error);
        assert!(result.output.contains("already exists"));
    }

    #[tokio::test]
    async fn test_team_create_empty_description() {
        let tool = TeamCreateTool;
        let input = serde_json::json!({
            "name": "empty-desc-team",
            "description": ""
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("Created team"));
    }
}
