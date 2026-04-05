//! TaskList tool - List background tasks

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use anyhow::Result;
use serde_json::Value;

/// Input schema for task_list tool
pub fn task_list_input_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "status": {
                "type": "string",
                "description": "Optional status filter: running, completed, stopped",
                "enum": ["running", "completed", "stopped"]
            }
        }
    })
}

/// TaskList tool
pub struct TaskListTool;

#[async_trait::async_trait]
impl Tool for TaskListTool {
    fn name(&self) -> &'static str {
        "task_list"
    }

    fn description(&self) -> &'static str {
        "List background tasks."
    }

    fn input_schema(&self) -> Value {
        task_list_input_schema()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        true
    }

    async fn execute(&self, input: Value, _context: ToolExecutionContext) -> Result<ToolResult> {
        let status = input["status"].as_str();

        // In a full implementation, this would query the task manager
        // For now, return a placeholder response
        let status_filter = match status {
            Some("running") => " (running only)",
            Some("completed") => " (completed only)",
            Some("stopped") => " (stopped only)",
            _ => ""
        };

        let output = format!("No tasks found{}", status_filter);

        Ok(ToolResult::success(output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_task_list() {
        let tool = TaskListTool;
        let input = serde_json::json!({});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
    }

    #[tokio::test]
    async fn test_task_list_with_status() {
        let tool = TaskListTool;
        let input = serde_json::json!({"status": "running"});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("running"));
    }
}
