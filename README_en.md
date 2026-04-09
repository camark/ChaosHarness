# Rust Harness

RustHarness is an AI-powered coding assistant.

English | [简体中文](README_zh.md)

## Quick Start

### Using Launcher Script (Recommended)

**Windows:**
```bash
launch.bat
```

**Linux/macOS:**
```bash
chmod +x launch.sh
./launch.sh
```

The launcher provides 3 options:
1. **REPL** - Interactive CLI mode
2. **Frontend** - React TUI mode (requires TTY support)
3. **Backend Only** - WebSocket server mode

### Manual Start

**REPL Mode (CLI):**
```bash
cargo run -- <your prompt>
```

**Frontend Mode (React TUI):**

On Windows (requires Windows Terminal for TTY support):
```bash
cd frontend
start.bat
```

On Linux/macOS:
```bash
cd frontend
./start.sh
```

**Note:** The frontend requires TTY support. If you see "Raw mode is not supported" error:
- On Windows: Run in Windows Terminal (`wt.exe`) or CMD, not Git Bash
- On Linux/macOS: Run in a native terminal, not inside VS Code terminal

**Backend Only Mode (WebSocket server):**
```bash
cargo run -- --backend-only
```

**Backend Only Mode (Stdio/OHJSON protocol):**
```bash
cargo run -- --stdio-backend
```

## Project Structure

```
src/
├── main.rs              # Entry point and CLI
├── api/                 # API client for AI models
│   ├── client.rs        # Anthropic API client with retry logic
│   ├── errors.rs        # API error types
│   └── usage.rs         # Token usage tracking
├── commands/            # Slash command system
│   ├── types.rs         # Command types
│   ├── registry.rs      # Command registry and handlers
│   └── mod.rs           # Module exports
├── config/              # Configuration
│   ├── paths.rs         # Path resolution for config/data dirs
│   └── settings.rs      # Settings model and loading
├── engine/              # Core engine
│   ├── messages.rs      # Conversation message types
│   └── query.rs         # Query engine with tool-use loop
├── hooks/               # Extensible event handling
│   ├── events.rs        # Hook event types
│   ├── executor.rs      # Hook execution with timeout
│   ├── registry.rs      # Hook registry
│   └── builtins.rs      # Built-in hooks
├── mcp/                 # Model Context Protocol support
│   ├── client.rs        # MCP client with stdio/sse transport
│   ├── config.rs        # MCP server config loading
│   └── types.rs         # MCP types and schemas
├── memory/              # Persistent context memory
│   ├── manager.rs       # Memory manager
│   └── types.rs         # Memory types
├── permissions/         # Permission checking
│   ├── modes.rs         # Permission modes
│   └── checker.rs       # Permission checker
├── plugins/             # Plugin system
│   ├── loader.rs        # Plugin loader
│   └── installer.rs     # Plugin installer
├── services/            # Background services
│   ├── backend_server.rs # WebSocket server for React TUI
│   ├── stdio_backend.rs  # Stdio backend with OHJSON protocol
│   ├── session_storage.rs # Session persistence
│   └── cron.rs          # Cron scheduler
├── skills/              # Skills system
│   ├── loader.rs        # Skill loader from .md/.skill files
│   ├── registry.rs      # Skill registry
│   └── installer.rs     # Download and install skills from GitHub
├── tools/               # AI agent toolkit
│   ├── base.rs          # Tool trait and registry
│   ├── bash.rs          # Shell command execution
│   ├── file_read.rs     # Read files
│   ├── file_write.rs    # Write files
│   ├── file_edit.rs     # Edit files
│   ├── glob.rs          # File pattern matching
│   ├── grep.rs          # Content search
│   ├── web_fetch.rs     # Fetch URLs
│   ├── web_search.rs    # Web search
│   └── mcp.rs           # MCP tool integration
└── ui/                  # User interface
    └── repl.rs          # REPL (Read-Eval-Print Loop)
```

## Slash Commands

| Command | Description |
|---------|-------------|
| `/help` | Show available commands |
| `/clear` | Clear conversation history |
| `/exit` | Exit the REPL |
| `/status` | Show session status |
| `/usage` | Show token usage statistics |
| `/skills` | List or show available skills |
| `/skills list` | List all installed skills |
| `/skills show <name>` | Show skill content |
| `/skills install <name\|url>` | Install skill from SkillsMP or GitHub URL |
| `/skills search <query>` | Search SkillsMP for skills |
| `/skills remove <name>` | Remove an installed skill |
| `/plugin` | Manage plugins (list/install/uninstall/enable/disable) |
| `/hooks` | Show configured hooks |
| `/mcp` | List configured MCP servers |
| `/mcp list` | List all configured MCP servers |
| `/mcp query <server-name>` | Query a specific MCP server for its configuration details |
| `/config` | Show or update configuration |
| `/memory` | Manage project memory (list/show/add/remove) |
| `/resume <id>` | Resume a previous session |
| `/sessions` | List all saved sessions |
| `/export` | Export current session to markdown |
| `/delete_session <id>` | Delete a session |
| `/init` | Initialize default configuration |
| `/version` | Show version information |
| `/permissions` | Change permission mode |
| `/plan` | Toggle plan mode |

## MCP (Model Context Protocol)

MCP servers can be configured in `~/.rust_harness/settings.json`:

```json
{
  "mcp_servers": {
    "test-server": {
      "name": "test-server",
      "command": "node",
      "args": ["/path/to/server.js"],
      "transport": "stdio",
      "enabled": true
    }
  }
}
```

Supported transports:
- **stdio**: Spawn a process and communicate via stdin/stdout
- **sse**: Connect to a Server-Sent Events endpoint

## ACP (Agent Communication Protocol)

RustHarness supports the ACP protocol for interoperable communication between AI agents.

**Start ACP Server:**

```bash
cargo run -- --acp-server 8080
```

**Endpoints:**
- `GET /.well-known/agent.json` - AgentCard discovery
- `GET /acp` - AgentCard information
- `POST /tasks` - Create tasks
- `GET /tasks/{id}` - Get task status
- `POST /tasks/{id}/send` - Send messages
- `GET /tasks/{id}/artifacts` - Get task artifacts

See [ACP.md](ACP.md) for full documentation.

## CLI Options

| Option | Description |
|--------|-------------|
| `-m, --model <MODEL>` | Model alias or full model ID |
| `-p, --print <PRINT>` | Print response and exit (non-interactive mode) |
| `-c, --continue` | Continue the most recent conversation |
| `-r, --resume <RESUME>` | Resume a conversation by session ID |
| `-s, --system-prompt` | Override the default system prompt |
| `-k, --api-key` | API key (or set ANTHROPIC_API_KEY env var) |
| `--base-url` | Custom API base URL |
| `--api-format` | API format: 'anthropic' or 'openai' |
| `--permission-mode` | Permission mode: default, plan, or full_auto |
| `--bare` | Minimal mode: skip hooks, plugins, MCP |
| `--backend-only` | Run WebSocket server for React TUI frontend |
| `--stdio-backend` | Run stdio backend with OHJSON protocol |

## Environment Variables

- `ANTHROPIC_API_KEY` - API key
- `ANTHROPIC_MODEL` - Default model
- `ANTHROPIC_BASE_URL` - Custom API base URL
- `RUST_HARNESS_CONFIG_DIR` - Custom config directory
- `RUST_HARNESS_MODEL` - Model override
- `RUST_HARNESS_BASE_URL` - Base URL override
- `RUST_HARNESS_MAX_TOKENS` - Max tokens override
- `RUST_HARNESS_API_FORMAT` - API format override

## Building

```bash
# Development build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Lint
cargo clippy -- -D warnings

# Format
cargo fmt
```

## Implementation Status

### Core Features
- ✅ CLI with all flags (clap)
- ✅ API client with retry logic (exponential backoff, 3 retries)
- ✅ Query engine with tool-use loop
- ✅ Tool registry and execution framework
- ✅ Permission checker integration
- ✅ 11+ core tools (bash, file I/O, search, web)
- ✅ MCP client with stdio/sse transport
- ✅ Auto-compaction for long conversations
- ✅ Token usage tracking
- ✅ WebSocket backend server (axum)
- ✅ Stdio backend with OHJSON protocol
- ✅ Configuration system with env overrides
- ✅ Permission modes (Default/Plan/FullAuto)
- ✅ Hooks system with 8 events
- ✅ Slash commands (18 commands)
- ✅ Skills loader from ~/.rust_harness/skills/
- ✅ Plugin loader (claude-code compatible)
- ✅ Memory manager with MEMORY.md persistence
- ✅ Session storage and resume functionality
- ✅ Multi-agent coordination system (4 collaboration modes)
- ✅ ACP protocol support

## License

[See LICENSE file](LICENSE)
