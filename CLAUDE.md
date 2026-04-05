# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**RustHarness** is a Rust rewrite of the OpenHarness Python application—an AI-powered coding assistant harness. It provides core agent infrastructure: tool-use, skills, memory, hooks, permissions, and multi-agent coordination.

## Build & Run

```bash
# Build
cargo build
cargo build --release

# Run interactive REPL
cargo run -- <your prompt>

# Non-interactive mode (print response and exit)
cargo run -- -p "Explain this codebase"

# Backend-only mode (for React TUI frontend)
cargo run -- --backend-only

# With custom model
cargo run -- -m claude-sonnet-4-20250514 "Review my code"

# Continue previous session
cargo run -- -c

# Run tests
cargo test

# Run specific test
cargo test -- package::module::test_name

# Lint
cargo clippy -- -D warnings

# Format
cargo fmt
```

## CLI Options

| Flag | Description |
|------|-------------|
| `-m, --model` | Model alias or full model ID |
| `-p, --print` | Print response and exit (non-interactive) |
| `-c, --continue` | Continue most recent conversation |
| `-r, --resume` | Resume conversation by session ID |
| `-s, --system-prompt` | Override default system prompt |
| `-k, --api-key` | API key (or set `ANTHROPIC_API_KEY`) |
| `--base-url` | Custom API base URL |
| `--api-format` | API format: `anthropic` or `openai` |
| `--permission-mode` | Permission mode: `default`, `plan`, `full_auto` |
| `--bare` | Minimal mode: skip hooks, plugins, MCP |
| `--backend-only` | Run WebSocket server for React TUI |

## Environment Variables

- `ANTHROPIC_API_KEY` - API key
- `ANTHROPIC_MODEL` - Default model
- `ANTHROPIC_BASE_URL` - Custom API base URL
- `RUST_HARNESS_CONFIG_DIR` - Custom config directory
- `RUST_HARNESS_MODEL` - Model override
- `RUST_HARNESS_BASE_URL` - Base URL override
- `RUST_HARNESS_MAX_TOKENS` - Max tokens override
- `RUST_HARNESS_API_FORMAT` - API format override

## Architecture

### Module Structure

```
src/
├── main.rs              # Entry point, CLI parsing with clap
├── api/                 # API client layer
│   ├── client.rs        # Anthropic-compatible client with retry logic
│   ├── errors.rs        # Error types (Authentication, RateLimit, Request, Network)
│   └── usage.rs         # Token usage tracking
├── commands/            # Slash command system
│   ├── types.rs         # CommandResult, CommandContext, SlashCommand types
│   ├── registry.rs      # CommandRegistry with built-in command handlers
│   └── mod.rs           # Module exports
├── config/              # Configuration
│   ├── paths.rs         # Path resolution (~/.rust_harness)
│   └── settings.rs      # Settings model, loading, env overrides
├── engine/              # Core conversation engine
│   ├── messages.rs      # ConversationMessage, MessageContent, ToolUseData types
│   └── query.rs         # QueryEngine with tool-use loop support
├── permissions/         # Permission checking
│   ├── modes.rs         # PermissionMode enum (Default/Plan/FullAuto)
│   └── checker.rs       # PermissionChecker with path rules, command denies
├── hooks/               # Extensible event system
│   ├── events.rs        # HookEvent enum (PreToolUse, PostToolUse, OnError, OnTurnComplete)
│   ├── executor.rs      # HookExecutor - runs hooks with timeout, collects output
│   ├── mod.rs           # Module exports
│   ├── registry.rs      # HookRegistry - manages hooks by event type (sync with parking_lot)
│   ├── schemas.rs       # HookDefinition - hook configuration (name, event, command, timeout)
│   └── types.rs         # HookContext, HookResult, HookDecision types
├── mcp/                 # Model Context Protocol
│   ├── client.rs        # MCP client
│   ├── config.rs        # MCP server config loading
│   └── types.rs         # MCP types
├── memory/              # Persistent context memory
│   ├── manager.rs       # Memory manager
│   ├── paths.rs         # Memory file paths
│   └── types.rs         # Memory types
├── plugins/             # Plugin system (claude-code compatible)
│   ├── loader.rs        # Plugin loading
│   ├── installer.rs     # Plugin installation
│   ├── schemas.rs       # Plugin schemas
│   └── types.rs         # Plugin types
├── skills/              # Skills system (anthropics/skills compatible)
│   ├── loader.rs        # Skill loading from .md files
│   ├── registry.rs      # Skill registry
│   └── types.rs         # Skill types
├── prompts/             # Prompt generation
│   ├── system_prompt.rs # System prompt assembly
│   ├── context.rs       # Context building
│   └── environment.rs   # Environment info
├── services/            # Background services
│   ├── backend_server.rs # WebSocket server (axum) for React TUI
│   ├── cron.rs          # Cron job management
│   ├── session_storage.rs # Session persistence
│   └── token_estimation.rs # Token counting
├── state/               # State management
│   ├── app_state.rs     # Application state
│   └── store.rs         # State store
├── tools/               # AI agent toolkit (Phase 1 implemented)
│   ├── base.rs          # Tool trait, ToolRegistry, ToolResult, ToolExecutionContext
│   ├── init.rs          # Tool initialization
│   ├── bash.rs          # Shell command execution
│   ├── file_read.rs     # Read UTF-8 text files with line numbers
│   ├── file_write.rs    # Create or overwrite files
│   ├── file_edit.rs     # Replace text in existing files
│   ├── glob.rs          # List files matching glob patterns
│   ├── grep.rs          # Search file contents with regex
│   ├── web_fetch.rs     # Fetch content from URLs
│   ├── web_search.rs    # Search the web (DuckDuckGo)
│   ├── notebook_edit.rs # Edit Jupyter notebook cells
│   └── ask_user.rs      # Interactive user prompts
└── ui/                  # User interface
    └── repl.rs          # REPL with rustyline
```

### Key Patterns

1. **API Client Layer**: `ApiClient` in `api/client.rs` handles streaming responses with exponential backoff retry (max 3 retries, 1-30s delay). Uses reqwest with SSE parsing.

2. **Message Flow**: User input → `QueryEngine::send_message()` → `ApiClient::send_message()` → parse response with tool_uses → execute tools in parallel → append tool results → loop until model stops requesting tools.

3. **Tool System**: 10 core tools implemented in `tools/` module:
   - `bash` - Shell command execution with timeout
   - `read_file` - Read UTF-8 text files with line numbers (offset/limit support)
   - `write_file` - Create or overwrite files with auto-directory creation
   - `edit_file` - String replacement in existing files (single or all occurrences)
   - `glob` - List files matching glob patterns
   - `grep` - Search file contents with regex (case-insensitive option)
   - `web_fetch` - Fetch content from HTTP/HTTPS URLs
   - `web_search` - Web search via DuckDuckGo HTML
   - `notebook_edit` - Edit Jupyter notebook cells (replace/insert/delete)
   - `ask_user` - Interactive user prompts

4. **Auto-Compaction**: `engine::compact` module provides automatic message history compaction when token count exceeds threshold (50k tokens or 50 messages). Truncates old tool results and keeps only recent messages.

5. **Hooks System**: `hooks` module provides extensible event handling:
   - `HookRegistry` - manages hooks by event type, uses `parking_lot::RwLock` for sync access
   - `HookExecutor` - executes hooks with timeout, collects output, handles blocking decisions
   - Events: `PreToolUse` (can block), `PostToolUse`, `OnError`, `OnTurnComplete`
   - Hooks configured in `settings.json` with `hooks.hooks` array
   - Each hook: `{ name, event, command, timeout, blocking, cwd }`
   - Blocking hooks: non-empty stdout = block operation

6. **Permission Checking**: `PermissionChecker` evaluates tool/command allowance based on mode:
   - `Default`: Ask before write/execute, path rules, command deny list
   - `Plan`: Block all writes
   - `FullAuto`: Allow everything

6. **Configuration**: Settings loaded from `~/.rust_harness/settings.json` with environment variable overrides. `Settings` struct in `config/settings.rs` is the central config model.

7. **WebSocket Protocol**: Backend server (`services/backend_server.rs`) communicates with React TUI via WebSocket on `127.0.0.1:3000/ws`. Uses JSON messages with type discrimination (`ClientMessage`/`ServerMessage`).

### Reference: OpenHarness (Python)

This project is a Rust rewrite of OpenHarness, which implements:
- 43+ tools (file I/O, shell, search, web, MCP, tasks, scheduling)
- Skills system (on-demand .md knowledge loading)
- Plugin system (claude-code compatible)
- 54 slash commands (/help, /commit, /plan, /resume, etc.)
- Multi-agent swarm coordination

When implementing missing features, reference the OpenHarness Python implementation at `C:\opt\OpenHarness` for design patterns and tool specifications.

## Current Implementation Status

### Phase 1: Core Tools - COMPLETED ✅
### Phase 2: Query Engine Enhancements - COMPLETED ✅
### Phase 3: Hooks System - COMPLETED ✅
### Phase 4: Skills & Plugins - COMPLETED ✅
### Phase 5: Advanced Features - COMPLETED ✅

**Implemented:**
- ✅ CLI with clap (all flags from Python version)
- ✅ API client with retry logic (exponential backoff, 3 retries)
- ✅ Query engine with tool-use loop
- ✅ Tool registry and execution framework
- ✅ Permission checker integration
- ✅ 10 core tools with tests (30 passing tests):
  - Bash, Read, Write, Edit, Glob, Grep, WebFetch, WebSearch, NotebookEdit, AskUser
- ✅ Auto-compaction for long conversations (50k tokens / 50 messages threshold)
- ✅ Token usage tracking
- ✅ Stream events module (structure ready)
- ✅ Basic REPL with rustyline
- ✅ WebSocket backend server
- ✅ Configuration system with env overrides
- ✅ Permission modes (Default/Plan/FullAuto)
- ✅ Hooks system with PreToolUse/PostToolUse/OnError/OnTurnComplete events
- ✅ Slash commands system (12 commands: /help, /exit, /clear, /version, /status, /usage, /skills, /plugin, /hooks, /config, /memory, /resume)
- ✅ Skills loader from ~/.rust_harness/skills/
- ✅ Plugin loader (claude-code compatible)
- ✅ Memory manager with MEMORY.md persistence
- ✅ Session storage and resume functionality

## Code Style

- Use `anyhow::Result` for application errors, `thiserror` for library errors
- Async code with tokio runtime
- Tracing for logging (`tracing::info!`, `tracing::error!`, etc.)
- Serde for serialization with `#[serde(rename_all = "snake_case")]` convention
- Module exports via `mod.rs` with `pub use` re-exports
