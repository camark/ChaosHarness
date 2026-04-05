//! AskUser tool - Interactive user prompts

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use anyhow::Result;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Input schema for ask user tool
pub fn ask_user_input_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "prompt": {
                "type": "string",
                "description": "Question or prompt to display to the user"
            },
            "default": {
                "type": "string",
                "description": "Default value if user presses Enter"
            }
        },
        "required": ["prompt"]
    })
}

/// Callback type for asking user questions
pub type AskUserCallback = Arc<Mutex<dyn Fn(&str) -> Result<String> + Send + Sync>>;

/// AskUser tool
pub struct AskUserTool {
    ask_user_callback: Option<AskUserCallback>,
}

impl AskUserTool {
    pub fn new(ask_user_callback: Option<AskUserCallback>) -> Self {
        Self { ask_user_callback }
    }
}

#[async_trait::async_trait]
impl Tool for AskUserTool {
    fn name(&self) -> &'static str {
        "ask_user"
    }

    fn description(&self) -> &'static str {
        "Ask the user a question and return their response. Use when you need clarification or want user input on a decision."
    }

    fn input_schema(&self) -> Value {
        ask_user_input_schema()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        true
    }

    async fn execute(&self, input: Value, _context: ToolExecutionContext) -> Result<ToolResult> {
        let prompt = input["prompt"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'prompt' field"))?
            .to_string();

        let default = input["default"].as_str().map(String::from);

        // Check if we have a callback to ask the user
        if let Some(callback) = &self.ask_user_callback {
            let callback = callback.clone();
            let response = tokio::task::spawn_blocking(move || {
                let callback = callback.lock();
                let callback = futures::executor::block_on(callback);
                callback(&prompt)
            })
            .await;

            match response {
                Ok(Ok(answer)) => {
                    return Ok(ToolResult::success(answer));
                }
                Ok(Err(e)) => {
                    return Ok(ToolResult::error(format!("Failed to get user response: {}", e)));
                }
                Err(e) => {
                    return Ok(ToolResult::error(format!("Task failed: {}", e)));
                }
            }
        }

        // Fallback: use default or return empty
        if let Some(default_value) = default {
            return Ok(ToolResult::success(default_value.to_string()));
        }

        Ok(ToolResult::success(String::new()))
    }
}

/// Simple blocking implementation for REPL
pub fn ask_user_blocking(prompt: &str, default: Option<&str>) -> Result<String> {
    use std::io::{self, Write};

    print!("{}", prompt);
    if let Some(d) = default {
        print!(" [{}]", d);
    }
    print!(": ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();

    if input.is_empty() {
        Ok(default.unwrap_or("").to_string())
    } else {
        Ok(input.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_ask_user_missing_prompt() {
        let tool = AskUserTool::new(None);
        let input = json!({});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await;

        // Should return error result wrapped in Ok
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing 'prompt'"));
    }

    #[tokio::test]
    async fn test_ask_user_with_default() {
        let tool = AskUserTool::new(None);
        let input = json!({
            "prompt": "What is your name?",
            "default": "Anonymous"
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        // Without a callback, should return default
        assert!(!result.is_error);
        assert_eq!(result.output, "Anonymous");
    }
}
