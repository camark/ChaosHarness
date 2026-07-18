//! BM25 ranking engine for full-text search over knowledge store documents.

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::collections::{HashMap, HashSet};

/// Stopwords to filter out during tokenization.
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

/// BM25 ranking engine.
///
/// Uses the Okapi BM25 formula to score documents against a query.
/// Documents are stored in SQLite tables `bm25_index` and `bm25_doc_stats`.
pub struct Bm25Engine {
    /// Term frequency saturation parameter (typically 1.2–2.0).
    pub k1: f64,
    /// Length normalization parameter (typically 0.75).
    pub b: f64,
}

impl Bm25Engine {
    /// Create a new BM25 engine with default parameters.
    pub fn new() -> Self {
        Self { k1: 1.2, b: 0.75 }
    }

    /// Create a BM25 engine with custom parameters.
    pub fn with_params(k1: f64, b: f64) -> Self {
        Self { k1, b }
    }

    /// Tokenize text: lowercase, split on non-alphanumeric, remove stopwords
    /// and single-character tokens.
    pub fn tokenize(text: &str) -> Vec<String> {
        let lower = text.to_lowercase();
        let stopword_set: HashSet<&str> = STOPWORDS.iter().copied().collect();

        lower
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty() && s.len() > 1 && !stopword_set.contains(*s))
            .map(|s| s.to_string())
            .collect()
    }

    /// Index a document: tokenize the text, compute term frequencies, and
    /// store them in the `bm25_index` and `bm25_doc_stats` tables.
    pub fn index_document(
        &self,
        conn: &Connection,
        doc_type: &str,
        doc_id: i64,
        text: &str,
    ) -> Result<()> {
        let tokens = Self::tokenize(text);
        let doc_length = tokens.len() as i64;

        // Compute term frequencies
        let mut tf_map: HashMap<String, f64> = HashMap::new();
        for token in &tokens {
            *tf_map.entry(token.clone()).or_insert(0.0) += 1.0;
        }

        // Store doc stats (upsert)
        conn.execute(
            "INSERT OR REPLACE INTO bm25_doc_stats (doc_type, doc_id, doc_length)
             VALUES (?1, ?2, ?3)",
            params![doc_type, doc_id, doc_length],
        )
        .context("Failed to insert bm25_doc_stats")?;

        // Remove old index entries for this document
        conn.execute(
            "DELETE FROM bm25_index WHERE doc_type = ?1 AND doc_id = ?2",
            params![doc_type, doc_id],
        )
        .context("Failed to delete old bm25_index entries")?;

        // Insert new index entries
        {
            let mut stmt = conn
                .prepare(
                    "INSERT INTO bm25_index (doc_type, doc_id, term, tf)
                     VALUES (?1, ?2, ?3, ?4)",
                )
                .context("Failed to prepare bm25_index insert")?;

            for (term, tf) in &tf_map {
                stmt.execute(params![doc_type, doc_id, term, tf])
                    .with_context(|| format!("Failed to insert bm25_index entry for term '{term}'"))?;
            }
        }

        Ok(())
    }

    /// Search for documents matching the query using BM25 scoring.
    ///
    /// Returns a list of `(doc_type, doc_id, score)` tuples sorted by
    /// descending score, limited to `limit` results.
    pub fn search(
        &self,
        conn: &Connection,
        query: &str,
        limit: usize,
    ) -> Result<Vec<(String, i64, f64)>> {
        let query_terms = Self::tokenize(query);
        if query_terms.is_empty() {
            return Ok(Vec::new());
        }

        // Total number of indexed documents
        let n: f64 = conn
            .query_row("SELECT COUNT(*) FROM bm25_doc_stats", [], |row| row.get(0))
            .context("Failed to count total documents")?;

        if n == 0.0 {
            return Ok(Vec::new());
        }

        // Compute average document length
        let avgdl: f64 = conn
            .query_row(
                "SELECT COALESCE(AVG(CAST(doc_length AS REAL)), 1.0) FROM bm25_doc_stats",
                [],
                |row| row.get(0),
            )
            .context("Failed to compute average document length")?;

        let avgdl = if avgdl == 0.0 { 1.0 } else { avgdl };

        // Accumulate scores per (doc_type, doc_id)
        let mut scores: HashMap<(String, i64), f64> = HashMap::new();

        for term in &query_terms {
            // Document frequency: number of documents containing this term
            let df: f64 = conn
                .query_row(
                    "SELECT COUNT(DISTINCT doc_type || ':' || doc_id) FROM bm25_index WHERE term = ?1",
                    params![term],
                    |row| row.get(0),
                )
                .context("Failed to compute document frequency")?;

            if df == 0.0 {
                continue;
            }

            // IDF: ln((N - df + 0.5) / (df + 0.5) + 1)
            let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();

            // Find all documents containing this term
            let mut stmt = conn
                .prepare(
                    "SELECT i.doc_type, i.doc_id, i.tf, d.doc_length
                     FROM bm25_index i
                     JOIN bm25_doc_stats d ON i.doc_type = d.doc_type AND i.doc_id = d.doc_id
                     WHERE i.term = ?1",
                )
                .context("Failed to prepare term lookup query")?;

            let rows = stmt
                .query_map(params![term], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, f64>(2)?,
                        row.get::<_, i64>(3)?,
                    ))
                })
                .context("Failed to query term matches")?;

            for row in rows {
                let (dt, did, tf, dl) = row.context("Failed to read term match row")?;
                let dl_f = dl as f64;

                // BM25 score component: idf * (tf * (k1 + 1)) / (tf + k1 * (1 - b + b * dl/avgdl))
                let numerator = tf * (self.k1 + 1.0);
                let denominator = tf + self.k1 * (1.0 - self.b + self.b * dl_f / avgdl);
                let score = idf * numerator / denominator;

                *scores.entry((dt, did)).or_insert(0.0) += score;
            }
        }

        // Sort by score descending
        let mut results: Vec<(String, i64, f64)> = scores.into_iter().map(|((t, id), s)| (t, id, s)).collect();
        results.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);

        Ok(results)
    }
}

impl Default for Bm25Engine {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    /// Create an in-memory database with the required BM25 tables.
    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().expect("Failed to open in-memory DB");
        conn.execute_batch(
            "
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
        .expect("Failed to create tables");
        conn
    }

    #[test]
    fn test_tokenize_basic() {
        let tokens = Bm25Engine::tokenize("The Quick Brown Fox Jumps Over the Lazy Dog");
        // "the" and "over" are stopwords, single chars removed
        assert!(tokens.contains(&"quick".to_string()));
        assert!(tokens.contains(&"brown".to_string()));
        assert!(tokens.contains(&"fox".to_string()));
        assert!(tokens.contains(&"jumps".to_string()));
        assert!(tokens.contains(&"lazy".to_string()));
        assert!(tokens.contains(&"dog".to_string()));
        // Stopwords should be removed
        assert!(!tokens.contains(&"the".to_string()));
        assert!(!tokens.contains(&"over".to_string()));
    }

    #[test]
    fn test_tokenize_empty() {
        let tokens = Bm25Engine::tokenize("");
        assert!(tokens.is_empty());

        // Only stopwords
        let tokens = Bm25Engine::tokenize("the is a an");
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_index_and_search() -> Result<()> {
        let conn = setup_db();
        let engine = Bm25Engine::new();

        // Index 3 documents
        engine.index_document(&conn, "knowledge", 1, "Rust ownership and borrowing rules for memory safety")?;
        engine.index_document(&conn, "knowledge", 2, "Python garbage collection and memory management")?;
        engine.index_document(&conn, "knowledge", 3, "Rust lifetimes borrow checker memory compile time safety")?;

        // Search for "Rust" - docs 1 and 3 should match (doc 2 doesn't contain "rust")
        let results = engine.search(&conn, "Rust", 10)?;
        assert!(!results.is_empty(), "Expected at least one result for 'Rust'");
        // All results should be docs 1 or 3
        for (dt, did, score) in &results {
            assert_eq!(dt, "knowledge");
            assert!(did == &1 || did == &3, "Unexpected doc_id: {did}");
            assert!(*score > 0.0, "Score should be positive");
        }
        // Doc 2 should NOT appear
        assert!(results.iter().all(|(_, did, _)| *did != 2), "Doc 2 should not match 'Rust'");

        // Search for "memory" - all 3 docs mention "memory"
        let results = engine.search(&conn, "memory", 10)?;
        assert_eq!(results.len(), 3, "All 3 docs contain 'memory'");

        Ok(())
    }

    #[test]
    fn test_search_empty_query() -> Result<()> {
        let conn = setup_db();
        let engine = Bm25Engine::new();

        engine.index_document(&conn, "knowledge", 1, "Some text here")?;

        let results = engine.search(&conn, "", 10)?;
        assert!(results.is_empty(), "Empty query should return empty results");

        // Query with only stopwords
        let results = engine.search(&conn, "the is a", 10)?;
        assert!(results.is_empty(), "Stopword-only query should return empty results");

        Ok(())
    }

    #[test]
    fn test_search_no_matches() -> Result<()> {
        let conn = setup_db();
        let engine = Bm25Engine::new();

        engine.index_document(&conn, "knowledge", 1, "Rust ownership borrowing")?;
        engine.index_document(&conn, "knowledge", 2, "Python decorators and context managers")?;

        let results = engine.search(&conn, "quantum computing blockchain", 10)?;
        assert!(results.is_empty(), "No documents should match unrelated query");

        Ok(())
    }

    #[test]
    fn test_index_empty_text() -> Result<()> {
        let conn = setup_db();
        let engine = Bm25Engine::new();

        // Indexing empty text should not error
        engine.index_document(&conn, "knowledge", 1, "")?;
        engine.index_document(&conn, "knowledge", 2, "   ")?;

        // Verify doc stats were written (even though doc_length is 0)
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM bm25_doc_stats",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(count, 2);

        // Search should still work (just no matches)
        let results = engine.search(&conn, "anything", 10)?;
        assert!(results.is_empty());

        Ok(())
    }
}
