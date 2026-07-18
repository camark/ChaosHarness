//! Rust Harness - An AI-powered coding assistant
//!
//! A rewrite of the OpenHarness Python application in Rust.

mod api;
mod commands;
mod config;
mod engine;
mod hooks;
mod mcp;
mod memory;
mod multi_agent;
mod permissions;
mod plugins;
mod prompts;
mod services;
mod skills;
mod state;
mod tools;
mod ui;
mod tui_frontend;
mod acp;
mod learning;

pub use ui::repl;

use anyhow::Result;
use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(name = "rust_harness")]
#[command(about = "An AI-powered coding assistant", long_about = None)]
pub struct Args {
    /// Continue the most recent conversation in the current directory
    #[arg(short = 'c', long)]
    r#continue: bool,

    /// Resume a conversation by session ID
    #[arg(short = 'r', long)]
    resume: Option<String>,

    /// Set a display name for this session
    #[arg(short = 'n', long)]
    name: Option<String>,

    /// Model alias or full model ID
    #[arg(short = 'm', long, env = "ANTHROPIC_MODEL")]
    model: Option<String>,

    /// Effort level for the session (low, medium, high, max)
    #[arg(long)]
    effort: Option<String>,

    /// Enable verbose logging
    #[arg(long)]
    verbose: bool,

    /// Maximum number of agentic turns
    #[arg(long)]
    max_turns: Option<u32>,

    /// Print response and exit (pass your prompt as the value)
    #[arg(short = 'p', long)]
    print: Option<String>,

    /// Output format with --print: text, json, or stream-json
    #[arg(long)]
    output_format: Option<String>,

    /// Permission mode: default, plan, or full_auto
    #[arg(long)]
    permission_mode: Option<String>,

    /// Bypass all permission checks (only for sandboxed environments)
    #[arg(long)]
    dangerously_skip_permissions: bool,

    /// Comma-separated list of tool names to allow
    #[arg(long)]
    allowed_tools: Option<String>,

    /// Comma-separated list of tool names to deny
    #[arg(long)]
    disallowed_tools: Option<String>,

    /// Override the default system prompt
    #[arg(short = 's', long)]
    system_prompt: Option<String>,

    /// Append text to the default system prompt
    #[arg(long)]
    append_system_prompt: Option<String>,

    /// Path to a JSON settings file
    #[arg(long)]
    settings: Option<String>,

    /// Anthropic-compatible API base URL
    #[arg(long, env = "ANTHROPIC_BASE_URL")]
    base_url: Option<String>,

    /// API key (overrides config and environment)
    #[arg(short = 'k', long, env = "ANTHROPIC_API_KEY")]
    api_key: Option<String>,

    /// Minimal mode: skip hooks, plugins, MCP, and auto-discovery
    #[arg(long)]
    bare: bool,

    /// API format: 'anthropic' or 'openai'
    #[arg(long)]
    api_format: Option<String>,

    /// Enable debug logging
    #[arg(short = 'd', long)]
    debug: bool,

    /// Working directory for the session
    #[arg(long, hide = true)]
    cwd: Option<String>,

    /// Run the structured backend host for the React terminal UI
    #[arg(long, hide = true)]
    backend_only: bool,

    /// Run stdio backend for React terminal UI (OHJSON protocol)
    #[arg(long, hide = true)]
    stdio_backend: bool,

    /// Use native TUI frontend (ratatui, better Windows support)
    #[arg(long)]
    tui: bool,

    /// Run ACP server on specified port
    #[arg(long)]
    acp_server: Option<u16>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let args = Args::parse();

    // Initialize configuration
    let settings = config::load_settings(args.settings.as_deref())
        .map_err(|e| anyhow::anyhow!("Failed to load settings: {}", e))?;

    // Handle --backend-only mode (WebSocket)
    if args.backend_only {
        let port = 3000; // Default backend port
        return services::backend_server::run_backend_server(settings, port)
            .await
            .map_err(|e| anyhow::anyhow!("Backend server error: {}", e));
    }

    // Handle --stdio-backend mode (OHJSON protocol)
    if args.stdio_backend {
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel(1);
        return tokio::select! {
            result = services::stdio_backend::run_stdio_backend(settings, shutdown_tx.clone()) => {
                result.map_err(|e| anyhow::anyhow!("Stdio backend error: {}", e))
            }
            _ = shutdown_rx.recv() => {
                Ok(()) // Clean shutdown
            }
        };
    }

    // Handle --tui mode (native ratatui TUI frontend)
    if args.tui {
        return tui_frontend::run_tui_frontend()
            .map_err(|e| anyhow::anyhow!("TUI frontend error: {}", e));
    }

    // Handle --acp-server mode
    if let Some(port) = args.acp_server {
        return acp::server::run_acp_server(port)
            .await
            .map_err(|e| anyhow::anyhow!("ACP server error: {}", e));
    }

    // Handle --continue and --resume flags
    if args.r#continue || args.resume.is_some() {
        return ui::run_resume_session(&args, settings).await;
    }

    // Handle --print mode
    if let Some(prompt) = &args.print {
        return ui::run_print_mode(prompt, &args, settings).await;
    }

    // Run interactive REPL
    ui::run_repl(&args, settings).await
}
