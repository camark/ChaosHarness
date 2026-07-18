# Self-Learning & Context Management Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add self-learning capabilities and intelligent context management to RustHarness using SQLite + BM25.

**Architecture:** A `learning/` module with 6 sub-components: types, store (SQLite), BM25 engine, smart compactor, context retriever, and learning engine. Integrates with the existing query loop, compaction system, and session lifecycle.

**Tech Stack:** rusqlite (SQLite), existing API client for summarization, TF-IDF/BM25 for retrieval

---

## File Structure

| File | Responsibility |
|------|---------------|
| `src/learning/mod.rs` | Module exports |
| `src/learning/types.rs` | Shared types: KnowledgeEntry, Pattern, Summary, RetrievedContext |
| `src/learning/store.rs` | SQLite KnowledgeStore: schema, CRUD operations |
| `src/learning/bm25.rs` | BM25 tokenization, indexing, and scoring |
| `src/learning/summarizer.rs` | SmartCompactor: LLM-powered conversation summarization |
| `src/learning/retriever.rs` | ContextRetriever: BM25-based context injection |
| `src/learning/extractor.rs` | LearningEngine: marker parsing + LLM extraction at session end |
| `src/learning/skill_gen.rs` | SkillGenerator: auto-generate .skill files from patterns |
| `src/config/settings.rs` | Modify: add LearningSettings struct |
| `src/main.rs` | Modify: add `mod learning` |
| `src/engine/query.rs` | Modify: integrate ContextRetriever into prompt building |
| `src/engine/compact.rs` | Modify: delegate to SmartCompactor when learning enabled |
| `src/ui/repl.rs` | Modify: trigger LearningEngine at session end |
| `src/prompts/system_prompt.rs` | Modify: add learning marker instructions |

---

### Task 1: Add Dependencies and Module Structure

**Files:**
- Modify: `Cargo.toml`
- Create: `src/learning/mod.rs`
- Create: `src/learning/types.rs`
- Modify: `src/main.rs`
- Modify: `src/config/settings.rs`

- [ ] **Step 1: Add rusqlite dependency to Cargo.toml**

```toml
# Add to [dependencies] section:
rusqlite = { version = "0.31", features = ["bundled"] }
```

- [ ] **Step 2: Create learning module structure**

Create `src/learning/mod.rs`:
```rust
//! Self-learning and context management system

pub mod types;
pub mod store;
pub mod bm25;
pub mod summarizer;
pub mod retriever;
pub mod extractor;
pub mod skill_gen;
```

Create `src/learning/types.rs` with placeholder:
```rust
//! Types for the self-learning system

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KnowledgeCategory {
    Fact,
    Decision,
    Solution,
    Preference,
}

impl KnowledgeCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Fact => "fact",
            Self::Decision => "decision",
            Self::Solution => "solution",
            Self::Preference => "preference",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "fact" => Self::Fact,
            "decision" => Self::Decision,
            "solution" => Self::Solution,
            "preference" => Self::Preference,
            _ => Self::Fact,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternType {
    CodingStyle,
    Workflow,
    ToolPreference,
}

impl PatternType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CodingStyle => "coding_style",
            Self::Workflow => "workflow",
            Self::ToolPreference => "tool_preference",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "coding_style" => Self::CodingStyle,
            "workflow" => Self::Workflow,
            "tool_preference" => Self::ToolPreference,
            _ => Self::Workflow,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEntry {
    pub id: Option<i64>,
    pub category: KnowledgeCategory,
    pub topic: String,
    pub content: String,
    pub source_session_id: Option<String>,
    pub confidence: f64,
    pub access_count: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub last_accessed: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub id: Option<i64>,
    pub pattern_type: PatternType,
    pub description: String,
    pub example: Option<String>,
    pub frequency: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub last_seen: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSummary {
    pub id: Option<i64>,
    pub session_id: String,
    pub summary: String,
    pub message_range_start: i64,
    pub message_range_end: i64,
    pub tokens_saved: i64,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub enum RetrievedContext {
    Summary {
        text: String,
        score: f64,
    },
    Knowledge {
        entry: KnowledgeEntry,
        score: f64,
    },
    Pattern {
        pattern: Pattern,
        score: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningMarker {
    pub category: String,
    pub topic: String,
    pub content: String,
}

#[derive(Debug, Default)]
pub struct LearningResult {
    pub knowledge_extracted: usize,
    pub patterns_extracted: usize,
    pub skills_generated: usize,
}
```

- [ ] **Step 3: Add module to main.rs**

In `src/main.rs`, add after the existing module declarations:
```rust
mod learning;
```

- [ ] **Step 4: Add LearningSettings to config**

In `src/config/settings.rs`, add the `LearningSettings` struct and add it to `Settings`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningSettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub knowledge_db_path: Option<String>,
    #[serde(default = "default_bm25_top_k")]
    pub bm25_top_k: usize,
    #[serde(default = "default_bm25_k1")]
    pub bm25_k1: f64,
    #[serde(default = "default_bm25_b")]
    pub bm25_b: f64,
    #[serde(default = "default_summary_token_threshold")]
    pub summary_token_threshold: u32,
    #[serde(default = "default_summary_segment_size")]
    pub summary_segment_size: usize,
    #[serde(default = "default_true")]
    pub session_end_extraction: bool,
    #[serde(default = "default_true")]
    pub auto_skill_generation: bool,
    #[serde(default = "default_pattern_promotion_threshold")]
    pub pattern_promotion_threshold: i64,
    #[serde(default = "default_max_context_injection_tokens")]
    pub max_context_injection_tokens: usize,
}

fn default_bm25_top_k() -> usize { 5 }
fn default_bm25_k1() -> f64 { 1.2 }
fn default_bm25_b() -> f64 { 0.75 }
fn default_summary_token_threshold() -> u32 { 30000 }
fn default_summary_segment_size() -> usize { 20 }
fn default_pattern_promotion_threshold() -> i64 { 3 }
fn default_max_context_injection_tokens() -> usize { 2000 }

impl Default for LearningSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            knowledge_db_path: None,
            bm25_top_k: 5,
            bm25_k1: 1.2,
            bm25_b: 0.75,
            summary_token_threshold: 30000,
            summary_segment_size: 20,
            session_end_extraction: true,
            auto_skill_generation: true,
            pattern_promotion_threshold: 3,
            max_context_injection_tokens: 2000,
        }
    }
}
```

Add to the `Settings` struct:
```rust
#[serde(default)]
pub learning: LearningSettings,
```

Add to `Settings::default()`:
```rust
learning: LearningSettings::default(),
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully (new module is empty but valid)

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml src/learning/mod.rs src/learning/types.rs src/main.rs src/config/settings.rs
git commit -m "feat(learning): add learning module structure, types, and config"
```

---

### Task 2: SQLite KnowledgeStore

**Files:**
- Create: `src/learning/store.rs`
- Test: Inline `#[cfg(test)]` module

- [ ] **Step 1: Write tests for KnowledgeStore**

Create `src/learning/store.rs`:
```rust
//! SQLite-backed knowledge store

use anyhow::Result;
use rusqlite::{Connection, params};
use std::path::Path;
use super::types::*;

pub struct KnowledgeStore {
    conn: Connection,
}

impl KnowledgeStore {
    pub fn new(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        let store = Self { conn };
        store.create_tables()?;
        Ok(store)
    }

    pub fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let store = Self { conn };
        store.create_tables()?;
        Ok(store)
    }

    fn create_tables(&self) -> Result<()> {
        self.conn.execute_batch("
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
        ")?;
        Ok(())
    }

    pub fn add_summary(&self, summary: &ConversationSummary) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO conversation_summaries (session_id, summary, message_range_start, message_range_end, tokens_saved)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![summary.session_id, summary.summary, summary.message_range_start, summary.message_range_end, summary.tokens_saved],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn add_knowledge(&self, entry: &KnowledgeEntry) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO knowledge_entries (category, topic, content, source_session_id, confidence)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![entry.category.as_str(), entry.topic, entry.content, entry.source_session_id, entry.confidence],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn add_pattern(&self, pattern: &Pattern) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO patterns (pattern_type, description, example, frequency)
             VALUES (?1, ?2, ?3, ?4)",
            params![pattern.pattern_type.as_str(), pattern.description, pattern.example, pattern.frequency],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn get_recent_summaries(&self, session_id: &str, limit: usize) -> Result<Vec<ConversationSummary>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, summary, message_range_start, message_range_end, tokens_saved
             FROM conversation_summaries WHERE session_id = ?1 ORDER BY id DESC LIMIT ?2"
        )?;
        let rows = stmt.query_map(params![session_id, limit as i64], |row| {
            Ok(ConversationSummary {
                id: row.get(0)?,
                session_id: row.get(1)?,
                summary: row.get(2)?,
                message_range_start: row.get(3)?,
                message_range_end: row.get(4)?,
                tokens_saved: row.get(5)?,
                created_at: None,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn get_all_knowledge(&self) -> Result<Vec<KnowledgeEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, category, topic, content, source_session_id, confidence, access_count
             FROM knowledge_entries ORDER BY confidence DESC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(KnowledgeEntry {
                id: row.get(0)?,
                category: KnowledgeCategory::from_str(&row.get::<_, String>(1)?),
                topic: row.get(2)?,
                content: row.get(3)?,
                source_session_id: row.get(4)?,
                confidence: row.get(5)?,
                access_count: row.get(6)?,
                created_at: None,
                last_accessed: None,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn get_all_patterns(&self) -> Result<Vec<Pattern>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, pattern_type, description, example, frequency
             FROM patterns ORDER BY frequency DESC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Pattern {
                id: row.get(0)?,
                pattern_type: PatternType::from_str(&row.get::<_, String>(1)?),
                description: row.get(2)?,
                example: row.get(3)?,
                frequency: row.get(4)?,
                created_at: None,
                last_seen: None,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn update_pattern_frequency(&self, pattern_id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE patterns SET frequency = frequency + 1, last_seen = datetime('now') WHERE id = ?1",
            params![pattern_id],
        )?;
        Ok(())
    }

    pub fn increment_access_count(&self, knowledge_id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE knowledge_entries SET access_count = access_count + 1, last_accessed = datetime('now') WHERE id = ?1",
            params![knowledge_id],
        )?;
        Ok(())
    }

    pub fn find_similar_knowledge(&self, topic: &str, content: &str) -> Result<Option<KnowledgeEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, category, topic, content, source_session_id, confidence, access_count
             FROM knowledge_entries WHERE topic = ?1 OR content = ?2 LIMIT 1"
        )?;
        let mut rows = stmt.query_map(params![topic, content], |row| {
            Ok(KnowledgeEntry {
                id: row.get(0)?,
                category: KnowledgeCategory::from_str(&row.get::<_, String>(1)?),
                topic: row.get(2)?,
                content: row.get(3)?,
                source_session_id: row.get(4)?,
                confidence: row.get(5)?,
                access_count: row.get(6)?,
                created_at: None,
                last_accessed: None,
            })
        })?;
        Ok(rows.next().and_then(|r| r.ok()))
    }

    pub fn boost_knowledge_confidence(&self, knowledge_id: i64, boost: f64) -> Result<()> {
        self.conn.execute(
            "UPDATE knowledge_entries SET confidence = MIN(1.0, confidence + ?1) WHERE id = ?2",
            params![boost, knowledge_id],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_store() {
        let store = KnowledgeStore::new_in_memory().unwrap();
        // Tables should exist
        let count: i64 = store.conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table'", [],
            |row| row.get(0)
        ).unwrap();
        assert_eq!(count, 5);
    }

    #[test]
    fn test_add_and_get_summary() {
        let store = KnowledgeStore::new_in_memory().unwrap();
        let summary = ConversationSummary {
            id: None,
            session_id: "test-session".to_string(),
            summary: "User discussed Rust patterns".to_string(),
            message_range_start: 0,
            message_range_end: 20,
            tokens_saved: 5000,
            created_at: None,
        };
        let id = store.add_summary(&summary).unwrap();
        assert!(id > 0);

        let summaries = store.get_recent_summaries("test-session", 10).unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].summary, "User discussed Rust patterns");
    }

    #[test]
    fn test_add_and_get_knowledge() {
        let store = KnowledgeStore::new_in_memory().unwrap();
        let entry = KnowledgeEntry {
            id: None,
            category: KnowledgeCategory::Preference,
            topic: "testing".to_string(),
            content: "User prefers integration tests".to_string(),
            source_session_id: Some("session-1".to_string()),
            confidence: 0.7,
            access_count: 0,
            created_at: None,
            last_accessed: None,
        };
        let id = store.add_knowledge(&entry).unwrap();
        assert!(id > 0);

        let entries = store.get_all_knowledge().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].topic, "testing");
    }

    #[test]
    fn test_add_and_get_pattern() {
        let store = KnowledgeStore::new_in_memory().unwrap();
        let pattern = Pattern {
            id: None,
            pattern_type: PatternType::CodingStyle,
            description: "Uses anyhow::Result".to_string(),
            example: Some("fn foo() -> anyhow::Result<()>".to_string()),
            frequency: 1,
            created_at: None,
            last_seen: None,
        };
        let id = store.add_pattern(&pattern).unwrap();
        assert!(id > 0);

        let patterns = store.get_all_patterns().unwrap();
        assert_eq!(patterns.len(), 1);
    }

    #[test]
    fn test_update_pattern_frequency() {
        let store = KnowledgeStore::new_in_memory().unwrap();
        let pattern = Pattern {
            id: None,
            pattern_type: PatternType::Workflow,
            description: "TDD workflow".to_string(),
            example: None,
            frequency: 1,
            created_at: None,
            last_seen: None,
        };
        let id = store.add_pattern(&pattern).unwrap();
        store.update_pattern_frequency(id).unwrap();

        let patterns = store.get_all_patterns().unwrap();
        assert_eq!(patterns[0].frequency, 2);
    }

    #[test]
    fn test_find_similar_knowledge() {
        let store = KnowledgeStore::new_in_memory().unwrap();
        let entry = KnowledgeEntry {
            id: None,
            category: KnowledgeCategory::Fact,
            topic: "async_runtime".to_string(),
            content: "Project uses tokio".to_string(),
            source_session_id: None,
            confidence: 0.5,
            access_count: 0,
            created_at: None,
            last_accessed: None,
        };
        store.add_knowledge(&entry).unwrap();

        let found = store.find_similar_knowledge("async_runtime", "").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().topic, "async_runtime");

        let not_found = store.find_similar_knowledge("nonexistent", "").unwrap();
        assert!(not_found.is_none());
    }
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test learning::store`
Expected: All 6 tests PASS

- [ ] **Step 3: Commit**

```bash
git add src/learning/store.rs
git commit -m "feat(learning): add SQLite KnowledgeStore with CRUD operations"
```

---

### Task 3: BM25 Engine

**Files:**
- Create: `src/learning/bm25.rs`

- [ ] **Step 1: Write BM25 tests**

Create `src/learning/bm25.rs`:
```rust
//! BM25 scoring and TF-IDF indexing

use anyhow::Result;
use rusqlite::{Connection, params};
use std::collections::HashMap;

/// Common English stopwords to filter out
const STOPWORDS: &[&str] = &[
    "a", "an", "the", "is", "are", "was", "were", "be", "been", "being",
    "have", "has", "had", "do", "does", "did", "will", "would", "could",
    "should", "may", "might", "shall", "can", "to", "of", "in", "for",
    "on", "with", "at", "by", "from", "as", "into", "through", "during",
    "before", "after", "above", "below", "between", "out", "off", "over",
    "under", "again", "further", "then", "once", "here", "there", "when",
    "where", "why", "how", "all", "both", "each", "few", "more", "most",
    "other", "some", "such", "no", "nor", "not", "only", "own", "same",
    "so", "than", "too", "very", "just", "because", "but", "and", "or",
    "if", "while", "about", "up", "it", "its", "this", "that", "these",
    "those", "i", "me", "my", "we", "our", "you", "your", "he", "him",
    "his", "she", "her", "they", "them", "their", "what", "which", "who",
];

pub struct Bm25Engine {
    pub k1: f64,
    pub b: f64,
}

impl Bm25Engine {
    pub fn new(k1: f64, b: f64) -> Self {
        Self { k1, b }
    }

    /// Tokenize text: lowercase, split on non-alphanumeric, remove stopwords
    pub fn tokenize(text: &str) -> Vec<String> {
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty() && !STOPWORDS.contains(s) && s.len() > 1)
            .map(|s| s.to_string())
            .collect()
    }

    /// Index a document: compute term frequencies and store in SQLite
    pub fn index_document(
        &self,
        conn: &Connection,
        doc_type: &str,
        doc_id: i64,
        text: &str,
    ) -> Result<()> {
        let tokens = Self::tokenize(text);
        if tokens.is_empty() {
            return Ok(());
        }

        let doc_length = tokens.len() as f64;

        // Count term frequencies
        let mut tf_map: HashMap<String, usize> = HashMap::new();
        for token in &tokens {
            *tf_map.entry(token.clone()).or_insert(0) += 1;
        }

        // Store doc stats
        conn.execute(
            "INSERT OR REPLACE INTO bm25_doc_stats (doc_type, doc_id, doc_length) VALUES (?1, ?2, ?3)",
            params![doc_type, doc_id, tokens.len() as i64],
        )?;

        // Store term frequencies
        for (term, count) in &tf_map {
            let tf = *count as f64 / doc_length;
            conn.execute(
                "INSERT OR REPLACE INTO bm25_index (doc_type, doc_id, term, tf) VALUES (?1, ?2, ?3, ?4)",
                params![doc_type, doc_id, term, tf],
            )?;
        }

        Ok(())
    }

    /// Search for documents matching a query using BM25 scoring
    pub fn search(
        &self,
        conn: &Connection,
        query: &str,
        limit: usize,
    ) -> Result<Vec<(String, i64, f64)>> {
        let query_tokens = Self::tokenize(query);
        if query_tokens.is_empty() {
            return Ok(Vec::new());
        }

        // Get total document count
        let total_docs: f64 = conn.query_row(
            "SELECT COUNT(*) FROM bm25_doc_stats", [],
            |row| row.get::<_, i64>(0),
        ).unwrap_or(0) as f64;

        if total_docs == 0.0 {
            return Ok(Vec::new());
        }

        // Get average document length
        let avg_doc_length: f64 = conn.query_row(
            "SELECT AVG(doc_length) FROM bm25_doc_stats", [],
            |row| row.get::<_, f64>(0),
        ).unwrap_or(1.0);

        let mut scores: HashMap<(String, i64), f64> = HashMap::new();

        for term in &query_tokens {
            // Get document frequency (number of docs containing this term)
            let df: f64 = conn.query_row(
                "SELECT COUNT(DISTINCT doc_type || ':' || doc_id) FROM bm25_index WHERE term = ?1",
                params![term],
                |row| row.get::<_, i64>(0),
            ).unwrap_or(0) as f64;

            if df == 0.0 {
                continue;
            }

            // IDF component
            let idf = ((total_docs - df + 0.5) / (df + 0.5) + 1.0).ln();

            // Get all documents containing this term
            let mut stmt = conn.prepare(
                "SELECT i.doc_type, i.doc_id, i.tf, d.doc_length
                 FROM bm25_index i
                 JOIN bm25_doc_stats d ON i.doc_type = d.doc_type AND i.doc_id = d.doc_id
                 WHERE i.term = ?1"
            )?;

            let rows = stmt.query_map(params![term], |row| {
                let doc_type: String = row.get(0)?;
                let doc_id: i64 = row.get(1)?;
                let tf: f64 = row.get(2)?;
                let doc_length: f64 = row.get(3)?;
                Ok((doc_type, doc_id, tf, doc_length))
            })?;

            for row in rows.flatten() {
                let (doc_type, doc_id, tf, doc_length) = row;

                // BM25 score component for this term
                let numerator = tf * (self.k1 + 1.0);
                let denominator = tf + self.k1 * (1.0 - self.b + self.b * (doc_length / avg_doc_length));
                let score = idf * numerator / denominator;

                *scores.entry((doc_type, doc_id)).or_insert(0.0) += score;
            }
        }

        // Sort by score descending
        let mut results: Vec<(String, i64, f64)> = scores
            .into_iter()
            .map(|((doc_type, doc_id), score)| (doc_type, doc_id, score))
            .collect();
        results.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("
            CREATE TABLE bm25_index (
                id INTEGER PRIMARY KEY,
                doc_type TEXT NOT NULL,
                doc_id INTEGER NOT NULL,
                term TEXT NOT NULL,
                tf REAL NOT NULL,
                UNIQUE(doc_type, doc_id, term)
            );
            CREATE TABLE bm25_doc_stats (
                doc_type TEXT NOT NULL,
                doc_id INTEGER NOT NULL,
                doc_length INTEGER NOT NULL,
                PRIMARY KEY(doc_type, doc_id)
            );
        ").unwrap();
        conn
    }

    #[test]
    fn test_tokenize_basic() {
        let tokens = Bm25Engine::tokenize("Hello World! This is a test.");
        assert!(tokens.contains(&"hello".to_string()));
        assert!(tokens.contains(&"world".to_string()));
        assert!(tokens.contains(&"test".to_string()));
        assert!(!tokens.contains(&"is".to_string())); // stopword
        assert!(!tokens.contains(&"a".to_string())); // stopword
    }

    #[test]
    fn test_tokenize_empty() {
        let tokens = Bm25Engine::tokenize("");
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_index_and_search() {
        let conn = setup_db();
        let bm25 = Bm25Engine::new(1.2, 0.75);

        // Index some documents
        bm25.index_document(&conn, "knowledge", 1, "User prefers Rust for systems programming").unwrap();
        bm25.index_document(&conn, "knowledge", 2, "Python is used for data science tasks").unwrap();
        bm25.index_document(&conn, "knowledge", 3, "Rust has excellent memory safety guarantees").unwrap();

        // Search for "Rust"
        let results = bm25.search(&conn, "Rust programming", 5).unwrap();
        assert!(!results.is_empty());
        // Doc 1 and 3 should match (both mention Rust)
        let matching_ids: Vec<i64> = results.iter().map(|(_, id, _)| *id).collect();
        assert!(matching_ids.contains(&1));
        assert!(matching_ids.contains(&3));
    }

    #[test]
    fn test_search_empty_query() {
        let conn = setup_db();
        let bm25 = Bm25Engine::new(1.2, 0.75);
        let results = bm25.search(&conn, "", 5).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_no_matches() {
        let conn = setup_db();
        let bm25 = Bm25Engine::new(1.2, 0.75);
        bm25.index_document(&conn, "knowledge", 1, "Hello world").unwrap();
        let results = bm25.search(&conn, "xyz nonexistent", 5).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_index_empty_text() {
        let conn = setup_db();
        let bm25 = Bm25Engine::new(1.2, 0.75);
        // Should not error on empty text
        bm25.index_document(&conn, "knowledge", 1, "").unwrap();
        let results = bm25.search(&conn, "test", 5).unwrap();
        assert!(results.is_empty());
    }
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test learning::bm25`
Expected: All 5 tests PASS

- [ ] **Step 3: Commit**

```bash
git add src/learning/bm25.rs
git commit -m "feat(learning): add BM25 engine with tokenization and scoring"
```

---

### Task 4: KnowledgeStore + BM25 Integration

**Files:**
- Modify: `src/learning/store.rs`

- [ ] **Step 1: Add BM25 indexing methods to KnowledgeStore**

Add to `impl KnowledgeStore` in `src/learning/store.rs`:
```rust
    /// Index a knowledge entry in BM25
    pub fn index_knowledge_bm25(&self, knowledge_id: i64, text: &str, bm25: &super::bm25::Bm25Engine) -> Result<()> {
        bm25.index_document(&self.conn, "knowledge", knowledge_id, text)
    }

    /// Index a summary in BM25
    pub fn index_summary_bm25(&self, summary_id: i64, text: &str, bm25: &super::bm25::Bm25Engine) -> Result<()> {
        bm25.index_document(&self.conn, "summary", summary_id, text)
    }

    /// Index a pattern in BM25
    pub fn index_pattern_bm25(&self, pattern_id: i64, text: &str, bm25: &super::bm25::Bm25Engine) -> Result<()> {
        bm25.index_document(&self.conn, "pattern", pattern_id, text)
    }

    /// Search all document types using BM25
    pub fn bm25_search(&self, query: &str, limit: usize, bm25: &super::bm25::Bm25Engine) -> Result<Vec<super::types::RetrievedContext>> {
        let scored_docs = bm25.search(&self.conn, query, limit)?;
        let mut results = Vec::new();

        for (doc_type, doc_id, score) in scored_docs {
            match doc_type.as_str() {
                "summary" => {
                    let mut stmt = self.conn.prepare(
                        "SELECT id, session_id, summary, message_range_start, message_range_end, tokens_saved FROM conversation_summaries WHERE id = ?1"
                    )?;
                    if let Ok(mut rows) = stmt.query_map(params![doc_id], |row| {
                        Ok(ConversationSummary {
                            id: row.get(0)?,
                            session_id: row.get(1)?,
                            summary: row.get(2)?,
                            message_range_start: row.get(3)?,
                            message_range_end: row.get(4)?,
                            tokens_saved: row.get(5)?,
                            created_at: None,
                        })
                    }) {
                        if let Some(Ok(summary)) = rows.next() {
                            results.push(super::types::RetrievedContext::Summary {
                                text: summary.summary,
                                score,
                            });
                        }
                    }
                }
                "knowledge" => {
                    let mut stmt = self.conn.prepare(
                        "SELECT id, category, topic, content, source_session_id, confidence, access_count FROM knowledge_entries WHERE id = ?1"
                    )?;
                    if let Ok(mut rows) = stmt.query_map(params![doc_id], |row| {
                        Ok(KnowledgeEntry {
                            id: row.get(0)?,
                            category: KnowledgeCategory::from_str(&row.get::<_, String>(1)?),
                            topic: row.get(2)?,
                            content: row.get(3)?,
                            source_session_id: row.get(4)?,
                            confidence: row.get(5)?,
                            access_count: row.get(6)?,
                            created_at: None,
                            last_accessed: None,
                        })
                    }) {
                        if let Some(Ok(entry)) = rows.next() {
                            results.push(super::types::RetrievedContext::Knowledge {
                                entry,
                                score,
                            });
                        }
                    }
                }
                "pattern" => {
                    let mut stmt = self.conn.prepare(
                        "SELECT id, pattern_type, description, example, frequency FROM patterns WHERE id = ?1"
                    )?;
                    if let Ok(mut rows) = stmt.query_map(params![doc_id], |row| {
                        Ok(Pattern {
                            id: row.get(0)?,
                            pattern_type: PatternType::from_str(&row.get::<_, String>(1)?),
                            description: row.get(2)?,
                            example: row.get(3)?,
                            frequency: row.get(4)?,
                            created_at: None,
                            last_seen: None,
                        })
                    }) {
                        if let Some(Ok(pattern)) = rows.next() {
                            results.push(super::types::RetrievedContext::Pattern {
                                pattern,
                                score,
                            });
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(results)
    }
```

- [ ] **Step 2: Add integration test**

Add to the `tests` module in `store.rs`:
```rust
    #[test]
    fn test_bm25_integration() {
        use super::super::bm25::Bm25Engine;

        let store = KnowledgeStore::new_in_memory().unwrap();
        let bm25 = Bm25Engine::new(1.2, 0.75);

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
        store.index_knowledge_bm25(id, &entry.content, &bm25).unwrap();

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
```

- [ ] **Step 3: Run tests**

Run: `cargo test learning::store`
Expected: All tests PASS (including new integration test)

- [ ] **Step 4: Commit**

```bash
git add src/learning/store.rs
git commit -m "feat(learning): integrate BM25 search into KnowledgeStore"
```

---

### Task 5: Context Retriever

**Files:**
- Create: `src/learning/retriever.rs`

- [ ] **Step 1: Write ContextRetriever**

Create `src/learning/retriever.rs`:
```rust
//! Context retrieval for prompt injection

use anyhow::Result;
use super::store::KnowledgeStore;
use super::bm25::Bm25Engine;
use super::types::RetrievedContext;

pub struct ContextRetriever {
    store: KnowledgeStore,
    bm25: Bm25Engine,
    top_k: usize,
    max_tokens: usize,
}

impl ContextRetriever {
    pub fn new(store: KnowledgeStore, bm25_k1: f64, bm25_b: f64, top_k: usize, max_tokens: usize) -> Self {
        Self {
            store,
            bm25: Bm25Engine::new(bm25_k1, bm25_b),
            top_k,
            max_tokens,
        }
    }

    /// Retrieve relevant context for a query
    pub fn retrieve(&self, query: &str) -> Result<Vec<RetrievedContext>> {
        self.store.bm25_search(query, self.top_k, &self.bm25)
    }

    /// Format retrieved context for injection into system prompt
    pub fn format_for_prompt(&self, contexts: &[RetrievedContext]) -> String {
        if contexts.is_empty() {
            return String::new();
        }

        let mut lines = vec![
            "## Relevant Context".to_string(),
            "".to_string(),
            "The following information from past sessions may be relevant:".to_string(),
            "".to_string(),
        ];

        let mut total_chars = 0;
        let max_chars = self.max_tokens * 4; // rough estimate: 1 token ≈ 4 chars

        for ctx in contexts {
            if total_chars >= max_chars {
                break;
            }

            let entry = match ctx {
                RetrievedContext::Summary { text, .. } => {
                    format!("- **Past Session Summary**: {}", text)
                }
                RetrievedContext::Knowledge { entry, .. } => {
                    format!("- **{}** ({}): {}", entry.topic, entry.category.as_str(), entry.content)
                }
                RetrievedContext::Pattern { pattern, .. } => {
                    format!("- **Pattern** ({}): {}", pattern.pattern_type.as_str(), pattern.description)
                }
            };

            total_chars += entry.len();
            lines.push(entry);
        }

        lines.join("\n")
    }

    /// Get reference to the underlying store
    pub fn store(&self) -> &KnowledgeStore {
        &self.store
    }

    /// Get mutable reference to the underlying store
    pub fn store_mut(&mut self) -> &mut KnowledgeStore {
        &mut self.store
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::*;

    fn setup_retriever() -> ContextRetriever {
        let store = KnowledgeStore::new_in_memory().unwrap();
        let bm25 = Bm25Engine::new(1.2, 0.75);

        // Add some test data
        let entry1 = KnowledgeEntry {
            id: None, category: KnowledgeCategory::Fact,
            topic: "testing".to_string(),
            content: "User prefers integration tests over unit tests".to_string(),
            source_session_id: None, confidence: 0.8, access_count: 0,
            created_at: None, last_accessed: None,
        };
        let id1 = store.add_knowledge(&entry1).unwrap();
        store.index_knowledge_bm25(id1, &entry1.content, &bm25).unwrap();

        let entry2 = KnowledgeEntry {
            id: None, category: KnowledgeCategory::Preference,
            topic: "async".to_string(),
            content: "Project uses tokio for async runtime".to_string(),
            source_session_id: None, confidence: 0.7, access_count: 0,
            created_at: None, last_accessed: None,
        };
        let id2 = store.add_knowledge(&entry2).unwrap();
        store.index_knowledge_bm25(id2, &entry2.content, &bm25).unwrap();

        ContextRetriever::new(store, 1.2, 0.75, 5, 2000)
    }

    #[test]
    fn test_retrieve_relevant() {
        let retriever = setup_retriever();
        let results = retriever.retrieve("testing").unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_retrieve_empty_query() {
        let retriever = setup_retriever();
        let results = retriever.retrieve("").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_format_for_prompt() {
        let retriever = setup_retriever();
        let results = retriever.retrieve("testing").unwrap();
        let formatted = retriever.format_for_prompt(&results);
        assert!(formatted.contains("## Relevant Context"));
        assert!(formatted.contains("testing"));
    }

    #[test]
    fn test_format_empty() {
        let retriever = setup_retriever();
        let formatted = retriever.format_for_prompt(&[]);
        assert!(formatted.is_empty());
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test learning::retriever`
Expected: All 4 tests PASS

- [ ] **Step 3: Commit**

```bash
git add src/learning/retriever.rs
git commit -m "feat(learning): add ContextRetriever with BM25-based prompt injection"
```

---

### Task 6: SmartCompactor

**Files:**
- Create: `src/learning/summarizer.rs`
- Modify: `src/engine/compact.rs` (add delegation)

- [ ] **Step 1: Write SmartCompactor**

Create `src/learning/summarizer.rs`:
```rust
//! Smart compaction with LLM-powered summarization

use anyhow::Result;
use crate::api::client::{ApiClient, ApiRequest};
use crate::engine::messages::{ConversationMessage, MessageContent, MessageRole};
use super::store::KnowledgeStore;
use super::bm25::Bm25Engine;

pub struct SmartCompactor {
    api_client: ApiClient,
    segment_size: usize,
    token_threshold: u32,
    model: String,
}

impl SmartCompactor {
    pub fn new(api_client: ApiClient, model: String, segment_size: usize, token_threshold: u32) -> Self {
        Self {
            api_client,
            segment_size,
            token_threshold,
            model,
        }
    }

    /// Compact messages if needed using LLM summarization
    pub async fn compact_if_needed(
        &self,
        messages: Vec<ConversationMessage>,
        session_id: &str,
        store: &KnowledgeStore,
        bm25: &Bm25Engine,
    ) -> Result<(Vec<ConversationMessage>, bool)> {
        let estimated_tokens = estimate_tokens(&messages);

        if estimated_tokens < self.token_threshold && messages.len() <= 50 {
            return Ok((messages, false));
        }

        // Split into segments
        let segments: Vec<&[ConversationMessage]> = messages
            .chunks(self.segment_size)
            .collect();

        if segments.len() <= 1 {
            return Ok((messages, false));
        }

        let mut result = Vec::new();
        let mut was_compacted = false;

        // Summarize all segments except the last one
        for (i, segment) in segments.iter().enumerate() {
            if i < segments.len() - 1 {
                // Old segment — summarize
                match self.summarize_segment(segment).await {
                    Ok(summary_text) => {
                        // Store summary in DB
                        let summary = super::types::ConversationSummary {
                            id: None,
                            session_id: session_id.to_string(),
                            summary: summary_text.clone(),
                            message_range_start: (i * self.segment_size) as i64,
                            message_range_end: ((i + 1) * self.segment_size) as i64,
                            tokens_saved: estimate_tokens(segment) as i64,
                            created_at: None,
                        };
                        if let Ok(id) = store.add_summary(&summary) {
                            let _ = store.index_summary_bm25(id, &summary_text, bm25);
                        }

                        // Replace segment with summary message
                        result.push(ConversationMessage {
                            role: MessageRole::System,
                            content: vec![MessageContent::Text {
                                text: format!("[SESSION SUMMARY: {}]", summary_text),
                            }],
                            tool_uses: Vec::new(),
                        });
                        was_compacted = true;
                    }
                    Err(_) => {
                        // Fallback: keep segment as-is
                        result.extend(segment.to_vec());
                    }
                }
            } else {
                // Last segment — keep verbatim
                result.extend(segment.to_vec());
            }
        }

        Ok((result, was_compacted))
    }

    async fn summarize_segment(&self, messages: &[ConversationMessage]) -> Result<String> {
        let conversation_text = messages_to_text(messages);

        let prompt = format!(
            "Summarize this conversation segment concisely. Preserve:\n\
             - Key decisions made\n\
             - Files modified or created\n\
             - Bugs found and solutions\n\
             - User preferences expressed\n\
             - Technical facts learned\n\n\
             Conversation:\n{}\n\n\
             Summary:",
            conversation_text
        );

        let request = ApiRequest {
            model: self.model.clone(),
            messages: vec![ConversationMessage::user_text(prompt)],
            system_prompt: Some("You are a concise summarizer. Output only the summary, no preamble.".to_string()),
            max_tokens: 1000,
            tools: Vec::new(),
        };

        let response = self.api_client.send_message(request).await?;

        let summary = response.content.iter()
            .filter_map(|c| match c {
                MessageContent::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");

        Ok(summary)
    }
}

fn messages_to_text(messages: &[ConversationMessage]) -> String {
    let mut parts = Vec::new();
    for msg in messages {
        let role = match msg.role {
            MessageRole::User => "User",
            MessageRole::Assistant => "Assistant",
            MessageRole::System => "System",
        };
        for content in &msg.content {
            match content {
                MessageContent::Text { text } => {
                    parts.push(format!("{}: {}", role, text));
                }
                MessageContent::ToolUse { name, input, .. } => {
                    parts.push(format!("{}: [Tool Call: {} {:?}]", role, name, input));
                }
                MessageContent::ToolResult { content, is_error, .. } => {
                    if *is_error {
                        parts.push(format!("[Tool Error]: {}", content));
                    } else {
                        parts.push(format!("[Tool Result]: {}", content.chars().take(200).collect::<String>()));
                    }
                }
            }
        }
    }
    parts.join("\n")
}

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
    ((total_chars as f32 * 0.25) as u32) + (messages.len() * 10) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens() {
        let messages = vec![
            ConversationMessage::user_text("Hello, this is a test message"),
        ];
        let tokens = estimate_tokens(&messages);
        assert!(tokens > 0);
    }

    #[test]
    fn test_messages_to_text() {
        let messages = vec![
            ConversationMessage::user_text("Hello"),
            ConversationMessage::assistant_text("Hi there!"),
        ];
        let text = messages_to_text(&messages);
        assert!(text.contains("User: Hello"));
        assert!(text.contains("Assistant: Hi there!"));
    }
}
```

- [ ] **Step 2: Add delegation in compact.rs**

Add to the top of `src/engine/compact.rs`:
```rust
/// Check if smart compaction should be used instead of naive compaction
pub fn should_use_smart_compaction(messages: &[ConversationMessage], token_threshold: u32) -> bool {
    estimate_tokens(messages) >= token_threshold || messages.len() > 50
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test learning::summarizer`
Expected: All tests PASS

- [ ] **Step 4: Commit**

```bash
git add src/learning/summarizer.rs src/engine/compact.rs
git commit -m "feat(learning): add SmartCompactor with LLM-powered summarization"
```

---

### Task 7: LearningEngine (Extractor)

**Files:**
- Create: `src/learning/extractor.rs`

- [ ] **Step 1: Write LearningEngine**

Create `src/learning/extractor.rs`:
```rust
//! Learning engine for session-end extraction

use anyhow::Result;
use regex::Regex;
use crate::api::client::{ApiClient, ApiRequest};
use crate::engine::messages::{ConversationMessage, MessageContent};
use super::types::*;
use super::store::KnowledgeStore;
use super::bm25::Bm25Engine;

pub struct LearningEngine {
    store: KnowledgeStore,
    bm25: Bm25Engine,
    api_client: Option<ApiClient>,
    model: String,
    use_llm_extraction: bool,
}

impl LearningEngine {
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
            bm25: Bm25Engine::new(bm25_k1, bm25_b),
            api_client,
            model,
            use_llm_extraction,
        }
    }

    /// Process a completed session: extract markers + optional LLM extraction
    pub async fn process_session(
        &self,
        messages: &[ConversationMessage],
        session_id: &str,
    ) -> Result<LearningResult> {
        let mut result = LearningResult::default();

        // Pass 1: Parse in-conversation markers
        let markers = self.parse_markers(messages);
        for marker in markers {
            if let Some(entry) = marker_to_knowledge(&marker, session_id) {
                match self.store.find_similar_knowledge(&entry.topic, &entry.content)? {
                    Some(existing) => {
                        // Boost existing knowledge confidence
                        if let Some(id) = existing.id {
                            self.store.boost_knowledge_confidence(id, 0.1)?;
                        }
                    }
                    None => {
                        // Store new knowledge
                        let id = self.store.add_knowledge(&entry)?;
                        self.store.index_knowledge_bm25(id, &entry.content, &self.bm25)?;
                        result.knowledge_extracted += 1;
                    }
                }
            }
        }

        // Pass 2: Optional LLM extraction
        if self.use_llm_extraction {
            if let Some(ref client) = self.api_client {
                match self.llm_extract(messages).await {
                    Ok(items) => {
                        for item in items {
                            match item {
                                ExtractedItem::Knowledge(entry) => {
                                    match self.store.find_similar_knowledge(&entry.topic, &entry.content)? {
                                        Some(existing) => {
                                            if let Some(id) = existing.id {
                                                self.store.boost_knowledge_confidence(id, 0.1)?;
                                            }
                                        }
                                        None => {
                                            let entry_with_session = KnowledgeEntry {
                                                source_session_id: Some(session_id.to_string()),
                                                ..entry
                                            };
                                            let id = self.store.add_knowledge(&entry_with_session)?;
                                            self.store.index_knowledge_bm25(id, &entry_with_session.content, &self.bm25)?;
                                            result.knowledge_extracted += 1;
                                        }
                                    }
                                }
                                ExtractedItem::Pattern(pattern) => {
                                    let id = self.store.add_pattern(&pattern)?;
                                    let index_text = format!("{} {}", pattern.description, pattern.example.as_deref().unwrap_or(""));
                                    self.store.index_pattern_bm25(id, &index_text, &self.bm25)?;
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

    /// Parse <!-- LEARN: ... --> markers from conversation
    fn parse_markers(&self, messages: &[ConversationMessage]) -> Vec<LearningMarker> {
        let re = Regex::new(r#"<!--\s*LEARN:\s*category="([^"]+)"\s*topic="([^"]+)"\s*content="([^"]+)"\s*-->"#).unwrap();
        let mut markers = Vec::new();

        for msg in messages {
            for content in &msg.content {
                if let MessageContent::Text { text } = content {
                    for cap in re.captures_iter(text) {
                        markers.push(LearningMarker {
                            category: cap[1].to_string(),
                            topic: cap[2].to_string(),
                            content: cap[3].to_string(),
                        });
                    }
                }
            }
        }

        markers
    }

    /// Use LLM to extract knowledge and patterns from conversation
    async fn llm_extract(&self, messages: &[ConversationMessage]) -> Result<Vec<ExtractedItem>> {
        let client = self.api_client.as_ref().unwrap();
        let conversation_text = messages_to_text(messages);

        let prompt = format!(
            "Analyze this conversation and extract:\n\
             1. Knowledge facts (technical decisions, solutions, preferences)\n\
             2. Behavioral patterns (recurring workflows, tool preferences)\n\n\
             For each item, output a JSON line:\n\
             {{\"type\": \"knowledge\", \"category\": \"fact|decision|solution|preference\", \"topic\": \"...\", \"content\": \"...\"}}\n\
             {{\"type\": \"pattern\", \"pattern_type\": \"coding_style|workflow|tool_preference\", \"description\": \"...\", \"example\": \"...\"}}\n\n\
             Only extract genuinely useful information. Output one JSON object per line, no other text.\n\n\
             Conversation:\n{}",
            conversation_text
        );

        let request = ApiRequest {
            model: self.model.clone(),
            messages: vec![ConversationMessage::user_text(prompt)],
            system_prompt: Some("You are a knowledge extractor. Output only JSON objects, one per line.".to_string()),
            max_tokens: 2000,
            tools: Vec::new(),
        };

        let response = client.send_message(request).await?;
        let text = response.content.iter()
            .filter_map(|c| match c {
                MessageContent::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");

        let mut items = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || !line.starts_with('{') {
                continue;
            }
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
                if let Some(item) = parse_extracted_item(&value) {
                    items.push(item);
                }
            }
        }

        Ok(items)
    }
}

enum ExtractedItem {
    Knowledge(KnowledgeEntry),
    Pattern(Pattern),
}

fn parse_extracted_item(value: &serde_json::Value) -> Option<ExtractedItem> {
    let item_type = value.get("type")?.as_str()?;

    match item_type {
        "knowledge" => {
            Some(ExtractedItem::Knowledge(KnowledgeEntry {
                id: None,
                category: KnowledgeCategory::from_str(value.get("category")?.as_str()?),
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
            Some(ExtractedItem::Pattern(Pattern {
                id: None,
                pattern_type: PatternType::from_str(value.get("pattern_type")?.as_str()?),
                description: value.get("description")?.as_str()?.to_string(),
                example: value.get("example").and_then(|v| v.as_str()).map(|s| s.to_string()),
                frequency: 1,
                created_at: None,
                last_seen: None,
            }))
        }
        _ => None,
    }
}

fn marker_to_knowledge(marker: &LearningMarker, session_id: &str) -> Option<KnowledgeEntry> {
    Some(KnowledgeEntry {
        id: None,
        category: KnowledgeCategory::from_str(&marker.category),
        topic: marker.topic.clone(),
        content: marker.content.clone(),
        source_session_id: Some(session_id.to_string()),
        confidence: 0.7,
        access_count: 0,
        created_at: None,
        last_accessed: None,
    })
}

fn messages_to_text(messages: &[ConversationMessage]) -> String {
    let mut parts = Vec::new();
    for msg in messages {
        let role = match msg.role {
            crate::engine::messages::MessageRole::User => "User",
            crate::engine::messages::MessageRole::Assistant => "Assistant",
            crate::engine::messages::MessageRole::System => "System",
        };
        for content in &msg.content {
            match content {
                MessageContent::Text { text } => {
                    parts.push(format!("{}: {}", role, text));
                }
                MessageContent::ToolUse { name, .. } => {
                    parts.push(format!("{}: [Tool: {}]", role, name));
                }
                MessageContent::ToolResult { content, is_error, .. } => {
                    if *is_error {
                        parts.push(format!("[Error]: {}", content.chars().take(200).collect::<String>()));
                    } else {
                        parts.push(format!("[Result]: {}", content.chars().take(200).collect::<String>()));
                    }
                }
            }
        }
    }
    parts.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_markers() {
        let engine = create_test_engine();
        let messages = vec![
            ConversationMessage::user_text(
                "I prefer integration tests\n<!-- LEARN: category=\"preference\" topic=\"testing\" content=\"User prefers integration tests over unit tests\" -->"
            ),
        ];
        let markers = engine.parse_markers(&messages);
        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].category, "preference");
        assert_eq!(markers[0].topic, "testing");
        assert_eq!(markers[0].content, "User prefers integration tests over unit tests");
    }

    #[test]
    fn test_parse_multiple_markers() {
        let engine = create_test_engine();
        let messages = vec![
            ConversationMessage::user_text(
                "Using tokio\n<!-- LEARN: category=\"fact\" topic=\"async\" content=\"Project uses tokio\" -->\nAlso <!-- LEARN: category=\"decision\" topic=\"error_handling\" content=\"Using anyhow for errors\" -->"
            ),
        ];
        let markers = engine.parse_markers(&messages);
        assert_eq!(markers.len(), 2);
    }

    #[test]
    fn test_parse_no_markers() {
        let engine = create_test_engine();
        let messages = vec![
            ConversationMessage::user_text("Just a normal message"),
        ];
        let markers = engine.parse_markers(&messages);
        assert!(markers.is_empty());
    }

    #[test]
    fn test_marker_to_knowledge() {
        let marker = LearningMarker {
            category: "fact".to_string(),
            topic: "testing".to_string(),
            content: "Uses cargo test".to_string(),
        };
        let entry = marker_to_knowledge(&marker, "session-1").unwrap();
        assert_eq!(entry.topic, "testing");
        assert_eq!(entry.confidence, 0.7);
    }

    #[test]
    fn test_parse_extracted_item_knowledge() {
        let value = serde_json::json!({
            "type": "knowledge",
            "category": "fact",
            "topic": "testing",
            "content": "Uses cargo test"
        });
        let item = parse_extracted_item(&value).unwrap();
        match item {
            ExtractedItem::Knowledge(entry) => {
                assert_eq!(entry.topic, "testing");
            }
            _ => panic!("Expected Knowledge"),
        }
    }

    #[test]
    fn test_parse_extracted_item_pattern() {
        let value = serde_json::json!({
            "type": "pattern",
            "pattern_type": "coding_style",
            "description": "Uses anyhow::Result",
            "example": "fn foo() -> anyhow::Result<()>"
        });
        let item = parse_extracted_item(&value).unwrap();
        match item {
            ExtractedItem::Pattern(pattern) => {
                assert_eq!(pattern.description, "Uses anyhow::Result");
            }
            _ => panic!("Expected Pattern"),
        }
    }

    fn create_test_engine() -> LearningEngine {
        let store = KnowledgeStore::new_in_memory().unwrap();
        LearningEngine::new(store, 1.2, 0.75, None, "test".to_string(), false)
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test learning::extractor`
Expected: All 7 tests PASS

- [ ] **Step 3: Commit**

```bash
git add src/learning/extractor.rs
git commit -m "feat(learning): add LearningEngine with marker parsing and LLM extraction"
```

---

### Task 8: Skill Generator

**Files:**
- Create: `src/learning/skill_gen.rs`

- [ ] **Step 1: Write SkillGenerator**

Create `src/learning/skill_gen.rs`:
```rust
//! Auto-generate skills from high-frequency patterns

use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use super::types::Pattern;

pub struct SkillGenerator {
    skills_dir: PathBuf,
    promotion_threshold: i64,
}

impl SkillGenerator {
    pub fn new(project_dir: &Path, promotion_threshold: i64) -> Self {
        Self {
            skills_dir: project_dir.join(".rust_harness").join("skills").join("auto_generated"),
            promotion_threshold,
        }
    }

    /// Check if a pattern should be promoted to a skill
    pub fn should_generate(&self, pattern: &Pattern) -> bool {
        pattern.frequency >= self.promotion_threshold
    }

    /// Generate a .skill file from a pattern
    pub fn generate_skill(&self, pattern: &Pattern) -> Result<PathBuf> {
        fs::create_dir_all(&self.skills_dir)?;

        let slug = slugify(&pattern.description);
        let filename = format!("auto_{}_{}.md", pattern.pattern_type.as_str(), slug);
        let path = self.skills_dir.join(&filename);

        let example_section = match &pattern.example {
            Some(ex) => format!("\n## Example\n\n```\n{}\n```", ex),
            None => String::new(),
        };

        let content = format!(
            "---\n\
             name: auto_{}_{}\n\
             description: Auto-generated from observed pattern: {}\n\
             auto_generated: true\n\
             ---\n\n\
             # Auto-Generated Skill\n\n\
             > This skill was automatically generated from observed conversation patterns.\n\
             > Review and edit before relying on it.\n\n\
             ## Pattern\n\n\
             **Type:** {}\n\
             **Description:** {}\n\
             **Frequency:** {} observations\n\
             {}\n\
             ## When to Apply\n\n\
             Apply this pattern when the user's request involves {}.\n",
            pattern.pattern_type.as_str(),
            slug,
            pattern.description,
            pattern.pattern_type.as_str(),
            pattern.description,
            pattern.frequency,
            example_section,
            pattern.description.to_lowercase()
        );

        fs::write(&path, content)?;
        Ok(path)
    }
}

fn slugify(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .split('_')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_should_generate_above_threshold() {
        let temp = TempDir::new().unwrap();
        let gen = SkillGenerator::new(temp.path(), 3);
        let pattern = Pattern {
            id: None,
            pattern_type: super::super::types::PatternType::CodingStyle,
            description: "Uses anyhow".to_string(),
            example: None,
            frequency: 3,
            created_at: None,
            last_seen: None,
        };
        assert!(gen.should_generate(&pattern));
    }

    #[test]
    fn test_should_generate_below_threshold() {
        let temp = TempDir::new().unwrap();
        let gen = SkillGenerator::new(temp.path(), 3);
        let pattern = Pattern {
            id: None,
            pattern_type: super::super::types::PatternType::CodingStyle,
            description: "Uses anyhow".to_string(),
            example: None,
            frequency: 2,
            created_at: None,
            last_seen: None,
        };
        assert!(!gen.should_generate(&pattern));
    }

    #[test]
    fn test_generate_skill() {
        let temp = TempDir::new().unwrap();
        let gen = SkillGenerator::new(temp.path(), 3);
        let pattern = Pattern {
            id: None,
            pattern_type: super::super::types::PatternType::Workflow,
            description: "TDD workflow".to_string(),
            example: Some("write test first, then implement".to_string()),
            frequency: 5,
            created_at: None,
            last_seen: None,
        };
        let path = gen.generate_skill(&pattern).unwrap();
        assert!(path.exists());

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("auto_generated: true"));
        assert!(content.contains("TDD workflow"));
        assert!(content.contains("write test first"));
    }

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Hello World"), "hello_world");
        assert_eq!(slugify("TDD workflow!"), "tdd_workflow");
        assert_eq!(slugify("  spaces  "), "spaces");
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test learning::skill_gen`
Expected: All 4 tests PASS

- [ ] **Step 3: Commit**

```bash
git add src/learning/skill_gen.rs
git commit -m "feat(learning): add SkillGenerator for auto-generating skills from patterns"
```

---

### Task 9: Integration — Wire into QueryEngine

**Files:**
- Modify: `src/engine/query.rs`
- Modify: `src/prompts/system_prompt.rs`

- [ ] **Step 1: Add learning components to QueryEngine**

In `src/engine/query.rs`, add fields to `QueryEngine`:
```rust
use crate::learning::retriever::ContextRetriever;
use crate::learning::store::KnowledgeStore;
use crate::learning::bm25::Bm25Engine;
use crate::learning::summarizer::SmartCompactor;
use crate::learning::extractor::LearningEngine;

// Add to QueryEngine struct:
pub context_retriever: Option<ContextRetriever>,
pub smart_compactor: Option<SmartCompactor>,
pub learning_engine: Option<LearningEngine>,
```

- [ ] **Step 2: Initialize learning components in QueryEngine::new**

Add initialization logic after the existing setup:
```rust
// Initialize learning system if enabled
let (context_retriever, smart_compactor, learning_engine) = if settings.learning.enabled {
    let db_path = settings.learning.knowledge_db_path.as_ref()
        .map(|p| std::path::PathBuf::from(p))
        .unwrap_or_else(|| {
            cwd.join(".rust_harness").join("knowledge.db")
        });

    // Ensure parent directory exists
    if let Some(parent) = db_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    match KnowledgeStore::new(&db_path) {
        Ok(store) => {
            let retriever = ContextRetriever::new(
                store,
                settings.learning.bm25_k1,
                settings.learning.bm25_b,
                settings.learning.bm25_top_k,
                settings.learning.max_context_injection_tokens,
            );

            let compactor = SmartCompactor::new(
                ApiClient::new(api_key.clone(), settings.base_url.clone()),
                settings.model.clone(),
                settings.learning.summary_segment_size,
                settings.learning.summary_token_threshold,
            );

            let extractor = LearningEngine::new(
                KnowledgeStore::new(&db_path).unwrap(), // separate connection for session-end
                settings.learning.bm25_k1,
                settings.learning.bm25_b,
                Some(ApiClient::new(api_key.clone(), settings.base_url.clone())),
                settings.model.clone(),
                settings.learning.session_end_extraction,
            );

            (Some(retriever), Some(compactor), Some(extractor))
        }
        Err(e) => {
            tracing::warn!("Failed to initialize learning system: {}", e);
            (None, None, None)
        }
    }
} else {
    (None, None, None)
};
```

- [ ] **Step 3: Integrate context retrieval into run_loop**

In `run_loop()`, before building the API request, inject retrieved context:
```rust
// After getting messages, before building request:
let mut system_prompt = self.settings.system_prompt.clone();

if let Some(ref retriever) = self.context_retriever {
    // Get the current user message for retrieval query
    let current_query = messages.last()
        .filter(|m| m.role == MessageRole::User)
        .and_then(|m| m.content.first())
        .and_then(|c| match c {
            MessageContent::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .unwrap_or("");

    if !current_query.is_empty() {
        if let Ok(contexts) = retriever.retrieve(current_query) {
            let context_section = retriever.format_for_prompt(&contexts);
            if !context_section.is_empty() {
                system_prompt = Some(match system_prompt {
                    Some(mut prompt) => {
                        prompt.push_str("\n\n");
                        prompt.push_str(&context_section);
                        prompt
                    }
                    None => context_section,
                });
            }
        }
    }
}
```

- [ ] **Step 4: Add system prompt instructions for learning markers**

In `src/prompts/system_prompt.rs`, append to the base prompt:
```rust
// Add after the existing base prompt:
"\n\nWhen you observe user preferences, coding patterns, or important facts worth remembering, \
embed a learning marker in your response:\n\
<!-- LEARN: category=\"<category>\" topic=\"<topic>\" content=\"<content>\" -->\n\n\
Categories: fact, decision, solution, preference\n\
Only mark genuinely useful information, not trivial observations."
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully

- [ ] **Step 6: Commit**

```bash
git add src/engine/query.rs src/prompts/system_prompt.rs
git commit -m "feat(learning): integrate context retrieval and learning into QueryEngine"
```

---

### Task 10: Integration — Session End Trigger

**Files:**
- Modify: `src/ui/repl.rs`

- [ ] **Step 1: Add learning engine to REPL context**

In the REPL module, ensure the `QueryEngine` is accessible and add session-end learning trigger.

Find the session save logic (when user exits or session ends) and add:
```rust
// After saving session, trigger learning engine
if let Some(ref learning_engine) = query_engine.learning_engine {
    let messages = query_engine.get_messages().await;
    if !messages.is_empty() {
        match learning_engine.process_session(&messages, &session_id).await {
            Ok(result) => {
                if result.knowledge_extracted > 0 || result.patterns_extracted > 0 {
                    tracing::info!(
                        "Learning: extracted {} knowledge entries, {} patterns, {} skills",
                        result.knowledge_extracted,
                        result.patterns_extracted,
                        result.skills_generated
                    );
                }
            }
            Err(e) => {
                tracing::warn!("Learning engine error: {}", e);
            }
        }
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully

- [ ] **Step 3: Commit**

```bash
git add src/ui/repl.rs
git commit -m "feat(learning): trigger learning engine at session end"
```

---

### Task 11: End-to-End Test

**Files:**
- Create: `src/learning/integration_test.rs` (or inline test module)

- [ ] **Step 1: Write end-to-end integration test**

Add a test that exercises the full flow:
```rust
#[cfg(test)]
mod e2e_tests {
    use super::*;
    use crate::learning::store::KnowledgeStore;
    use crate::learning::bm25::Bm25Engine;
    use crate::learning::retriever::ContextRetriever;
    use crate::learning::extractor::LearningEngine;
    use crate::learning::types::*;
    use crate::engine::messages::ConversationMessage;

    #[tokio::test]
    async fn test_full_learning_cycle() {
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
        retriever.store().index_knowledge_bm25(id, &entry.content, &Bm25Engine::new(1.2, 0.75)).unwrap();

        // 3. Retrieve context for a query
        let results = retriever.retrieve("testing approach").unwrap();
        assert!(!results.is_empty());

        // 4. Format for prompt
        let formatted = retriever.format_for_prompt(&results);
        assert!(formatted.contains("integration tests"));

        // 5. Simulate session with learning markers
        let messages = vec![
            ConversationMessage::user_text(
                "Let's write tests\n<!-- LEARN: category=\"decision\" topic=\"test_framework\" content=\"Decided to use cargo-nextest for faster test execution\" -->"
            ),
        ];

        // 6. Process session
        let store2 = KnowledgeStore::new_in_memory().unwrap();
        let engine = LearningEngine::new(store2, 1.2, 0.75, None, "test".to_string(), false);
        let result = engine.process_session(&messages, "test-session").await.unwrap();

        assert_eq!(result.knowledge_extracted, 1);
    }
}
```

- [ ] **Step 2: Run all learning tests**

Run: `cargo test learning`
Expected: All tests PASS

- [ ] **Step 3: Run full test suite**

Run: `cargo test`
Expected: All existing tests still pass, no regressions

- [ ] **Step 4: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: No warnings

- [ ] **Step 5: Final commit**

```bash
git add -A
git commit -m "feat(learning): complete self-learning system with end-to-end tests"
```

---

## Summary

| Task | Component | Files | Tests |
|------|-----------|-------|-------|
| 1 | Setup & Config | Cargo.toml, main.rs, settings.rs, learning/mod.rs, learning/types.rs | Compile check |
| 2 | KnowledgeStore | learning/store.rs | 6 tests |
| 3 | BM25 Engine | learning/bm25.rs | 5 tests |
| 4 | Store+BM25 Integration | learning/store.rs | 1 test |
| 5 | ContextRetriever | learning/retriever.rs | 4 tests |
| 6 | SmartCompactor | learning/summarizer.rs | 2 tests |
| 7 | LearningEngine | learning/extractor.rs | 7 tests |
| 8 | SkillGenerator | learning/skill_gen.rs | 4 tests |
| 9 | QueryEngine Integration | engine/query.rs, prompts/system_prompt.rs | Compile check |
| 10 | Session End Trigger | ui/repl.rs | Compile check |
| 11 | End-to-End Test | Integration test | 1 test + regression |

**Total new tests:** ~30
**Total new files:** 8
**Modified files:** 5
