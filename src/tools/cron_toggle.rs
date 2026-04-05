//! CronToggle tool - Enable or disable a cron job

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use crate::services::cron::set_job_enabled;
use anyhow::Result;
use serde_json::Value;

/// Input schema for cron_toggle tool
pub fn cron_toggle_input_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "name": {
                "type": "string",
                "description": "Name of the cron job to toggle"
            },
            "enabled": {
                "type": "boolean",
                "description": "Whether to enable or disable the job"
            }
        },
        "required": ["name", "enabled"]
    })
}

/// CronToggle tool
pub struct CronToggleTool;

#[async_trait::async_trait]
impl Tool for CronToggleTool {
    fn name(&self) -> &'static str {
        "cron_toggle"
    }

    fn description(&self) -> &'static str {
        "Enable or disable a cron job."
    }

    fn input_schema(&self) -> Value {
        cron_toggle_input_schema()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        false
    }

    async fn execute(&self, input: Value, _context: ToolExecutionContext) -> Result<ToolResult> {
        let name = input["name"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'name' field"))?;

        let enabled = input["enabled"]
            .as_bool()
            .ok_or_else(|| anyhow::anyhow!("Missing 'enabled' field"))?;

        let action = if enabled { "enabled" } else { "disabled" };

        // In a full implementation, this would update the cron registry
        let success = set_job_enabled(name, enabled);

        if success {
            Ok(ToolResult::success(format!(
                "Cron job '{}' {}",
                name, action
            )))
        } else {
            Ok(ToolResult::error(format!(
                "Failed to {} cron job '{}'",
                if enabled { "enable" } else { "disable" },
                name
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_cron_toggle_enable() {
        let tool = CronToggleTool;
        let input = serde_json::json!({"name": "test-job", "enabled": true});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("enabled"));
    }

    #[tokio::test]
    async fn test_cron_toggle_disable() {
        let tool = CronToggleTool;
        let input = serde_json::json!({"name": "test-job", "enabled": false});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("disabled"));
    }
}
