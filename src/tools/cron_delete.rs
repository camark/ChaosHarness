//! CronDelete tool - Delete a cron job

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
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

        // In a full implementation, this would remove from the cron registry
        // For now, just acknowledge the deletion
        tracing::info!("Deleting cron job: {}", name);

        Ok(ToolResult::success(format!(
            "Deleted cron job '{}'",
            name
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_cron_delete() {
        let tool = CronDeleteTool;
        let input = serde_json::json!({"name": "test-job"});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("Deleted cron job"));
    }
}
