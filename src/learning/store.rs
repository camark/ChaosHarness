//! SQLite-backed knowledge store for the self-learning system.

use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;

use crate::learning::types::{ConversationSummary, KnowledgeCategory, KnowledgeEntry, Pattern, PatternType};

/// SQLite-backed store for conversation summaries, knowledge entries, and patterns.
pub struct KnowledgeStore {
    conn: Mutex<Connection>,
}

impl KnowledgeStore {
    /// Open or create a database at the given path.
    pub fn new(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)
            .with_context(|| format!("Failed to open database at {}", db_path.display()))?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.create_tables()?;
        Ok(store)
    }

    /// Create an in-memory database (for testing).
    pub fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()
            .context("Failed to open in-memory database")?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.create_tables()?;
        Ok(store)
    }

    /// Create all required tables.
    fn create_tables(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS conversation_summaries (
                id INTEGER PRIMARY KEY,
                session_id TEXT NOT NULL,
                summary TEXT NOT NULL,
                message_range_start INTEGER,
                message_range_end INTEGER,
                tokens_saved INTEGER,
                created_at TEXT DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS knowledge_entries (
                id INTEGER PRIMARY KEY,
                category TEXT NOT NULL,
                topic TEXT NOT NULL,
                content TEXT NOT NULL,
                source_session_id TEXT,
                confidence REAL DEFAULT 0.5,
                access_count INTEGER DEFAULT 0,
                created_at TEXT DEFAULT (datetime('now')),
                last_accessed TEXT
            );

            CREATE TABLE IF NOT EXISTS patterns (
                id INTEGER PRIMARY KEY,
                pattern_type TEXT NOT NULL,
                description TEXT NOT NULL,
                example TEXT,
                frequency INTEGER DEFAULT 1,
                created_at TEXT DEFAULT (datetime('now')),
                last_seen TEXT
            );

            CREATE TABLE IF NOT EXISTS bm25_index (
                id INTEGER PRIMARY KEY,
                doc_type TEXT NOT NULL,
                doc_id INTEGER NOT NULL,
                term TEXT NOT NULL,
                tf REAL NOT NULL,
                UNIQUE(doc_type, doc_id, term)
            );

            CREATE TABLE IF NOT EXISTS bm25_doc_stats (
                doc_type TEXT NOT NULL,
                doc_id INTEGER NOT NULL,
                doc_length INTEGER NOT NULL,
                PRIMARY KEY(doc_type, doc_id)
            );
            ",
        )
        .context("Failed to create tables")?;
        Ok(())
    }

    // ── Conversation Summaries ──────────────────────────────────────────

    /// Insert a conversation summary and return its row id.
    pub fn add_summary(&self, summary: &ConversationSummary) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let created_at = summary
            .created_at
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|| Utc::now().to_rfc3339());
        conn.execute(
            "INSERT INTO conversation_summaries (session_id, summary, message_range_start, message_range_end, tokens_saved, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                summary.session_id,
                summary.summary,
                summary.message_range_start,
                summary.message_range_end,
                summary.tokens_saved,
                created_at,
            ],
        )
        .context("Failed to insert conversation summary")?;
        Ok(conn.last_insert_rowid())
    }

    /// Get recent summaries for a session, ordered by most recent first.
    pub fn get_recent_summaries(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<ConversationSummary>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, session_id, summary, message_range_start, message_range_end, tokens_saved, created_at
                 FROM conversation_summaries
                 WHERE session_id = ?1
                 ORDER BY id DESC
                 LIMIT ?2",
            )
            .context("Failed to prepare get_recent_summaries query")?;

        let rows = stmt
            .query_map(params![session_id, limit as i64], |row| {
                Ok(ConversationSummary {
                    id: Some(row.get(0)?),
                    session_id: row.get(1)?,
                    summary: row.get(2)?,
                    message_range_start: row.get(3)?,
                    message_range_end: row.get(4)?,
                    tokens_saved: row.get(5)?,
                    created_at: parse_datetime(row.get(6)?),
                })
            })
            .context("Failed to query recent summaries")?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.context("Failed to read summary row")?);
        }
        Ok(results)
    }

    // ── Knowledge Entries ───────────────────────────────────────────────

    /// Insert a knowledge entry and return its row id.
    pub fn add_knowledge(&self, entry: &KnowledgeEntry) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let created_at = entry
            .created_at
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|| Utc::now().to_rfc3339());
        let last_accessed = entry.last_accessed.map(|dt| dt.to_rfc3339());
        conn.execute(
            "INSERT INTO knowledge_entries (category, topic, content, source_session_id, confidence, access_count, created_at, last_accessed)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                entry.category.as_str(),
                entry.topic,
                entry.content,
                entry.source_session_id,
                entry.confidence,
                entry.access_count,
                created_at,
                last_accessed,
            ],
        )
        .context("Failed to insert knowledge entry")?;
        Ok(conn.last_insert_rowid())
    }

    /// Retrieve all knowledge entries.
    pub fn get_all_knowledge(&self) -> Result<Vec<KnowledgeEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, category, topic, content, source_session_id, confidence, access_count, created_at, last_accessed
                 FROM knowledge_entries
                 ORDER BY id",
            )
            .context("Failed to prepare get_all_knowledge query")?;

        let rows = stmt
            .query_map([], |row| {
                Ok(KnowledgeEntry {
                    id: Some(row.get(0)?),
                    category: KnowledgeCategory::from_str(&row.get::<_, String>(1)?),
                    topic: row.get(2)?,
                    content: row.get(3)?,
                    source_session_id: row.get(4)?,
                    confidence: row.get(5)?,
                    access_count: row.get(6)?,
                    created_at: parse_datetime(row.get(7)?),
                    last_accessed: parse_datetime(row.get(8)?),
                })
            })
            .context("Failed to query knowledge entries")?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.context("Failed to read knowledge row")?);
        }
        Ok(results)
    }

    /// Find a knowledge entry whose topic contains the given text and whose
    /// content is most similar (simple LIKE match on topic, then best
    /// confidence). Returns `None` if nothing matches.
    pub fn find_similar_knowledge(
        &self,
        topic: &str,
        content: &str,
    ) -> Result<Option<KnowledgeEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, category, topic, content, source_session_id, confidence, access_count, created_at, last_accessed
                 FROM knowledge_entries
                 WHERE topic LIKE '%' || ?1 || '%' OR content LIKE '%' || ?2 || '%'
                 ORDER BY confidence DESC
                 LIMIT 1",
            )
            .context("Failed to prepare find_similar_knowledge query")?;

        let mut rows = stmt
            .query_map(params![topic, content], |row| {
                Ok(KnowledgeEntry {
                    id: Some(row.get(0)?),
                    category: KnowledgeCategory::from_str(&row.get::<_, String>(1)?),
                    topic: row.get(2)?,
                    content: row.get(3)?,
                    source_session_id: row.get(4)?,
                    confidence: row.get(5)?,
                    access_count: row.get(6)?,
                    created_at: parse_datetime(row.get(7)?),
                    last_accessed: parse_datetime(row.get(8)?),
                })
            })
            .context("Failed to query similar knowledge")?;

        match rows.next() {
            Some(row) => Ok(Some(row.context("Failed to read similar knowledge row")?)),
            None => Ok(None),
        }
    }

    /// Increment the access_count for a knowledge entry and update last_accessed.
    pub fn increment_access_count(&self, knowledge_id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE knowledge_entries SET access_count = access_count + 1, last_accessed = ?1 WHERE id = ?2",
            params![now, knowledge_id],
        )
        .context("Failed to increment access count")?;
        Ok(())
    }

    /// Boost the confidence of a knowledge entry by `boost` (clamped to 1.0).
    pub fn boost_knowledge_confidence(&self, knowledge_id: i64, boost: f64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE knowledge_entries SET confidence = MIN(1.0, confidence + ?1) WHERE id = ?2",
            params![boost, knowledge_id],
        )
        .context("Failed to boost knowledge confidence")?;
        Ok(())
    }

    // ── Patterns ────────────────────────────────────────────────────────

    /// Insert a pattern and return its row id.
    pub fn add_pattern(&self, pattern: &Pattern) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let created_at = pattern
            .created_at
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|| Utc::now().to_rfc3339());
        let last_seen = pattern.last_seen.map(|dt| dt.to_rfc3339());
        conn.execute(
            "INSERT INTO patterns (pattern_type, description, example, frequency, created_at, last_seen)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                pattern.pattern_type.as_str(),
                pattern.description,
                pattern.example,
                pattern.frequency,
                created_at,
                last_seen,
            ],
        )
        .context("Failed to insert pattern")?;
        Ok(conn.last_insert_rowid())
    }

    /// Retrieve all patterns.
    pub fn get_all_patterns(&self) -> Result<Vec<Pattern>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, pattern_type, description, example, frequency, created_at, last_seen
                 FROM patterns
                 ORDER BY id",
            )
            .context("Failed to prepare get_all_patterns query")?;

        let rows = stmt
            .query_map([], |row| {
                Ok(Pattern {
                    id: Some(row.get(0)?),
                    pattern_type: PatternType::from_str(&row.get::<_, String>(1)?),
                    description: row.get(2)?,
                    example: row.get(3)?,
                    frequency: row.get(4)?,
                    created_at: parse_datetime(row.get(5)?),
                    last_seen: parse_datetime(row.get(6)?),
                })
            })
            .context("Failed to query patterns")?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.context("Failed to read pattern row")?);
        }
        Ok(results)
    }

    /// Increment the frequency counter for a pattern and update last_seen.
    pub fn update_pattern_frequency(&self, pattern_id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE patterns SET frequency = frequency + 1, last_seen = ?1 WHERE id = ?2",
            params![now, pattern_id],
        )
        .context("Failed to update pattern frequency")?;
        Ok(())
    }

    // ── BM25 Integration ─────────────────────────────────────────────────

    /// Index a knowledge entry in BM25.
    pub fn index_knowledge_bm25(
        &self,
        knowledge_id: i64,
        text: &str,
        bm25: &super::bm25::Bm25Engine,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        bm25.index_document(&conn, "knowledge", knowledge_id, text)
    }

    /// Index a summary in BM25.
    pub fn index_summary_bm25(
        &self,
        summary_id: i64,
        text: &str,
        bm25: &super::bm25::Bm25Engine,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        bm25.index_document(&conn, "summary", summary_id, text)
    }

    /// Index a pattern in BM25.
    pub fn index_pattern_bm25(
        &self,
        pattern_id: i64,
        text: &str,
        bm25: &super::bm25::Bm25Engine,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        bm25.index_document(&conn, "pattern", pattern_id, text)
    }

    /// Search across all indexed documents using BM25 scoring.
    ///
    /// Calls `bm25.search()` to get scored docs, then for each result looks up
    /// the actual document from the appropriate table and returns
    /// `Vec<RetrievedContext>`.
    pub fn bm25_search(
        &self,
        query: &str,
        limit: usize,
        bm25: &super::bm25::Bm25Engine,
    ) -> Result<Vec<super::types::RetrievedContext>> {
        let conn = self.conn.lock().unwrap();
        let scored_docs = bm25.search(&conn, query, limit)?;
        let mut results = Vec::new();

        for (doc_type, doc_id, score) in scored_docs {
            match doc_type.as_str() {
                "summary" => {
                    let mut stmt = conn
                        .prepare(
                            "SELECT id, session_id, summary, message_range_start, \
                             message_range_end, tokens_saved, created_at \
                             FROM conversation_summaries WHERE id = ?1",
                        )
                        .context("Failed to prepare summary lookup")?;
                    let mut rows = stmt
                        .query_map(params![doc_id], |row| {
                            Ok(ConversationSummary {
                                id: Some(row.get(0)?),
                                session_id: row.get(1)?,
                                summary: row.get(2)?,
                                message_range_start: row.get(3)?,
                                message_range_end: row.get(4)?,
                                tokens_saved: row.get(5)?,
                                created_at: parse_datetime(row.get(6)?),
                            })
                        })
                        .context("Failed to query summary for BM25 result")?;
                    if let Some(row) = rows.next() {
                        let summary = row.context("Failed to read summary row")?;
                        results.push(super::types::RetrievedContext::Summary {
                            text: summary.summary,
                            score,
                        });
                    }
                }
                "knowledge" => {
                    let mut stmt = conn
                        .prepare(
                            "SELECT id, category, topic, content, source_session_id, \
                             confidence, access_count, created_at, last_accessed \
                             FROM knowledge_entries WHERE id = ?1",
                        )
                        .context("Failed to prepare knowledge lookup")?;
                    let mut rows = stmt
                        .query_map(params![doc_id], |row| {
                            Ok(KnowledgeEntry {
                                id: Some(row.get(0)?),
                                category: KnowledgeCategory::from_str(
                                    &row.get::<_, String>(1)?,
                                ),
                                topic: row.get(2)?,
                                content: row.get(3)?,
                                source_session_id: row.get(4)?,
                                confidence: row.get(5)?,
                                access_count: row.get(6)?,
                                created_at: parse_datetime(row.get(7)?),
                                last_accessed: parse_datetime(row.get(8)?),
                            })
                        })
                        .context("Failed to query knowledge for BM25 result")?;
                    if let Some(row) = rows.next() {
                        let entry = row.context("Failed to read knowledge row")?;
                        results.push(super::types::RetrievedContext::Knowledge {
                            entry,
                            score,
                        });
                    }
                }
                "pattern" => {
                    let mut stmt = conn
                        .prepare(
                            "SELECT id, pattern_type, description, example, frequency, \
                             created_at, last_seen \
                             FROM patterns WHERE id = ?1",
                        )
                        .context("Failed to prepare pattern lookup")?;
                    let mut rows = stmt
                        .query_map(params![doc_id], |row| {
                            Ok(Pattern {
                                id: Some(row.get(0)?),
                                pattern_type: PatternType::from_str(
                                    &row.get::<_, String>(1)?,
                                ),
                                description: row.get(2)?,
                                example: row.get(3)?,
                                frequency: row.get(4)?,
                                created_at: parse_datetime(row.get(5)?),
                                last_seen: parse_datetime(row.get(6)?),
                            })
                        })
                        .context("Failed to query pattern for BM25 result")?;
                    if let Some(row) = rows.next() {
                        let pattern = row.context("Failed to read pattern row")?;
                        results.push(super::types::RetrievedContext::Pattern {
                            pattern,
                            score,
                        });
                    }
                }
                _ => {
                    // Unknown doc type, skip
                }
            }
        }

        Ok(results)
    }
}

/// Parse an optional RFC 3339 datetime string from SQLite.
fn parse_datetime(s: Option<String>) -> Option<chrono::DateTime<chrono::Utc>> {
    s.and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&chrono::Utc)))
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::learning::types::*;

    #[test]
    fn test_create_store() {
        let store = KnowledgeStore::new_in_memory().expect("Failed to create store");
        let conn = store.conn.lock().unwrap();

        // Verify all 5 tables exist
        let tables: Vec<String> = {
            let mut stmt = conn
                .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
                .unwrap();
            stmt.query_map([], |row| row.get(0))
                .unwrap()
                .filter_map(|r| r.ok())
                .collect()
        };

        assert!(tables.contains(&"conversation_summaries".to_string()));
        assert!(tables.contains(&"knowledge_entries".to_string()));
        assert!(tables.contains(&"patterns".to_string()));
        assert!(tables.contains(&"bm25_index".to_string()));
        assert!(tables.contains(&"bm25_doc_stats".to_string()));
    }

    #[test]
    fn test_add_and_get_summary() {
        let store = KnowledgeStore::new_in_memory().expect("Failed to create store");

        let summary = ConversationSummary {
            id: None,
            session_id: "sess-001".to_string(),
            summary: "User asked about Rust lifetimes".to_string(),
            message_range_start: 0,
            message_range_end: 10,
            tokens_saved: 500,
            created_at: None,
        };

        let id = store.add_summary(&summary).expect("Failed to add summary");
        assert!(id > 0);

        let summaries = store
            .get_recent_summaries("sess-001", 10)
            .expect("Failed to get summaries");
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].session_id, "sess-001");
        assert_eq!(summaries[0].summary, "User asked about Rust lifetimes");
        assert_eq!(summaries[0].tokens_saved, 500);
        assert!(summaries[0].created_at.is_some());

        // Different session returns nothing
        let empty = store
            .get_recent_summaries("other-session", 10)
            .expect("Query failed");
        assert!(empty.is_empty());
    }

    #[test]
    fn test_add_and_get_knowledge() {
        let store = KnowledgeStore::new_in_memory().expect("Failed to create store");

        let entry = KnowledgeEntry {
            id: None,
            category: KnowledgeCategory::Solution,
            topic: "Rust ownership".to_string(),
            content: "Each value has exactly one owner".to_string(),
            source_session_id: Some("sess-001".to_string()),
            confidence: 0.8,
            access_count: 0,
            created_at: None,
            last_accessed: None,
        };

        let id = store.add_knowledge(&entry).expect("Failed to add knowledge");
        assert!(id > 0);

        let all = store.get_all_knowledge().expect("Failed to get knowledge");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].topic, "Rust ownership");
        assert_eq!(all[0].confidence, 0.8);
    }

    #[test]
    fn test_add_and_get_pattern() {
        let store = KnowledgeStore::new_in_memory().expect("Failed to create store");

        let pattern = Pattern {
            id: None,
            pattern_type: PatternType::CodingStyle,
            description: "Prefers explicit type annotations".to_string(),
            example: Some("let x: i32 = 42;".to_string()),
            frequency: 1,
            created_at: None,
            last_seen: None,
        };

        let id = store.add_pattern(&pattern).expect("Failed to add pattern");
        assert!(id > 0);

        let all = store.get_all_patterns().expect("Failed to get patterns");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].description, "Prefers explicit type annotations");
        assert_eq!(all[0].frequency, 1);
    }

    #[test]
    fn test_update_pattern_frequency() {
        let store = KnowledgeStore::new_in_memory().expect("Failed to create store");

        let pattern = Pattern {
            id: None,
            pattern_type: PatternType::Workflow,
            description: "Uses TDD approach".to_string(),
            example: None,
            frequency: 1,
            created_at: None,
            last_seen: None,
        };

        let id = store.add_pattern(&pattern).expect("Failed to add pattern");

        store
            .update_pattern_frequency(id)
            .expect("Failed to update frequency");

        let all = store.get_all_patterns().expect("Failed to get patterns");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].frequency, 2);
        assert!(all[0].last_seen.is_some());

        // Increment again
        store
            .update_pattern_frequency(id)
            .expect("Failed to update frequency again");

        let all = store.get_all_patterns().expect("Failed to get patterns");
        assert_eq!(all[0].frequency, 3);
    }

    #[test]
    fn test_find_similar_knowledge() {
        let store = KnowledgeStore::new_in_memory().expect("Failed to create store");

        let entry = KnowledgeEntry {
            id: None,
            category: KnowledgeCategory::Fact,
            topic: "Rust borrow checker".to_string(),
            content: "The borrow checker ensures memory safety at compile time".to_string(),
            source_session_id: None,
            confidence: 0.7,
            access_count: 0,
            created_at: None,
            last_accessed: None,
        };

        store.add_knowledge(&entry).expect("Failed to add knowledge");

        // Find by topic
        let found = store
            .find_similar_knowledge("borrow checker", "")
            .expect("Query failed");
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.topic, "Rust borrow checker");

        // Find by content
        let found = store
            .find_similar_knowledge("", "memory safety")
            .expect("Query failed");
        assert!(found.is_some());

        // Find nonexistent
        let not_found = store
            .find_similar_knowledge("quantum computing", "qubits")
            .expect("Query failed");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_increment_access_count() {
        let store = KnowledgeStore::new_in_memory().expect("Failed to create store");

        let entry = KnowledgeEntry {
            id: None,
            category: KnowledgeCategory::Preference,
            topic: "Editor".to_string(),
            content: "Prefers VS Code".to_string(),
            source_session_id: None,
            confidence: 0.5,
            access_count: 0,
            created_at: None,
            last_accessed: None,
        };

        let id = store.add_knowledge(&entry).expect("Failed to add knowledge");

        store
            .increment_access_count(id)
            .expect("Failed to increment access");

        let all = store.get_all_knowledge().expect("Failed to get knowledge");
        assert_eq!(all[0].access_count, 1);
        assert!(all[0].last_accessed.is_some());

        store
            .increment_access_count(id)
            .expect("Failed to increment access again");

        let all = store.get_all_knowledge().expect("Failed to get knowledge");
        assert_eq!(all[0].access_count, 2);
    }

    #[test]
    fn test_boost_knowledge_confidence() {
        let store = KnowledgeStore::new_in_memory().expect("Failed to create store");

        let entry = KnowledgeEntry {
            id: None,
            category: KnowledgeCategory::Fact,
            topic: "Test".to_string(),
            content: "Test content".to_string(),
            source_session_id: None,
            confidence: 0.5,
            access_count: 0,
            created_at: None,
            last_accessed: None,
        };

        let id = store.add_knowledge(&entry).expect("Failed to add knowledge");

        store
            .boost_knowledge_confidence(id, 0.3)
            .expect("Failed to boost confidence");

        let all = store.get_all_knowledge().expect("Failed to get knowledge");
        let conf = all[0].confidence;
        assert!((conf - 0.8).abs() < f64::EPSILON, "Expected 0.8, got {conf}");

        // Boost again -- should clamp to 1.0
        store
            .boost_knowledge_confidence(id, 0.5)
            .expect("Failed to boost confidence");

        let all = store.get_all_knowledge().expect("Failed to get knowledge");
        let conf = all[0].confidence;
        assert!((conf - 1.0).abs() < f64::EPSILON, "Expected 1.0, got {conf}");
    }

    #[test]
    fn test_bm25_integration() {
        use super::super::bm25::Bm25Engine;
        use super::super::types::RetrievedContext;

        let store = KnowledgeStore::new_in_memory().unwrap();
        let bm25 = Bm25Engine::new();

        // Add and index knowledge
        let entry = KnowledgeEntry {
            id: None,
            category: KnowledgeCategory::Fact,
            topic: "async_runtime".to_string(),
            content: "Project uses tokio for async runtime".to_string(),
            source_session_id: None,
            confidence: 0.8,
            access_count: 0,
            created_at: None,
            last_accessed: None,
        };
        let id = store.add_knowledge(&entry).unwrap();
        store
            .index_knowledge_bm25(id, &entry.content, &bm25)
            .unwrap();

        // Search
        let results = store.bm25_search("tokio async", 5, &bm25).unwrap();
        assert_eq!(results.len(), 1);
        match &results[0] {
            RetrievedContext::Knowledge { entry, score } => {
                assert!(entry.content.contains("tokio"));
                assert!(*score > 0.0);
            }
            _ => panic!("Expected Knowledge result"),
        }
    }

    #[test]
    fn test_bm25_search_summary() {
        use super::super::bm25::Bm25Engine;
        use super::super::types::RetrievedContext;

        let store = KnowledgeStore::new_in_memory().unwrap();
        let bm25 = Bm25Engine::new();

        let summary = ConversationSummary {
            id: None,
            session_id: "sess-001".to_string(),
            summary: "User asked about Rust lifetimes and borrow checker".to_string(),
            message_range_start: 0,
            message_range_end: 10,
            tokens_saved: 500,
            created_at: None,
        };
        let id = store.add_summary(&summary).unwrap();
        store
            .index_summary_bm25(id, &summary.summary, &bm25)
            .unwrap();

        let results = store.bm25_search("borrow checker", 5, &bm25).unwrap();
        assert_eq!(results.len(), 1);
        match &results[0] {
            RetrievedContext::Summary { text, score } => {
                assert!(text.contains("borrow checker"));
                assert!(*score > 0.0);
            }
            _ => panic!("Expected Summary result"),
        }
    }

    #[test]
    fn test_bm25_search_pattern() {
        use super::super::bm25::Bm25Engine;
        use super::super::types::RetrievedContext;

        let store = KnowledgeStore::new_in_memory().unwrap();
        let bm25 = Bm25Engine::new();

        let pattern = Pattern {
            id: None,
            pattern_type: PatternType::CodingStyle,
            description: "Prefers explicit type annotations in Rust code".to_string(),
            example: Some("let x: i32 = 42;".to_string()),
            frequency: 1,
            created_at: None,
            last_seen: None,
        };
        let id = store.add_pattern(&pattern).unwrap();
        store
            .index_pattern_bm25(id, &pattern.description, &bm25)
            .unwrap();

        let results = store.bm25_search("explicit type annotations", 5, &bm25).unwrap();
        assert_eq!(results.len(), 1);
        match &results[0] {
            RetrievedContext::Pattern { pattern, score } => {
                assert!(pattern.description.contains("explicit type"));
                assert!(*score > 0.0);
            }
            _ => panic!("Expected Pattern result"),
        }
    }

    #[test]
    fn test_bm25_search_mixed_types() {
        use super::super::bm25::Bm25Engine;
        use super::super::types::RetrievedContext;

        let store = KnowledgeStore::new_in_memory().unwrap();
        let bm25 = Bm25Engine::new();

        // Add one of each type, all mentioning "rust"
        let entry = KnowledgeEntry {
            id: None,
            category: KnowledgeCategory::Fact,
            topic: "language".to_string(),
            content: "Rust is a systems programming language".to_string(),
            source_session_id: None,
            confidence: 0.8,
            access_count: 0,
            created_at: None,
            last_accessed: None,
        };
        let kid = store.add_knowledge(&entry).unwrap();
        store.index_knowledge_bm25(kid, &entry.content, &bm25).unwrap();

        let summary = ConversationSummary {
            id: None,
            session_id: "sess-002".to_string(),
            summary: "Discussed Rust memory safety guarantees".to_string(),
            message_range_start: 0,
            message_range_end: 5,
            tokens_saved: 300,
            created_at: None,
        };
        let sid = store.add_summary(&summary).unwrap();
        store.index_summary_bm25(sid, &summary.summary, &bm25).unwrap();

        let pattern = Pattern {
            id: None,
            pattern_type: PatternType::Workflow,
            description: "Always run cargo clippy before committing Rust code".to_string(),
            example: None,
            frequency: 1,
            created_at: None,
            last_seen: None,
        };
        let pid = store.add_pattern(&pattern).unwrap();
        store.index_pattern_bm25(pid, &pattern.description, &bm25).unwrap();

        let results = store.bm25_search("Rust", 10, &bm25).unwrap();
        assert_eq!(results.len(), 3);

        // Verify all three types are present
        let has_knowledge = results.iter().any(|r| matches!(r, RetrievedContext::Knowledge { .. }));
        let has_summary = results.iter().any(|r| matches!(r, RetrievedContext::Summary { .. }));
        let has_pattern = results.iter().any(|r| matches!(r, RetrievedContext::Pattern { .. }));
        assert!(has_knowledge, "Expected at least one Knowledge result");
        assert!(has_summary, "Expected at least one Summary result");
        assert!(has_pattern, "Expected at least one Pattern result");
    }

    #[test]
    fn test_bm25_search_empty_index() {
        use super::super::bm25::Bm25Engine;

        let store = KnowledgeStore::new_in_memory().unwrap();
        let bm25 = Bm25Engine::new();

        let results = store.bm25_search("anything", 5, &bm25).unwrap();
        assert!(results.is_empty());
    }
}
