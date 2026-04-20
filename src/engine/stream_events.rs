//! Stream events for real-time output

#![allow(dead_code)]

use crate::engine::messages::ToolUseData;
use crate::api::client::ApiUsage;

/// Stream event types
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// Text delta from the model
    TextDelta(String),
    /// Tool execution started
    ToolStarted {
        tool_name: String,
        tool_input: serde_json::Value,
    },
    /// Tool execution completed
    ToolCompleted {
        tool_name: String,
        output: String,
        is_error: bool,
    },
    /// Assistant turn complete
    TurnComplete {
        text: String,
        tool_uses: Vec<ToolUseData>,
        usage: Option<ApiUsage>,
    },
    /// Error occurred
    Error(String),
}
