# Self-Learning & Context Management Design

**Date:** 2026-07-18
**Status:** Draft
**Scope:** Add self-learning capabilities and intelligent context management to RustHarness

## Problem Statement

RustHarness currently has two limitations:

1. **Context length management** is naive — truncates old messages when over 50k tokens or 50 messages. No summarization, no semantic retrieval. Important context from earlier in the conversation is lost.

2. **No self-learning** — the system doesn't learn from interactions. Each session starts from scratch. User preferences, coding patterns, and accumulated knowledge are not persisted or applied.

## Goals

- Replace naive compaction with LLM-powered summarization
- Add BM25-based retrieval for relevant past context
- Extract knowledge, patterns, and skills from conversations
- Store everything in a local SQLite database
- Trigger learning at session end

## Non-Goals

- External vector databases (Qdrant, Pinecone, etc.)
- Neural embedding models — TF-IDF/BM25 is sufficient
- Real-time learning during conversation (future enhancement)
- Cross-user knowledge sharing

## Architecture

### Module Structure

```
src/learning/
├── mod.rs              # Module exports
├── store.rs            # SQLite-backed KnowledgeStore
├── bm25.rs             # BM25 scoring and indexing
├── extractor.rs        # Pattern & knowledge extraction
├── summarizer.rs       # LLM-powered conversation summarization
├── skill_gen.rs        # Auto-generate skills from patterns
├── retriever.rs        # Context retrieval for prompt injection
└── types.rs            # Shared types (KnowledgeEntry, Pattern, Summary, etc.)
```

### Data Flow

```
During conversation:
  User message → BM25 retrieve relevant context → Inject into system prompt → API call → Response

At compaction trigger:
  Old message segment → LLM summarize → Store in SQLite → Index in BM25 → Replace with summary marker

At session end:
  Full conversation → Parse learning markers → Optional LLM extraction → Store knowledge/patterns → Generate skills if threshold met
```

## SQLite Schema

Database location: `.rust_harness/knowledge.db` (project-local, under the working directory). If the user sets `learning.knowledge_db_path` in settings, that path is used instead. A global fallback at `~/.rust_harness/knowledge.db` is used when no project directory is available (e.g., running from home directory).

### conversation_summaries

Stores LLM-generated summaries of compacted conversation segments.

```sql
CREATE TABLE conversation_summaries (
    id INTEGER PRIMARY KEY,
    session_id TEXT NOT NULL,
    summary TEXT NOT NULL,
    message_range_start INTEGER,
    message_range_end INTEGER,
    tokens_saved INTEGER,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

### knowledge_entries

Stores factual knowledge extracted from conversations.

```sql
CREATE TABLE knowledge_entries (
    id INTEGER PRIMARY KEY,
    category TEXT NOT NULL,          -- 'fact', 'decision', 'solution', 'preference'
    topic TEXT NOT NULL,
    content TEXT NOT NULL,
    source_session_id TEXT,
    confidence REAL DEFAULT 0.5,
    access_count INTEGER DEFAULT 0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    last_accessed TIMESTAMP
);
```

### patterns

Stores observed behavioral patterns.

```sql
CREATE TABLE patterns (
    id INTEGER PRIMARY KEY,
    pattern_type TEXT NOT NULL,      -- 'coding_style', 'workflow', 'tool_preference'
    description TEXT NOT NULL,
    example TEXT,
    frequency INTEGER DEFAULT 1,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    last_seen TIMESTAMP
);
```

### bm25_index

Term frequency index for BM25 retrieval across all document types.

```sql
CREATE TABLE bm25_index (
    id INTEGER PRIMARY KEY,
    doc_type TEXT NOT NULL,          -- 'summary', 'knowledge', 'pattern'
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
```

## Components

### 1. KnowledgeStore (`store.rs`)

Central storage layer. Handles all SQLite operations.

```rust
pub struct KnowledgeStore {
    db: rusqlite::Connection,
}

impl KnowledgeStore {
    pub fn new(path: &Path) -> Result<Self>;
    pub fn add_summary(&self, summary: &Summary) -> Result<i64>;
    pub fn add_knowledge(&self, entry: &KnowledgeEntry) -> Result<i64>;
    pub fn add_pattern(&self, pattern: &Pattern) -> Result<i64>;
    pub fn search_knowledge(&self, query: &str, limit: usize) -> Result<Vec<ScoredResult>>;
    pub fn get_recent_summaries(&self, session_id: &str, limit: usize) -> Result<Vec<Summary>>;
    pub fn update_pattern_frequency(&self, pattern_id: i64) -> Result<()>;
    pub fn increment_access_count(&self, knowledge_id: i64) -> Result<()>;
}
```

### 2. BM25 Engine (`bm25.rs`)

Implements Okapi BM25 scoring with TF-IDF indexing.

```rust
pub struct Bm25Engine {
    k1: f64,  // term frequency saturation (default: 1.2)
    b: f64,   // length normalization (default: 0.75)
}

impl Bm25Engine {
    pub fn new(k1: f64, b: f64) -> Self;
    pub fn index_document(&self, db: &Connection, doc_type: &str, doc_id: i64, text: &str) -> Result<()>;
    pub fn search(&self, db: &Connection, query: &str, limit: usize) -> Result<Vec<ScoredDoc>>;
    pub fn tokenize(text: &str) -> Vec<String>;  // lowercase, split, remove stopwords
}
```

Tokenization: lowercase, split on whitespace/punctuation, remove common English stopwords. No stemming (keeps it simple and predictable).

### 3. SmartCompactor (`summarizer.rs`)

Replaces the naive `compact_messages` function.

```rust
pub struct SmartCompactor {
    api_client: ApiClient,
    segment_size: usize,         // messages per segment (default: 20)
    token_threshold: u32,        // trigger compaction at this token count (default: 30000)
}

impl SmartCompactor {
    pub async fn compact_if_needed(
        &self,
        messages: Vec<ConversationMessage>,
        session_id: &str,
        store: &KnowledgeStore,
    ) -> Result<(Vec<ConversationMessage>, bool)>;
    
    async fn summarize_segment(&self, messages: &[ConversationMessage]) -> Result<String>;
}
```

**Summarization prompt:**
```
Summarize this conversation segment concisely. Preserve:
- Key decisions made
- Files modified or created
- Bugs found and solutions
- User preferences expressed
- Technical facts learned

Conversation:
{messages}

Summary:
```

The summary is stored as a system message: `[SESSION SUMMARY: {summary_text}]`

### 4. Context Retriever (`retriever.rs`)

Retrieves relevant context before each API call.

```rust
pub struct ContextRetriever {
    store: KnowledgeStore,
    bm25: Bm25Engine,
    top_k: usize,  // default: 5
}

impl ContextRetriever {
    pub fn retrieve(&self, query: &str) -> Result<Vec<RetrievedContext>>;
    pub fn format_for_prompt(&self, contexts: &[RetrievedContext]) -> String;
}

pub enum RetrievedContext {
    Summary { text: String, score: f64 },
    Knowledge { entry: KnowledgeEntry, score: f64 },
    Pattern { pattern: Pattern, score: f64 },
}
```

The formatted output is injected into the system prompt under `## Relevant Context`.

### 5. LearningEngine (`extractor.rs`)

Runs at session end to extract knowledge and patterns.

```rust
pub struct LearningEngine {
    store: KnowledgeStore,
    api_client: Option<ApiClient>,  // for optional LLM extraction
}

impl LearningEngine {
    pub async fn process_session(
        &self,
        messages: &[ConversationMessage],
        session_id: &str,
    ) -> Result<LearningResult>;
    
    fn parse_markers(&self, messages: &[ConversationMessage]) -> Vec<ExtractedItem>;
    async fn llm_extract(&self, messages: &[ConversationMessage]) -> Result<Vec<ExtractedItem>>;
    fn deduplicate(&self, items: Vec<ExtractedItem>) -> Vec<ExtractedItem>;
}
```

**In-conversation markers:**
The system prompt instructs the model to embed structured markers when it observes something worth remembering:

```html
<!-- LEARN: category="preference" topic="testing" content="User prefers integration tests over unit tests" -->
```

These are parsed at session end — zero additional API cost.

**Optional LLM extraction:**
A dedicated API call that analyzes the full conversation for deeper patterns. Uses a focused prompt:

```
Analyze this conversation and extract:
1. Knowledge facts (technical decisions, solutions, preferences)
2. Behavioral patterns (recurring workflows, tool preferences)
3. Potential reusable skills (multi-step workflows that could be automated)

Format each as JSON:
{ "type": "knowledge|pattern|skill", "category": "...", "topic": "...", "content": "..." }

Conversation:
{full_conversation}
```

### 6. Skill Generator (`skill_gen.rs`)

Generates `.skill` files from high-frequency patterns.

```rust
pub struct SkillGenerator {
    skills_dir: PathBuf,  // .rust_harness/skills/auto_generated/
}

impl SkillGenerator {
    pub fn generate_skill(&self, pattern: &Pattern, examples: &[String]) -> Result<PathBuf>;
    pub fn should_generate(&self, pattern: &Pattern) -> bool;  // frequency >= threshold
}
```

Generated skills go to `.rust_harness/skills/auto_generated/` with a warning header:
```markdown
---
name: auto_{pattern_type}_{hash}
description: Auto-generated from observed pattern: {description}
auto_generated: true
---

# Auto-Generated Skill

> This skill was automatically generated from observed conversation patterns.
> Review and edit before relying on it.

## When to Use
{when_to_use}

## Steps
{extracted_steps}

## Examples
{examples}
```

## Integration Points

### 1. Query Engine (`engine/query.rs`)

In `run_loop()`, before building the API request:

```rust
// Before: static system prompt
// After: system prompt + retrieved context
let retrieved = self.context_retriever.retrieve(&current_user_message);
let enriched_prompt = format!("{}\n\n{}", base_prompt, retrieved.format_for_prompt());
```

### 2. Compaction (`engine/compact.rs`)

Replace `auto_compact_if_needed` with `SmartCompactor::compact_if_needed`.

### 3. Session End (`ui/repl.rs`)

When the user exits or starts a new session:

```rust
// Before: just save session
// After: save session + run learning engine
let learning_engine = LearningEngine::new(store, api_client);
learning_engine.process_session(&messages, &session_id).await?;
```

### 4. System Prompt (`prompts/system_prompt.rs`)

Add instructions for in-conversation learning markers:

```
When you observe user preferences, coding patterns, or important facts,
embed a learning marker:
<!-- LEARN: category="<category>" topic="<topic>" content="<content>" -->

Categories: fact, decision, solution, preference, workflow
Only mark genuinely useful information, not trivial observations.
```

### 5. Memory Compatibility (`memory/manager.rs`)

Knowledge entries with high confidence (>0.8) get mirrored to the existing MEMORY.md system for backward compatibility.

## Configuration

Additions to `settings.json`:

```json
{
  "learning": {
    "enabled": true,
    "knowledge_db_path": "~/.rust_harness/knowledge.db",
    "bm25_top_k": 5,
    "bm25_k1": 1.2,
    "bm25_b": 0.75,
    "summary_token_threshold": 30000,
    "summary_segment_size": 20,
    "session_end_extraction": true,
    "auto_skill_generation": true,
    "pattern_promotion_threshold": 3,
    "max_context_injection_tokens": 2000
  }
}
```

## Dependencies

- `rusqlite` (with `bundled` feature) — SQLite bindings
- No external embedding model or vector DB needed

## Testing Strategy

1. **Unit tests**: BM25 scoring, tokenization, marker parsing, deduplication
2. **Integration tests**: SQLite store operations, end-to-end compaction flow
3. **Prompt tests**: Verify summarization quality with sample conversations
4. **Regression tests**: Ensure existing compaction behavior is preserved as fallback

## Migration

- Existing `compact_messages` function is preserved as fallback if learning is disabled or SQLite is unavailable
- No breaking changes to existing APIs
- Knowledge DB is created on first use

## Future Enhancements

- Real-time learning during conversation (not just session end)
- Cross-session pattern correlation
- User-facing `/knowledge` command to browse and edit stored knowledge
- Export/import knowledge databases
- Embedding-based retrieval (when local models improve)
