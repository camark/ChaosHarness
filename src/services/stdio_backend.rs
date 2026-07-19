//! Stdio backend for React TUI frontend using OHJSON protocol
//!
//! Protocol format: Messages are prefixed with "OHJSON:" followed by JSON

#![allow(dead_code)]

use crate::config::Settings;
use crate::commands::registry::CommandRegistry;
use crate::engine::query::QueryEngine;
use crate::tools::init_tools;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{self, Write};
use tokio::io::AsyncBufReadExt;
use tokio::sync::mpsc;

const PROTOCOL_PREFIX: &str = "OHJSON:";

/// Backend events sent to frontend
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BackendEvent {
    Ready {
        state: Value,
        tasks: Vec<Value>,
        commands: Vec<String>,
        mcp_servers: Vec<Value>,
        bridge_sessions: Vec<Value>,
    },
    StateSnapshot {
        state: Value,
        mcp_servers: Vec<Value>,
        bridge_sessions: Vec<Value>,
    },
    TasksSnapshot {
        tasks: Vec<Value>,
    },
    TranscriptItem {
        item: TranscriptItem,
    },
    AssistantDelta {
        message: String,
    },
    AssistantComplete {
        message: Option<String>,
    },
    ToolStarted {
        tool_name: String,
        tool_input: Value,
        item: Option<TranscriptItem>,
    },
    ToolCompleted {
        tool_name: String,
        output: String,
        is_error: bool,
        item: Option<TranscriptItem>,
    },
    LineComplete,
    ClearTranscript,
    SelectRequest {
        modal: Value,
        select_options: Vec<SelectOption>,
    },
    ModalRequest {
        modal: Value,
    },
    Error {
        message: String,
    },
    Shutdown,
}

#[derive(Debug, Serialize, Clone)]
pub struct TranscriptItem {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_input: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Messages received from frontend
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FrontendMessage {
    SubmitLine { line: String },
    Shutdown,
    ListSessions,
    QuestionResponse { request_id: String, answer: String },
    PermissionResponse { request_id: String, allowed: bool },
}

/// Backend runner
pub struct StdioBackend {
    settings: Settings,
    command_registry: CommandRegistry,
    shutdown_tx: mpsc::Sender<()>,
    query_engine: Option<QueryEngine>,
    cwd: String,
}

impl StdioBackend {
    pub fn new(settings: Settings, shutdown_tx: mpsc::Sender<()>) -> Self {
        let command_registry = CommandRegistry::new();
        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        Self {
            settings,
            command_registry,
            shutdown_tx,
            query_engine: None,
            cwd,
        }
    }

    async fn init_query_engine(&mut self) -> io::Result<()> {
        if self.query_engine.is_none() {
            let tool_registry = init_tools().await;
            let mut query_engine = QueryEngine::new(
                self.settings.clone(),
                tool_registry,
                self.cwd.clone().into(),
            ).map_err(|e| {
                io::Error::other(format!("Failed to create query engine: {}", e))
            })?;

            // Initialize MCP connections
            let connected_servers = query_engine.initialize_mcp().await;

            // Store connected server info for status display
            let _ = connected_servers; // TODO: Store in state

            self.query_engine = Some(query_engine);
        }
        Ok(())
    }

    pub async fn run(mut self) -> io::Result<()> {
        // Initialize query engine first (including MCP connections)
        self.init_query_engine().await?;

        let stdin = tokio::io::stdin();
        let mut reader = tokio::io::BufReader::new(stdin);
        let mut line = String::new();

        // Send ready event
        {
            let mut stdout = io::stdout();
            self.send_event(&mut stdout, BackendEvent::Ready {
                state: self.get_initial_state(),
                tasks: vec![],
                commands: self.get_available_commands(),
                mcp_servers: self.get_mcp_servers(),
                bridge_sessions: vec![],
            })?;
        }

        loop {
            line.clear();
            let bytes_read = reader.read_line(&mut line).await?;
            if bytes_read == 0 {
                break; // EOF
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Parse JSON message
            if let Ok(msg) = serde_json::from_str::<FrontendMessage>(trimmed) {
                let mut stdout = io::stdout();
                self.handle_message(&mut stdout, msg).await?;
            } else {
                // Try to handle as raw input
                let mut stdout = io::stdout();
                self.handle_message(&mut stdout, FrontendMessage::SubmitLine { line: trimmed.to_string() }).await?;
            }
        }

        Ok(())
    }

    fn get_initial_state(&self) -> Value {
        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        serde_json::json!({
            "permission_mode": "default",
            "model": self.settings.model,
            "working_directory": cwd,
            "busy": false,
        })
    }

    fn get_available_commands(&self) -> Vec<String> {
        vec![
            "/help".to_string(),
            "/clear".to_string(),
            "/exit".to_string(),
            "/status".to_string(),
            "/usage".to_string(),
            "/skills".to_string(),
            "/plugin".to_string(),
            "/hooks".to_string(),
            "/mcp".to_string(),
            "/config".to_string(),
            "/memory".to_string(),
            "/resume".to_string(),
            "/sessions".to_string(),
            "/export".to_string(),
            "/delete_session".to_string(),
            "/init".to_string(),
            "/version".to_string(),
            "/permissions".to_string(),
            "/plan".to_string(),
        ]
    }

    fn get_mcp_servers(&self) -> Vec<Value> {
        use crate::mcp::config::load_mcp_server_configs;

        let mcp_configs = load_mcp_server_configs(&self.settings);
        mcp_configs
            .iter()
            .map(|(name, config)| {
                serde_json::json!({
                    "name": name,
                    "state": if config.enabled { "connected" } else { "disconnected" },
                    "transport": config.transport,
                    "auth_configured": config.env.is_some(),
                    "tool_count": 0,
                    "resource_count": 0,
                })
            })
            .collect()
    }

    fn send_event<W: Write>(
        &self,
        writer: &mut W,
        event: BackendEvent,
    ) -> io::Result<()> {
        let json = serde_json::to_string(&event).map_err(|e| {
            io::Error::other(format!("Failed to serialize event: {}", e))
        })?;
        let line = format!("{}{}\n", PROTOCOL_PREFIX, json);
        writer.write_all(line.as_bytes())?;
        writer.flush()?;
        Ok(())
    }

    async fn handle_message(
        &mut self,
        stdout: &mut io::Stdout,
        msg: FrontendMessage,
    ) -> io::Result<()> {
        match msg {
            FrontendMessage::SubmitLine { line } => {
                // Echo user input
                if self.send_event(stdout, BackendEvent::TranscriptItem {
                    item: TranscriptItem {
                        role: "user".to_string(),
                        text: Some(line.clone()),
                        tool_name: None,
                        tool_input: None,
                        is_error: None,
                    },
                }).is_err() {
                    eprintln!("[StdioBackend] Failed to send event, frontend may have disconnected");
                    return Ok(()); // Don't propagate error, just log it
                }

                // Process the line
                if self.process_line(stdout, &line).await.is_err() {
                    eprintln!("[StdioBackend] Failed to process line, frontend may have disconnected");
                    return Ok(()); // Don't propagate error, just log it
                }
            }
            FrontendMessage::Shutdown => {
                let _ = self.send_event(stdout, BackendEvent::Shutdown);
                let _ = self.shutdown_tx.send(()).await;
            }
            FrontendMessage::ListSessions => {
                // For now, send empty session list
                let _ = self.send_event(stdout, BackendEvent::SelectRequest {
                    modal: serde_json::json!({
                        "title": "Sessions",
                        "submit_prefix": "/resume ",
                    }),
                    select_options: vec![],
                });
            }
            FrontendMessage::QuestionResponse { request_id: _, answer } => {
                let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                    item: TranscriptItem {
                        role: "user".to_string(),
                        text: Some(answer),
                        tool_name: None,
                        tool_input: None,
                        is_error: None,
                    },
                });
            }
            FrontendMessage::PermissionResponse { request_id: _, allowed } => {
                let status = if allowed { "allowed" } else { "denied" };
                let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                    item: TranscriptItem {
                        role: "system".to_string(),
                        text: Some(format!("Permission {}", status)),
                        tool_name: None,
                        tool_input: None,
                        is_error: None,
                    },
                });
            }
        }
        Ok(())
    }

    async fn process_line(
        &mut self,
        stdout: &mut io::Stdout,
        line: &str,
    ) -> io::Result<()> {
        let trimmed = line.trim();

        // Handle slash commands
        if trimmed.starts_with('/') {
            let _ = self.handle_command(stdout, trimmed).await;
            let _ = self.send_event(stdout, BackendEvent::LineComplete);
            return Ok(());
        }

        // Initialize query engine if not already done
        if self.query_engine.is_none()
            && self.init_query_engine().await.is_err() {
                let _ = self.send_event(stdout, BackendEvent::Error {
                    message: "Failed to initialize query engine".to_string(),
                });
                let _ = self.send_event(stdout, BackendEvent::LineComplete);
                return Ok(());
            }

        // Send to AI
        if let Some(ref mut query_engine) = self.query_engine {
            match query_engine.send_message(trimmed.to_string()).await {
                Ok(response) => {
                    // Send AI response
                    let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                        item: TranscriptItem {
                            role: "assistant".to_string(),
                            text: Some(response),
                            tool_name: None,
                            tool_input: None,
                            is_error: None,
                        },
                    });
                }
                Err(e) => {
                    let _ = self.send_event(stdout, BackendEvent::Error {
                        message: format!("Error: {}", e),
                    });
                }
            }
        } else {
            let _ = self.send_event(stdout, BackendEvent::Error {
                message: "Query engine not initialized".to_string(),
            });
        }

        let _ = self.send_event(stdout, BackendEvent::LineComplete);
        Ok(())
    }

    async fn handle_command(
        &mut self,
        stdout: &mut io::Stdout,
        cmd: &str,
    ) -> io::Result<()> {
        match cmd {
            "/exit" | "/quit" => {
                let _ = self.send_event(stdout, BackendEvent::Shutdown);
                let _ = self.shutdown_tx.send(()).await;
            }
            "/clear" => {
                let _ = self.send_event(stdout, BackendEvent::ClearTranscript);
            }
            "/help" => {
                let help_text = self.get_available_commands().join("\n");
                let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                    item: TranscriptItem {
                        role: "system".to_string(),
                        text: Some(format!("Available commands:\n{}", help_text)),
                        tool_name: None,
                        tool_input: None,
                        is_error: None,
                    },
                });
            }
            "/status" => {
                let state = self.get_initial_state();
                let _ = self.send_event(stdout, BackendEvent::StateSnapshot {
                    state: state.clone(),
                    mcp_servers: self.get_mcp_servers(),
                    bridge_sessions: vec![],
                });
                // Also show status in transcript
                let model = state.get("model").and_then(|v| v.as_str()).unwrap_or("unknown");
                let mode = state.get("permission_mode").and_then(|v| v.as_str()).unwrap_or("default");
                let cwd = state.get("working_directory").and_then(|v| v.as_str()).unwrap_or(".");
                let status_text = format!("Model: {} | Mode: {} | CWD: {}", model, mode, cwd);
                let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                    item: TranscriptItem {
                        role: "system".to_string(),
                        text: Some(status_text),
                        tool_name: None,
                        tool_input: None,
                        is_error: None,
                    },
                });
            }
            cmd if cmd.starts_with("/plugin") => {
                use crate::plugins::loader::load_plugins;
                use crate::plugins::installer::{install_plugin_from_path, uninstall_plugin, enable_plugin, disable_plugin};

                let parts: Vec<&str> = cmd.split_whitespace().collect();
                let subcommand = parts.get(1).copied().unwrap_or("list");
                let arg = parts.get(2).copied();

                let message = match subcommand {
                    "list" => {
                        let plugins = load_plugins(&self.settings, &self.cwd);
                        if plugins.is_empty() {
                            "No plugins discovered.".to_string()
                        } else {
                            let lines: Vec<_> = plugins
                                .iter()
                                .map(|p| {
                                    let status = if p.enabled { "✓" } else { "✗" };
                                    format!("  [{}] {} v{} - {}", status, p.name, p.version, p.description.as_deref().unwrap_or(""))
                                })
                                .collect();
                            format!("Installed plugins:\n{}", lines.join("\n"))
                        }
                    }
                    "install" => {
                        if let Some(path) = arg {
                            match install_plugin_from_path(path, &self.cwd) {
                                Ok(msg) => msg,
                                Err(e) => format!("Failed to install: {}", e),
                            }
                        } else {
                            "Usage: /plugin install <path>".to_string()
                        }
                    }
                    "uninstall" => {
                        if let Some(name) = arg {
                            match uninstall_plugin(name, &self.cwd) {
                                Ok(msg) => msg,
                                Err(e) => format!("Failed to uninstall: {}", e),
                            }
                        } else {
                            "Usage: /plugin uninstall <name>".to_string()
                        }
                    }
                    "enable" => {
                        if let Some(name) = arg {
                            match enable_plugin(name, &self.cwd) {
                                Ok(msg) => msg,
                                Err(e) => format!("Failed to enable: {}", e),
                            }
                        } else {
                            "Usage: /plugin enable <name>".to_string()
                        }
                    }
                    "disable" => {
                        if let Some(name) = arg {
                            match disable_plugin(name, &self.cwd) {
                                Ok(msg) => msg,
                                Err(e) => format!("Failed to disable: {}", e),
                            }
                        } else {
                            "Usage: /plugin disable <name>".to_string()
                        }
                    }
                    _ => {
                        "Usage: /plugin [list|install PATH|uninstall NAME|enable NAME|disable NAME]".to_string()
                    }
                };

                let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                    item: TranscriptItem {
                        role: "system".to_string(),
                        text: Some(message),
                        tool_name: None,
                        tool_input: None,
                        is_error: None,
                    },
                });
            }
            cmd if cmd.starts_with("/permissions") => {
                if cmd.contains("set") {
                    let mode = cmd.split_whitespace().nth(2).unwrap_or("default");
                    let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                        item: TranscriptItem {
                            role: "system".to_string(),
                            text: Some(format!("Permission mode set to: {}", mode)),
                            tool_name: None,
                            tool_input: None,
                            is_error: None,
                        },
                    });
                } else {
                    let _ = self.send_event(stdout, BackendEvent::SelectRequest {
                        modal: serde_json::json!({
                            "title": "Permission Mode",
                            "kind": "permission_picker",
                        }),
                        select_options: vec![
                            SelectOption { value: "default".to_string(), label: "Default".to_string(), description: Some("Ask before write/execute".to_string()) },
                            SelectOption { value: "full_auto".to_string(), label: "Auto".to_string(), description: Some("Allow all tools".to_string()) },
                            SelectOption { value: "plan".to_string(), label: "Plan".to_string(), description: Some("Block writes".to_string()) },
                        ],
                    });
                }
            }
            "/plan" => {
                let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                    item: TranscriptItem {
                        role: "system".to_string(),
                        text: Some("Plan mode toggled".to_string()),
                        tool_name: None,
                        tool_input: None,
                        is_error: None,
                    },
                });
            }
            cmd if cmd.starts_with("/skills") => {
                use crate::skills::{loader::load_skill_registry, installer::{SkillInstaller, get_user_skills_dir}};
                use std::path::Path;

                let parts: Vec<&str> = cmd.split_whitespace().collect();
                let subcommand = parts.get(1).copied().unwrap_or("list");
                let args = if parts.len() > 2 { parts[2..].join(" ") } else { String::new() };

                match subcommand {
                    "list" => {
                        let registry = load_skill_registry(Path::new(&self.cwd));
                        let skills = registry.list();
                        if skills.is_empty() {
                            let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                item: TranscriptItem {
                                    role: "system".to_string(),
                                    text: Some("No skills installed. Use /skills install <name> to install from SkillsMP.".to_string()),
                                    tool_name: None,
                                    tool_input: None,
                                    is_error: None,
                                },
                            });
                        } else {
                            let lines: Vec<_> = skills
                                .iter()
                                .map(|s| format!("  - {}: {}", s.name, s.description))
                                .collect();
                            let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                item: TranscriptItem {
                                    role: "system".to_string(),
                                    text: Some(format!("Installed skills ({} total):\n{}", skills.len(), lines.join("\n"))),
                                    tool_name: None,
                                    tool_input: None,
                                    is_error: None,
                                },
                            });
                        }
                    }
                    "show" | "view" => {
                        if args.is_empty() {
                            let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                item: TranscriptItem {
                                    role: "system".to_string(),
                                    text: Some("Usage: /skills show <name>".to_string()),
                                    tool_name: None,
                                    tool_input: None,
                                    is_error: Some(true),
                                },
                            });
                        } else {
                            let registry = load_skill_registry(Path::new(&self.cwd));
                            match registry.get(&args) {
                                Some(skill) => {
                                    let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                        item: TranscriptItem {
                                            role: "system".to_string(),
                                            text: Some(skill.content.clone()),
                                            tool_name: None,
                                            tool_input: None,
                                            is_error: None,
                                        },
                                    });
                                }
                                None => {
                                    let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                        item: TranscriptItem {
                                            role: "system".to_string(),
                                            text: Some(format!("Skill not found: {}", args)),
                                            tool_name: None,
                                            tool_input: None,
                                            is_error: Some(true),
                                        },
                                    });
                                }
                            }
                        }
                    }
                    "remove" | "delete" => {
                        if args.is_empty() {
                            let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                item: TranscriptItem {
                                    role: "system".to_string(),
                                    text: Some("Usage: /skills remove <name>".to_string()),
                                    tool_name: None,
                                    tool_input: None,
                                    is_error: Some(true),
                                },
                            });
                        } else {
                            let installer = SkillInstaller::new(&get_user_skills_dir());
                            match installer.remove_skill(&args) {
                                Ok(true) => {
                                    let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                        item: TranscriptItem {
                                            role: "system".to_string(),
                                            text: Some(format!("Removed skill: {}", args)),
                                            tool_name: None,
                                            tool_input: None,
                                            is_error: None,
                                        },
                                    });
                                }
                                Ok(false) => {
                                    let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                        item: TranscriptItem {
                                            role: "system".to_string(),
                                            text: Some(format!("Skill not found: {}", args)),
                                            tool_name: None,
                                            tool_input: None,
                                            is_error: Some(true),
                                        },
                                    });
                                }
                                Err(e) => {
                                    let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                        item: TranscriptItem {
                                            role: "system".to_string(),
                                            text: Some(format!("Failed to remove skill: {}", e)),
                                            tool_name: None,
                                            tool_input: None,
                                            is_error: Some(true),
                                        },
                                    });
                                }
                            }
                        }
                    }
                    "install" => {
                        if args.is_empty() {
                            let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                item: TranscriptItem {
                                    role: "system".to_string(),
                                    text: Some("Usage: /skills install <name|github-url>".to_string()),
                                    tool_name: None,
                                    tool_input: None,
                                    is_error: Some(true),
                                },
                            });
                        } else if args.starts_with("http") {
                            // Install from GitHub URL
                            let url = args.clone();
                            let skills_dir = get_user_skills_dir();
                            let result = tokio::task::spawn_blocking(move || {
                                let installer = SkillInstaller::new(&skills_dir);
                                installer.install_from_github(&url)
                            })
                            .await
                            .map_err(|e| io::Error::other(format!("Task failed: {}", e)))?;

                            match result {
                                Ok(path) => {
                                    let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                        item: TranscriptItem {
                                            role: "system".to_string(),
                                            text: Some(format!("Installed skill from URL: {}", path)),
                                            tool_name: None,
                                            tool_input: None,
                                            is_error: None,
                                        },
                                    });
                                }
                                Err(e) => {
                                    let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                        item: TranscriptItem {
                                            role: "system".to_string(),
                                            text: Some(format!("Failed to install skill: {}", e)),
                                            tool_name: None,
                                            tool_input: None,
                                            is_error: Some(true),
                                        },
                                    });
                                }
                            }
                        } else {
                            // Search SkillsMP and install
                            let query = args.clone();
                            let skills_dir = get_user_skills_dir();
                            let query_for_closure = query.clone();
                            let skills_dir_for_closure = skills_dir.clone();
                            let search_result = tokio::task::spawn_blocking(move || {
                                let installer = SkillInstaller::new(&skills_dir_for_closure);
                                installer.search(&query_for_closure)
                            })
                            .await
                            .map_err(|e| io::Error::other(format!("Task failed: {}", e)))?;

                            match search_result {
                                Ok(skills) => {
                                    if skills.is_empty() {
                                        let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                            item: TranscriptItem {
                                                role: "system".to_string(),
                                                text: Some(format!("No skills found for: {}", query)),
                                                tool_name: None,
                                                tool_input: None,
                                                is_error: Some(true),
                                            },
                                        });
                                        return Ok(());
                                    }
                                    // Extract data before spawning new blocking task
                                    let skill_url = skills[0].skill_url.clone();
                                    let skill_name = skills[0].name.clone();
                                    let skill_name_for_msg = skill_name.clone();
                                    let skill_author = skills[0].author.clone();
                                    let skills_dir_for_download = skills_dir.clone();
                                    let download_result = tokio::task::spawn_blocking(move || {
                                        let installer = SkillInstaller::new(&skills_dir_for_download);
                                        installer.download_skill(&skill_url, Some(&skill_name))
                                    })
                                    .await
                                    .map_err(|e| io::Error::other(format!("Task failed: {}", e)))?;
                                    match download_result {
                                        Ok(path) => {
                                            let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                                item: TranscriptItem {
                                                    role: "system".to_string(),
                                                    text: Some(format!("Installed skill '{}' from {}:\n  {}", skill_name_for_msg, skill_author, path)),
                                                    tool_name: None,
                                                    tool_input: None,
                                                    is_error: None,
                                                },
                                            });
                                        }
                                        Err(e) => {
                                            let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                                item: TranscriptItem {
                                                    role: "system".to_string(),
                                                    text: Some(format!("Failed to download skill: {}", e)),
                                                    tool_name: None,
                                                    tool_input: None,
                                                    is_error: Some(true),
                                                },
                                            });
                                        }
                                    }
                                }
                                Err(e) => {
                                    let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                        item: TranscriptItem {
                                            role: "system".to_string(),
                                            text: Some(format!("Search failed: {}", e)),
                                            tool_name: None,
                                            tool_input: None,
                                            is_error: Some(true),
                                        },
                                    });
                                }
                            }
                        }
                    }
                    "search" => {
                        if args.is_empty() {
                            let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                item: TranscriptItem {
                                    role: "system".to_string(),
                                    text: Some("Usage: /skills search <query>".to_string()),
                                    tool_name: None,
                                    tool_input: None,
                                    is_error: Some(true),
                                },
                            });
                        } else {
                            let installer = SkillInstaller::new(&get_user_skills_dir());
                            let query = args.clone();
                            match tokio::task::spawn_blocking(move || installer.search(&query))
                                .await
                                .map_err(|e| io::Error::other(format!("Task failed: {}", e)))?
                            {
                                Ok(skills) => {
                                    if skills.is_empty() {
                                        let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                            item: TranscriptItem {
                                                role: "system".to_string(),
                                                text: Some(format!("No skills found for: {}", args)),
                                                tool_name: None,
                                                tool_input: None,
                                                is_error: Some(true),
                                            },
                                        });
                                    } else {
                                        let lines: Vec<_> = skills
                                            .iter()
                                            .take(10)
                                            .map(|s| format!("  - {} by {}: {}", s.name, s.author, s.description))
                                            .collect();
                                        let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                            item: TranscriptItem {
                                                role: "system".to_string(),
                                                text: Some(format!("Found {} skills for '{}':\n{}", skills.len(), args, lines.join("\n"))),
                                                tool_name: None,
                                                tool_input: None,
                                                is_error: None,
                                            },
                                        });
                                    }
                                }
                                Err(e) => {
                                    let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                        item: TranscriptItem {
                                            role: "system".to_string(),
                                            text: Some(format!("Search failed: {}", e)),
                                            tool_name: None,
                                            tool_input: None,
                                            is_error: Some(true),
                                        },
                                    });
                                }
                            }
                        }
                    }
                    _ => {
                        // Fallback - show skill content or list all
                        let registry = load_skill_registry(Path::new(&self.cwd));
                        match registry.get(subcommand) {
                            Some(skill) => {
                                let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                    item: TranscriptItem {
                                        role: "system".to_string(),
                                        text: Some(skill.content.clone()),
                                        tool_name: None,
                                        tool_input: None,
                                        is_error: None,
                                    },
                                });
                            }
                            None => {
                                let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                    item: TranscriptItem {
                                        role: "system".to_string(),
                                        text: Some("Usage: /skills <list|show <name>|remove <name>|install <name|url>|search <query>>".to_string()),
                                        tool_name: None,
                                        tool_input: None,
                                        is_error: Some(true),
                                    },
                                });
                            }
                        }
                    }
                }
            }
            "/hooks" => {
                use crate::config::load_settings;

                let settings = load_settings(None).unwrap_or_default();
                let hooks = settings.hooks.hooks;
                if hooks.is_empty() {
                    let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                        item: TranscriptItem {
                            role: "system".to_string(),
                            text: Some("No hooks configured. Add hooks to ~/.rust_harness/settings.json".to_string()),
                            tool_name: None,
                            tool_input: None,
                            is_error: None,
                        },
                    });
                } else {
                    let lines: Vec<_> = hooks
                        .iter()
                        .map(|h| format!("  - {} [{}]: {}", h.name, h.event, h.command))
                        .collect();
                    let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                        item: TranscriptItem {
                            role: "system".to_string(),
                            text: Some(format!("Configured hooks:\n{}", lines.join("\n"))),
                            tool_name: None,
                            tool_input: None,
                            is_error: None,
                        },
                    });
                }
            }
            cmd if cmd.starts_with("/mcp") => {
                use crate::config::load_settings;
                use crate::mcp::config::load_mcp_server_configs;

                let args = cmd.strip_prefix("/mcp").unwrap_or("").trim();
                let tokens: Vec<&str> = args.split_whitespace().collect();

                if tokens.is_empty() || tokens[0] == "list" {
                    let settings = load_settings(None).unwrap_or_default();
                    let mcp_servers = load_mcp_server_configs(&settings);
                    if mcp_servers.is_empty() {
                        let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                            item: TranscriptItem {
                                role: "system".to_string(),
                                text: Some("No MCP servers configured.\n\nAdd MCP servers in ~/.rust_harness/settings.json:\n```json\n{\n  \"mcp_servers\": {\n    \"test-server\": {\n      \"name\": \"test-server\",\n      \"command\": \"node\",\n      \"args\": [\"/path/to/server.js\"],\n      \"transport\": \"stdio\",\n      \"enabled\": true\n    }\n  }\n}\n```".to_string()),
                                tool_name: None,
                                tool_input: None,
                                is_error: None,
                            },
                        });
                    } else {
                        let lines: Vec<_> = mcp_servers
                            .iter()
                            .map(|(name, config)| {
                                let status = if config.enabled { "✓" } else { "✗" };
                                let transport = &config.transport;
                                let details = if transport == "stdio" {
                                    format!("{}: {} {}",
                                        config.command.as_deref().unwrap_or("unknown"),
                                        config.args.as_ref().map(|a| a.join(" ")).unwrap_or_default(),
                                        if let Some(env) = &config.env {
                                            format!("({} env vars)", env.len())
                                        } else {
                                            String::new()
                                        }
                                    )
                                } else if transport == "sse" {
                                    config.url.clone().unwrap_or_default()
                                } else {
                                    transport.clone()
                                };
                                format!("  [{}] {} - {}", status, name, details)
                            })
                            .collect();
                        let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                            item: TranscriptItem {
                                role: "system".to_string(),
                                text: Some(format!("Configured MCP servers ({} total):\n{}", mcp_servers.len(), lines.join("\n"))),
                                tool_name: None,
                                tool_input: None,
                                is_error: None,
                            },
                        });
                    }
                } else if tokens[0] == "query" {
                    if tokens.len() < 2 {
                        let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                            item: TranscriptItem {
                                role: "system".to_string(),
                                text: Some("Usage: /mcp query <server-name>\n\nQuery a specific MCP server for its capabilities and tools.".to_string()),
                                tool_name: None,
                                tool_input: None,
                                is_error: None,
                            },
                        });
                    } else {
                        let server_name = tokens[1];
                        let settings = load_settings(None).unwrap_or_default();
                        let mcp_servers = load_mcp_server_configs(&settings);

                        if let Some(config) = mcp_servers.get(server_name) {
                            let mut info = Vec::new();
                            info.push(format!("MCP Server: {}", server_name));
                            info.push(format!("  Status: {}", if config.enabled { "Enabled" } else { "Disabled" }));
                            info.push(format!("  Transport: {}", config.transport));

                            if config.transport == "stdio" {
                                if let Some(cmd) = &config.command {
                                    info.push(format!("  Command: {}", cmd));
                                }
                                if let Some(args) = &config.args {
                                    info.push(format!("  Args: {}", args.join(" ")));
                                }
                                if let Some(env) = &config.env {
                                    info.push(format!("  Environment variables: {}", env.len()));
                                }
                            } else if config.transport == "sse" {
                                if let Some(url) = &config.url {
                                    info.push(format!("  URL: {}", url));
                                }
                            }

                            let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                item: TranscriptItem {
                                    role: "system".to_string(),
                                    text: Some(info.join("\n")),
                                    tool_name: None,
                                    tool_input: None,
                                    is_error: None,
                                },
                            });
                        } else {
                            let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                item: TranscriptItem {
                                    role: "system".to_string(),
                                    text: Some(format!("MCP server '{}' not found. Use /mcp list to see available servers.", server_name)),
                                    tool_name: None,
                                    tool_input: None,
                                    is_error: None,
                                },
                            });
                        }
                    }
                } else {
                    let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                        item: TranscriptItem {
                            role: "system".to_string(),
                            text: Some("Usage: /mcp [list|query <server-name>]".to_string()),
                            tool_name: None,
                            tool_input: None,
                            is_error: None,
                        },
                    });
                }
            }
            "/usage" => {
                // Get token usage from query engine if available
                let usage_info = if let Some(ref qe) = self.query_engine {
                    let usage = qe.get_usage().await;
                    format!(
                        "Token Usage:\n  Input tokens: {}\n  Output tokens: {}\n  Total tokens: {}",
                        usage.total_input_tokens,
                        usage.total_output_tokens,
                        usage.total_input_tokens + usage.total_output_tokens
                    )
                } else {
                    "Token Usage:\n  No session active yet.".to_string()
                };
                let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                    item: TranscriptItem {
                        role: "system".to_string(),
                        text: Some(usage_info),
                        tool_name: None,
                        tool_input: None,
                        is_error: None,
                    },
                });
            }
            "/config" => {
                use crate::config::load_settings;
                use crate::config::get_config_file_path;

                let config_path = get_config_file_path();
                let settings = load_settings(None).unwrap_or_default();

                let config_text = format!(
                    "Configuration:\n  Config file: {}\n  Model: {}\n  Max tokens: {}\n  API format: {}\n  Base URL: {}\n  Memory enabled: {}\n  Hooks enabled: {}",
                    config_path.display(),
                    settings.model,
                    settings.max_tokens,
                    settings.api_format,
                    settings.base_url.as_deref().unwrap_or("(not set)"),
                    settings.memory.enabled,
                    settings.hooks.enabled
                );
                let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                    item: TranscriptItem {
                        role: "system".to_string(),
                        text: Some(config_text),
                        tool_name: None,
                        tool_input: None,
                        is_error: None,
                    },
                });
            }
            "/memory" => {
                use crate::memory::manager::MemoryManager;

                let parts: Vec<&str> = cmd.split_whitespace().collect();
                let subcommand = parts.get(1).copied().unwrap_or("");
                let args = if parts.len() > 2 { parts[2..].join(" ") } else { String::new() };

                match subcommand {
                    "list" => {
                        let memory_files = MemoryManager::list_memory_files(&self.cwd);
                        let memory_text = if memory_files.is_empty() {
                            "Memory:\n  No memory entries found.".to_string()
                        } else {
                            let mut text = format!("Memory entries ({} total):\n", memory_files.len());
                            for (i, path) in memory_files.iter().enumerate() {
                                if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                                    text.push_str(&format!("  {}. {}\n", i + 1, name));
                                }
                            }
                            text
                        };
                        let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                            item: TranscriptItem {
                                role: "system".to_string(),
                                text: Some(memory_text),
                                tool_name: None,
                                tool_input: None,
                                is_error: None,
                            },
                        });
                    }
                    "show" => {
                        if args.is_empty() {
                            let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                item: TranscriptItem {
                                    role: "system".to_string(),
                                    text: Some("Usage: /memory show <name>".to_string()),
                                    tool_name: None,
                                    tool_input: None,
                                    is_error: Some(true),
                                },
                            });
                        } else {
                            match MemoryManager::read_memory(&self.cwd, &args) {
                                Some(content) => {
                                    let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                        item: TranscriptItem {
                                            role: "system".to_string(),
                                            text: Some(content),
                                            tool_name: None,
                                            tool_input: None,
                                            is_error: None,
                                        },
                                    });
                                }
                                None => {
                                    let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                        item: TranscriptItem {
                                            role: "system".to_string(),
                                            text: Some(format!("Memory entry not found: {}", args)),
                                            tool_name: None,
                                            tool_input: None,
                                            is_error: Some(true),
                                        },
                                    });
                                }
                            }
                        }
                    }
                    "add" => {
                        // Format: /memory add TITLE :: CONTENT
                        if let Some(separator_pos) = args.find("::") {
                            let title = args[..separator_pos].trim();
                            let content = args[separator_pos + 2..].trim();

                            if title.is_empty() || content.is_empty() {
                                let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                    item: TranscriptItem {
                                        role: "system".to_string(),
                                        text: Some("Usage: /memory add TITLE :: CONTENT".to_string()),
                                        tool_name: None,
                                        tool_input: None,
                                        is_error: Some(true),
                                    },
                                });
                            } else {
                                match MemoryManager::add_memory_entry(&self.cwd, title, content) {
                                    Ok(path) => {
                                        let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                            item: TranscriptItem {
                                                role: "system".to_string(),
                                                text: Some(format!(
                                                    "Added memory entry: {}",
                                                    path.file_name().and_then(|s| s.to_str()).unwrap_or("unknown")
                                                )),
                                                tool_name: None,
                                                tool_input: None,
                                                is_error: None,
                                            },
                                        });
                                    }
                                    Err(e) => {
                                        let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                            item: TranscriptItem {
                                                role: "system".to_string(),
                                                text: Some(format!("Error: {}", e)),
                                                tool_name: None,
                                                tool_input: None,
                                                is_error: Some(true),
                                            },
                                        });
                                    }
                                }
                            }
                        } else {
                            let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                item: TranscriptItem {
                                    role: "system".to_string(),
                                    text: Some("Usage: /memory add TITLE :: CONTENT".to_string()),
                                    tool_name: None,
                                    tool_input: None,
                                    is_error: Some(true),
                                },
                            });
                        }
                    }
                    "remove" => {
                        if args.is_empty() {
                            let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                item: TranscriptItem {
                                    role: "system".to_string(),
                                    text: Some("Usage: /memory remove <name>".to_string()),
                                    tool_name: None,
                                    tool_input: None,
                                    is_error: Some(true),
                                },
                            });
                        } else {
                            match MemoryManager::remove_memory_entry(&self.cwd, &args) {
                                Ok(true) => {
                                    let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                        item: TranscriptItem {
                                            role: "system".to_string(),
                                            text: Some(format!("Removed memory entry: {}", args)),
                                            tool_name: None,
                                            tool_input: None,
                                            is_error: None,
                                        },
                                    });
                                }
                                Ok(false) => {
                                    let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                        item: TranscriptItem {
                                            role: "system".to_string(),
                                            text: Some(format!("Memory entry not found: {}", args)),
                                            tool_name: None,
                                            tool_input: None,
                                            is_error: Some(true),
                                        },
                                    });
                                }
                                Err(e) => {
                                    let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                        item: TranscriptItem {
                                            role: "system".to_string(),
                                            text: Some(format!("Error: {}", e)),
                                            tool_name: None,
                                            tool_input: None,
                                            is_error: Some(true),
                                        },
                                    });
                                }
                            }
                        }
                    }
                    "" => {
                        // Show memory summary
                        let memory_dir = MemoryManager::get_project_memory_dir(&self.cwd);
                        let entrypoint = MemoryManager::get_memory_entrypoint(&self.cwd);
                        let memory_files = MemoryManager::list_memory_files(&self.cwd);

                        let mut memory_text = format!(
                            "Memory:\n  Directory: {}\n  Index: {}\n  Entries: {}\n",
                            memory_dir.display(),
                            entrypoint.display(),
                            memory_files.len()
                        );

                        if memory_files.is_empty() {
                            memory_text.push_str("\n  No memory entries found.");
                        } else {
                            memory_text.push_str("\n  Memory files:\n");
                            for (i, path) in memory_files.iter().enumerate() {
                                if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                                    memory_text.push_str(&format!("    {}. {}\n", i + 1, name));
                                }
                            }
                        }

                        let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                            item: TranscriptItem {
                                role: "system".to_string(),
                                text: Some(memory_text),
                                tool_name: None,
                                tool_input: None,
                                is_error: None,
                            },
                        });
                    }
                    _ => {
                        let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                            item: TranscriptItem {
                                role: "system".to_string(),
                                text: Some("Usage: /memory [list|show <name>|add TITLE :: CONTENT|remove <name>]".to_string()),
                                tool_name: None,
                                tool_input: None,
                                is_error: Some(true),
                            },
                        });
                    }
                }
            }
            "/resume" => {
                use crate::services::session_storage::load_session_by_id;

                let parts: Vec<&str> = cmd.split_whitespace().collect();
                if parts.len() < 2 {
                    let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                        item: TranscriptItem {
                            role: "system".to_string(),
                            text: Some("Usage: /resume <session-id>\n\nUse /sessions to list available sessions.".to_string()),
                            tool_name: None,
                            tool_input: None,
                            is_error: None,
                        },
                    });
                } else {
                    let session_id = parts[1];
                    match load_session_by_id(&self.cwd, session_id) {
                        Some(data) => {
                            // Load session messages into query engine
                            if self.query_engine.is_none() {
                                let _ = self.init_query_engine().await;
                            }

                            if let Some(ref qe) = self.query_engine {
                                let messages = serde_json::from_str::<Vec<serde_json::Value>>(
                                    &serde_json::to_string(&data.messages).unwrap_or_default()
                                ).unwrap_or_default();

                                qe.load_messages(messages).await;

                                let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                    item: TranscriptItem {
                                        role: "system".to_string(),
                                        text: Some(format!(
                                            "Resumed session: {}\n  Summary: {}\n  Messages loaded: {}",
                                            session_id,
                                            data.summary.as_deref().unwrap_or("(no summary)"),
                                            data.messages.len()
                                        )),
                                        tool_name: None,
                                        tool_input: None,
                                        is_error: None,
                                    },
                                });
                            }
                        }
                        None => {
                            let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                item: TranscriptItem {
                                    role: "system".to_string(),
                                    text: Some(format!("Session '{}' not found.", session_id)),
                                    tool_name: None,
                                    tool_input: None,
                                    is_error: Some(true),
                                },
                            });
                        }
                    }
                }
            }
            "/sessions" => {
                use crate::services::session_storage::{list_session_snapshots, get_project_session_dir};

                let session_dir = get_project_session_dir(&self.cwd);
                let sessions = list_session_snapshots(&self.cwd, 20);

                let mut session_text = format!(
                    "Sessions (directory: {}):\n",
                    session_dir.display()
                );

                if sessions.is_empty() {
                    session_text.push_str("\n  No saved sessions found.");
                } else {
                    for (i, session) in sessions.iter().enumerate() {
                        session_text.push_str(&format!(
                            "\n  {}. {} [{}]
      Created: {}
      Messages: {}",
                            i + 1,
                            session.session_id,
                            session.summary.chars().take(50).collect::<String>(),
                            session.created_at.format("%Y-%m-%d %H:%M"),
                            session.message_count
                        ));
                    }
                }

                let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                    item: TranscriptItem {
                        role: "system".to_string(),
                        text: Some(session_text),
                        tool_name: None,
                        tool_input: None,
                        is_error: None,
                    },
                });
            }
            "/export" => {
                use crate::services::session_storage::export_session_markdown;

                // Get messages from current session
                let messages = if let Some(ref qe) = self.query_engine {
                    let msgs = qe.get_messages().await;
                    // Convert ConversationMessage to serde_json::Value
                    msgs.iter()
                        .filter_map(|m| serde_json::to_value(m).ok())
                        .collect::<Vec<_>>()
                } else {
                    vec![]
                };

                if messages.is_empty() {
                    let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                        item: TranscriptItem {
                            role: "system".to_string(),
                            text: Some("No messages to export. Start a conversation first.".to_string()),
                            tool_name: None,
                            tool_input: None,
                            is_error: None,
                        },
                    });
                } else {
                    match export_session_markdown(&self.cwd, &messages) {
                        Ok(path) => {
                            let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                item: TranscriptItem {
                                    role: "system".to_string(),
                                    text: Some(format!(
                                        "Session exported to:\n  {}",
                                        path.display()
                                    )),
                                    tool_name: None,
                                    tool_input: None,
                                    is_error: None,
                                },
                            });
                        }
                        Err(e) => {
                            let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                item: TranscriptItem {
                                    role: "system".to_string(),
                                    text: Some(format!("Failed to export session: {}", e)),
                                    tool_name: None,
                                    tool_input: None,
                                    is_error: Some(true),
                                },
                            });
                        }
                    }
                }
            }
            "/delete_session" => {
                use std::fs;
                use crate::services::session_storage::get_project_session_dir;

                let parts: Vec<&str> = cmd.split_whitespace().collect();
                if parts.len() < 2 {
                    let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                        item: TranscriptItem {
                            role: "system".to_string(),
                            text: Some("Usage: /delete_session <session-id>".to_string()),
                            tool_name: None,
                            tool_input: None,
                            is_error: Some(true),
                        },
                    });
                } else {
                    let session_id = parts[1];
                    let session_path = get_project_session_dir(&self.cwd).join(format!("{}.json", session_id));

                    if !session_path.exists() {
                        let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                            item: TranscriptItem {
                                role: "system".to_string(),
                                text: Some(format!("Session not found: {}", session_id)),
                                tool_name: None,
                                tool_input: None,
                                is_error: Some(true),
                            },
                        });
                    } else {
                        match fs::remove_file(&session_path) {
                            Ok(_) => {
                                let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                    item: TranscriptItem {
                                        role: "system".to_string(),
                                        text: Some(format!("Session {} deleted.", session_id)),
                                        tool_name: None,
                                        tool_input: None,
                                        is_error: None,
                                    },
                                });
                            }
                            Err(e) => {
                                let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                    item: TranscriptItem {
                                        role: "system".to_string(),
                                        text: Some(format!("Failed to delete session: {}", e)),
                                        tool_name: None,
                                        tool_input: None,
                                        is_error: Some(true),
                                    },
                                });
                            }
                        }
                    }
                }
            }
            "/init" => {
                use crate::config::default_settings::{initialize_defaults, initialize_project};

                let parts: Vec<&str> = cmd.split_whitespace().collect();
                let mut results = Vec::new();

                // Initialize global config if --global flag or no args
                if parts.len() == 1 || parts.contains(&"--global") || parts.contains(&"-g") {
                    match initialize_defaults() {
                        Ok(msg) => results.push(msg),
                        Err(e) => {
                            let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                item: TranscriptItem {
                                    role: "system".to_string(),
                                    text: Some(format!("Error initializing global config: {}", e)),
                                    tool_name: None,
                                    tool_input: None,
                                    is_error: Some(true),
                                },
                            });
                            return Ok(());
                        }
                    }
                }

                // Initialize project config if --project flag or no args
                if parts.len() == 1 || parts.contains(&"--project") || parts.contains(&"-p") {
                    match initialize_project(&self.cwd) {
                        Ok(msg) => results.push(msg),
                        Err(e) => {
                            let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                                item: TranscriptItem {
                                    role: "system".to_string(),
                                    text: Some(format!("Error initializing project: {}", e)),
                                    tool_name: None,
                                    tool_input: None,
                                    is_error: Some(true),
                                },
                            });
                            return Ok(());
                        }
                    }
                }

                // Create CLAUDE.md if not exists
                let claude_md_path = std::path::Path::new(&self.cwd).join("CLAUDE.md");
                if !claude_md_path.exists() {
                    let content = "# CLAUDE.md\n\nThis file provides guidance to Claude Code when working with code in this repository.\n";
                    if let Err(e) = std::fs::write(&claude_md_path, content) {
                        let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                            item: TranscriptItem {
                                role: "system".to_string(),
                                text: Some(format!("Error creating CLAUDE.md: {}", e)),
                                tool_name: None,
                                tool_input: None,
                                is_error: Some(true),
                            },
                        });
                        return Ok(());
                    }
                    results.push(format!("Created: {}", claude_md_path.display()));
                }

                let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                    item: TranscriptItem {
                        role: "system".to_string(),
                        text: Some(results.join("\n\n")),
                        tool_name: None,
                        tool_input: None,
                        is_error: None,
                    },
                });
            }
            "/version" => {
                let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                    item: TranscriptItem {
                        role: "system".to_string(),
                        text: Some("RustHarness v0.1.0".to_string()),
                        tool_name: None,
                        tool_input: None,
                        is_error: None,
                    },
                });
            }
            _ => {
                let _ = self.send_event(stdout, BackendEvent::TranscriptItem {
                    item: TranscriptItem {
                        role: "system".to_string(),
                        text: Some(format!("Unknown command: {}", cmd)),
                        tool_name: None,
                        tool_input: None,
                        is_error: Some(true),
                    },
                });
            }
        }
        Ok(())
    }
}

/// Run the stdio backend
pub async fn run_stdio_backend(settings: Settings, shutdown_tx: mpsc::Sender<()>) -> io::Result<()> {
    let backend = StdioBackend::new(settings, shutdown_tx);
    backend.run().await
}
