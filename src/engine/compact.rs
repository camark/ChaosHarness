//! Auto-compaction for long conversations

use super::messages::{ConversationMessage, MessageContent};

/// Estimated tokens per character (rough approximation)
const TOKENS_PER_CHAR: f32 = 0.25;

/// Target token limit for message history
const TARGET_TOKEN_LIMIT: u32 = 50000;

/// Maximum messages to keep in history
const MAX_MESSAGES: usize = 50;

/// Compact messages by removing old tool results and truncating history
pub fn compact_messages(messages: &[ConversationMessage]) -> Vec<ConversationMessage> {
    if messages.is_empty() {
        return Vec::new();
    }

    let estimated_tokens = estimate_tokens(messages);

    // If under limit, no compaction needed
    if estimated_tokens < TARGET_TOKEN_LIMIT && messages.len() <= MAX_MESSAGES {
        return messages.to_vec();
    }

    let mut result = Vec::new();

    // Strategy 1: Keep system/first message if it exists
    // Strategy 2: Keep last N messages
    // Strategy 3: Remove detailed tool results, keep summaries

    let keep_count = MAX_MESSAGES.min(messages.len());
    let skip_count = messages.len() - keep_count;

    for (_i, msg) in messages.iter().enumerate().skip(skip_count) {
        let compacted = compact_message(msg);
        result.push(compacted);
    }

    result
}

/// Compact a single message by truncating long content
fn compact_message(message: &ConversationMessage) -> ConversationMessage {
    let max_content_length = 2000; // Characters per content block

    let compacted_content: Vec<MessageContent> = message
        .content
        .iter()
        .map(|content| match content {
            MessageContent::Text { text } => {
                if text.len() > max_content_length {
                    MessageContent::Text {
                        text: format!(
                            "[truncated]...\n{}",
                            &text[text.len() - max_content_length..]
                        ),
                    }
                } else {
                    MessageContent::Text { text: text.clone() }
                }
            }
            MessageContent::ToolResult { tool_use_id, content, is_error } => {
                if content.len() > max_content_length {
                    MessageContent::ToolResult {
                        tool_use_id: tool_use_id.clone(),
                        content: format!(
                            "[truncated]...\n{}",
                            &content[content.len() - max_content_length..]
                        ),
                        is_error: *is_error,
                    }
                } else {
                    MessageContent::ToolResult {
                        tool_use_id: tool_use_id.clone(),
                        content: content.clone(),
                        is_error: *is_error,
                    }
                }
            }
            _ => content.clone(),
        })
        .collect();

    ConversationMessage {
        role: message.role.clone(),
        content: compacted_content,
        tool_uses: message.tool_uses.clone(),
    }
}

/// Estimate total tokens in messages
fn estimate_tokens(messages: &[ConversationMessage]) -> u32 {
    let total_chars: usize = messages
        .iter()
        .flat_map(|m| &m.content)
        .map(|c| match c {
            MessageContent::Text { text } => text.len(),
            MessageContent::ToolResult { content, .. } => content.len(),
            _ => 0,
        })
        .sum();

    // Add overhead for message structure
    let structural_tokens = messages.len() * 10;

    ((total_chars as f32 * TOKENS_PER_CHAR) as u32) + structural_tokens as u32
}

/// Check if compaction is needed and perform if necessary
pub fn auto_compact_if_needed(
    messages: Vec<ConversationMessage>,
) -> (Vec<ConversationMessage>, bool) {
    let estimated = estimate_tokens(&messages);

    if estimated < TARGET_TOKEN_LIMIT && messages.len() <= MAX_MESSAGES {
        return (messages, false);
    }

    let compacted = compact_messages(&messages);
    let was_compacted = compacted.len() < messages.len()
        || estimate_tokens(&compacted) < estimated;

    (compacted, was_compacted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compact_empty_messages() {
        let result = compact_messages(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_compact_under_limit() {
        let messages = vec![
            ConversationMessage::user_text("Hello"),
            ConversationMessage::assistant_text("Hi there!"),
        ];
        let result = compact_messages(&messages);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_estimate_tokens() {
        let messages = vec![
            ConversationMessage::user_text("Short message"),
        ];
        let tokens = estimate_tokens(&messages);
        assert!(tokens > 0);
    }
}
