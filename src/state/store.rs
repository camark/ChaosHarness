//! State store

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
