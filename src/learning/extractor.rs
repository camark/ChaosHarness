//! Knowledge and pattern extraction engine.
//!
//! Parses in-conversation `<!-- LEARN: ... -->` markers and optionally uses an
//! LLM to extract knowledge entries and behavioural patterns from a session.

use anyhow::{Context, Result};
use regex::Regex;
use serde_json::Value;

use crate::api::client::{ApiClient, ApiRequest};
use crate::engine::messages::ConversationMessage;
use crate::learning::bm25::Bm25Engine;
use crate::learning::store::KnowledgeStore;
use crate::learning::types::{
    ExtractedItem, KnowledgeCategory, KnowledgeEntry, LearningMarker, LearningResult, Pattern,
    PatternType,
};

/// Core learning engine that orchestrates knowledge and pattern extraction.
pub struct LearningEngine {
    store: KnowledgeStore,
    bm25: Bm25Engine,
    api_client: Option<ApiClient>,
    model: String,
    use_llm_extraction: bool,
}

impl LearningEngine {
    /// Create a new `LearningEngine`.
    pub fn new(
        store: KnowledgeStore,
        bm25_k1: f64,
        bm25_b: f64,
        api_client: Option<ApiClient>,
        model: String,
        use_llm_extraction: bool,
    ) -> Self {
        Self {
            store,
            bm25: Bm25Engine::with_params(bm25_k1, bm25_b),
            api_client,
            model,
            use_llm_extraction,
        }
    }

    /// Main entry point: process a conversation session and extract knowledge.
    pub fn process_session(
        &self,
        messages: &[ConversationMessage],
        session_id: &str,
    ) -> Result<LearningResult> {
        let mut result = LearningResult::default();

        // Step 1: Parse in-conversation markers.
        let markers = self.parse_markers(messages);
        for marker in &markers {
            if let Some(entry) = marker_to_knowledge(marker, session_id) {
                let similar = self.store.find_similar_knowledge(&entry.topic, &entry.content)?;
                if let Some(existing) = similar {
                    // Boost existing knowledge confidence.
                    if let Some(id) = existing.id {
                        self.store.boost_knowledge_confidence(id, 0.1)?;
                    }
                } else {
                    // Add new knowledge entry and index it in BM25.
                    let id = self.store.add_knowledge(&entry)?;
                    let text = format!("{} {}", entry.topic, entry.content);
                    self.store.index_knowledge_bm25(id, &text, &self.bm25)?;
                }
                result.knowledge_extracted += 1;
            }
        }

        // Step 2: Optional LLM-based extraction.
        if self.use_llm_extraction {
            if let Some(_client) = &self.api_client {
                match self.llm_extract(messages) {
                    Ok(items) => {
                        for item in items {
                            match item {
                                ExtractedItem::Knowledge(entry) => {
                                    let similar = self
                                        .store
                                        .find_similar_knowledge(&entry.topic, &entry.content)?;
                                    if let Some(existing) = similar {
                                        if let Some(id) = existing.id {
                                            self.store.boost_knowledge_confidence(id, 0.1)?;
                                        }
                                    } else {
                                        let id = self.store.add_knowledge(&entry)?;
                                        let text = format!("{} {}", entry.topic, entry.content);
                                        self.store.index_knowledge_bm25(
                                            id, &text, &self.bm25,
                                        )?;
                                    }
                                    result.knowledge_extracted += 1;
                                }
                                ExtractedItem::Pattern(pattern) => {
                                    let id = self.store.add_pattern(&pattern)?;
                                    let text = format!(
                                        "{} {}",
                                        pattern.description,
                                        pattern.example.as_deref().unwrap_or("")
                                    );
                                    self.store.index_pattern_bm25(id, &text, &self.bm25)?;
                                    result.patterns_extracted += 1;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("LLM extraction failed: {}", e);
                    }
                }
            }
        }

        Ok(result)
    }

    /// Parse `<!-- LEARN: ... -->` markers from conversation messages.
    pub fn parse_markers(&self, messages: &[ConversationMessage]) -> Vec<LearningMarker> {
        let re = Regex::new(
            r#"<!--\s*LEARN:\s*category="([^"]+)"\s*topic="([^"]+)"\s*content="([^"]+)"\s*-->"#,
        )
        .expect("invalid marker regex");

        let mut markers = Vec::new();
        for msg in messages {
            let text = msg.text();
            for cap in re.captures_iter(&text) {
                markers.push(LearningMarker {
                    category: cap[1].to_string(),
                    topic: cap[2].to_string(),
                    content: cap[3].to_string(),
                });
            }
        }
        markers
    }

    /// Use an LLM to extract knowledge and patterns from the conversation.
    pub fn llm_extract(&self, messages: &[ConversationMessage]) -> Result<Vec<ExtractedItem>> {
        let client = self
            .api_client
            .as_ref()
            .context("No API client configured for LLM extraction")?;

        let conversation_text = messages_to_text(messages);

        let prompt = format!(
            "Analyze this conversation and extract:\n\
             1. Knowledge facts (technical decisions, solutions, preferences)\n\
             2. Behavioral patterns (recurring workflows, tool preferences)\n\n\
             For each item, output a JSON line:\n\
             {{\"type\": \"knowledge\", \"category\": \"fact|decision|solution|preference\", \
             \"topic\": \"...\", \"content\": \"...\"}}\n\
             {{\"type\": \"pattern\", \"pattern_type\": \"coding_style|workflow|tool_preference\", \
             \"description\": \"...\", \"example\": \"...\"}}\n\n\
             Only extract genuinely useful information. Output one JSON object per line, \
             no other text.\n\nConversation:\n{}",
            conversation_text
        );

        let request = ApiRequest {
            model: self.model.clone(),
            messages: vec![ConversationMessage::user_text(prompt)],
            system_prompt: None,
            max_tokens: 4096,
            tools: vec![],
        };

        // This is a blocking context — we use a tokio runtime handle to drive the
        // async client. If we are already inside a tokio runtime we use
        // `Handle::current().block_on()`, otherwise we create a new one.
        let response = tokio::task::block_in_place(|| {
            let handle = tokio::runtime::Handle::current();
            handle.block_on(async { client.send_message(request).await })
        })
        .map_err(|e| anyhow::anyhow!("LLM request failed: {}", e))?;

        // Extract text from response content blocks.
        let response_text: String = response
            .content
            .iter()
            .filter_map(|c| match c {
                crate::engine::messages::MessageContent::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");

        let mut items = Vec::new();
        for line in response_text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
                if let Some(item) = parse_extracted_item(&value) {
                    items.push(item);
                }
            }
        }

        Ok(items)
    }
}

/// Convert a `LearningMarker` to a `KnowledgeEntry`.
pub fn marker_to_knowledge(marker: &LearningMarker, session_id: &str) -> Option<KnowledgeEntry> {
    let category = KnowledgeCategory::from_str(&marker.category);
    Some(KnowledgeEntry {
        id: None,
        category,
        topic: marker.topic.clone(),
        content: marker.content.clone(),
        source_session_id: Some(session_id.to_string()),
        confidence: 0.7,
        access_count: 0,
        created_at: None,
        last_accessed: None,
    })
}

/// Parse a JSON value into an `ExtractedItem` (knowledge or pattern).
pub fn parse_extracted_item(value: &Value) -> Option<ExtractedItem> {
    let item_type = value.get("type")?.as_str()?;

    match item_type {
        "knowledge" => {
            let category = KnowledgeCategory::from_str(
                value.get("category")?.as_str().unwrap_or("fact"),
            );
            Some(ExtractedItem::Knowledge(KnowledgeEntry {
                id: None,
                category,
                topic: value.get("topic")?.as_str()?.to_string(),
                content: value.get("content")?.as_str()?.to_string(),
                source_session_id: None,
                confidence: 0.6,
                access_count: 0,
                created_at: None,
                last_accessed: None,
            }))
        }
        "pattern" => {
            let pattern_type = PatternType::from_str(
                value.get("pattern_type")?.as_str().unwrap_or("workflow"),
            );
            Some(ExtractedItem::Pattern(Pattern {
                id: None,
                pattern_type,
                description: value.get("description")?.as_str()?.to_string(),
                example: value
                    .get("example")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                frequency: 1,
                created_at: None,
                last_seen: None,
            }))
        }
        _ => None,
    }
}

/// Flatten conversation messages into a single text block.
pub fn messages_to_text(messages: &[ConversationMessage]) -> String {
    messages
        .iter()
        .map(|msg| {
            let role = match msg.role {
                crate::engine::messages::MessageRole::User => "User",
                crate::engine::messages::MessageRole::Assistant => "Assistant",
            };
            format!("{}: {}", role, msg.text())
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::learning::types::{KnowledgeCategory, PatternType};

    fn make_engine() -> LearningEngine {
        let store = KnowledgeStore::new_in_memory().expect("failed to create store");
        LearningEngine::new(store, 1.2, 0.75, None, "test-model".to_string(), false)
    }

    #[test]
    fn test_parse_markers() {
        let engine = make_engine();
        let messages = vec![ConversationMessage::user_text(
            "Some text <!-- LEARN: category=\"fact\" topic=\"rust\" content=\"Rust is fast\" --> after",
        )];
        let markers = engine.parse_markers(&messages);
        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].category, "fact");
        assert_eq!(markers[0].topic, "rust");
        assert_eq!(markers[0].content, "Rust is fast");
    }

    #[test]
    fn test_parse_multiple_markers() {
        let engine = make_engine();
        let messages = vec![ConversationMessage::user_text(
            "<!-- LEARN: category=\"fact\" topic=\"a\" content=\"first\" --> middle <!-- LEARN: category=\"decision\" topic=\"b\" content=\"second\" -->",
        )];
        let markers = engine.parse_markers(&messages);
        assert_eq!(markers.len(), 2);
        assert_eq!(markers[0].topic, "a");
        assert_eq!(markers[1].category, "decision");
        assert_eq!(markers[1].content, "second");
    }

    #[test]
    fn test_parse_no_markers() {
        let engine = make_engine();
        let messages = vec![
            ConversationMessage::user_text("Just a normal message"),
            ConversationMessage::assistant_text("Nothing special here"),
        ];
        let markers = engine.parse_markers(&messages);
        assert!(markers.is_empty());
    }

    #[test]
    fn test_marker_to_knowledge() {
        let marker = LearningMarker {
            category: "solution".to_string(),
            topic: "borrow checker".to_string(),
            content: "Use lifetime annotations".to_string(),
        };
        let entry = marker_to_knowledge(&marker, "sess-42").expect("should produce entry");
        assert!(matches!(entry.category, KnowledgeCategory::Solution));
        assert_eq!(entry.topic, "borrow checker");
        assert_eq!(entry.content, "Use lifetime annotations");
        assert_eq!(entry.source_session_id.as_deref(), Some("sess-42"));
        assert!((entry.confidence - 0.7).abs() < f64::EPSILON);
        assert!(entry.id.is_none());
    }

    #[test]
    fn test_parse_extracted_item_knowledge() {
        let json = serde_json::json!({
            "type": "knowledge",
            "category": "preference",
            "topic": "editor",
            "content": "Prefers vim keybindings"
        });
        let item = parse_extracted_item(&json).expect("should parse");
        match item {
            ExtractedItem::Knowledge(entry) => {
                assert!(matches!(entry.category, KnowledgeCategory::Preference));
                assert_eq!(entry.topic, "editor");
                assert_eq!(entry.content, "Prefers vim keybindings");
            }
            _ => panic!("Expected Knowledge variant"),
        }
    }

    #[test]
    fn test_parse_extracted_item_pattern() {
        let json = serde_json::json!({
            "type": "pattern",
            "pattern_type": "coding_style",
            "description": "Uses iterator chains",
            "example": "vec.iter().filter(|x| x > 0).collect()"
        });
        let item = parse_extracted_item(&json).expect("should parse");
        match item {
            ExtractedItem::Pattern(pattern) => {
                assert!(matches!(pattern.pattern_type, PatternType::CodingStyle));
                assert_eq!(pattern.description, "Uses iterator chains");
                assert_eq!(
                    pattern.example.as_deref(),
                    Some("vec.iter().filter(|x| x > 0).collect()")
                );
            }
            _ => panic!("Expected Pattern variant"),
        }
    }

    #[test]
    fn test_messages_to_text() {
        let messages = vec![
            ConversationMessage::user_text("Hello"),
            ConversationMessage::assistant_text("Hi!"),
        ];
        let text = messages_to_text(&messages);
        assert!(text.contains("User: Hello"));
        assert!(text.contains("Assistant: Hi!"));
    }
}
