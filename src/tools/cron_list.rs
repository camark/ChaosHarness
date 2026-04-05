//! CronList tool - List all cron jobs

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use crate::services::cron::load_cron_jobs;
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

        let jobs = load_cron_jobs();

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
    async fn test_cron_list() {
        let tool = CronListTool;
        let input = serde_json::json!({});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        // Will show "No cron jobs configured" since we don't have any
        assert!(result.output.contains("cron"));
    }
}
