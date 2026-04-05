//! Session storage

use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSnapshot {
    pub session_id: String,
    pub summary: String,
    pub message_count: u32,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub session_id: String,
    pub messages: Vec<serde_json::Value>,
    pub summary: Option<String>,
    pub model: Option<String>,
    pub system_prompt: Option<String>,
}

pub fn list_session_snapshots(_cwd: &str, _limit: usize) -> Vec<SessionSnapshot> {
    Vec::new()
}

pub fn load_session_by_id(_cwd: &str, _session_id: &str) -> Option<SessionData> {
    None
}

pub fn load_session_snapshot(_cwd: &str) -> Option<SessionData> {
    None
}

pub fn save_session(_cwd: &str, _data: &SessionData) -> Result<String, String> {
    let session_id = Uuid::new_v4().to_string();
    Ok(session_id)
}
