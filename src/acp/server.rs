//! ACP Server implementation
//!
//! REST API server exposing the agent's capabilities via ACP protocol.
//! Implements the standard ACP endpoints:
//! - GET /.well-known/agent.json - AgentCard discovery
//! - GET /acp - AgentCard endpoint
//! - POST /tasks - Create new task
//! - GET /tasks/{id} - Get task status
//! - POST /tasks/{id}/send - Send message to task
//! - GET /tasks/{id}/artifacts - Get task artifacts

use crate::acp::types::*;
use crate::acp::AgentCard;
use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    Json,
    routing::get,
    routing::post,
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};

/// ACP Server state
#[derive(Clone)]
#[allow(dead_code)]
pub struct AcpServerState {
    /// AgentCard for this server
    pub agent_card: Arc<AgentCard>,
    /// Active tasks
    pub tasks: Arc<RwLock<HashMap<String, Task>>>,
    /// Base URL for the server
    pub base_url: String,
}

/// Create the ACP router
pub fn create_acp_router(state: AcpServerState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // AgentCard discovery endpoint (standard location)
        .route("/.well-known/agent.json", get(get_agent_card))
        // AgentCard endpoint
        .route("/acp", get(get_agent_card))
        // Task endpoints
        .route("/tasks", post(create_task))
        .route("/tasks/:id", get(get_task))
        .route("/tasks/:id/send", post(send_message))
        .route("/tasks/:id/artifacts", get(get_artifacts))
        .route("/tasks/:id/cancel", post(cancel_task))
        .route("/tasks/:id/input", post(submit_input))
        .layer(cors)
        .with_state(state)
}

/// Get the AgentCard
async fn get_agent_card(State(state): State<AcpServerState>) -> Json<AgentCard> {
    Json((*state.agent_card).clone())
}

/// Create a new task
async fn create_task(
    State(state): State<AcpServerState>,
    Json(request): Json<CreateTaskRequest>,
) -> Result<Json<Task>, (StatusCode, Json<ErrorResponse>)> {
    let task_id = generate_task_id();
    let now = chrono::Utc::now().to_rfc3339();

    let mut artifacts = Vec::new();
    let mut history = Vec::new();

    // Add initial message if provided
    if let Some(message) = request.message {
        artifacts.push(TaskArtifact {
            id: format!("{}-artifact-1", task_id),
            name: Some("Initial message".to_string()),
            artifact_type: ArtifactType::Text,
            content: Some(serde_json::to_value(&message).unwrap_or_default()),
            mime_type: Some("application/json".to_string()),
            final_output: None,
        });

        history.push(TaskHistoryEntry {
            entry_type: HistoryEntryType::Message,
            timestamp: now.clone(),
            details: Some(json!({"role": "user"})),
        });
    }

    let task = Task {
        id: task_id.clone(),
        status: TaskStatus::Submitted,
        description: Some(request.description),
        artifacts,
        history,
        created_at: now.clone(),
        updated_at: Some(now),
        metadata: request.metadata,
    };

    // Store task
    {
        let mut tasks = state.tasks.write().await;
        tasks.insert(task_id, task.clone());
    }

    Ok(Json(task))
}

/// Get task status and details
async fn get_task(
    State(state): State<AcpServerState>,
    Path(task_id): Path<String>,
) -> Result<Json<Task>, (StatusCode, Json<ErrorResponse>)> {
    let tasks = state.tasks.read().await;
    tasks
        .get(&task_id)
        .map(|t| Json(t.clone()))
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    code: Some("TASK_NOT_FOUND".to_string()),
                    message: format!("Task '{}' not found", task_id),
                    details: None,
                }),
            )
        })
}

/// Send a message to a task
async fn send_message(
    State(state): State<AcpServerState>,
    Path(task_id): Path<String>,
    Json(request): Json<TaskSendRequest>,
) -> Result<Json<TaskSendResponse>, (StatusCode, Json<ErrorResponse>)> {
    let mut tasks = state.tasks.write().await;

    let task = tasks.get_mut(&task_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                code: Some("TASK_NOT_FOUND".to_string()),
                message: format!("Task '{}' not found", task_id),
                details: None,
            }),
        )
    })?;

    // Add message to artifacts
    let artifact = TaskArtifact {
        id: format!("{}-artifact-{}", task_id, task.artifacts.len() + 1),
        name: Some("User message".to_string()),
        artifact_type: ArtifactType::Text,
        content: Some(serde_json::to_value(&request.message).unwrap_or_default()),
        mime_type: Some("application/json".to_string()),
        final_output: None,
    };

    task.artifacts.push(artifact.clone());
    task.history.push(TaskHistoryEntry {
        entry_type: HistoryEntryType::Message,
        timestamp: chrono::Utc::now().to_rfc3339(),
        details: Some(json!({"role": request.message.role})),
    });

    // Update task status based on message role
    match request.message.role {
        MessageRole::User => {
            task.status = TaskStatus::Working;
        }
        MessageRole::Agent | MessageRole::System => {
            // Keep current status or mark as completed if this is a response
        }
    }

    task.updated_at = Some(chrono::Utc::now().to_rfc3339());

    Ok(Json(TaskSendResponse {
        id: task_id,
        artifacts: vec![artifact],
        status: task.status.clone(),
    }))
}

/// Get task artifacts
async fn get_artifacts(
    State(state): State<AcpServerState>,
    Path(task_id): Path<String>,
) -> Result<Json<Vec<TaskArtifact>>, (StatusCode, Json<ErrorResponse>)> {
    let tasks = state.tasks.read().await;
    let task = tasks.get(&task_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                code: Some("TASK_NOT_FOUND".to_string()),
                message: format!("Task '{}' not found", task_id),
                details: None,
            }),
        )
    })?;

    Ok(Json(task.artifacts.clone()))
}

/// Cancel a task
async fn cancel_task(
    State(state): State<AcpServerState>,
    Path(task_id): Path<String>,
) -> Result<Json<Task>, (StatusCode, Json<ErrorResponse>)> {
    let mut tasks = state.tasks.write().await;

    let task = tasks.get_mut(&task_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                code: Some("TASK_NOT_FOUND".to_string()),
                message: format!("Task '{}' not found", task_id),
                details: None,
            }),
        )
    })?;

    task.status = TaskStatus::Canceled;
    task.updated_at = Some(chrono::Utc::now().to_rfc3339());
    task.history.push(TaskHistoryEntry {
        entry_type: HistoryEntryType::StatusChange,
        timestamp: chrono::Utc::now().to_rfc3339(),
        details: Some(json!({"status": "canceled"})),
    });

    Ok(Json(task.clone()))
}

/// Submit input to a task requiring user input
async fn submit_input(
    State(state): State<AcpServerState>,
    Path(task_id): Path<String>,
    Json(request): Json<TaskSendRequest>,
) -> Result<Json<TaskSendResponse>, (StatusCode, Json<ErrorResponse>)> {
    let mut tasks = state.tasks.write().await;

    let task = tasks.get_mut(&task_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                code: Some("TASK_NOT_FOUND".to_string()),
                message: format!("Task '{}' not found", task_id),
                details: None,
            }),
        )
    })?;

    // Check if task requires input
    if task.status != TaskStatus::InputRequired {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                code: Some("INVALID_STATUS".to_string()),
                message: format!("Task status is '{:?}', not awaiting input", task.status),
                details: None,
            }),
        ));
    }

    // Add input to history
    task.history.push(TaskHistoryEntry {
        entry_type: HistoryEntryType::UserInput,
        timestamp: chrono::Utc::now().to_rfc3339(),
        details: Some(serde_json::to_value(&request.message).unwrap_or_default()),
    });

    // Update status back to working
    task.status = TaskStatus::Working;
    task.updated_at = Some(chrono::Utc::now().to_rfc3339());

    Ok(Json(TaskSendResponse {
        id: task_id,
        artifacts: vec![],
        status: TaskStatus::Working,
    }))
}

/// Generate a unique task ID
fn generate_task_id() -> String {
    use uuid::Uuid;
    Uuid::new_v4().to_string()
}

impl AcpServerState {
    /// Create a new ACP server state
    pub fn new(base_url: &str) -> Self {
        let agent_card = AgentCard::for_rust_harness(base_url);
        Self {
            agent_card: Arc::new(agent_card),
            tasks: Arc::new(RwLock::new(HashMap::new())),
            base_url: base_url.to_string(),
        }
    }

    /// Get the AgentCard
    pub fn agent_card(&self) -> &AgentCard {
        &self.agent_card
    }

    /// Get a task by ID
    #[allow(dead_code)]
    pub async fn get_task(&self, task_id: &str) -> Option<Task> {
        let tasks = self.tasks.read().await;
        tasks.get(task_id).cloned()
    }

    /// List all tasks
    #[allow(dead_code)]
    pub async fn list_tasks(&self) -> Vec<Task> {
        let tasks = self.tasks.read().await;
        tasks.values().cloned().collect()
    }
}

/// Run the ACP server on the specified port
pub async fn run_acp_server(port: u16) -> std::io::Result<()> {
    use std::net::SocketAddr;

    let state = AcpServerState::new(&format!("http://localhost:{}", port));
    let app = create_acp_router(state);

    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
    tracing::info!("Starting ACP server on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_task_id() {
        let id1 = generate_task_id();
        let id2 = generate_task_id();
        assert_ne!(id1, id2);
    }

    #[tokio::test]
    async fn test_acp_server_state() {
        let state = AcpServerState::new("http://localhost:8080");

        assert_eq!(state.agent_card().name, "RustHarness");
        assert!(state.list_tasks().await.is_empty());
    }
}
