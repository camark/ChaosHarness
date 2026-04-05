//! REPL (Read-Eval-Print Loop) implementation

use crate::Args;
use crate::config::Settings;
use crate::engine::query::QueryEngine;
use crate::tools::init_tools;
use anyhow::Result;
use rustyline::DefaultEditor;
use tracing::info;

pub async fn run_repl(_args: &Args, settings: Settings) -> Result<()> {
    info!("Starting REPL mode");

    let mut rl = DefaultEditor::new()?;

    // Initialize tools
    let tool_registry = init_tools().await;

    // Create query engine
    let cwd = std::env::current_dir().unwrap_or_default();
    let query_engine = QueryEngine::new(settings.clone(), tool_registry, cwd)
        .map_err(|e| anyhow::anyhow!(e))?;

    println!("Rust Harness REPL. Type 'quit' or Ctrl-D to exit.");
    println!("Type 'help' for available commands.");

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

        match input {
            "quit" | "exit" => {
                println!("Goodbye!");
                break;
            }
            "help" => {
                println!("Available commands:");
                println!("  quit, exit - Exit the REPL");
                println!("  help - Show this help message");
                println!("  clear - Clear the screen");
                println!("  usage - Show token usage");
                println!();
                println!("Or type any prompt to send to the AI.");
            }
            "clear" => {
                #[cfg(unix)]
                std::process::Command::new("clear").status().ok();
                #[cfg(windows)]
                std::process::Command::new("cmd").args(["/C", "cls"]).status().ok();
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
    let query_engine = QueryEngine::new(settings.clone(), tool_registry, cwd)
        .map_err(|e| anyhow::anyhow!(e))?;

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
    let _query_engine = QueryEngine::new(settings.clone(), tool_registry, cwd)
        .map_err(|e| anyhow::anyhow!(e))?;

    // In a full implementation, this would load the previous session
    // and restore the conversation history
    // For now, just run the REPL
    run_repl(_args, settings).await
}
