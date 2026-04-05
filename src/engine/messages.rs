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

    /// Create separate tool result messages for OpenAI format
    /// Each tool result is a separate message with role "tool"
    pub fn tool_results_openai(results: Vec<ToolResultBlock>) -> Vec<Self> {
        results
            .into_iter()
            .map(|r| Self {
                role: MessageRole::User, // Will be converted to "tool" by to_openai_api_param
                content: vec![MessageContent::ToolResult {
                    tool_use_id: r.tool_use_id,
                    content: r.content,
                    is_error: r.is_error,
                }],
                tool_uses: Vec::new(),
            })
            .collect()
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

    /// Convert message to OpenAI API format
    pub fn to_openai_api_param(&self) -> Value {
        let mut content_array: Vec<Value> = Vec::new();
        let mut tool_calls: Vec<Value> = Vec::new();

        for c in &self.content {
            match c {
                MessageContent::Text { text } => {
                    content_array.push(json!({"type": "text", "text": text}));
                }
                MessageContent::ToolUse { id, name, input } => {
                    tool_calls.push(json!({
                        "id": id,
                        "type": "function",
                        "function": {
                            "name": name,
                            "arguments": input.to_string()
                        }
                    }));
                }
                MessageContent::ToolResult { tool_use_id, content, is_error } => {
                    // OpenAI format: tool_result goes in content as text
                    content_array.push(json!({
                        "type": "text",
                        "text": content
                    }));
                }
                MessageContent::Image { source } => {
                    content_array.push(json!({
                        "type": "image_url",
                        "image_url": {
                            "url": format!("data:{};base64,{}", source.media_type, source.data)
                        }
                    }));
                }
            }
        }

        let mut param = json!({
            "role": self.role,
        });

        // OpenAI format: tool_calls is a separate field, not in content
        if !tool_calls.is_empty() {
            param["tool_calls"] = json!(tool_calls);
        }

        // Content: for assistant messages with tool_calls, content can be empty string or null
        // For user messages with tool results, content is the text
        if self.role == MessageRole::Assistant && !tool_calls.is_empty() {
            param["content"] = Value::Null;
            // Moonshot K2.5 requires reasoning_content field
            param["reasoning_content"] = json!("Analyzing tool requests...");
        } else {
            param["content"] = json!(content_array);
        }

        // OpenAI format: tool response messages have role "tool" and include tool_call_id
        if self.role == MessageRole::User && !self.content.is_empty() {
            // Check if this is a tool result message
            let is_tool_result = self.content.iter().any(|c| matches!(c, MessageContent::ToolResult { .. }));
            if is_tool_result && self.content.len() == 1 {
                // Single tool result - use OpenAI tool format
                if let MessageContent::ToolResult { tool_use_id, content, is_error } = &self.content[0] {
                    param["role"] = json!("tool");
                    param["tool_call_id"] = json!(tool_use_id);
                    param["content"] = json!(content);
                }
            } else if is_tool_result {
                // Multiple tool results - concatenate them
                let combined_content: String = self.content.iter()
                    .filter_map(|c| match c {
                        MessageContent::ToolResult { content, .. } => Some(content.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n---\n");
                param["content"] = json!(combined_content);
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
