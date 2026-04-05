//! TaskUpdate tool - Update task metadata

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use anyhow::Result;
use serde_json::Value;

/// Input schema for task_update tool
pub fn task_update_input_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "task_id": {
                "type": "string",
                "description": "Task identifier"
            },
            "description": {
                "type": "string",
                "description": "Updated task description"
            },
            "progress": {
                "type": "integer",
                "description": "Progress percentage (0-100)",
                "minimum": 0,
                "maximum": 100
            },
            "status_note": {
                "type": "string",
                "description": "Short human-readable task note"
            }
        },
        "required": ["task_id"]
    })
}

/// TaskUpdate tool
pub struct TaskUpdateTool;

#[async_trait::async_trait]
impl Tool for TaskUpdateTool {
    fn name(&self) -> &'static str {
        "task_update"
    }

    fn description(&self) -> &'static str {
        "Update a task description, progress, or status note."
    }

    fn input_schema(&self) -> Value {
        task_update_input_schema()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        false
    }

    async fn execute(&self, input: Value, _context: ToolExecutionContext) -> Result<ToolResult> {
        let task_id = input["task_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'task_id' field"))?;

        let description = input["description"].as_str();
        let progress = input["progress"].as_i64();
        let status_note = input["status_note"].as_str();

        // Validate progress range if provided
        if let Some(p) = progress {
            if p < 0 || p > 100 {
                return Ok(ToolResult::error(
                    "Progress must be between 0 and 100".to_string()
                ));
            }
        }

        // In a full implementation, this would update the task
        // For now, just acknowledge the update
        let mut updates = Vec::new();
        if let Some(desc) = description {
            updates.push(format!("description={}", desc));
        }
        if let Some(p) = progress {
            updates.push(format!("progress={}%", p));
        }
        if let Some(note) = status_note {
            updates.push(format!("note={}", note));
        }

        let output = if updates.is_empty() {
            format!("No updates specified for task {}", task_id)
        } else {
            format!("Updated task {} {}", task_id, updates.join(" "))
        };

        Ok(ToolResult::success(output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_task_update_description() {
        let tool = TaskUpdateTool;
        let input = serde_json::json!({
            "task_id": "test-123",
            "description": "New description"
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("description=New description"));
    }

    #[tokio::test]
    async fn test_task_update_progress() {
        let tool = TaskUpdateTool;
        let input = serde_json::json!({
            "task_id": "test-123",
            "progress": 50
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("progress=50%"));
    }

    #[tokio::test]
    async fn test_task_update_invalid_progress() {
        let tool = TaskUpdateTool;
        let input = serde_json::json!({
            "task_id": "test-123",
            "progress": 150
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(result.is_error);
        assert!(result.output.contains("0 and 100"));
    }
}
