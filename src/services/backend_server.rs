//! Backend server for React TUI frontend
//!
//! WebSocket protocol:
//! - Client -> Server: submit_line, shutdown, permission_response
//! - Server -> Client: transcript_update, status, commands, select_request, error

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::State,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast};
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, warn, error};

use crate::config::Settings;

#[derive(Clone)]
pub struct BackendState {
    pub settings: Arc<Settings>,
}

/// Messages from client to server
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    SubmitLine { line: String },
    Shutdown,
    ListSessions,
    SwitchSession { session_id: String },
    QuestionResponse { request_id: String, answer: String },
    PermissionResponse { request_id: String, allowed: bool },
    Ping,
}

/// Messages from server to client
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Connected { session_id: String },
    TranscriptUpdate { items: Vec<TranscriptItem> },
    Status { status: StatusInfo },
    Commands { commands: Vec<String> },
    SelectRequest { request_id: String, title: String, options: Vec<SelectOption> },
    PermissionRequest { request_id: String, tool_name: String, description: String },
    Error { message: String },
    Pong,
    SessionList { sessions: Vec<SessionInfo> },
}

#[derive(Debug, Serialize, Clone)]
pub struct TranscriptItem {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
}

#[derive(Debug, Serialize, Clone)]
pub struct StatusInfo {
    pub permission_mode: String,
    pub model: String,
    pub working_directory: String,
    pub busy: bool,
    pub token_usage: TokenUsage,
}

#[derive(Debug, Serialize, Clone, Default)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[derive(Debug, Serialize)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct SessionInfo {
    pub id: String,
    pub created_at: u64,
    pub last_activity: u64,
    pub message_count: usize,
}

/// Session state with full conversation history
#[derive(Clone)]
pub struct SessionState {
    pub session_id: String,
    pub transcript: Vec<TranscriptItem>,
    pub busy: bool,
    pub commands: Vec<String>,
    pub token_usage: TokenUsage,
    pub created_at: u64,
    pub last_activity: u64,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            session_id: uuid::Uuid::new_v4().to_string(),
            transcript: Vec::new(),
            busy: false,
            commands: vec![
                "/help".to_string(),
                "/permissions".to_string(),
                "/plan".to_string(),
                "/resume".to_string(),
                "/sessions".to_string(),
                "/clear".to_string(),
                "/usage".to_string(),
            ],
            token_usage: TokenUsage::default(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            last_activity: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}

/// Global server state with multiple sessions
#[derive(Clone)]
pub struct ServerState {
    pub settings: Arc<Settings>,
    pub sessions: Arc<Mutex<HashMap<String, SessionState>>>,
    pub active_session: Arc<Mutex<Option<String>>>,
    pub broadcast_tx: broadcast::Sender<String>,
}

impl ServerState {
    pub fn new(settings: Settings) -> Self {
        let (broadcast_tx, _) = broadcast::channel(100);
        let sessions = Arc::new(Mutex::new(HashMap::new()));

        // Create initial session
        let initial_session = SessionState::default();
        let session_id = initial_session.session_id.clone();
        sessions.blocking_lock().insert(session_id.clone(), initial_session);

        Self {
            settings: Arc::new(settings),
            sessions,
            active_session: Arc::new(Mutex::new(Some(session_id))),
            broadcast_tx,
        }
    }

    pub async fn get_active_session(&self) -> Option<SessionState> {
        let active = self.active_session.lock().await;
        if let Some(id) = active.as_ref() {
            let sessions = self.sessions.lock().await;
            sessions.get(id).cloned()
        } else {
            None
        }
    }

    pub async fn update_transcript(&self, item: TranscriptItem) {
        let active = self.active_session.lock().await;
        if let Some(session_id) = active.as_ref() {
            let mut sessions = self.sessions.lock().await;
            if let Some(session) = sessions.get_mut(session_id) {
                session.transcript.push(item);
                session.last_activity = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
            }
        }
    }

    pub async fn set_busy(&self, busy: bool) {
        let active = self.active_session.lock().await;
        if let Some(session_id) = active.as_ref() {
            let mut sessions = self.sessions.lock().await;
            if let Some(session) = sessions.get_mut(session_id) {
                session.busy = busy;
            }
        }
    }
}

pub async fn run_backend_server(settings: Settings, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let state = ServerState::new(settings);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = axum::Router::new()
        .route("/ws", axum::routing::get(websocket_handler))
        .route("/health", axum::routing::get(health_handler))
        .route("/sessions", axum::routing::get(list_sessions_handler))
        .layer(cors)
        .with_state(state);

    let addr = format!("127.0.0.1:{}", port);
    info!("Starting backend server on {}", addr);
    println!("Backend server running on ws://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_handler() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

async fn list_sessions_handler(
    State(state): State<ServerState>,
) -> impl IntoResponse {
    let sessions = state.sessions.lock().await;
    let session_list: Vec<SessionInfo> = sessions.values().map(|s| SessionInfo {
        id: s.session_id.clone(),
        created_at: s.created_at,
        last_activity: s.last_activity,
        message_count: s.transcript.len(),
    }).collect();

    Json(serde_json::json!({ "sessions": session_list }))
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<ServerState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: ServerState) {
    use futures::{SinkExt, StreamExt};
    let (mut sender, mut receiver) = socket.split();

    // Get or create session
    let session_id = {
        let active = state.active_session.lock().await;
        active.clone().unwrap_or_default()
    };

    // Send connected message with session ID
    let connected = ServerMessage::Connected { session_id: session_id.clone() };
    if sender.send(Message::Text(serde_json::to_string(&connected).unwrap())).await.is_err() {
        return;
    }

    // Send initial status
    let status = ServerMessage::Status {
        status: StatusInfo {
            permission_mode: "default".to_string(),
            model: state.settings.model.clone(),
            working_directory: std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default(),
            busy: false,
            token_usage: TokenUsage::default(),
        },
    };
    let _ = sender.send(Message::Text(serde_json::to_string(&status).unwrap())).await;

    // Send available commands
    if let Some(session) = state.get_active_session().await {
        let commands = ServerMessage::Commands {
            commands: session.commands.clone(),
        };
        let _ = sender.send(Message::Text(serde_json::to_string(&commands).unwrap())).await;
    }

    info!("WebSocket client connected, session: {}", session_id);

    // Handle incoming messages
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                    info!("Received: {:?}", client_msg);

                    match client_msg {
                        ClientMessage::SubmitLine { line } => {
                            // Add user message to transcript
                            let item = TranscriptItem {
                                role: "user".to_string(),
                                content: Some(line.clone()),
                                tool_name: None,
                                timestamp: Some(std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs()),
                            };
                            state.update_transcript(item).await;

                            // In full implementation, this would send to the AI
                            state.set_busy(true).await;

                            // For now, just echo back
                            let transcript = ServerMessage::TranscriptUpdate {
                                items: vec![TranscriptItem {
                                    role: "assistant".to_string(),
                                    content: Some(format!("Received: {}", line)),
                                    tool_name: None,
                                    timestamp: Some(std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap()
                                        .as_secs()),
                                }],
                            };
                            let _ = sender.send(Message::Text(serde_json::to_string(&transcript).unwrap())).await;
                            state.set_busy(false).await;
                        }
                        ClientMessage::Ping => {
                            let _ = sender.send(Message::Text(serde_json::to_string(&ServerMessage::Pong).unwrap())).await;
                        }
                        ClientMessage::ListSessions => {
                            let sessions = state.sessions.lock().await;
                            let session_list: Vec<SessionInfo> = sessions.values().map(|s| SessionInfo {
                                id: s.session_id.clone(),
                                created_at: s.created_at,
                                last_activity: s.last_activity,
                                message_count: s.transcript.len(),
                            }).collect();

                            let msg = ServerMessage::SessionList { sessions: session_list };
                            let _ = sender.send(Message::Text(serde_json::to_string(&msg).unwrap())).await;
                        }
                        ClientMessage::Shutdown => {
                            info!("Shutdown requested");
                            break;
                        }
                        _ => {
                            warn!("Unhandled message type: {:?}", client_msg);
                        }
                    }
                }
            }
            Ok(Message::Close(_)) => {
                info!("WebSocket client disconnected");
                break;
            }
            Ok(Message::Ping(data)) => {
                let _ = sender.send(Message::Pong(data)).await;
            }
            Err(e) => {
                error!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }
}
