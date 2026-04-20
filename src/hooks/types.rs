//! Hook types and context

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Context passed to hooks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookContext {
    pub event: String,
    pub payload: Value,
}

impl HookContext {
    pub fn new(event: String, payload: Value) -> Self {
        Self { event, payload }
    }

    pub fn pre_tool_use(tool_name: &str, tool_input: &Value) -> Self {
        Self {
            event: "pre_tool_use".to_string(),
            payload: serde_json::json!({
                "tool_name": tool_name,
                "tool_input": tool_input
            }),
        }
    }

    pub fn post_tool_use(tool_name: &str, tool_input: &Value, output: &str, is_error: bool) -> Self {
        Self {
            event: "post_tool_use".to_string(),
            payload: serde_json::json!({
                "tool_name": tool_name,
                "tool_input": tool_input,
                "output": output,
                "is_error": is_error
            }),
        }
    }

    pub fn on_error(error: &str) -> Self {
        Self {
            event: "on_error".to_string(),
            payload: serde_json::json!({
                "error": error
            }),
        }
    }

    pub fn on_turn_complete(usage: &Value) -> Self {
        Self {
            event: "on_turn_complete".to_string(),
            payload: serde_json::json!({
                "usage": usage
            }),
        }
    }
}

/// Result from hook execution
#[derive(Debug, Clone)]
pub struct HookResult {
    pub output: String,
    pub decision: HookDecision,
}

/// Decision from a hook (for blocking/modifying hooks)
#[derive(Debug, Clone)]
pub enum HookDecision {
    /// Continue normally
    Continue,
    /// Block the operation with a reason
    Block(String),
    /// Modify the input/output
    Modify(Value),
}

impl HookResult {
    pub fn continue_result(output: String) -> Self {
        Self {
            output,
            decision: HookDecision::Continue,
        }
    }

    pub fn block_result(reason: String) -> Self {
        Self {
            output: reason.clone(),
            decision: HookDecision::Block(reason),
        }
    }

    pub fn modify_result(output: String, new_value: Value) -> Self {
        Self {
            output,
            decision: HookDecision::Modify(new_value),
        }
    }
}
