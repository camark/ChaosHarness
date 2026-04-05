//! RemoteTrigger tool - Trigger a cron job immediately

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
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
        if timeout_seconds < 1 || timeout_seconds > 600 {
            return Ok(ToolResult::error(
                "timeout_seconds must be between 1 and 600".to_string()
            ));
        }

        // In a full implementation, this would look up the cron job
        // and execute its command
        tracing::info!(
            "Triggering cron job '{}' with timeout {}s",
            name,
            timeout_seconds
        );

        // For now, return a placeholder response
        Ok(ToolResult::success(format!(
            "Triggered {} (not implemented - job not found)",
            name
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_remote_trigger() {
        let tool = RemoteTriggerTool;
        let input = serde_json::json!({
            "name": "test-job"
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("Triggered"));
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
