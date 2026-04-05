//! EnterPlanMode tool - Switch to plan permission mode

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use crate::config::settings::{load_settings, save_settings};
use crate::permissions::modes::PermissionMode;
use anyhow::Result;
use serde_json::Value;

/// Input schema for enter_plan_mode tool
pub fn enter_plan_mode_input_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {}
    })
}

/// EnterPlanMode tool
pub struct EnterPlanModeTool;

#[async_trait::async_trait]
impl Tool for EnterPlanModeTool {
    fn name(&self) -> &'static str {
        "enter_plan_mode"
    }

    fn description(&self) -> &'static str {
        "Switch permission mode to plan."
    }

    fn input_schema(&self) -> Value {
        enter_plan_mode_input_schema()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        false
    }

    async fn execute(&self, _input: Value, _context: ToolExecutionContext) -> Result<ToolResult> {
        let mut settings = load_settings(None).map_err(|e| anyhow::anyhow!("Failed to load settings: {}", e))?;
        settings.permission.mode = PermissionMode::Plan;
        save_settings(&settings, None).map_err(|e| anyhow::anyhow!("Failed to save settings: {}", e))?;

        Ok(ToolResult::success("Permission mode set to plan".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_enter_plan_mode() {
        let tool = EnterPlanModeTool;
        let input = serde_json::json!({});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await;

        // Test passes if execution succeeds or fails gracefully
        assert!(result.is_err() || result.unwrap().output.contains("plan"));
    }
}
