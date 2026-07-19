//! TaskOutput tool - Read task output

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use crate::services::task_manager::GLOBAL_TASK_MANAGER;
use anyhow::Result;
use serde_json::Value;

/// Input schema for task_output tool
pub fn task_output_input_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "task_id": {
                "type": "string",
                "description": "Task identifier"
            },
            "max_bytes": {
                "type": "integer",
                "description": "Maximum bytes to read",
                "default": 12000,
                "minimum": 1,
                "maximum": 100000
            }
        },
        "required": ["task_id"]
    })
}

/// TaskOutput tool
pub struct TaskOutputTool;

#[async_trait::async_trait]
impl Tool for TaskOutputTool {
    fn name(&self) -> &'static str {
        "task_output"
    }

    fn description(&self) -> &'static str {
        "Read the output log for a background task."
    }

    fn input_schema(&self) -> Value {
        task_output_input_schema()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        true
    }

    async fn execute(&self, input: Value, _context: ToolExecutionContext) -> Result<ToolResult> {
        let task_id = input["task_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'task_id' field"))?;

        let max_bytes = input["max_bytes"].as_i64().unwrap_or(12000) as usize;

        // Validate max_bytes range
        if !(1..=100000).contains(&max_bytes) {
            return Ok(ToolResult::error(
                "max_bytes must be between 1 and 100000".to_string()
            ));
        }

        let output = GLOBAL_TASK_MANAGER.get_task_output(task_id, max_bytes).await;

        match output {
            Some(output) => {
                if output.is_empty() {
                    Ok(ToolResult::success(format!("(no output for task {})", task_id)))
                } else {
                    Ok(ToolResult::success(output))
                }
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
    async fn test_task_output_with_content() {
        let id = GLOBAL_TASK_MANAGER.create_bash_task("output test", "echo test_output_123").await;

        // Wait for task to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let tool = TaskOutputTool;
        let input = serde_json::json!({"task_id": id});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("test_output_123"));
    }

    #[tokio::test]
    async fn test_task_output_not_found() {
        let tool = TaskOutputTool;
        let input = serde_json::json!({"task_id": "nonexistent"});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(result.is_error);
        assert!(result.output.contains("not found"));
    }

    #[tokio::test]
    async fn test_task_output_invalid_max_bytes() {
        let tool = TaskOutputTool;
        let input = serde_json::json!({
            "task_id": "test-123",
            "max_bytes": 200000
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(result.is_error);
    }
}
