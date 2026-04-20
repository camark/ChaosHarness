//! Token estimation

#![allow(dead_code)]

pub fn estimate_tokens(text: &str) -> usize {
    // Rough estimation: ~4 characters per token for English
    text.len() / 4
}

pub fn estimate_message_tokens(messages: &[serde_json::Value]) -> usize {
    messages
        .iter()
        .map(|m| estimate_tokens(&m.to_string()))
        .sum()
}
