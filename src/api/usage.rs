//! Usage tracking for API calls

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

impl Usage {
    pub fn new(input_tokens: u32, output_tokens: u32) -> Self {
        Self {
            input_tokens,
            output_tokens,
        }
    }

    pub fn total_tokens(&self) -> u32 {
        self.input_tokens + self.output_tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usage_new() {
        let usage = Usage::new(100, 50);
        assert_eq!(usage.input_tokens, 100);
        assert_eq!(usage.output_tokens, 50);
    }

    #[test]
    fn test_usage_default() {
        let usage = Usage::default();
        assert_eq!(usage.input_tokens, 0);
        assert_eq!(usage.output_tokens, 0);
    }

    #[test]
    fn test_total_tokens() {
        let usage = Usage::new(100, 50);
        assert_eq!(usage.total_tokens(), 150);
    }

    #[test]
    fn test_total_tokens_zero() {
        let usage = Usage::new(0, 0);
        assert_eq!(usage.total_tokens(), 0);
    }

    #[test]
    fn test_usage_serialization() {
        let usage = Usage::new(100, 50);
        let json = serde_json::to_string(&usage).unwrap();
        assert!(json.contains("100"));
        assert!(json.contains("50"));
    }

    #[test]
    fn test_usage_deserialization() {
        let json = r#"{"input_tokens": 200, "output_tokens": 100}"#;
        let usage: Usage = serde_json::from_str(json).unwrap();
        assert_eq!(usage.input_tokens, 200);
        assert_eq!(usage.output_tokens, 100);
    }
}
