//! Dynamic model selection based on task complexity
//!
//! Selects the appropriate model based on:
//! - Task complexity (simple chat vs complex coding)
//! - Cost optimization (use cheaper models for simple tasks)
//! - Error recovery (fallback chain)

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// Task complexity levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskComplexity {
    /// Simple chat, greetings, basic questions
    Simple,
    /// Standard coding tasks, explanations
    Standard,
    /// Complex reasoning, architecture, debugging
    Complex,
    /// Critical tasks requiring highest capability
    Critical,
}

impl TaskComplexity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Simple => "simple",
            Self::Standard => "standard",
            Self::Complex => "complex",
            Self::Critical => "critical",
        }
    }
}

/// Model capability profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProfile {
    pub model_id: String,
    pub name: String,
    pub max_complexity: TaskComplexity,
    pub cost_tier: CostTier,
    pub capabilities: Vec<ModelCapability>,
    pub speed_tier: SpeedTier,
}

/// Cost tiers for models
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum CostTier {
    /// Free or very cheap models
    Free,
    /// Low cost models
    Low,
    /// Medium cost models
    Medium,
    /// High cost models
    High,
    /// Premium models
    Premium,
}

/// Speed tiers for models
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum SpeedTier {
    /// Very fast response
    Fast,
    /// Normal speed
    Normal,
    /// Slower but more capable
    Slow,
}

/// Model capabilities
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModelCapability {
    /// Code generation and understanding
    Coding,
    /// Complex reasoning
    Reasoning,
    /// Creative writing
    Creative,
    /// Fast responses
    Speed,
    /// Long context window
    LongContext,
    /// Tool use support
    ToolUse,
    /// Vision support
    Vision,
}

/// Model selector for dynamic model switching
pub struct ModelSelector {
    /// Available model profiles
    profiles: Vec<ModelProfile>,
    /// Default model to use
    default_model: String,
    /// Fallback chain
    fallback_chain: Vec<String>,
    /// Cost optimization enabled
    cost_optimization: bool,
}

impl ModelSelector {
    /// Create a new model selector with default profiles
    pub fn new(default_model: &str) -> Self {
        let profiles = Self::default_profiles();
        let fallback_chain = vec![
            default_model.to_string(),
            "claude-sonnet-4-20250514".to_string(),
            "claude-haiku-4-5-20251001".to_string(),
        ];

        Self {
            profiles,
            default_model: default_model.to_string(),
            fallback_chain,
            cost_optimization: false,
        }
    }

    /// Create with cost optimization enabled
    pub fn with_cost_optimization(default_model: &str) -> Self {
        let mut selector = Self::new(default_model);
        selector.cost_optimization = true;
        selector
    }

    /// Default model profiles for Anthropic models
    fn default_profiles() -> Vec<ModelProfile> {
        vec![
            ModelProfile {
                model_id: "claude-opus-4-20250514".to_string(),
                name: "Claude Opus 4".to_string(),
                max_complexity: TaskComplexity::Critical,
                cost_tier: CostTier::Premium,
                capabilities: vec![
                    ModelCapability::Coding,
                    ModelCapability::Reasoning,
                    ModelCapability::Creative,
                    ModelCapability::LongContext,
                    ModelCapability::ToolUse,
                    ModelCapability::Vision,
                ],
                speed_tier: SpeedTier::Slow,
            },
            ModelProfile {
                model_id: "claude-sonnet-4-20250514".to_string(),
                name: "Claude Sonnet 4".to_string(),
                max_complexity: TaskComplexity::Complex,
                cost_tier: CostTier::Medium,
                capabilities: vec![
                    ModelCapability::Coding,
                    ModelCapability::Reasoning,
                    ModelCapability::LongContext,
                    ModelCapability::ToolUse,
                    ModelCapability::Vision,
                ],
                speed_tier: SpeedTier::Normal,
            },
            ModelProfile {
                model_id: "claude-haiku-4-5-20251001".to_string(),
                name: "Claude Haiku 4.5".to_string(),
                max_complexity: TaskComplexity::Standard,
                cost_tier: CostTier::Low,
                capabilities: vec![
                    ModelCapability::Coding,
                    ModelCapability::Speed,
                    ModelCapability::ToolUse,
                ],
                speed_tier: SpeedTier::Fast,
            },
        ]
    }

    /// Select model based on task complexity
    pub fn select_model(&self, complexity: TaskComplexity) -> &str {
        if !self.cost_optimization {
            return &self.default_model;
        }

        // Find the cheapest model that can handle this complexity
        let mut candidates: Vec<&ModelProfile> = self.profiles.iter()
            .filter(|p| p.max_complexity >= complexity)
            .collect();

        // Sort by cost (cheapest first)
        candidates.sort_by(|a, b| a.cost_tier.cmp(&b.cost_tier));

        candidates.first()
            .map(|p| p.model_id.as_str())
            .unwrap_or(&self.default_model)
    }

    /// Select model based on task description
    pub fn select_for_task(&self, task_description: &str) -> &str {
        let complexity = self.assess_complexity(task_description);
        self.select_model(complexity)
    }

    /// Assess task complexity from description
    pub fn assess_complexity(&self, task_description: &str) -> TaskComplexity {
        let text = task_description.to_lowercase();

        // Simple patterns
        let simple_patterns = [
            "hello", "hi", "hey", "thanks", "thank you",
            "what is", "who is", "when is", "where is",
            "yes", "no", "ok", "okay", "sure",
        ];

        for pattern in &simple_patterns {
            if text.starts_with(pattern) || text == *pattern {
                return TaskComplexity::Simple;
            }
        }

        // Complex patterns
        let complex_patterns = [
            "refactor", "architecture", "design pattern", "system design",
            "debug", "complex", "optimize", "performance",
            "security", "scalable", "distributed", "concurrent",
            "algorithm", "data structure", "machine learning",
        ];

        let complex_count = complex_patterns.iter()
            .filter(|p| text.contains(*p))
            .count();

        if complex_count >= 2 {
            return TaskComplexity::Complex;
        }

        // Critical patterns
        let critical_patterns = [
            "production", "critical", "urgent", "emergency",
            "data loss", "security vulnerability", "exploit",
            "rollback", "disaster recovery",
        ];

        for pattern in &critical_patterns {
            if text.contains(pattern) {
                return TaskComplexity::Critical;
            }
        }

        // Default to standard
        TaskComplexity::Standard
    }

    /// Get fallback model for error recovery
    pub fn get_fallback(&self, current_model: &str) -> Option<&str> {
        let current_pos = self.fallback_chain.iter()
            .position(|m| m == current_model)?;

        self.fallback_chain.get(current_pos + 1)
            .map(|s| s.as_str())
    }

    /// Get model profile by ID
    pub fn get_profile(&self, model_id: &str) -> Option<&ModelProfile> {
        self.profiles.iter().find(|p| p.model_id == model_id)
    }

    /// List all available models
    pub fn list_models(&self) -> Vec<&str> {
        self.profiles.iter()
            .map(|p| p.model_id.as_str())
            .collect()
    }

    /// Check if a model supports a specific capability
    pub fn supports_capability(&self, model_id: &str, capability: &ModelCapability) -> bool {
        self.get_profile(model_id)
            .map(|p| p.capabilities.contains(capability))
            .unwrap_or(false)
    }
}

impl Default for ModelSelector {
    fn default() -> Self {
        Self::new("claude-sonnet-4-20250514")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assess_complexity_simple() {
        let selector = ModelSelector::new("test");
        assert_eq!(selector.assess_complexity("hello"), TaskComplexity::Simple);
        assert_eq!(selector.assess_complexity("what is rust"), TaskComplexity::Simple);
    }

    #[test]
    fn test_assess_complexity_standard() {
        let selector = ModelSelector::new("test");
        assert_eq!(selector.assess_complexity("write a function"), TaskComplexity::Standard);
    }

    #[test]
    fn test_assess_complexity_complex() {
        let selector = ModelSelector::new("test");
        assert_eq!(
            selector.assess_complexity("refactor the architecture for performance"),
            TaskComplexity::Complex
        );
    }

    #[test]
    fn test_assess_complexity_critical() {
        let selector = ModelSelector::new("test");
        assert_eq!(
            selector.assess_complexity("production security vulnerability"),
            TaskComplexity::Critical
        );
    }

    #[test]
    fn test_select_model_cost_optimization() {
        let selector = ModelSelector::with_cost_optimization("claude-opus-4-20250514");

        // Simple tasks should use cheap model
        let model = selector.select_model(TaskComplexity::Simple);
        assert!(model.contains("haiku"));

        // Complex tasks should use capable model
        let model = selector.select_model(TaskComplexity::Complex);
        assert!(model.contains("sonnet") || model.contains("opus"));
    }

    #[test]
    fn test_get_fallback() {
        let selector = ModelSelector::new("claude-opus-4-20250514");
        let fallback = selector.get_fallback("claude-opus-4-20250514");
        assert!(fallback.is_some());
    }

    #[test]
    fn test_list_models() {
        let selector = ModelSelector::new("test");
        let models = selector.list_models();
        assert!(!models.is_empty());
    }
}
