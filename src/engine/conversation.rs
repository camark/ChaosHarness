//! Multi-turn conversation optimization
//!
//! Tracks conversation state, extracts key facts, detects topic changes,
//! and maintains a running summary.

use crate::engine::messages::{ConversationMessage, MessageContent, MessageRole};
use serde::{Deserialize, Serialize};

/// Conversation state tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationState {
    /// Running summary of the conversation
    pub summary: String,
    /// Key facts extracted from the conversation
    pub facts: Vec<KeyFact>,
    /// Current topic
    pub current_topic: Option<String>,
    /// Topic history
    pub topic_history: Vec<TopicEntry>,
    /// Number of turns tracked
    pub turn_count: usize,
    /// Decisions made during conversation
    pub decisions: Vec<Decision>,
}

/// A key fact extracted from conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyFact {
    pub content: String,
    pub category: FactCategory,
    pub turn_number: usize,
    pub confidence: f64,
}

/// Categories for facts
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FactCategory {
    /// Technical fact (architecture, API, etc.)
    Technical,
    /// User preference
    Preference,
    /// Decision made
    Decision,
    /// File or path reference
    FileReference,
    /// Error or issue
    Issue,
}

/// A topic in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicEntry {
    pub topic: String,
    pub start_turn: usize,
    pub end_turn: Option<usize>,
}

/// A decision made during conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub description: String,
    pub turn_number: usize,
    pub rationale: Option<String>,
}

impl ConversationState {
    pub fn new() -> Self {
        Self {
            summary: String::new(),
            facts: Vec::new(),
            current_topic: None,
            topic_history: Vec::new(),
            turn_count: 0,
            decisions: Vec::new(),
        }
    }

    /// Update state with a new turn
    pub fn process_turn(&mut self, messages: &[ConversationMessage], turn_number: usize) {
        self.turn_count = turn_number;

        // Extract user message
        let user_text = messages.iter()
            .rev()
            .find(|m| m.role == MessageRole::User)
            .and_then(|m| m.content.first())
            .and_then(|c| match c {
                MessageContent::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .unwrap_or("");

        // Extract assistant response
        let assistant_text = messages.iter()
            .rev()
            .find(|m| m.role == MessageRole::Assistant)
            .and_then(|m| m.content.first())
            .and_then(|c| match c {
                MessageContent::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .unwrap_or("");

        // Extract facts from user message
        self.extract_facts(user_text, turn_number);

        // Detect topic change
        self.detect_topic_change(user_text, turn_number);

        // Update summary
        self.update_summary(user_text, assistant_text, turn_number);
    }

    /// Extract key facts from text
    fn extract_facts(&mut self, text: &str, turn_number: usize) {
        let text_lower = text.to_lowercase();

        // Extract file references
        for word in text.split_whitespace() {
            if word.contains('/') || word.contains('\\') || word.ends_with(".rs") || word.ends_with(".ts") || word.ends_with(".py") {
                self.facts.push(KeyFact {
                    content: word.to_string(),
                    category: FactCategory::FileReference,
                    turn_number,
                    confidence: 0.9,
                });
            }
        }

        // Extract decision indicators
        let decision_patterns = [
            "let's use", "i'll use", "we should", "decided to",
            "going with", "choosing", "i recommend", "best approach",
        ];
        for pattern in &decision_patterns {
            if text_lower.contains(pattern) {
                // Extract the sentence containing the decision
                if let Some(start) = text_lower.find(pattern) {
                    let end = text[start..].find('.').map(|i| start + i).unwrap_or(text.len());
                    let decision_text = text[start..end].trim();
                    self.decisions.push(Decision {
                        description: decision_text.to_string(),
                        turn_number,
                        rationale: None,
                    });
                }
            }
        }

        // Extract preference indicators
        let preference_patterns = [
            "i prefer", "i like", "i want", "please don't",
            "always", "never", "i usually", "my preference",
        ];
        for pattern in &preference_patterns {
            if text_lower.contains(pattern) {
                if let Some(start) = text_lower.find(pattern) {
                    let end = text[start..].find('.').map(|i| start + i).unwrap_or(text.len());
                    let pref_text = text[start..end].trim();
                    self.facts.push(KeyFact {
                        content: pref_text.to_string(),
                        category: FactCategory::Preference,
                        turn_number,
                        confidence: 0.8,
                    });
                }
            }
        }

        // Extract error/issue mentions
        let issue_patterns = ["error", "bug", "issue", "problem", "fail", "crash"];
        for pattern in &issue_patterns {
            if text_lower.contains(pattern) {
                if let Some(start) = text_lower.find(pattern) {
                    let end = text[start..].find('.').map(|i| start + i).unwrap_or(text.len());
                    let issue_text = text[start..end].trim();
                    self.facts.push(KeyFact {
                        content: issue_text.to_string(),
                        category: FactCategory::Issue,
                        turn_number,
                        confidence: 0.7,
                    });
                }
            }
        }
    }

    /// Detect topic changes
    fn detect_topic_change(&mut self, text: &str, turn_number: usize) {
        let new_topic = self.infer_topic(text);

        if let Some(ref current) = self.current_topic {
            if current != &new_topic {
                // Topic changed
                if let Some(last) = self.topic_history.last_mut() {
                    if last.end_turn.is_none() {
                        last.end_turn = Some(turn_number - 1);
                    }
                }
                self.topic_history.push(TopicEntry {
                    topic: new_topic.clone(),
                    start_turn: turn_number,
                    end_turn: None,
                });
                self.current_topic = Some(new_topic);
            }
        } else {
            // First topic
            self.current_topic = Some(new_topic.clone());
            self.topic_history.push(TopicEntry {
                topic: new_topic,
                start_turn: turn_number,
                end_turn: None,
            });
        }
    }

    /// Infer topic from text
    fn infer_topic(&self, text: &str) -> String {
        let text_lower = text.to_lowercase();

        // Simple keyword-based topic inference (order matters - more specific first)
        let topic_keywords = [
            ("code review", vec!["review", "pr", "pull request", "merge"]),
            ("debugging", vec!["bug", "error", "fix", "issue", "debug", "crash"]),
            ("testing", vec!["test", "spec", "coverage", "assert"]),
            ("refactoring", vec!["refactor", "clean", "improve", "optimize"]),
            ("documentation", vec!["doc", "readme", "comment", "explain"]),
            ("configuration", vec!["config", "setting", "env", "environment"]),
            ("deployment", vec!["deploy", "release", "publish", "ci", "cd"]),
            ("implementation", vec!["implement", "create", "add", "build", "write"]),
        ];

        for (topic, keywords) in &topic_keywords {
            for keyword in keywords {
                if text_lower.contains(keyword) {
                    return topic.to_string();
                }
            }
        }

        // Default: use first few words
        let words: Vec<&str> = text.split_whitespace().take(5).collect();
        words.join(" ")
    }

    /// Update conversation summary
    fn update_summary(&mut self, user_text: &str, _assistant_text: &str, turn_number: usize) {
        // Simple summary: keep last N turns summarized
        let max_summary_length = 500;

        if turn_number <= 1 {
            self.summary = format!("User: {}", truncate(user_text, 200));
        } else {
            // Append to summary
            let new_entry = format!("\n[Turn {}] {}", turn_number, truncate(user_text, 100));
            self.summary.push_str(&new_entry);

            // Truncate if too long
            if self.summary.len() > max_summary_length {
                let start = self.summary.len() - max_summary_length;
                if let Some(pos) = self.summary[start..].find('\n') {
                    self.summary = self.summary[start + pos + 1..].to_string();
                }
            }
        }
    }

    /// Get context for the current conversation state
    pub fn get_context(&self) -> String {
        let mut context = String::new();

        // Add summary if exists
        if !self.summary.is_empty() {
            context.push_str("## Conversation Summary\n\n");
            context.push_str(&self.summary);
            context.push('\n');
        }

        // Add current topic
        if let Some(ref topic) = self.current_topic {
            context.push_str(&format!("## Current Topic: {}\n\n", topic));
        }

        // Add recent decisions
        let recent_decisions: Vec<_> = self.decisions.iter()
            .rev()
            .take(3)
            .collect();
        if !recent_decisions.is_empty() {
            context.push_str("## Recent Decisions\n\n");
            for decision in recent_decisions {
                context.push_str(&format!("- {}\n", decision.description));
            }
            context.push('\n');
        }

        // Add relevant facts
        let relevant_facts: Vec<_> = self.facts.iter()
            .rev()
            .take(5)
            .collect();
        if !relevant_facts.is_empty() {
            context.push_str("## Key Facts\n\n");
            for fact in relevant_facts {
                context.push_str(&format!("- [{:?}] {}\n", fact.category, fact.content));
            }
            context.push('\n');
        }

        context
    }

    /// Get the number of active facts
    pub fn fact_count(&self) -> usize {
        self.facts.len()
    }

    /// Clear old facts to prevent unbounded growth
    pub fn prune_old_facts(&mut self, keep_recent: usize) {
        if self.facts.len() > keep_recent {
            let drain_count = self.facts.len() - keep_recent;
            self.facts.drain(0..drain_count);
        }
    }
}

impl Default for ConversationState {
    fn default() -> Self {
        Self::new()
    }
}

/// Truncate text to max length
fn truncate(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        format!("{}...", &text[..max_len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_state() {
        let state = ConversationState::new();
        assert_eq!(state.turn_count, 0);
        assert!(state.summary.is_empty());
        assert!(state.facts.is_empty());
    }

    #[test]
    fn test_infer_topic() {
        let state = ConversationState::new();
        assert_eq!(state.infer_topic("Please fix this bug"), "debugging");
        assert_eq!(state.infer_topic("Let's implement a new feature"), "implementation");
        assert_eq!(state.infer_topic("Write some tests"), "testing");
    }

    #[test]
    fn test_extract_file_references() {
        let mut state = ConversationState::new();
        state.extract_facts("Look at src/main.rs for the issue", 1);
        assert!(state.facts.iter().any(|f| f.content.contains("main.rs")));
    }

    #[test]
    fn test_extract_decisions() {
        let mut state = ConversationState::new();
        state.extract_facts("Let's use SQLite for the database.", 1);
        assert!(!state.decisions.is_empty());
    }

    #[test]
    fn test_topic_change() {
        let mut state = ConversationState::new();
        state.detect_topic_change("Fix this bug", 1);
        assert_eq!(state.current_topic, Some("debugging".to_string()));

        state.detect_topic_change("Write tests for it", 2);
        assert_eq!(state.current_topic, Some("testing".to_string()));
        assert_eq!(state.topic_history.len(), 2);
    }

    #[test]
    fn test_prune_old_facts() {
        let mut state = ConversationState::new();
        for i in 0..10 {
            state.facts.push(KeyFact {
                content: format!("fact {}", i),
                category: FactCategory::Technical,
                turn_number: i,
                confidence: 0.9,
            });
        }
        state.prune_old_facts(5);
        assert_eq!(state.facts.len(), 5);
    }
}
