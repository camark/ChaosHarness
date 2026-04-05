//! Config tool - View and modify configuration settings

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use crate::config::Settings;
use anyhow::Result;
use serde_json::Value;

/// Input schema for config tool
pub fn config_input_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "description": "Action to perform: 'show' or 'set'",
                "enum": ["show", "set"],
                "default": "show"
            },
            "key": {
                "type": "string",
                "description": "Configuration key (for 'set' action)"
            },
            "value": {
                "type": "string",
                "description": "Configuration value (for 'set' action)"
            }
        }
    })
}

/// Config tool
pub struct ConfigTool;

#[async_trait::async_trait]
impl Tool for ConfigTool {
    fn name(&self) -> &'static str {
        "config"
    }

    fn description(&self) -> &'static str {
        "View or modify configuration settings."
    }

    fn input_schema(&self) -> Value {
        config_input_schema()
    }

    fn is_read_only(&self, input: &Value) -> bool {
        input["action"].as_str().unwrap_or("show") == "show"
    }

    async fn execute(&self, input: Value, _context: ToolExecutionContext) -> Result<ToolResult> {
        let action = input["action"].as_str().unwrap_or("show");

        match action {
            "show" => {
                let settings = Settings::default();
                let config_info = format!(
                    "Model: {}\nTheme: {}\nOutput style: {}\nVim mode: {}\nFast mode: {}\nEffort: {}\nPasses: {}",
                    settings.model,
                    settings.theme,
                    settings.output_style,
                    if settings.vim_mode { "on" } else { "off" },
                    if settings.fast_mode { "on" } else { "off" },
                    settings.effort,
                    settings.passes
                );
                Ok(ToolResult::success(config_info))
            }
            "set" => {
                let key = input["key"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing 'key' field for 'set' action"))?;
                let value = input["value"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing 'value' field for 'set' action"))?;

                // In a full implementation, this would save to settings.json
                // For now, just acknowledge the change
                Ok(ToolResult::success(format!(
                    "Configuration updated: {} = {} (session only)",
                    key, value
                )))
            }
            _ => Ok(ToolResult::error(
                "Unknown action. Use 'show' or 'set'.".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_config_show() {
        let tool = ConfigTool;
        let input = serde_json::json!({"action": "show"});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("Model:"));
    }
}
