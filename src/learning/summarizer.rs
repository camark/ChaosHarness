//! LLM-powered conversation summarization via SmartCompactor.
//!
//! Splits a long conversation into segments, summarizes older segments using an
//! LLM, stores the summaries in the KnowledgeStore, and returns a compacted
//! version of the message history.

use anyhow::{Context, Result};
use tracing::{debug, info};

use crate::api::client::{ApiClient, ApiRequest};
use crate::engine::messages::{ConversationMessage, MessageContent};
use crate::learning::bm25::Bm25Engine;
use crate::learning::store::KnowledgeStore;
use crate::learning::types::ConversationSummary;

/// Estimated tokens per character (rough approximation).
const TOKENS_PER_CHAR: f32 = 0.25;

/// Overhead tokens per message for structural framing.
const TOKENS_PER_MESSAGE_OVERHEAD: u32 = 10;

/// LLM-powered smart compactor that summarizes old conversation segments.
pub struct SmartCompactor {
    api_client: ApiClient,
    segment_size: usize,
    token_threshold: u32,
    model: String,
}

impl SmartCompactor {
    /// Create a new SmartCompactor.
    ///
    /// - `api_client`: The API client used to call the LLM for summarization.
    /// - `model`: The model identifier to use for summary requests.
    /// - `segment_size`: Number of messages per segment when splitting.
    /// - `token_threshold`: Token count above which compaction is triggered.
    pub fn new(
        api_client: ApiClient,
        model: String,
        segment_size: usize,
        token_threshold: u32,
    ) -> Self {
        Self {
            api_client,
            segment_size,
            token_threshold,
            model,
        }
    }

    /// Compact the message history if it exceeds the token threshold.
    ///
    /// Returns `(compacted_messages, was_compacted)`. When compaction occurs the
    /// older segments are summarized via LLM and their summaries are persisted
    /// in the `KnowledgeStore` and indexed in BM25.
    pub async fn compact_if_needed(
        &self,
        messages: Vec<ConversationMessage>,
        session_id: &str,
        store: &KnowledgeStore,
        bm25: &Bm25Engine,
    ) -> Result<(Vec<ConversationMessage>, bool)> {
        let estimated = estimate_tokens(&messages);

        // Under threshold and under 50 messages -- nothing to do.
        if estimated < self.token_threshold && messages.len() <= 50 {
            debug!(
                "No compaction needed: {} tokens < {} threshold, {} messages",
                estimated,
                self.token_threshold,
                messages.len()
            );
            return Ok((messages, false));
        }

        // Split into chunks of `segment_size`.
        let chunks: Vec<&[ConversationMessage]> =
            messages.chunks(self.segment_size).collect();

        // If only one chunk, nothing to compact.
        if chunks.len() <= 1 {
            debug!("Only one segment -- no compaction needed");
            return Ok((messages, false));
        }

        info!(
            "Compacting {} messages into {} segments ({} tokens)",
            messages.len(),
            chunks.len(),
            estimated
        );

        let mut compacted: Vec<ConversationMessage> = Vec::new();
        let mut msg_offset: usize = 0;

        // Summarize all segments except the last one.
        for (i, chunk) in chunks.iter().enumerate() {
            let chunk_end = msg_offset + chunk.len();

            if i < chunks.len() - 1 {
                // Summarize this old segment.
                let summary_text = self.summarize_segment(chunk).await?;

                // Persist summary in the knowledge store.
                let summary_record = ConversationSummary {
                    id: None,
                    session_id: session_id.to_string(),
                    summary: summary_text.clone(),
                    message_range_start: msg_offset as i64,
                    message_range_end: chunk_end as i64,
                    tokens_saved: estimate_tokens(chunk) as i64,
                    created_at: None,
                };

                let summary_id = store
                    .add_summary(&summary_record)
                    .context("Failed to store conversation summary")?;

                // Index the summary in BM25 for later retrieval.
                store
                    .index_summary_bm25(summary_id, &summary_text, bm25)
                    .context("Failed to index summary in BM25")?;

                // Replace the segment with a system-level summary message.
                compacted.push(ConversationMessage::user_text(
                    format!("[SESSION SUMMARY: {}]", summary_text),
                ));

                debug!(
                    "Segment {} (messages {}..{}) summarized and stored (id={})",
                    i, msg_offset, chunk_end, summary_id
                );
            } else {
                // Keep the last chunk verbatim.
                compacted.extend(chunk.to_vec());
                debug!(
                    "Segment {} (messages {}..{}) kept verbatim",
                    i, msg_offset, chunk_end
                );
            }

            msg_offset = chunk_end;
        }

        info!(
            "Compaction complete: {} messages -> {} messages",
            messages.len(),
            compacted.len()
        );

        Ok((compacted, true))
    }

    /// Call the LLM to produce a concise summary of a message segment.
    pub async fn summarize_segment(
        &self,
        messages: &[ConversationMessage],
    ) -> Result<String> {
        let messages_text = messages_to_text(messages);

        let prompt = format!(
            "Summarize this conversation segment concisely. Preserve:\n\
             - Key decisions made\n\
             - Files modified or created\n\
             - Bugs found and solutions\n\
             - User preferences expressed\n\
             - Technical facts learned\n\n\
             Conversation:\n{}\n\n\
             Summary:",
            messages_text
        );

        let request = ApiRequest {
            model: self.model.clone(),
            messages: vec![ConversationMessage::user_text(prompt)],
            system_prompt: None,
            max_tokens: 1024,
            tools: vec![],
        };

        let response = self
            .api_client
            .send_message(request)
            .await
            .context("LLM summarization request failed")?;

        // Extract the text from the response.
        let summary: String = response
            .content
            .iter()
            .filter_map(|c| match c {
                MessageContent::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");

        Ok(summary.trim().to_string())
    }
}

// ── Helper functions ─────────────────────────────────────────────────────

/// Convert a slice of `ConversationMessage`s into a readable text
/// representation, suitable for inclusion in an LLM prompt.
pub fn messages_to_text(messages: &[ConversationMessage]) -> String {
    let mut lines: Vec<String> = Vec::new();

    for msg in messages {
        let role_label = match msg.role {
            crate::engine::messages::MessageRole::User => "User",
            crate::engine::messages::MessageRole::Assistant => "Assistant",
        };

        for content in &msg.content {
            match content {
                MessageContent::Text { text } => {
                    lines.push(format!("{}: {}", role_label, text));
                }
                MessageContent::ToolUse { name, .. } => {
                    lines.push(format!("{}: [tool use: {}]", role_label, name));
                }
                MessageContent::ToolResult { content, is_error, .. } => {
                    let prefix = if *is_error { "tool error" } else { "tool result" };
                    let preview = if content.len() > 200 {
                        format!("{}...", &content[..200])
                    } else {
                        content.clone()
                    };
                    lines.push(format!("{}: [{}: {}]", role_label, prefix, preview));
                }
                MessageContent::Image { .. } => {
                    lines.push(format!("{}: [image]", role_label));
                }
            }
        }
    }

    lines.join("\n")
}

/// Rough token estimate for a slice of messages.
///
/// Uses `chars * 0.25 + 10` per message as a heuristic.
pub fn estimate_tokens(messages: &[ConversationMessage]) -> u32 {
    let total_chars: usize = messages
        .iter()
        .flat_map(|m| &m.content)
        .map(|c| match c {
            MessageContent::Text { text } => text.len(),
            MessageContent::ToolResult { content, .. } => content.len(),
            MessageContent::ToolUse { input, .. } => input.to_string().len(),
            MessageContent::Image { .. } => 0,
        })
        .sum();

    let structural_tokens = messages.len() as u32 * TOKENS_PER_MESSAGE_OVERHEAD;

    ((total_chars as f32 * TOKENS_PER_CHAR) as u32) + structural_tokens
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens() {
        let messages = vec![
            ConversationMessage::user_text("Hello, how are you?"),
            ConversationMessage::assistant_text("I'm doing well, thanks for asking!"),
        ];
        let tokens = estimate_tokens(&messages);
        assert!(tokens > 0, "Token estimate should be non-zero for non-empty messages");
        // 2 messages * 10 overhead = 20, plus ~16 chars * 0.25 = 4 => at least 24
        assert!(tokens >= 20, "Expected at least 20 tokens, got {}", tokens);
    }

    #[test]
    fn test_estimate_tokens_empty() {
        let messages: Vec<ConversationMessage> = vec![];
        let tokens = estimate_tokens(&messages);
        assert_eq!(tokens, 0);
    }

    #[test]
    fn test_messages_to_text_user_assistant() {
        let messages = vec![
            ConversationMessage::user_text("What is Rust?"),
            ConversationMessage::assistant_text("Rust is a systems programming language."),
        ];
        let text = messages_to_text(&messages);
        assert!(
            text.contains("User: What is Rust?"),
            "Expected 'User: ...' format, got: {}",
            text
        );
        assert!(
            text.contains("Assistant: Rust is a systems programming language."),
            "Expected 'Assistant: ...' format, got: {}",
            text
        );
    }

    #[test]
    fn test_messages_to_text_tool_use() {
        use serde_json::json;

        let messages = vec![ConversationMessage {
            role: crate::engine::messages::MessageRole::Assistant,
            content: vec![MessageContent::ToolUse {
                id: "t1".to_string(),
                name: "bash".to_string(),
                input: json!({"command": "ls"}),
            }],
            tool_uses: vec![],
        }];
        let text = messages_to_text(&messages);
        assert!(
            text.contains("[tool use: bash]"),
            "Expected tool use label, got: {}",
            text
        );
    }

    #[test]
    fn test_messages_to_text_tool_result() {
        let messages = vec![ConversationMessage {
            role: crate::engine::messages::MessageRole::User,
            content: vec![MessageContent::ToolResult {
                tool_use_id: "t1".to_string(),
                content: "file1.txt\nfile2.txt".to_string(),
                is_error: false,
            }],
            tool_uses: vec![],
        }];
        let text = messages_to_text(&messages);
        assert!(
            text.contains("[tool result:"),
            "Expected tool result label, got: {}",
            text
        );
    }
}
