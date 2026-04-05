//! REPL (Read-Eval-Print Loop) implementation

use crate::Args;
use crate::config::Settings;
use crate::engine::query::QueryEngine;
use crate::tools::init_tools;
use crate::commands::{CommandContext, create_default_command_registry};
use anyhow::Result;
use rustyline::DefaultEditor;
use tracing::info;

pub async fn run_repl(_args: &Args, settings: Settings) -> Result<()> {
    info!("Starting REPL mode");

    let mut rl = DefaultEditor::new()?;

    // Initialize tools
    let tool_registry = init_tools().await;

    // Create query engine
    let cwd = std::env::current_dir()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let mut query_engine = QueryEngine::new(settings.clone(), tool_registry, cwd.clone().into())
        .map_err(|e| anyhow::anyhow!(e))?;

    // Initialize MCP connections
    let connected_servers = query_engine.initialize_mcp().await;
    if !connected_servers.is_empty() {
        println!("Connected to {} MCP server(s): {}", connected_servers.len(), connected_servers.join(", "));
    }

    // Create command registry
    let command_registry = create_default_command_registry();

    println!("Rust Harness REPL. Type 'quit' or Ctrl-D to exit.");
    println!("Type '/help' for available commands.");

    loop {
        let input = match rl.readline("> ") {
            Ok(text) => text,
            Err(rustyline::error::ReadlineError::Interrupted) => {
                println!("Interrupted");
                continue;
            }
            Err(rustyline::error::ReadlineError::Eof) => {
                println!("EOF");
                break;
            }
            Err(err) => {
                eprintln!("Error reading input: {}", err);
                break;
            }
        };

        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        // Check for slash commands
        if input.starts_with('/') {
            let ctx = CommandContext {
                cwd: cwd.clone(),
                settings: &settings,
                registry: &command_registry,
            };

            if let Some(cmd) = command_registry.lookup(input) {
                // Extract args from input
                let args = input[1..].split_whitespace().skip(1).collect::<Vec<_>>().join(" ");
                let result = (cmd.handler)(&args, &ctx);

                if result.clear_screen {
                    #[cfg(unix)]
                    std::process::Command::new("clear").status().ok();
                    #[cfg(windows)]
                    std::process::Command::new("cmd").args(["/C", "cls"]).status().ok();
                }

                if let Some(msg) = result.message {
                    println!("{}", msg);
                }

                if result.should_exit {
                    break;
                }

                rl.add_history_entry(input)?;
                continue;
            } else {
                println!("Unknown command. Type '/help' for available commands.");
                rl.add_history_entry(input)?;
                continue;
            }
        }

        // Handle built-in REPL commands (no slash)
        match input {
            "quit" | "exit" => {
                println!("Goodbye!");
                break;
            }
            "clear" => {
                #[cfg(unix)]
                std::process::Command::new("clear").status().ok();
                #[cfg(windows)]
                std::process::Command::new("cmd").args(["/C", "cls"]).status().ok();
            }
            "help" => {
                println!("{}", command_registry.help_text());
            }
            "usage" => {
                let usage = query_engine.get_usage().await;
                println!(
                    "Token usage: {} input, {} output",
                    usage.total_input_tokens, usage.total_output_tokens
                );
            }
            _ => {
                // Send to AI
                println!("Processing...");

                match query_engine.send_message(input.to_string()).await {
                    Ok(response) => {
                        println!("\n{}", response);
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                    }
                }
            }
        }

        rl.add_history_entry(input)?;
    }

    Ok(())
}

pub async fn run_print_mode(prompt: &str, _args: &Args, settings: Settings) -> Result<()> {
    info!("Running in print mode with prompt: {}", prompt);

    // Initialize tools
    let tool_registry = init_tools().await;

    // Create query engine
    let cwd = std::env::current_dir().unwrap_or_default();
    let mut query_engine = QueryEngine::new(settings.clone(), tool_registry, cwd)
        .map_err(|e| anyhow::anyhow!(e))?;

    // Initialize MCP connections
    let _ = query_engine.initialize_mcp().await;

    match query_engine.send_message(prompt.to_string()).await {
        Ok(response) => {
            println!("{}", response);
            Ok(())
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

pub async fn run_resume_session(_args: &Args, settings: Settings) -> Result<()> {
    info!("Resuming session");

    // Initialize tools
    let tool_registry = init_tools().await;

    // Create query engine
    let cwd = std::env::current_dir().unwrap_or_default();
    let mut query_engine = QueryEngine::new(settings.clone(), tool_registry, cwd)
        .map_err(|e| anyhow::anyhow!(e))?;

    // Initialize MCP connections
    let _ = query_engine.initialize_mcp().await;

    // In a full implementation, this would load the previous session
    // and restore the conversation history
    // For now, just run the REPL
    run_repl(_args, settings).await
}
