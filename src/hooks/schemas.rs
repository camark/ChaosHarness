//! Hook definition schema

use serde::{Deserialize, Serialize};

/// A hook definition from configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookDefinition {
    /// Unique name for this hook
    pub name: String,
    /// Command to execute (shell command or script path)
    pub command: String,
    /// Event that triggers this hook
    pub event: String,
    /// If true, the hook output can block/modify the operation
    #[serde(default)]
    pub blocking: bool,
    /// Timeout in seconds (default: 30)
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    /// Working directory for hook execution
    #[serde(default)]
    pub cwd: Option<String>,
}

fn default_timeout() -> u64 {
    30
}

impl HookDefinition {
    pub fn new(name: String, command: String, event: String) -> Self {
        Self {
            name,
            command,
            event,
            blocking: false,
            timeout: 30,
            cwd: None,
        }
    }

    pub fn blocking(name: String, command: String, event: String) -> Self {
        Self {
            name,
            command,
            event,
            blocking: true,
            timeout: 30,
            cwd: None,
        }
    }
}
