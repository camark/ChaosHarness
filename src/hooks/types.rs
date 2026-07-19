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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_context_new() {
        let ctx = HookContext::new("test_event".to_string(), serde_json::json!({}));
        assert_eq!(ctx.event, "test_event");
    }

    #[test]
    fn test_hook_context_pre_tool_use() {
        let input = serde_json::json!({"command": "ls"});
        let ctx = HookContext::pre_tool_use("bash", &input);
        assert_eq!(ctx.event, "pre_tool_use");
        assert!(ctx.payload["tool_name"].as_str() == Some("bash"));
    }

    #[test]
    fn test_hook_context_post_tool_use() {
        let input = serde_json::json!({"command": "ls"});
        let ctx = HookContext::post_tool_use("bash", &input, "file.txt", false);
        assert_eq!(ctx.event, "post_tool_use");
        assert!(ctx.payload["output"].as_str() == Some("file.txt"));
    }

    #[test]
    fn test_hook_context_on_error() {
        let ctx = HookContext::on_error("test error");
        assert_eq!(ctx.event, "on_error");
        assert!(ctx.payload["error"].as_str() == Some("test error"));
    }

    #[test]
    fn test_hook_context_on_turn_complete() {
        let usage = serde_json::json!({"input_tokens": 100});
        let ctx = HookContext::on_turn_complete(&usage);
        assert_eq!(ctx.event, "on_turn_complete");
    }

    #[test]
    fn test_hook_result_continue() {
        let result = HookResult::continue_result("output".to_string());
        assert!(matches!(result.decision, HookDecision::Continue));
    }

    #[test]
    fn test_hook_result_block() {
        let result = HookResult::block_result("blocked".to_string());
        assert!(matches!(result.decision, HookDecision::Block(_)));
    }

    #[test]
    fn test_hook_result_modify() {
        let new_value = serde_json::json!({"modified": true});
        let result = HookResult::modify_result("output".to_string(), new_value);
        assert!(matches!(result.decision, HookDecision::Modify(_)));
    }
}
