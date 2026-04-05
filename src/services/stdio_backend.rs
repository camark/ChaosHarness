//! Stdio backend for React TUI frontend using OHJSON protocol
//!
//! Protocol format: Messages are prefixed with "OHJSON:" followed by JSON

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
            self.query_engine = Some(QueryEngine::new(
                self.settings.clone(),
                tool_registry,
                self.cwd.clone().into(),
            ).map_err(|e| {
                io::Error::new(io::ErrorKind::Other, format!("Failed to create query engine: {}", e))
            })?);
        }
        Ok(())
    }

    pub async fn run(mut self) -> io::Result<()> {
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
                mcp_servers: vec![],
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
            "/config".to_string(),
            "/memory".to_string(),
            "/resume".to_string(),
            "/sessions".to_string(),
            "/export".to_string(),
            "/permissions".to_string(),
            "/plan".to_string(),
        ]
    }

    fn send_event<W: Write>(
        &self,
        writer: &mut W,
        event: BackendEvent,
    ) -> io::Result<()> {
        let json = serde_json::to_string(&event).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("Failed to serialize event: {}", e))
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
                self.send_event(stdout, BackendEvent::TranscriptItem {
                    item: TranscriptItem {
                        role: "user".to_string(),
                        text: Some(line.clone()),
                        tool_name: None,
                        tool_input: None,
                        is_error: None,
                    },
                })?;

                // Process the line
                self.process_line(stdout, &line).await?;
            }
            FrontendMessage::Shutdown => {
                self.send_event(stdout, BackendEvent::Shutdown)?;
                let _ = self.shutdown_tx.send(()).await;
            }
            FrontendMessage::ListSessions => {
                // For now, send empty session list
                self.send_event(stdout, BackendEvent::SelectRequest {
                    modal: serde_json::json!({
                        "title": "Sessions",
                        "submit_prefix": "/resume ",
                    }),
                    select_options: vec![],
                })?;
            }
            FrontendMessage::QuestionResponse { request_id: _, answer } => {
                self.send_event(stdout, BackendEvent::TranscriptItem {
                    item: TranscriptItem {
                        role: "user".to_string(),
                        text: Some(answer),
                        tool_name: None,
                        tool_input: None,
                        is_error: None,
                    },
                })?;
            }
            FrontendMessage::PermissionResponse { request_id: _, allowed } => {
                let status = if allowed { "allowed" } else { "denied" };
                self.send_event(stdout, BackendEvent::TranscriptItem {
                    item: TranscriptItem {
                        role: "system".to_string(),
                        text: Some(format!("Permission {}", status)),
                        tool_name: None,
                        tool_input: None,
                        is_error: None,
                    },
                })?;
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
            self.handle_command(stdout, trimmed).await?;
            self.send_event(stdout, BackendEvent::LineComplete)?;
            return Ok(());
        }

        // Initialize query engine if not already done
        if self.query_engine.is_none() {
            self.init_query_engine().await?;
        }

        // Send to AI
        if let Some(ref mut query_engine) = self.query_engine {
            match query_engine.send_message(trimmed.to_string()).await {
                Ok(response) => {
                    // Send AI response
                    self.send_event(stdout, BackendEvent::TranscriptItem {
                        item: TranscriptItem {
                            role: "assistant".to_string(),
                            text: Some(response),
                            tool_name: None,
                            tool_input: None,
                            is_error: None,
                        },
                    })?;
                }
                Err(e) => {
                    self.send_event(stdout, BackendEvent::Error {
                        message: format!("Error: {}", e),
                    })?;
                }
            }
        } else {
            self.send_event(stdout, BackendEvent::Error {
                message: "Query engine not initialized".to_string(),
            })?;
        }

        self.send_event(stdout, BackendEvent::LineComplete)?;
        Ok(())
    }

    async fn handle_command(
        &mut self,
        stdout: &mut io::Stdout,
        cmd: &str,
    ) -> io::Result<()> {
        match cmd {
            "/exit" | "/quit" => {
                self.send_event(stdout, BackendEvent::Shutdown)?;
                let _ = self.shutdown_tx.send(()).await;
            }
            "/clear" => {
                self.send_event(stdout, BackendEvent::ClearTranscript)?;
            }
            "/help" => {
                let help_text = self.get_available_commands().join("\n");
                self.send_event(stdout, BackendEvent::TranscriptItem {
                    item: TranscriptItem {
                        role: "system".to_string(),
                        text: Some(format!("Available commands:\n{}", help_text)),
                        tool_name: None,
                        tool_input: None,
                        is_error: None,
                    },
                })?;
            }
            "/status" => {
                self.send_event(stdout, BackendEvent::StateSnapshot {
                    state: self.get_initial_state(),
                    mcp_servers: vec![],
                    bridge_sessions: vec![],
                })?;
            }
            cmd if cmd.starts_with("/permissions") => {
                if cmd.contains("set") {
                    let mode = cmd.split_whitespace().nth(2).unwrap_or("default");
                    self.send_event(stdout, BackendEvent::TranscriptItem {
                        item: TranscriptItem {
                            role: "system".to_string(),
                            text: Some(format!("Permission mode set to: {}", mode)),
                            tool_name: None,
                            tool_input: None,
                            is_error: None,
                        },
                    })?;
                } else {
                    self.send_event(stdout, BackendEvent::SelectRequest {
                        modal: serde_json::json!({
                            "title": "Permission Mode",
                            "kind": "permission_picker",
                        }),
                        select_options: vec![
                            SelectOption { value: "default".to_string(), label: "Default".to_string(), description: Some("Ask before write/execute".to_string()) },
                            SelectOption { value: "full_auto".to_string(), label: "Auto".to_string(), description: Some("Allow all tools".to_string()) },
                            SelectOption { value: "plan".to_string(), label: "Plan".to_string(), description: Some("Block writes".to_string()) },
                        ],
                    })?;
                }
            }
            "/plan" => {
                self.send_event(stdout, BackendEvent::TranscriptItem {
                    item: TranscriptItem {
                        role: "system".to_string(),
                        text: Some("Plan mode toggled".to_string()),
                        tool_name: None,
                        tool_input: None,
                        is_error: None,
                    },
                })?;
            }
            _ => {
                self.send_event(stdout, BackendEvent::TranscriptItem {
                    item: TranscriptItem {
                        role: "system".to_string(),
                        text: Some(format!("Unknown command: {}", cmd)),
                        tool_name: None,
                        tool_input: None,
                        is_error: Some(true),
                    },
                })?;
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
