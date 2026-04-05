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
    /// When a message is sent by the user
    OnMessageSent,
    /// When a response is received from the AI
    OnResponseReceived,
    /// When session starts
    OnSessionStart,
    /// When session ends
    OnSessionEnd,
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
            HookEvent::OnMessageSent => "on_message_sent",
            HookEvent::OnResponseReceived => "on_response_received",
            HookEvent::OnSessionStart => "on_session_start",
            HookEvent::OnSessionEnd => "on_session_end",
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
            "on_message_sent" => Some(HookEvent::OnMessageSent),
            "on_response_received" => Some(HookEvent::OnResponseReceived),
            "on_session_start" => Some(HookEvent::OnSessionStart),
            "on_session_end" => Some(HookEvent::OnSessionEnd),
            _ => None,
        }
    }
}
