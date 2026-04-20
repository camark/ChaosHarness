//! Application state

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use crate::engine::messages::ConversationMessage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    pub session_id: String,
    pub messages: Vec<ConversationMessage>,
    pub current_model: String,
    pub working_directory: String,
}

impl AppState {
    pub fn new(working_directory: String) -> Self {
        use uuid::Uuid;
        Self {
            session_id: Uuid::new_v4().to_string(),
            messages: Vec::new(),
            current_model: "claude-sonnet-4-20250514".to_string(),
            working_directory,
        }
    }

    pub fn add_message(&mut self, message: ConversationMessage) {
        self.messages.push(message);
    }
}
