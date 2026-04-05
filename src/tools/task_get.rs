//! TaskGet tool - Get task details

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use anyhow::Result;
use serde_json::Value;

/// Input schema for task_get tool
pub fn task_get_input_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "task_id": {
                "type": "string",
                "description": "Task identifier"
            }
        },
        "required": ["task_id"]
    })
}

/// TaskGet tool
pub struct TaskGetTool;

#[async_trait::async_trait]
impl Tool for TaskGetTool {
    fn name(&self) -> &'static str {
        "task_get"
    }

    fn description(&self) -> &'static str {
        "Get details for a background task."
    }

    fn input_schema(&self) -> Value {
        task_get_input_schema()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        true
    }

    async fn execute(&self, input: Value, _context: ToolExecutionContext) -> Result<ToolResult> {
        let task_id = input["task_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'task_id' field"))?;

        // In a full implementation, this would query the task manager
        // For now, return a placeholder response
        let output = format!("Task {} not found", task_id);

        Ok(ToolResult::error(output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_task_get() {
        let tool = TaskGetTool;
        let input = serde_json::json!({"task_id": "test-123"});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(result.is_error);
        assert!(result.output.contains("not found"));
    }

    #[tokio::test]
    async fn test_task_get_missing_id() {
        let tool = TaskGetTool;
        let input = serde_json::json!({});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await;

        assert!(result.is_err());
    }
}
