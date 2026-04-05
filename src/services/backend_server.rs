//! Backend server for React TUI frontend

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::State,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

use crate::config::Settings;

#[derive(Clone)]
pub struct BackendState {
    pub settings: Arc<Settings>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    SubmitLine { line: String },
    Shutdown,
    ListSessions,
    QuestionResponse { request_id: String, answer: String },
    PermissionResponse { request_id: String, allowed: bool },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Transcript { items: Vec<TranscriptItem> },
    Status { status: StatusInfo },
    Commands { commands: Vec<String> },
    SelectRequest { title: String, options: Vec<SelectOption> },
    Error { message: String },
}

#[derive(Debug, Serialize, Clone)]
pub struct TranscriptItem {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct StatusInfo {
    pub permission_mode: String,
    pub model: String,
    pub working_directory: String,
}

#[derive(Debug, Serialize)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Clone)]
pub struct SessionState {
    pub transcript: Vec<TranscriptItem>,
    pub busy: bool,
    pub commands: Vec<String>,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            transcript: Vec::new(),
            busy: false,
            commands: vec![
                "/help".to_string(),
                "/permissions".to_string(),
                "/plan".to_string(),
                "/resume".to_string(),
                "/clear".to_string(),
            ],
        }
    }
}

pub type SharedSessionState = Arc<Mutex<SessionState>>;

pub async fn run_backend_server(settings: Settings, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let state = BackendState {
        settings: Arc::new(settings),
    };
    let session_state: SharedSessionState = Arc::new(Mutex::new(SessionState::default()));

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = axum::Router::new()
        .route("/ws", axum::routing::get(websocket_handler))
        .route("/health", axum::routing::get(health_handler))
        .layer(cors)
        .with_state((state, session_state));

    let addr = format!("127.0.0.1:{}", port);
    info!("Starting backend server on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_handler() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State((_backend_state, session_state)): State<(BackendState, SharedSessionState)>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, session_state))
}

async fn handle_socket(socket: WebSocket, _session_state: SharedSessionState) {
    use futures::{SinkExt, StreamExt};
    let (mut sender, mut receiver) = socket.split();

    // Send initial state
    let initial_status = ServerMessage::Status {
        status: StatusInfo {
            permission_mode: "default".to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            working_directory: std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default(),
        },
    };
    let _ = sender
        .send(Message::Text(serde_json::to_string(&initial_status).unwrap()))
        .await;

    // Send commands
    let commands = ServerMessage::Commands {
        commands: vec![
            "/help".to_string(),
            "/permissions".to_string(),
            "/plan".to_string(),
            "/resume".to_string(),
        ],
    };
    let _ = sender
        .send(Message::Text(serde_json::to_string(&commands).unwrap()))
        .await;

    // Handle incoming messages
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                    info!("Received: {:?}", client_msg);
                    // Handle message - in a full implementation, this would process the request
                }
            }
            Ok(Message::Close(_)) => break,
            _ => {}
        }
    }
}
