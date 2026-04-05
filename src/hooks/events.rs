//! Hook event types

use serde::{Deserialize, Serialize};

/// Hook events that can be triggered during agent execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum HookEvent {
    /// Before a tool is executed (can block or modify)
    PreToolUse,
    /// After a tool is executed (can modify output or log)
    PostToolUse,
    /// Before sending request to the model
    PreModelRequest,
    /// After receiving response from the model
    PostModelRequest,
    /// When an error occurs
    OnError,
    /// When a turn completes successfully
    OnTurnComplete,
}

impl HookEvent {
    pub fn as_str(&self) -> &'static str {
        match self {
            HookEvent::PreToolUse => "pre_tool_use",
            HookEvent::PostToolUse => "post_tool_use",
            HookEvent::PreModelRequest => "pre_model_request",
            HookEvent::PostModelRequest => "post_model_request",
            HookEvent::OnError => "on_error",
            HookEvent::OnTurnComplete => "on_turn_complete",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pre_tool_use" => Some(HookEvent::PreToolUse),
            "post_tool_use" => Some(HookEvent::PostToolUse),
            "pre_model_request" => Some(HookEvent::PreModelRequest),
            "post_model_request" => Some(HookEvent::PostModelRequest),
            "on_error" => Some(HookEvent::OnError),
            "on_turn_complete" => Some(HookEvent::OnTurnComplete),
            _ => None,
        }
    }
}
