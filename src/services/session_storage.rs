//! Session storage

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use uuid::Uuid;
use chrono::Utc;

/// Session snapshot for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSnapshot {
    pub session_id: String,
    pub summary: String,
    pub message_count: u32,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Session data for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub session_id: String,
    pub messages: Vec<serde_json::Value>,
    pub summary: Option<String>,
    pub model: Option<String>,
    pub system_prompt: Option<String>,
}

/// Get the session directory for a project
pub fn get_project_session_dir(cwd: &str) -> PathBuf {
    Path::new(cwd).join(".rust_harness").join("sessions")
}

/// Save session snapshot and return session ID
pub fn save_session(cwd: &str, data: &SessionData) -> Result<String, String> {
    let session_dir = get_project_session_dir(cwd);
    fs::create_dir_all(&session_dir)
        .map_err(|e| format!("Failed to create session directory: {}", e))?;

    let session_id = if data.session_id.is_empty() {
        Uuid::new_v4().to_string()
    } else {
        data.session_id.clone()
    };

    // Save session file
    let session_path = session_dir.join(format!("{}.json", session_id));
    let content = serde_json::to_string_pretty(data)
        .map_err(|e| format!("Failed to serialize session: {}", e))?;
    fs::write(&session_path, &content)
        .map_err(|e| format!("Failed to write session: {}", e))?;

    // Update latest.json symlink (copy)
    let latest_path = session_dir.join("latest.json");
    fs::write(&latest_path, &content)
        .map_err(|e| format!("Failed to update latest session: {}", e))?;

    Ok(session_id)
}

/// Load the latest session snapshot
pub fn load_session_snapshot(cwd: &str) -> Option<SessionData> {
    let latest_path = get_project_session_dir(cwd).join("latest.json");
    if !latest_path.exists() {
        return None;
    }

    let content = fs::read_to_string(&latest_path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Load a session by ID
pub fn load_session_by_id(cwd: &str, session_id: &str) -> Option<SessionData> {
    let session_path = get_project_session_dir(cwd).join(format!("{}.json", session_id));
    if !session_path.exists() {
        return None;
    }

    let content = fs::read_to_string(&session_path).ok()?;
    serde_json::from_str(&content).ok()
}

/// List session snapshots
pub fn list_session_snapshots(cwd: &str, limit: usize) -> Vec<SessionSnapshot> {
    let session_dir = get_project_session_dir(cwd);
    if !session_dir.exists() {
        return Vec::new();
    }

    let mut snapshots: Vec<SessionSnapshot> = fs::read_dir(&session_dir)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(|e| e.ok()))
        .filter_map(|e| {
            let path = e.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                return None;
            }
            if path.file_name().and_then(|s| s.to_str()) == Some("latest.json") {
                return None;
            }

            let content = fs::read_to_string(&path).ok()?;
            let data: SessionData = serde_json::from_str(&content).ok()?;

            // Get file metadata for created_at
            let metadata = fs::metadata(&path).ok()?;
            let created_at = metadata.created()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| chrono::DateTime::from_timestamp(d.as_secs() as i64, 0))
                .flatten()
                .unwrap_or_else(Utc::now);

            Some(SessionSnapshot {
                session_id: data.session_id,
                summary: data.summary.unwrap_or_default(),
                message_count: data.messages.len() as u32,
                created_at,
            })
        })
        .collect();

    // Sort by created_at descending (newest first)
    snapshots.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    // Limit results
    if snapshots.len() > limit {
        snapshots.truncate(limit);
    }

    snapshots
}

/// Export session to markdown
pub fn export_session_markdown(cwd: &str, messages: &[serde_json::Value]) -> Result<PathBuf, String> {
    let session_dir = get_project_session_dir(cwd);
    fs::create_dir_all(&session_dir)
        .map_err(|e| format!("Failed to create session directory: {}", e))?;

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let export_path = session_dir.join(format!("transcript_{}.md", timestamp));

    let mut markdown = String::from("# Session Transcript\n\n");

    for msg in messages {
        let role = msg.get("role").and_then(|v| v.as_str()).unwrap_or("unknown");
        let content = msg.get("content").and_then(|v| v.as_str()).unwrap_or("");

        markdown.push_str(&format!("## {}\n\n{}\n\n", role.to_uppercase(), content));
    }

    fs::write(&export_path, markdown)
        .map_err(|e| format!("Failed to write transcript: {}", e))?;

    Ok(export_path)
}

/// Compact messages (simple implementation)
pub fn compact_messages(messages: &[serde_json::Value], preserve_recent: usize) -> Vec<serde_json::Value> {
    if messages.len() <= preserve_recent {
        return messages.to_vec();
    }

    // Keep only recent messages
    messages[messages.len() - preserve_recent..].to_vec()
}
