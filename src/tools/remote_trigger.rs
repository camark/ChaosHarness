//! RemoteTrigger tool - Trigger a cron job immediately

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use crate::services::cron::CRON_MANAGER;
use crate::services::task_manager::GLOBAL_TASK_MANAGER;
use anyhow::Result;
use serde_json::Value;

/// Input schema for remote_trigger tool
pub fn remote_trigger_input_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "name": {
                "type": "string",
                "description": "Cron job name"
            },
            "timeout_seconds": {
                "type": "integer",
                "description": "Execution timeout in seconds",
                "default": 120,
                "minimum": 1,
                "maximum": 600
            }
        },
        "required": ["name"]
    })
}

/// RemoteTrigger tool
pub struct RemoteTriggerTool;

#[async_trait::async_trait]
impl Tool for RemoteTriggerTool {
    fn name(&self) -> &'static str {
        "remote_trigger"
    }

    fn description(&self) -> &'static str {
        "Trigger a configured local cron-style job immediately."
    }

    fn input_schema(&self) -> Value {
        remote_trigger_input_schema()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        false
    }

    async fn execute(&self, input: Value, _context: ToolExecutionContext) -> Result<ToolResult> {
        let name = input["name"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'name' field"))?;

        let timeout_seconds = input["timeout_seconds"].as_i64().unwrap_or(120);

        // Validate timeout range
        if !(1..=600).contains(&timeout_seconds) {
            return Ok(ToolResult::error(
                "timeout_seconds must be between 1 and 600".to_string()
            ));
        }

        // Look up the cron job
        let job = CRON_MANAGER.get_job(name).await;
        let job = match job {
            Some(j) => j,
            None => return Ok(ToolResult::error(format!("Cron job '{}' not found", name))),
        };

        // Create a background task to execute the command
        let task_id = GLOBAL_TASK_MANAGER.create_bash_task(
            &format!("Triggered: {}", name),
            &job.command,
        ).await;

        Ok(ToolResult::success(format!(
            "Triggered cron job '{}' as task {} (command: {}, timeout: {}s)",
            name, task_id, job.command, timeout_seconds
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_remote_trigger_existing_job() {
        // Create a cron job first
        CRON_MANAGER.create_job("trigger-test", "*/5 * * * *", "echo triggered").await;

        let tool = RemoteTriggerTool;
        let input = serde_json::json!({
            "name": "trigger-test"
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("Triggered cron job"));
        assert!(result.output.contains("task"));
    }

    #[tokio::test]
    async fn test_remote_trigger_not_found() {
        let tool = RemoteTriggerTool;
        let input = serde_json::json!({
            "name": "nonexistent-job"
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(result.is_error);
        assert!(result.output.contains("not found"));
    }

    #[tokio::test]
    async fn test_remote_trigger_invalid_timeout() {
        let tool = RemoteTriggerTool;
        let input = serde_json::json!({
            "name": "test-job",
            "timeout_seconds": 1000
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(result.is_error);
        assert!(result.output.contains("1 and 600"));
    }
}
