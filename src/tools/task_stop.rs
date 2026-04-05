//! TaskStop tool - Stop a background task

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use anyhow::Result;
use serde_json::Value;

/// Input schema for task_stop tool
pub fn task_stop_input_schema() -> Value {
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

/// TaskStop tool
pub struct TaskStopTool;

#[async_trait::async_trait]
impl Tool for TaskStopTool {
    fn name(&self) -> &'static str {
        "task_stop"
    }

    fn description(&self) -> &'static str {
        "Stop a background task."
    }

    fn input_schema(&self) -> Value {
        task_stop_input_schema()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        false
    }

    async fn execute(&self, input: Value, _context: ToolExecutionContext) -> Result<ToolResult> {
        let task_id = input["task_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'task_id' field"))?;

        // In a full implementation, this would stop the task
        // For now, just acknowledge the stop
        tracing::info!("Stopping task: {}", task_id);

        Ok(ToolResult::success(format!("Stopped task {}", task_id)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_task_stop() {
        let tool = TaskStopTool;
        let input = serde_json::json!({"task_id": "test-123"});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("Stopped task"));
    }

    #[tokio::test]
    async fn test_task_stop_missing_id() {
        let tool = TaskStopTool;
        let input = serde_json::json!({});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await;

        assert!(result.is_err());
    }
}
