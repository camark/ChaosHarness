//! Application state

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

    #[allow(dead_code)]
    pub fn add_message(&mut self, message: ConversationMessage) {
        self.messages.push(message);
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new(".".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_new() {
        let state = AppState::new("/test".to_string());
        assert!(!state.session_id.is_empty());
        assert!(state.messages.is_empty());
        assert_eq!(state.current_model, "claude-sonnet-4-20250514");
        assert_eq!(state.working_directory, "/test");
    }

    #[test]
    fn test_app_state_default() {
        let state = AppState::default();
        assert!(!state.session_id.is_empty());
        assert_eq!(state.working_directory, ".");
    }

    #[test]
    fn test_app_state_add_message() {
        let mut state = AppState::new(".".to_string());
        let msg = ConversationMessage::user_text("test");
        state.add_message(msg);
        assert_eq!(state.messages.len(), 1);
    }

    #[test]
    fn test_app_state_serialization() {
        let state = AppState::new(".".to_string());
        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains("session_id"));
    }

    #[test]
    fn test_app_state_deserialization() {
        let state = AppState::new(".".to_string());
        let json = serde_json::to_string(&state).unwrap();
        let deserialized: AppState = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.session_id, state.session_id);
    }
}
