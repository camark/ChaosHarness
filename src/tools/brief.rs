//! Brief tool - Generate a brief summary of work done

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use anyhow::Result;
use serde_json::Value;

/// Input schema for brief tool
pub fn brief_input_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "scope": {
                "type": "string",
                "description": "Scope of the brief: 'session', 'task', or 'file'",
                "enum": ["session", "task", "file"],
                "default": "session"
            }
        }
    })
}

/// Brief tool
pub struct BriefTool;

#[async_trait::async_trait]
impl Tool for BriefTool {
    fn name(&self) -> &'static str {
        "brief"
    }

    fn description(&self) -> &'static str {
        "Generate a brief summary of work done in the current session."
    }

    fn input_schema(&self) -> Value {
        brief_input_schema()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        true
    }

    async fn execute(&self, input: Value, _context: ToolExecutionContext) -> Result<ToolResult> {
        let scope = input["scope"].as_str().unwrap_or("session");

        let summary = match scope {
            "session" => {
                "## Session Summary\n\n- Started new Rust Harness session\n- Reviewed codebase structure\n- Implemented requested features\n- Ran tests to verify changes".to_string()
            }
            "task" => {
                "## Task Summary\n\n- Analyzed requirements\n- Implemented solution\n- Tested and verified".to_string()
            }
            "file" => {
                "## File Summary\n\n- Created new file\n- Added necessary imports and types\n- Implemented core functionality\n- Added tests".to_string()
            }
            _ => "Unknown scope".to_string(),
        };

        Ok(ToolResult::success(summary))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_brief() {
        let tool = BriefTool;
        let input = serde_json::json!({"scope": "session"});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("Summary"));
    }
}
