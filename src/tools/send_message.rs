//! SendMessage tool - Send a message to a user or agent

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use anyhow::Result;
use serde_json::Value;

/// Input schema for send_message tool
pub fn send_message_input_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "recipient": {
                "type": "string",
                "description": "Message recipient: 'user' or agent name",
                "enum": ["user"]
            },
            "message": {
                "type": "string",
                "description": "Message content"
            }
        },
        "required": ["recipient", "message"]
    })
}

/// SendMessage tool
pub struct SendMessageTool;

#[async_trait::async_trait]
impl Tool for SendMessageTool {
    fn name(&self) -> &'static str {
        "send_message"
    }

    fn description(&self) -> &'static str {
        "Send a message to a user or agent."
    }

    fn input_schema(&self) -> Value {
        send_message_input_schema()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        false
    }

    async fn execute(&self, input: Value, _context: ToolExecutionContext) -> Result<ToolResult> {
        let recipient = input["recipient"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'recipient' field"))?;

        let message = input["message"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'message' field"))?;

        // Log the message
        tracing::info!("Message to {}: {}", recipient, message);

        // For 'user' recipient, the message would be sent via the TUI
        // For agent recipients, it would be broadcast to the agent swarm
        let output = match recipient {
            "user" => format!("Message sent to user: {}", message),
            _ => format!("Message sent to {}: {}", recipient, message),
        };

        Ok(ToolResult::success(output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_send_message_to_user() {
        let tool = SendMessageTool;
        let input = serde_json::json!({
            "recipient": "user",
            "message": "Hello!"
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("Message sent to user"));
    }

    #[tokio::test]
    async fn test_send_message_missing_fields() {
        let tool = SendMessageTool;
        let input = serde_json::json!({});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await;

        assert!(result.is_err());
    }
}
