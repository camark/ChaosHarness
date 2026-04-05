//! Conversation message types with tool support

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessageContent {
    Text { text: String },
    Image { source: ImageSource },
    ToolUse { id: String, name: String, input: Value },
    ToolResult { tool_use_id: String, content: String, is_error: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSource {
    #[serde(rename = "type")]
    pub source_type: String,
    pub media_type: String,
    pub data: String,
}

#[derive(Debug, Clone)]
pub struct ToolUse {
    pub id: String,
    pub name: String,
    pub input: Value,
}

#[derive(Debug, Clone)]
pub struct ToolResultBlock {
    pub tool_use_id: String,
    pub content: String,
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub role: MessageRole,
    #[serde(default)]
    pub content: Vec<MessageContent>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_uses: Vec<ToolUseData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUseData {
    pub id: String,
    pub name: String,
    pub input: Value,
}

impl ConversationMessage {
    pub fn user_text(text: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: vec![MessageContent::Text { text: text.into() }],
            tool_uses: Vec::new(),
        }
    }

    pub fn assistant_text(text: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: vec![MessageContent::Text { text: text.into() }],
            tool_uses: Vec::new(),
        }
    }

    pub fn tool_results(results: Vec<ToolResultBlock>) -> Self {
        Self {
            role: MessageRole::User,
            content: results
                .into_iter()
                .map(|r| MessageContent::ToolResult {
                    tool_use_id: r.tool_use_id,
                    content: r.content,
                    is_error: r.is_error,
                })
                .collect(),
            tool_uses: Vec::new(),
        }
    }

    pub fn with_tool_uses(mut self, tool_uses: Vec<ToolUseData>) -> Self {
        self.tool_uses = tool_uses;
        self
    }

    pub fn to_api_param(&self) -> Value {
        let mut param = json!({
            "role": self.role,
            "content": self.content.iter().map(|c| content_to_api_param(c)).collect::<Vec<_>>()
        });

        // For assistant messages with tool uses, add them to the content array
        if self.role == MessageRole::Assistant && !self.tool_uses.is_empty() {
            if let Some(content) = param["content"].as_array_mut() {
                for tool_use in &self.tool_uses {
                    content.push(json!({
                        "type": "tool_use",
                        "id": tool_use.id,
                        "name": tool_use.name,
                        "input": tool_use.input
                    }));
                }
            }
        }

        param
    }

    /// Get the primary text content of a message
    pub fn text(&self) -> String {
        self.content
            .iter()
            .filter_map(|c| match c {
                MessageContent::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }

    /// Extract tool uses from an assistant message
    pub fn extract_tool_uses(&self) -> Vec<ToolUse> {
        self.content
            .iter()
            .filter_map(|c| match c {
                MessageContent::ToolUse { id, name, input } => Some(ToolUse {
                    id: id.clone(),
                    name: name.clone(),
                    input: input.clone(),
                }),
                _ => None,
            })
            .collect()
    }
}

fn content_to_api_param(content: &MessageContent) -> Value {
    match content {
        MessageContent::Text { text } => {
            json!({"type": "text", "text": text})
        }
        MessageContent::Image { source } => {
            json!({
                "type": "image",
                "source": {
                    "type": source.source_type,
                    "media_type": source.media_type,
                    "data": source.data
                }
            })
        }
        MessageContent::ToolResult { tool_use_id, content, is_error } => {
            json!({
                "type": "tool_result",
                "tool_use_id": tool_use_id,
                "content": content,
                "is_error": is_error
            })
        }
        MessageContent::ToolUse { .. } => json!({}), // Handled separately
    }
}

/// Create an assistant message from an API response
pub fn assistant_message_from_api(
    text: &str,
    tool_uses: Vec<ToolUseData>,
) -> ConversationMessage {
    let mut content = Vec::new();
    if !text.is_empty() {
        content.push(MessageContent::Text { text: text.to_string() });
    }
    for tool_use in &tool_uses {
        content.push(MessageContent::ToolUse {
            id: tool_use.id.clone(),
            name: tool_use.name.clone(),
            input: tool_use.input.clone(),
        });
    }

    ConversationMessage {
        role: MessageRole::Assistant,
        content,
        tool_uses,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_message() {
        let msg = ConversationMessage::user_text("Hello");
        assert!(matches!(msg.role, MessageRole::User));
        assert_eq!(msg.text(), "Hello");
    }

    #[test]
    fn test_assistant_message() {
        let msg = ConversationMessage::assistant_text("Hi there!");
        assert!(matches!(msg.role, MessageRole::Assistant));
        assert_eq!(msg.text(), "Hi there!");
    }

    #[test]
    fn test_tool_results_message() {
        let results = vec![ToolResultBlock {
            tool_use_id: "tool_1".to_string(),
            content: "result".to_string(),
            is_error: false,
        }];
        let msg = ConversationMessage::tool_results(results);
        assert!(matches!(msg.role, MessageRole::User));
        assert!(msg.content.iter().any(|c| matches!(c, MessageContent::ToolResult { .. })));
    }
}
