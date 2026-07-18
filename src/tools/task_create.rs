//! TaskCreate tool - Create a background task

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use crate::services::task_manager::GLOBAL_TASK_MANAGER;
use anyhow::Result;
use serde_json::Value;

/// Input schema for task_create tool
pub fn task_create_input_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "type": {
                "type": "string",
                "description": "Task type: local_bash or local_agent",
                "enum": ["local_bash", "local_agent"],
                "default": "local_bash"
            },
            "description": {
                "type": "string",
                "description": "Short task description"
            },
            "command": {
                "type": "string",
                "description": "Shell command for local_bash"
            },
            "prompt": {
                "type": "string",
                "description": "Prompt for local_agent"
            },
            "model": {
                "type": "string",
                "description": "Model for local_agent task"
            }
        },
        "required": ["description"]
    })
}

/// TaskCreate tool
pub struct TaskCreateTool;

#[async_trait::async_trait]
impl Tool for TaskCreateTool {
    fn name(&self) -> &'static str {
        "task_create"
    }

    fn description(&self) -> &'static str {
        "Create a background shell or local-agent task."
    }

    fn input_schema(&self) -> Value {
        task_create_input_schema()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        false
    }

    async fn execute(&self, input: Value, _context: ToolExecutionContext) -> Result<ToolResult> {
        let task_type = input["type"].as_str().unwrap_or("local_bash");
        let description = input["description"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'description' field"))?;

        match task_type {
            "local_bash" => {
                let command = input["command"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("'command' is required for local_bash tasks"))?;

                let id = GLOBAL_TASK_MANAGER.create_bash_task(description, command).await;

                Ok(ToolResult::success(format!(
                    "Created bash task {} (command: {})",
                    id, command
                )))
            }
            "local_agent" => {
                let prompt = input["prompt"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("'prompt' is required for local_agent tasks"))?;

                let model = input["model"].as_str().unwrap_or("claude-sonnet-4-6");

                let id = GLOBAL_TASK_MANAGER.create_agent_task(description, prompt, model).await;

                Ok(ToolResult::success(format!(
                    "Created agent task {} (model: {}, prompt: {})",
                    id, model, prompt
                )))
            }
            _ => Ok(ToolResult::error(format!(
                "Unsupported task type: {}",
                task_type
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_task_create_bash() {
        let tool = TaskCreateTool;
        let input = serde_json::json!({
            "type": "local_bash",
            "description": "Test bash task",
            "command": "echo hello"
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("Created bash task"));
    }

    #[tokio::test]
    async fn test_task_create_agent() {
        let tool = TaskCreateTool;
        let input = serde_json::json!({
            "type": "local_agent",
            "description": "Test agent task",
            "prompt": "Analyze the codebase"
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("Created agent task"));
    }

    #[tokio::test]
    async fn test_task_create_missing_command() {
        let tool = TaskCreateTool;
        let input = serde_json::json!({
            "type": "local_bash",
            "description": "Test task"
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await;

        assert!(result.is_err() || result.unwrap().is_error);
    }
}
