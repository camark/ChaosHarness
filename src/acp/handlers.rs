//! ACP request handlers
//!
//! Handlers for processing ACP requests and integrating with local tools.

use crate::acp::types::*;
use crate::acp::server::AcpServerState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde_json::json;
use tracing::{info, debug};

/// Handler for AgentCard requests
pub async fn handle_get_agent_card(
    State(state): State<AcpServerState>,
) -> Json<AgentCard> {
    Json(state.agent_card().clone())
}

/// Handler for task creation
pub async fn handle_create_task(
    State(state): State<AcpServerState>,
    Json(request): Json<CreateTaskRequest>,
) -> Result<Json<Task>, (StatusCode, Json<ErrorResponse>)> {
    use chrono::Utc;

    let task_id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    let mut artifacts = Vec::new();
    let mut history = Vec::new();

    // Process initial message if provided
    if let Some(message) = request.message {
        // Store the message as an artifact
        artifacts.push(TaskArtifact {
            id: format!("{}-msg-1", task_id),
            name: Some("Initial user message".to_string()),
            artifact_type: ArtifactType::Text,
            content: Some(serde_json::to_value(&message).unwrap_or_default()),
            mime_type: Some("application/json".to_string()),
            final_output: None,
        });

        history.push(TaskHistoryEntry {
            entry_type: HistoryEntryType::Message,
            timestamp: now.clone(),
            details: Some(json!({"role": "user", "preview": message_preview(&message)})),
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

    // Store the task
    {
        let mut tasks = state.tasks.write().await;
        tasks.insert(task_id, task.clone());
    }

    info!("Created new task: {}", task.id);
    Ok(Json(task))
}

/// Handler for getting task details
pub async fn handle_get_task(
    State(state): State<AcpServerState>,
    Path(task_id): Path<String>,
) -> Result<Json<Task>, (StatusCode, Json<ErrorResponse>)> {
    let tasks = state.tasks.read().await;

    match tasks.get(&task_id) {
        Some(task) => Ok(Json(task.clone())),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                code: Some("TASK_NOT_FOUND".to_string()),
                message: format!("Task '{}' not found", task_id),
                details: None,
            }),
        )),
    }
}

/// Handler for sending messages to a task
pub async fn handle_send_message(
    State(state): State<AcpServerState>,
    Path(task_id): Path<String>,
    Json(request): Json<TaskSendRequest>,
) -> Result<Json<TaskSendResponse>, (StatusCode, Json<ErrorResponse>)> {
    use chrono::Utc;

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

    // Create artifact from the message
    let artifact = TaskArtifact {
        id: format!("{}-msg-{}", task_id, task.artifacts.len() + 1),
        name: Some(format!("{:?} message", request.message.role)),
        artifact_type: ArtifactType::Text,
        content: Some(serde_json::to_value(&request.message).unwrap_or_default()),
        mime_type: Some("application/json".to_string()),
        final_output: None,
    };

    task.artifacts.push(artifact.clone());
    task.history.push(TaskHistoryEntry {
        entry_type: HistoryEntryType::Message,
        timestamp: Utc::now().to_rfc3339(),
        details: Some(json!({
            "role": request.message.role,
            "parts": request.message.content.len(),
        })),
    });

    // Update status based on message
    match request.message.role {
        MessageRole::User => {
            task.status = TaskStatus::Working;
        }
        MessageRole::Agent => {
            // Agent responded, might be complete or need more input
            if task.status == TaskStatus::Working {
                // Keep working status until agent indicates completion
            }
        }
        MessageRole::System => {
            // System messages don't change status
        }
    }

    task.updated_at = Some(Utc::now().to_rfc3339());

    debug!("Message sent to task {}: {:?}", task.id, task.status);

    Ok(Json(TaskSendResponse {
        id: task_id,
        artifacts: vec![artifact],
        status: task.status.clone(),
    }))
}

/// Handler for getting task artifacts
pub async fn handle_get_artifacts(
    State(state): State<AcpServerState>,
    Path(task_id): Path<String>,
) -> Result<Json<Vec<TaskArtifact>>, (StatusCode, Json<ErrorResponse>)> {
    let tasks = state.tasks.read().await;

    match tasks.get(&task_id) {
        Some(task) => Ok(Json(task.artifacts.clone())),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                code: Some("TASK_NOT_FOUND".to_string()),
                message: format!("Task '{}' not found", task_id),
                details: None,
            }),
        )),
    }
}

/// Handler for canceling a task
pub async fn handle_cancel_task(
    State(state): State<AcpServerState>,
    Path(task_id): Path<String>,
) -> Result<Json<Task>, (StatusCode, Json<ErrorResponse>)> {
    use chrono::Utc;

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
    task.updated_at = Some(Utc::now().to_rfc3339());
    task.history.push(TaskHistoryEntry {
        entry_type: HistoryEntryType::StatusChange,
        timestamp: Utc::now().to_rfc3339(),
        details: Some(json!({"status": "canceled"})),
    });

    info!("Task {} canceled", task.id);
    Ok(Json(task.clone()))
}

/// Handler for submitting input to a task
pub async fn handle_submit_input(
    State(state): State<AcpServerState>,
    Path(task_id): Path<String>,
    Json(request): Json<TaskSendRequest>,
) -> Result<Json<TaskSendResponse>, (StatusCode, Json<ErrorResponse>)> {
    use chrono::Utc;

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

    // Validate task is awaiting input
    if task.status != TaskStatus::InputRequired {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                code: Some("INVALID_STATUS".to_string()),
                message: format!(
                    "Task is not awaiting input (current status: {:?})",
                    task.status
                ),
                details: None,
            }),
        ));
    }

    // Record the input
    task.history.push(TaskHistoryEntry {
        entry_type: HistoryEntryType::UserInput,
        timestamp: Utc::now().to_rfc3339(),
        details: Some(serde_json::to_value(&request.message).unwrap_or_default()),
    });

    // Transition back to working
    task.status = TaskStatus::Working;
    task.updated_at = Some(Utc::now().to_rfc3339());

    debug!("Input submitted to task {}", task.id);

    Ok(Json(TaskSendResponse {
        id: task_id,
        artifacts: vec![],
        status: TaskStatus::Working,
    }))
}

/// Helper function to generate a preview of a message
fn message_preview(message: &Message) -> String {
    message
        .content
        .iter()
        .filter_map(|part| {
            if let MessagePart::Text { text } = part {
                Some(text.as_str())
            } else {
                None
            }
        })
        .next()
        .unwrap_or("[non-text content]")
        .chars()
        .take(100)
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::acp::client::MessageBuilder;

    #[test]
    fn test_message_preview_text() {
        let message = MessageBuilder::user()
            .add_text("Hello, this is a test message!")
            .build();

        let preview = message_preview(&message);
        assert_eq!(preview, "Hello, this is a test message!");
    }

    #[test]
    fn test_message_preview_truncated() {
        let long_text = "a".repeat(200);
        let message = MessageBuilder::user()
            .add_text(&long_text)
            .build();

        let preview = message_preview(&message);
        assert_eq!(preview.len(), 100);
    }

    #[test]
    fn test_message_preview_non_text() {
        let message = MessageBuilder::user()
            .add_data(json!({"key": "value"}))
            .build();

        let preview = message_preview(&message);
        assert_eq!(preview, "[non-text content]");
    }
}

