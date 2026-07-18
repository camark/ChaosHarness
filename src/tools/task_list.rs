//! TaskList tool - List background tasks

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use crate::services::task_manager::GLOBAL_TASK_MANAGER;
use anyhow::Result;
use serde_json::Value;

/// Input schema for task_list tool
pub fn task_list_input_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "status": {
                "type": "string",
                "description": "Optional status filter: running, completed, stopped, failed",
                "enum": ["running", "completed", "stopped", "failed"]
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

        let tasks = GLOBAL_TASK_MANAGER.list_tasks(status).await;

        if tasks.is_empty() {
            let filter_msg = match status {
                Some(s) => format!(" with status '{}'", s),
                None => String::new(),
            };
            return Ok(ToolResult::success(format!("No tasks found{}", filter_msg)));
        }

        let mut output = String::new();
        for task in &tasks {
            output.push_str(&format!(
                "[{}] {} ({}) - {}\n",
                task.id,
                task.description,
                task.task_type.as_str(),
                task.status.as_str()
            ));
        }

        Ok(ToolResult::success(output.trim_end().to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_task_list_empty() {
        // Filter by a status that likely has no tasks
        let tool = TaskListTool;
        let input = serde_json::json!({"status": "stopped"});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        // Either "No tasks found" or some stopped tasks
        assert!(result.output.contains("No tasks found") || result.output.contains("stopped"));
    }

    #[tokio::test]
    async fn test_task_list_with_tasks() {
        // Create a task first
        let _ = GLOBAL_TASK_MANAGER.create_bash_task("test list", "echo listed").await;

        let tool = TaskListTool;
        let input = serde_json::json!({});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("test list"));
    }

    #[tokio::test]
    async fn test_task_list_with_status_filter() {
        let tool = TaskListTool;
        let input = serde_json::json!({"status": "running"});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
    }
}
