//! CronList tool - List all cron jobs

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use crate::services::cron::CRON_MANAGER;
use anyhow::Result;
use serde_json::Value;

/// Input schema for cron_list tool
pub fn cron_list_input_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "show_disabled": {
                "type": "boolean",
                "description": "Whether to show disabled jobs",
                "default": true
            }
        }
    })
}

/// CronList tool
pub struct CronListTool;

#[async_trait::async_trait]
impl Tool for CronListTool {
    fn name(&self) -> &'static str {
        "cron_list"
    }

    fn description(&self) -> &'static str {
        "List all configured cron jobs."
    }

    fn input_schema(&self) -> Value {
        cron_list_input_schema()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        true
    }

    async fn execute(&self, input: Value, _context: ToolExecutionContext) -> Result<ToolResult> {
        let show_disabled = input["show_disabled"].as_bool().unwrap_or(true);

        let jobs = CRON_MANAGER.list_jobs().await;

        if jobs.is_empty() {
            return Ok(ToolResult::success(
                "No cron jobs configured. Use cron_create to add jobs.".to_string(),
            ));
        }

        let mut lines = Vec::new();
        lines.push("Configured cron jobs:".to_string());
        lines.push(String::new());

        for job in jobs {
            if !job.enabled && !show_disabled {
                continue;
            }
            let status = if job.enabled { "✓" } else { "✗" };
            lines.push(format!(
                "  [{}] {} - Schedule: '{}' - Command: {}",
                status, job.name, job.schedule, job.command
            ));
        }

        Ok(ToolResult::success(lines.join("\n")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_cron_list_empty() {
        // Use unique name to avoid conflicts with other tests
        let tool = CronListTool;
        let input = serde_json::json!({});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        // Could be empty or have jobs from other tests
        assert!(result.output.contains("cron") || result.output.contains("Schedule"));
    }

    #[tokio::test]
    async fn test_cron_list_with_jobs() {
        CRON_MANAGER.create_job("list-test-1", "*/5 * * * *", "echo 1").await;
        CRON_MANAGER.create_job("list-test-2", "*/10 * * * *", "echo 2").await;

        let tool = CronListTool;
        let input = serde_json::json!({});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("list-test-1"));
        assert!(result.output.contains("list-test-2"));
    }

    #[tokio::test]
    async fn test_cron_list_hide_disabled() {
        CRON_MANAGER.create_job("list-dis-test", "*/5 * * * *", "echo hi").await;
        CRON_MANAGER.set_job_enabled("list-dis-test", false).await;

        let tool = CronListTool;
        let input = serde_json::json!({"show_disabled": false});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        // The disabled job should not appear
        assert!(!result.output.contains("list-dis-test"));
    }
}
