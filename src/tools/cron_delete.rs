//! CronDelete tool - Delete a cron job

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use crate::services::cron::CRON_MANAGER;
use anyhow::Result;
use serde_json::Value;

/// Input schema for cron_delete tool
pub fn cron_delete_input_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "name": {
                "type": "string",
                "description": "Name of the cron job to delete"
            }
        },
        "required": ["name"]
    })
}

/// CronDelete tool
pub struct CronDeleteTool;

#[async_trait::async_trait]
impl Tool for CronDeleteTool {
    fn name(&self) -> &'static str {
        "cron_delete"
    }

    fn description(&self) -> &'static str {
        "Delete a cron job by name."
    }

    fn input_schema(&self) -> Value {
        cron_delete_input_schema()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        false
    }

    async fn execute(&self, input: Value, _context: ToolExecutionContext) -> Result<ToolResult> {
        let name = input["name"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'name' field"))?;

        let deleted = CRON_MANAGER.delete_job(name).await;

        if deleted {
            Ok(ToolResult::success(format!("Deleted cron job '{}'", name)))
        } else {
            Ok(ToolResult::error(format!("Cron job '{}' not found", name)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_cron_delete_existing() {
        // Create a job first
        CRON_MANAGER.create_job("delete-test", "*/5 * * * *", "echo hi").await;

        let tool = CronDeleteTool;
        let input = serde_json::json!({"name": "delete-test"});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("Deleted cron job"));

        // Verify it's gone
        let job = CRON_MANAGER.get_job("delete-test").await;
        assert!(job.is_none());
    }

    #[tokio::test]
    async fn test_cron_delete_not_found() {
        let tool = CronDeleteTool;
        let input = serde_json::json!({"name": "nonexistent"});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(result.is_error);
        assert!(result.output.contains("not found"));
    }
}
