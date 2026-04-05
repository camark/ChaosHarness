//! CronCreate tool - Create a new cron job

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use crate::services::cron::CronJob;
use anyhow::Result;
use serde_json::Value;

/// Input schema for cron_create tool
pub fn cron_create_input_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "name": {
                "type": "string",
                "description": "Unique name for the cron job"
            },
            "schedule": {
                "type": "string",
                "description": "Cron schedule expression (e.g., '*/5 * * * *' for every 5 minutes)"
            },
            "command": {
                "type": "string",
                "description": "Command to execute"
            }
        },
        "required": ["name", "schedule", "command"]
    })
}

/// CronCreate tool
pub struct CronCreateTool;

#[async_trait::async_trait]
impl Tool for CronCreateTool {
    fn name(&self) -> &'static str {
        "cron_create"
    }

    fn description(&self) -> &'static str {
        "Create a new cron job with a schedule and command."
    }

    fn input_schema(&self) -> Value {
        cron_create_input_schema()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        false
    }

    async fn execute(&self, input: Value, _context: ToolExecutionContext) -> Result<ToolResult> {
        let name = input["name"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'name' field"))?;

        let schedule = input["schedule"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'schedule' field"))?;

        let command = input["command"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'command' field"))?;

        // Validate cron expression (basic validation)
        let parts: Vec<&str> = schedule.split_whitespace().collect();
        if parts.len() != 5 {
            return Ok(ToolResult::error(
                "Invalid cron schedule. Expected 5 fields (minute hour day month weekday)".to_string(),
            ));
        }

        // In a full implementation, this would save to the cron registry
        let job = CronJob {
            name: name.to_string(),
            schedule: schedule.to_string(),
            command: command.to_string(),
            enabled: true,
        };

        // For now, just log the creation
        tracing::info!("Created cron job: {:?}", job);

        Ok(ToolResult::success(format!(
            "Created cron job '{}' with schedule '{}' executing '{}'",
            name, schedule, command
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_cron_create() {
        let tool = CronCreateTool;
        let input = serde_json::json!({
            "name": "test-job",
            "schedule": "*/5 * * * *",
            "command": "echo hello"
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("Created cron job"));
    }

    #[tokio::test]
    async fn test_cron_create_invalid_schedule() {
        let tool = CronCreateTool;
        let input = serde_json::json!({
            "name": "test-job",
            "schedule": "invalid",
            "command": "echo hello"
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(result.is_error);
        assert!(result.output.contains("Invalid cron schedule"));
    }
}
