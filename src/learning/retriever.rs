//! BM25-based context retrieval and prompt formatting.
//!
//! `ContextRetriever` searches the knowledge store using BM25 scoring and
//! formats the results as markdown suitable for injection into a system prompt.

use anyhow::Result;

use crate::learning::bm25::Bm25Engine;
use crate::learning::store::KnowledgeStore;
use crate::learning::types::RetrievedContext;

/// Retrieves relevant context from the knowledge store using BM25 ranking
/// and formats it for prompt injection.
pub struct ContextRetriever {
    store: KnowledgeStore,
    bm25: Bm25Engine,
    top_k: usize,
    max_tokens: usize,
}

impl ContextRetriever {
    /// Create a new context retriever.
    ///
    /// - `store`: the backing knowledge store
    /// - `bm25_k1`: BM25 term-frequency saturation parameter
    /// - `bm25_b`: BM25 length-normalization parameter
    /// - `top_k`: maximum number of results to retrieve
    /// - `max_tokens`: rough token budget for formatted output (truncated at
    ///   `max_tokens * 4` characters)
    pub fn new(
        store: KnowledgeStore,
        bm25_k1: f64,
        bm25_b: f64,
        top_k: usize,
        max_tokens: usize,
    ) -> Self {
        Self {
            store,
            bm25: Bm25Engine::with_params(bm25_k1, bm25_b),
            top_k,
            max_tokens,
        }
    }

    /// Retrieve the most relevant contexts for the given query using BM25
    /// scoring over the knowledge store.
    pub fn retrieve(&self, query: &str) -> Result<Vec<RetrievedContext>> {
        self.store.bm25_search(query, self.top_k, &self.bm25)
    }

    /// Format retrieved contexts as a markdown block for injection into a
    /// system prompt.
    ///
    /// Returns an empty string if `contexts` is empty. Output is truncated
    /// when it exceeds `max_tokens * 4` characters (a rough character-level
    /// token estimate).
    pub fn format_for_prompt(&self, contexts: &[RetrievedContext]) -> String {
        if contexts.is_empty() {
            return String::new();
        }

        let mut output = String::from("## Relevant Context\n\nThe following information from past sessions may be relevant:\n\n");

        for ctx in contexts {
            let line = match ctx {
                RetrievedContext::Summary { text, .. } => {
                    format!("- **Past Session Summary**: {}\n", text)
                }
                RetrievedContext::Knowledge { entry, .. } => {
                    format!(
                        "- **{}** ({}): {}\n",
                        entry.topic,
                        entry.category.as_str(),
                        entry.content,
                    )
                }
                RetrievedContext::Pattern { pattern, .. } => {
                    format!(
                        "- **Pattern** ({}): {}\n",
                        pattern.pattern_type.as_str(),
                        pattern.description,
                    )
                }
            };
            output.push_str(&line);
        }

        // Truncate if output exceeds the character budget.
        let max_chars = self.max_tokens * 4;
        if output.len() > max_chars {
            output.truncate(max_chars);
            // Ensure we don't cut in the middle of a UTF-8 character.
            while !output.is_char_boundary(output.len()) {
                output.pop();
            }
        }

        output
    }

    /// Borrow the underlying knowledge store.
    pub fn store(&self) -> &KnowledgeStore {
        &self.store
    }

    /// Mutably borrow the underlying knowledge store (e.g. to add new entries).
    pub fn store_mut(&mut self) -> &mut KnowledgeStore {
        &mut self.store
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::learning::types::*;

    /// Helper: build a retriever backed by an in-memory store.
    fn make_retriever(top_k: usize, max_tokens: usize) -> ContextRetriever {
        let store = KnowledgeStore::new_in_memory().expect("in-memory store");
        ContextRetriever::new(store, 1.2, 0.75, top_k, max_tokens)
    }

    #[test]
    fn test_retrieve_relevant() {
        let store = KnowledgeStore::new_in_memory().expect("in-memory store");
        let bm25 = Bm25Engine::with_params(1.2, 0.75);

        // Add and index a knowledge entry
        let entry = KnowledgeEntry {
            id: None,
            category: KnowledgeCategory::Solution,
            topic: "Rust ownership".to_string(),
            content: "Each value in Rust has exactly one owner".to_string(),
            source_session_id: Some("sess-001".to_string()),
            confidence: 0.9,
            access_count: 0,
            created_at: None,
            last_accessed: None,
        };
        let id = store.add_knowledge(&entry).unwrap();
        store.index_knowledge_bm25(id, &entry.content, &bm25).unwrap();

        // Build the retriever with the populated store
        let retriever = ContextRetriever {
            store,
            bm25,
            top_k: 10,
            max_tokens: 1000,
        };

        // Search for relevant terms
        let results = retriever.retrieve("Rust owner").unwrap();
        assert!(!results.is_empty(), "Expected at least one result");

        let found = &results[0];
        match found {
            RetrievedContext::Knowledge { entry: e, score } => {
                assert!(e.content.contains("owner"));
                assert!(*score > 0.0);
            }
            other => panic!("Expected Knowledge, got {:?}", other),
        }
    }

    #[test]
    fn test_retrieve_empty_query() {
        let retriever = make_retriever(10, 1000);
        let results = retriever.retrieve("").unwrap();
        assert!(results.is_empty(), "Empty query should return no results");
    }

    #[test]
    fn test_format_for_prompt() {
        let contexts = vec![
            RetrievedContext::Summary {
                text: "Discussed async runtime choices".to_string(),
                score: 3.5,
            },
            RetrievedContext::Knowledge {
                entry: KnowledgeEntry {
                    id: Some(1),
                    category: KnowledgeCategory::Fact,
                    topic: "async_runtime".to_string(),
                    content: "Project uses tokio".to_string(),
                    source_session_id: None,
                    confidence: 0.8,
                    access_count: 0,
                    created_at: None,
                    last_accessed: None,
                },
                score: 2.1,
            },
            RetrievedContext::Pattern {
                pattern: Pattern {
                    id: Some(1),
                    pattern_type: PatternType::CodingStyle,
                    description: "Prefers explicit error handling with anyhow".to_string(),
                    example: None,
                    frequency: 3,
                    created_at: None,
                    last_seen: None,
                },
                score: 1.0,
            },
        ];

        let retriever = make_retriever(10, 1000);
        let output = retriever.format_for_prompt(&contexts);

        assert!(output.contains("## Relevant Context"));
        assert!(output.contains("Past Session Summary"));
        assert!(output.contains("Discussed async runtime choices"));
        assert!(output.contains("**async_runtime** (fact)"));
        assert!(output.contains("Project uses tokio"));
        assert!(output.contains("**Pattern** (coding_style)"));
        assert!(output.contains("Prefers explicit error handling"));
    }

    #[test]
    fn test_format_empty() {
        let retriever = make_retriever(10, 1000);
        let output = retriever.format_for_prompt(&[]);
        assert!(output.is_empty(), "Empty contexts should produce empty string");
    }
}
