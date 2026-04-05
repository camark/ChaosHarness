# Rust Harness

An AI-powered coding assistant - Rust rewrite of the OpenHarness Python application.

## Project Structure

```
src/
├── main.rs              # Entry point and CLI
├── api/                 # API client for AI models
│   ├── client.rs        # Anthropic API client with retry logic
│   ├── errors.rs        # API error types
│   └── usage.rs         # Token usage tracking
├── config/              # Configuration
│   ├── paths.rs         # Path resolution for config/data dirs
│   └── settings.rs      # Settings model and loading
├── engine/              # Core engine
│   ├── messages.rs      # Conversation message types
│   └── query.rs         # Query engine for processing messages
├── hooks/               # Extensible event handling
├── mcp/                 # Model Context Protocol support
├── memory/              # Persistent context memory
├── permissions/         # Permission checking
├── plugins/             # Plugin system
├── prompts/             # Prompt generation
├── services/            # Background services
│   ├── backend_server.rs # WebSocket server for React TUI
│   ├── cron.rs          # Cron scheduler
│   └── session_storage.rs # Session persistence
├── skills/              # Skills system
├── state/               # State management
└── ui/                  # User interface
    └── repl.rs          # REPL (Read-Eval-Print Loop)
```

## Features

### CLI Options

- `-m, --model <MODEL>` - Model alias or full model ID
- `-p, --print <PRINT>` - Print response and exit (non-interactive mode)
- `-c, --continue` - Continue the most recent conversation
- `-r, --resume <RESUME>` - Resume a conversation by session ID
- `-s, --system-prompt` - Override the default system prompt
- `-k, --api-key` - API key (or set ANTHROPIC_API_KEY env var)
- `--base-url` - Custom API base URL
- `--api-format` - API format: 'anthropic' or 'openai'
- `--permission-mode` - Permission mode: default, plan, or full_auto
- `--bare` - Minimal mode: skip hooks, plugins, MCP
- `--backend-only` - Run WebSocket server for React TUI frontend

### Usage

```bash
# Interactive REPL
cargo run -- <your prompt>

# Non-interactive mode
cargo run -- -p "Explain this codebase"

# With custom model
cargo run -- -m claude-opus-4-6 "Review my code"

# Backend-only mode (for React TUI frontend)
cargo run -- --backend-only

# Then start the React frontend:
# cd C:\Opt\OpenHarness\frontend\terminal
# npm start
```

## Frontend Integration

The original React/Ink terminal frontend (`C:\Opt\OpenHarness\frontend\terminal`) can be reused with the Rust backend:

1. Start the Rust backend:
   ```bash
   cargo run -- --backend-only
   ```

2. Start the React frontend in another terminal:
   ```bash
   cd C:\Opt\OpenHarness\frontend\terminal
   npm start
   ```

The backend runs a WebSocket server on `http://127.0.0.1:3000/ws` that communicates with the React frontend using the same protocol as the Python backend.

## Configuration

Configuration file: `~/.rust_harness/settings.json`

Environment variables:
- `ANTHROPIC_API_KEY` - API key
- `ANTHROPIC_MODEL` - Default model
- `ANTHROPIC_BASE_URL` - Custom API base URL
- `RUST_HARNESS_CONFIG_DIR` - Custom config directory

## Building

```bash
cargo build --release
```

## Differences from Python Version

This is a skeleton implementation that provides:
- Core CLI interface (same flags as Python version)
- Basic API client with retry logic
- Simple REPL interface
- WebSocket backend for React TUI

Some advanced features from the Python version are stubbed or simplified:
- Plugin system (basic structure only)
- MCP support (configuration loading only)
- Memory system (basic structure)
- Skills system (basic structure)
- Session storage (stub implementation)
