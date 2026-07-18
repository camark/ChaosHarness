//! TaskStop tool - Stop a running task

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use crate::services::task_manager::GLOBAL_TASK_MANAGER;
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
        "Stop a running background task."
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

        let stopped = GLOBAL_TASK_MANAGER.stop_task(task_id).await;

        if stopped {
            Ok(ToolResult::success(format!("Stopped task {}", task_id)))
        } else {
            // Check if task exists
            let task = GLOBAL_TASK_MANAGER.get_task(task_id).await;
            match task {
                Some(task) => Ok(ToolResult::error(format!(
                    "Task {} is not running (status: {})",
                    task_id,
                    task.status.as_str()
                ))),
                None => Ok(ToolResult::error(format!("Task {} not found", task_id))),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_task_stop_running() {
        let id = GLOBAL_TASK_MANAGER.create_bash_task("stop test", "sleep 100").await;

        let tool = TaskStopTool;
        let input = serde_json::json!({"task_id": id});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("Stopped"));
    }

    #[tokio::test]
    async fn test_task_stop_completed() {
        let id = GLOBAL_TASK_MANAGER.create_bash_task("completed", "echo done").await;

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let tool = TaskStopTool;
        let input = serde_json::json!({"task_id": id});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(result.is_error);
        assert!(result.output.contains("not running"));
    }

    #[tokio::test]
    async fn test_task_stop_not_found() {
        let tool = TaskStopTool;
        let input = serde_json::json!({"task_id": "nonexistent"});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(result.is_error);
        assert!(result.output.contains("not found"));
    }
}
