//! State store

#![allow(dead_code)]

use crate::state::app_state::AppState;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct StateStore {
    states: HashMap<String, AppState>,
}

impl StateStore {
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
        }
    }

    pub fn get(&self, session_id: &str) -> Option<&AppState> {
        self.states.get(session_id)
    }

    pub fn insert(&mut self, state: AppState) {
        let session_id = state.session_id.clone();
        self.states.insert(session_id, state);
    }
}

impl Default for StateStore {
    fn default() -> Self {
        Self::new()
    }
}

// Thread-safe version
pub type SharedStateStore = Arc<Mutex<StateStore>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_store_new() {
        let store = StateStore::new();
        assert!(store.states.is_empty());
    }

    #[test]
    fn test_state_store_default() {
        let store = StateStore::default();
        assert!(store.states.is_empty());
    }

    #[test]
    fn test_state_store_get_nonexistent() {
        let store = StateStore::new();
        assert!(store.get("nonexistent").is_none());
    }

    #[test]
    fn test_state_store_insert_and_get() {
        let mut store = StateStore::new();
        let state = AppState {
            session_id: "test-session".to_string(),
            ..Default::default()
        };
        store.insert(state);
        assert!(store.get("test-session").is_some());
    }
}
