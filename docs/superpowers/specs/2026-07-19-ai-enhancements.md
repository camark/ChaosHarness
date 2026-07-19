# AI Capabilities Enhancement Plan

> Created: 2026-07-19
> Updated: 2026-07-19

## 1. System Prompt Enhancement

**Status:** ✅ Completed

**Done:**
- [x] Inject tool descriptions into system prompt
- [x] Inject CLAUDE.md project instructions
- [x] Inject user preferences from memory
- [x] Add task-specific prompts (coding, debug, review, explain)
- [x] Environment detection (OS, arch, shell)

**Files:** `src/prompts/system_prompt.rs`, `src/prompts/context.rs`

---

## 2. Context Building Enhancement

**Status:** ✅ Completed

**Done:**
- [x] Parse Cargo.toml/package.json for dependencies
- [x] Build directory tree with depth limit (2 levels)
- [x] Add git status (modified files, branch info, recent commits)
- [x] Get recently modified files (top 10)
- [x] Project type detection (Rust, JS/TS, Python, Go, Java, C/C++)

**Files:** `src/prompts/context.rs`

---

## 3. Learning System Enhancement

**Status:** 🔄 Partially Implemented

**Done:**
- [x] BM25 keyword search
- [x] Knowledge store (SQLite)
- [x] Learning extractor (LEARN markers)
- [x] Smart compactor

**TODO:**
- [ ] Add semantic embedding support (local or API)
- [ ] Cross-session knowledge persistence
- [ ] User feedback loop (thumbs up/down on responses)
- [ ] Preference learning from conversation patterns
- [ ] Auto-categorize knowledge (code, decision, fact, preference)

**Files:** `src/learning/`

---

## 4. Intelligent Tool Selection

**Status:** ✅ Completed

**Done:**
- [x] Suggest similar tools on unknown tool name (Levenshtein distance)
- [x] Tool usage statistics tracking
- [x] Tool registry with sync name listing

**Files:** `src/engine/query.rs`, `src/tools/base.rs`

---

## 5. Multi-turn Conversation Optimization

**Status:** 🔄 Partially Implemented

**Done:**
- [x] Auto-compaction at 50k tokens / 50 messages
- [x] Smart compactor with API-based summarization

**TODO:**
- [ ] Extract key facts after each turn
- [ ] Maintain conversation summary
- [ ] Track decisions and commitments
- [ ] Detect topic changes
- [ ] Inject relevant history into context

**Files:** `src/engine/query.rs`, `src/learning/summarizer.rs`

---

## 6. Error Recovery

**Status:** ✅ Completed

**Done:**
- [x] Auto-retry with adjusted approach on tool failure (timeout/network errors)
- [x] Suggest fixes for common errors (file not found, permission, timeout, command not found)
- [x] Error context preservation with helpful hints
- [x] Tool suggestion on unknown tool name

**Files:** `src/engine/query.rs`

---

## 7. Dynamic Model Selection

**Status:** ⏳ Not Started

**TODO:**
- [ ] Task complexity assessment
- [ ] Auto-switch model based on task type
- [ ] Fallback model chain on errors
- [ ] Cost optimization (use cheaper model for simple tasks)
- [ ] Model capability matching (code vs chat vs reasoning)

**Files:** `src/api/client.rs`, `src/engine/query.rs`

---

## Summary

| # | Feature | Status |
|---|---------|--------|
| 1 | System Prompt | ✅ Done |
| 2 | Context Building | ✅ Done |
| 3 | Learning System | 🔄 Partial |
| 4 | Tool Selection | ✅ Done |
| 5 | Multi-turn | 🔄 Partial |
| 6 | Error Recovery | ✅ Done |
| 7 | Dynamic Model | ⏳ Pending |
