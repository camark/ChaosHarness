//! TaskGet tool - Get task details

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use crate::services::task_manager::GLOBAL_TASK_MANAGER;
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

        let task = GLOBAL_TASK_MANAGER.get_task(task_id).await;

        match task {
            Some(task) => {
                let output = format!(
                    "Task: {} ({})\n\
                     Description: {}\n\
                     Status: {}\n\
                     Created: {}\n\
                     {}",
                    task.id,
                    task.task_type.as_str(),
                    task.description,
                    task.status.as_str(),
                    task.created_at.format("%Y-%m-%d %H:%M:%S UTC"),
                    if let Some(cmd) = &task.command {
                        format!("Command: {}", cmd)
                    } else if let Some(prompt) = &task.prompt {
                        format!("Prompt: {}", prompt)
                    } else {
                        String::new()
                    }
                );
                Ok(ToolResult::success(output))
            }
            None => Ok(ToolResult::error(format!("Task {} not found", task_id))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_task_get_existing() {
        let id = GLOBAL_TASK_MANAGER.create_bash_task("get test", "echo get").await;

        let tool = TaskGetTool;
        let input = serde_json::json!({"task_id": id});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("get test"));
    }

    #[tokio::test]
    async fn test_task_get_not_found() {
        let tool = TaskGetTool;
        let input = serde_json::json!({"task_id": "nonexistent"});
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
