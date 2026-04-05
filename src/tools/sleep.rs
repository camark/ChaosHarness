//! Sleep tool - Pause execution for a specified duration

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use anyhow::Result;
use serde_json::Value;

/// Input schema for sleep tool
pub fn sleep_input_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "seconds": {
                "type": "number",
                "description": "Duration to sleep in seconds (0.1 to 30)",
                "default": 1.0,
                "minimum": 0.1,
                "maximum": 30.0
            }
        }
    })
}

/// Sleep tool
pub struct SleepTool;

#[async_trait::async_trait]
impl Tool for SleepTool {
    fn name(&self) -> &'static str {
        "sleep"
    }

    fn description(&self) -> &'static str {
        "Pause execution for a short duration (0.1 to 30 seconds)."
    }

    fn input_schema(&self) -> Value {
        sleep_input_schema()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        true
    }

    async fn execute(&self, input: Value, _context: ToolExecutionContext) -> Result<ToolResult> {
        let seconds = input["seconds"]
            .as_f64()
            .unwrap_or(1.0)
            .max(0.1)
            .min(30.0);

        tokio::time::sleep(tokio::time::Duration::from_secs_f64(seconds)).await;

        Ok(ToolResult::success(format!("Slept for {} seconds", seconds)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_sleep() {
        let tool = SleepTool;
        let input = serde_json::json!({"seconds": 0.1});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();
        assert!(!result.is_error);
        assert!(result.output.contains("Slept for"));
    }
}
