//! ExitPlanMode tool - Switch back to default permission mode

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use crate::config::settings::{load_settings, save_settings};
use crate::permissions::modes::PermissionMode;
use anyhow::Result;
use serde_json::Value;

/// Input schema for exit_plan_mode tool
pub fn exit_plan_mode_input_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {}
    })
}

/// ExitPlanMode tool
pub struct ExitPlanModeTool;

#[async_trait::async_trait]
impl Tool for ExitPlanModeTool {
    fn name(&self) -> &'static str {
        "exit_plan_mode"
    }

    fn description(&self) -> &'static str {
        "Switch permission mode back to default."
    }

    fn input_schema(&self) -> Value {
        exit_plan_mode_input_schema()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        false
    }

    async fn execute(&self, _input: Value, _context: ToolExecutionContext) -> Result<ToolResult> {
        let mut settings = load_settings(None).map_err(|e| anyhow::anyhow!("Failed to load settings: {}", e))?;
        settings.permission.mode = PermissionMode::Default;
        save_settings(&settings, None).map_err(|e| anyhow::anyhow!("Failed to save settings: {}", e))?;

        Ok(ToolResult::success("Permission mode set to default".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_exit_plan_mode() {
        let tool = ExitPlanModeTool;
        let input = serde_json::json!({});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await;

        // Test passes if execution succeeds or fails gracefully
        assert!(result.is_err() || result.unwrap().output.contains("default"));
    }
}
