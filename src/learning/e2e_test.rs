//! End-to-end integration tests for the full learning cycle.

#[cfg(test)]
mod e2e_tests {
    use crate::learning::bm25::Bm25Engine;
    use crate::learning::extractor::LearningEngine;
    use crate::learning::retriever::ContextRetriever;
    use crate::learning::store::KnowledgeStore;
    use crate::learning::types::*;
    use crate::engine::messages::ConversationMessage;

    #[test]
    fn test_full_learning_cycle() {
        // 1. Create store and retriever
        let store = KnowledgeStore::new_in_memory().unwrap();
        let retriever = ContextRetriever::new(store, 1.2, 0.75, 5, 2000);

        // 2. Simulate adding knowledge (as if from previous sessions)
        let entry = KnowledgeEntry {
            id: None,
            category: KnowledgeCategory::Preference,
            topic: "testing".to_string(),
            content: "User prefers integration tests".to_string(),
            source_session_id: Some("prev-session".to_string()),
            confidence: 0.8,
            access_count: 0,
            created_at: None,
            last_accessed: None,
        };
        let id = retriever.store().add_knowledge(&entry).unwrap();
        retriever
            .store()
            .index_knowledge_bm25(id, &entry.content, &Bm25Engine::with_params(1.2, 0.75))
            .unwrap();

        // 3. Retrieve context for a query (must match actual tokens in content)
        let results = retriever.retrieve("integration tests").unwrap();
        assert!(!results.is_empty());

        // 4. Format for prompt
        let formatted = retriever.format_for_prompt(&results);
        assert!(formatted.contains("integration tests"));

        // 5. Simulate session with learning markers
        let messages = vec![ConversationMessage::user_text(
            "Let's write tests\n<!-- LEARN: category=\"decision\" topic=\"test_framework\" \
             content=\"Decided to use cargo-nextest for faster test execution\" -->",
        )];

        // 6. Process session
        let store2 = KnowledgeStore::new_in_memory().unwrap();
        let engine =
            LearningEngine::new(store2, 1.2, 0.75, None, "test".to_string(), false);
        let result = engine.process_session(&messages, "test-session").unwrap();

        assert_eq!(result.knowledge_extracted, 1);
    }

    #[test]
    fn test_learning_cycle_with_multiple_markers() {
        let messages = vec![
            ConversationMessage::user_text(
                "We need good tests\n\
                 <!-- LEARN: category=\"decision\" topic=\"test_runner\" \
                 content=\"Use cargo-nextest for parallel test execution\" -->\n\
                 Also we discussed logging\n\
                 <!-- LEARN: category=\"preference\" topic=\"logging\" \
                 content=\"Prefer tracing over log crate for structured logging\" -->",
            ),
            ConversationMessage::assistant_text(
                "Great choices! I'll set up tracing and nextest.",
            ),
        ];

        let store = KnowledgeStore::new_in_memory().unwrap();
        let engine = LearningEngine::new(store, 1.2, 0.75, None, "test".to_string(), false);
        let result = engine.process_session(&messages, "session-multi").unwrap();

        assert_eq!(result.knowledge_extracted, 2);

        // Verify both entries were stored
        let all = engine.store().get_all_knowledge().unwrap();
        assert_eq!(all.len(), 2);

        let topics: Vec<&str> = all.iter().map(|e| e.topic.as_str()).collect();
        assert!(topics.contains(&"test_runner"));
        assert!(topics.contains(&"logging"));
    }

    #[test]
    fn test_learning_cycle_deduplicates_similar() {
        // First session adds knowledge
        let messages1 = vec![ConversationMessage::user_text(
            "<!-- LEARN: category=\"fact\" topic=\"compiler\" \
             content=\"Rust compiler catches most errors at compile time\" -->",
        )];

        let store = KnowledgeStore::new_in_memory().unwrap();
        let engine = LearningEngine::new(store, 1.2, 0.75, None, "test".to_string(), false);

        let result1 = engine.process_session(&messages1, "session-1").unwrap();
        assert_eq!(result1.knowledge_extracted, 1);

        // Second session with same topic/content should boost existing, not duplicate
        let messages2 = vec![ConversationMessage::user_text(
            "<!-- LEARN: category=\"fact\" topic=\"compiler\" \
             content=\"Rust compiler catches most errors at compile time\" -->",
        )];

        let result2 = engine.process_session(&messages2, "session-2").unwrap();
        assert_eq!(result2.knowledge_extracted, 1);

        // Should still have only 1 entry (boosted, not duplicated)
        let all = engine.store().get_all_knowledge().unwrap();
        assert_eq!(all.len(), 1);
        // Confidence should have been boosted from 0.7
        assert!(all[0].confidence > 0.7);
    }

    #[test]
    fn test_retrieve_after_learning() {
        // Build a retriever, add knowledge, then verify retrieval
        let store = KnowledgeStore::new_in_memory().unwrap();
        let bm25 = Bm25Engine::with_params(1.2, 0.75);

        let entries = vec![
            KnowledgeEntry {
                id: None,
                category: KnowledgeCategory::Fact,
                topic: "async_runtime".to_string(),
                content: "Project uses tokio for async runtime".to_string(),
                source_session_id: Some("sess-1".to_string()),
                confidence: 0.9,
                access_count: 0,
                created_at: None,
                last_accessed: None,
            },
            KnowledgeEntry {
                id: None,
                category: KnowledgeCategory::Decision,
                topic: "database".to_string(),
                content: "Use rusqlite for local storage with SQLite".to_string(),
                source_session_id: Some("sess-1".to_string()),
                confidence: 0.8,
                access_count: 0,
                created_at: None,
                last_accessed: None,
            },
            KnowledgeEntry {
                id: None,
                category: KnowledgeCategory::Solution,
                topic: "error_handling".to_string(),
                content: "Use anyhow for application errors, thiserror for library errors"
                    .to_string(),
                source_session_id: Some("sess-2".to_string()),
                confidence: 0.85,
                access_count: 0,
                created_at: None,
                last_accessed: None,
            },
        ];

        for entry in &entries {
            let id = store.add_knowledge(entry).unwrap();
            let text = format!("{} {}", entry.topic, entry.content);
            store.index_knowledge_bm25(id, &text, &bm25).unwrap();
        }

        let retriever = ContextRetriever::new(store, 1.2, 0.75, 5, 2000);

        // Search for async-related knowledge
        let results = retriever.retrieve("async runtime tokio").unwrap();
        assert!(!results.is_empty());
        let top = &results[0];
        match top {
            RetrievedContext::Knowledge { entry, .. } => {
                assert!(entry.content.contains("tokio"));
            }
            _ => panic!("Expected Knowledge result"),
        }

        // Search for error handling knowledge
        let results = retriever.retrieve("error handling anyhow").unwrap();
        assert!(!results.is_empty());
        let top = &results[0];
        match top {
            RetrievedContext::Knowledge { entry, .. } => {
                assert!(entry.content.contains("anyhow"));
            }
            _ => panic!("Expected Knowledge result"),
        }

        // Format for prompt and verify content
        let results = retriever.retrieve("async runtime").unwrap();
        let formatted = retriever.format_for_prompt(&results);
        assert!(formatted.contains("## Relevant Context"));
        assert!(formatted.contains("tokio"));
    }

    #[test]
    fn test_empty_session_produces_no_knowledge() {
        let messages = vec![
            ConversationMessage::user_text("Hello!"),
            ConversationMessage::assistant_text("Hi there! How can I help?"),
            ConversationMessage::user_text("Just chatting."),
        ];

        let store = KnowledgeStore::new_in_memory().unwrap();
        let engine = LearningEngine::new(store, 1.2, 0.75, None, "test".to_string(), false);
        let result = engine.process_session(&messages, "empty-session").unwrap();

        assert_eq!(result.knowledge_extracted, 0);
        assert_eq!(result.patterns_extracted, 0);
        assert_eq!(result.skills_generated, 0);

        let all = engine.store().get_all_knowledge().unwrap();
        assert!(all.is_empty());
    }
}
