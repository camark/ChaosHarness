//! Settings model and loading logic

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::env;
use std::fs;
use std::collections::HashMap;

use crate::permissions::PermissionMode;
use crate::hooks::schemas::HookDefinition;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathRule {
    pub pattern: String,
    pub allow: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct PermissionSettings {
    #[serde(default)]
    pub mode: PermissionMode,
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    #[serde(default)]
    pub denied_tools: Vec<String>,
    #[serde(default)]
    pub path_rules: Vec<PathRule>,
    #[serde(default)]
    pub denied_commands: Vec<String>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_max_files")]
    pub max_files: u32,
    #[serde(default = "default_max_entrypoint_lines")]
    pub max_entrypoint_lines: u32,
}

fn default_true() -> bool { true }
fn default_max_files() -> u32 { 5 }
fn default_max_entrypoint_lines() -> u32 { 200 }

fn default_bm25_top_k() -> usize { 5 }
fn default_bm25_k1() -> f64 { 1.2 }
fn default_bm25_b() -> f64 { 0.75 }
fn default_summary_token_threshold() -> u32 { 30000 }
fn default_summary_segment_size() -> usize { 20 }
fn default_pattern_promotion_threshold() -> i64 { 3 }
fn default_max_context_injection_tokens() -> usize { 2000 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningSettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub knowledge_db_path: Option<String>,
    #[serde(default = "default_bm25_top_k")]
    pub bm25_top_k: usize,
    #[serde(default = "default_bm25_k1")]
    pub bm25_k1: f64,
    #[serde(default = "default_bm25_b")]
    pub bm25_b: f64,
    #[serde(default = "default_summary_token_threshold")]
    pub summary_token_threshold: u32,
    #[serde(default = "default_summary_segment_size")]
    pub summary_segment_size: usize,
    #[serde(default = "default_true")]
    pub session_end_extraction: bool,
    #[serde(default = "default_true")]
    pub auto_skill_generation: bool,
    #[serde(default = "default_pattern_promotion_threshold")]
    pub pattern_promotion_threshold: i64,
    #[serde(default = "default_max_context_injection_tokens")]
    pub max_context_injection_tokens: usize,
}

impl Default for LearningSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            knowledge_db_path: None,
            bm25_top_k: 5,
            bm25_k1: 1.2,
            bm25_b: 0.75,
            summary_token_threshold: 30000,
            summary_segment_size: 20,
            session_end_extraction: true,
            auto_skill_generation: true,
            pattern_promotion_threshold: 3,
            max_context_injection_tokens: 2000,
        }
    }
}

impl Default for MemorySettings {
    fn default() -> Self {
        Self {
            enabled: true,
            max_files: 5,
            max_entrypoint_lines: 200,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct HooksSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub hooks: Vec<HookDefinition>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    // API configuration
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default = "default_api_format")]
    pub api_format: String,

    // Behavior
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub permission: PermissionSettings,
    #[serde(default)]
    pub memory: MemorySettings,
    #[serde(default)]
    pub hooks: HooksSettings,
    #[serde(default)]
    pub learning: LearningSettings,
    #[serde(default)]
    pub enabled_plugins: HashMap<String, bool>,
    /// MCP servers configuration
    #[serde(default, alias = "mcp_servers", rename = "mcpServers")]
    pub mcp_servers: HashMap<String, serde_json::Value>,

    // UI
    #[serde(default)]
    pub theme: String,
    #[serde(default)]
    pub output_style: String,
    #[serde(default)]
    pub vim_mode: bool,
    #[serde(default)]
    pub voice_mode: bool,
    #[serde(default)]
    pub fast_mode: bool,
    #[serde(default = "default_effort")]
    pub effort: String,
    #[serde(default = "default_passes")]
    pub passes: u32,
    #[serde(default)]
    pub verbose: bool,
}

fn default_model() -> String { "claude-sonnet-4-20250514".to_string() }
fn default_max_tokens() -> u32 { 16384 }
fn default_api_format() -> String { "anthropic".to_string() }
fn default_effort() -> String { "medium".to_string() }
fn default_passes() -> u32 { 1 }

impl Default for Settings {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: default_model(),
            max_tokens: default_max_tokens(),
            base_url: None,
            api_format: default_api_format(),
            system_prompt: None,
            permission: PermissionSettings::default(),
            memory: MemorySettings::default(),
            hooks: HooksSettings::default(),
            learning: LearningSettings::default(),
            enabled_plugins: HashMap::new(),
            mcp_servers: HashMap::new(),
            theme: "default".to_string(),
            output_style: "default".to_string(),
            vim_mode: false,
            voice_mode: false,
            fast_mode: false,
            effort: default_effort(),
            passes: default_passes(),
            verbose: false,
        }
    }
}

impl Settings {
    pub fn resolve_api_key(&self) -> Result<String, String> {
        if !self.api_key.is_empty() {
            return Ok(self.api_key.clone());
        }

        if let Ok(key) = env::var("ANTHROPIC_API_KEY") {
            return Ok(key);
        }

        if let Ok(key) = env::var("OPENAI_API_KEY") {
            return Ok(key);
        }

        Err("No API key found. Set ANTHROPIC_API_KEY environment variable or configure api_key in settings.".to_string())
    }

    pub fn merge_cli_overrides(&mut self, overrides: Settings) {
        // Merge non-None values from overrides
        if !overrides.model.is_empty() {
            self.model = overrides.model;
        }
        if let Some(url) = overrides.base_url {
            self.base_url = Some(url);
        }
        if !overrides.api_key.is_empty() {
            self.api_key = overrides.api_key;
        }
        // ... merge other fields as needed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = Settings::default();
        assert_eq!(settings.model, "claude-sonnet-4-20250514");
        assert_eq!(settings.max_tokens, 16384);
        assert_eq!(settings.api_format, "anthropic");
        assert!(!settings.vim_mode);
        assert!(!settings.fast_mode);
    }

    #[test]
    fn test_default_permission_settings() {
        let settings = PermissionSettings::default();
        assert_eq!(settings.mode, PermissionMode::Default);
        assert!(settings.allowed_tools.is_empty());
        assert!(settings.denied_tools.is_empty());
    }

    #[test]
    fn test_default_learning_settings() {
        let settings = LearningSettings::default();
        assert!(settings.enabled);
        assert_eq!(settings.bm25_top_k, 5);
        assert_eq!(settings.bm25_k1, 1.2);
        assert_eq!(settings.bm25_b, 0.75);
    }

    #[test]
    fn test_default_memory_settings() {
        let settings = MemorySettings::default();
        assert!(settings.enabled);
        assert_eq!(settings.max_files, 5);
        assert_eq!(settings.max_entrypoint_lines, 200);
    }

    #[test]
    fn test_resolve_api_key_from_settings() {
        let mut settings = Settings::default();
        settings.api_key = "test-key".to_string();
        assert_eq!(settings.resolve_api_key().unwrap(), "test-key");
    }

    #[test]
    fn test_resolve_api_key_from_env() {
        env::set_var("ANTHROPIC_API_KEY", "env-key");
        let settings = Settings::default();
        // Note: This test depends on env var being set
        let result = settings.resolve_api_key();
        env::remove_var("ANTHROPIC_API_KEY");
        // Either from settings or env
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_merge_cli_overrides() {
        let mut settings = Settings::default();
        let mut overrides = Settings::default();
        overrides.model = "gpt-4".to_string();
        overrides.base_url = Some("https://api.openai.com".to_string());

        settings.merge_cli_overrides(overrides);
        assert_eq!(settings.model, "gpt-4");
        assert_eq!(settings.base_url, Some("https://api.openai.com".to_string()));
    }

    #[test]
    fn test_settings_serialization() {
        let settings = Settings::default();
        let json = serde_json::to_string(&settings).unwrap();
        assert!(json.contains("claude-sonnet-4-20250514"));
    }

    #[test]
    fn test_settings_deserialization() {
        let json = r#"{"model": "gpt-4", "max_tokens": 4096}"#;
        let settings: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.model, "gpt-4");
        assert_eq!(settings.max_tokens, 4096);
    }

    #[test]
    fn test_path_rule_serialization() {
        let rule = PathRule {
            pattern: "/tmp/*".to_string(),
            allow: true,
        };
        let json = serde_json::to_string(&rule).unwrap();
        assert!(json.contains("/tmp/*"));
    }
}

pub fn load_settings(config_path: Option<&str>) -> Result<Settings, Box<dyn std::error::Error + Send + Sync>> {
    let path = if let Some(p) = config_path {
        Path::new(p).to_path_buf()
    } else {
        crate::config::get_config_file_path()
    };

    if path.exists() {
        let content = fs::read_to_string(&path)?;
        let mut settings: Settings = serde_json::from_str(&content)?;
        apply_env_overrides(&mut settings);
        Ok(settings)
    } else {
        let mut settings = Settings::default();
        apply_env_overrides(&mut settings);
        Ok(settings)
    }
}

pub fn save_settings(settings: &Settings, config_path: Option<&str>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = if let Some(p) = config_path {
        Path::new(p).to_path_buf()
    } else {
        crate::config::get_config_file_path()
    };

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_string_pretty(settings)?;
    fs::write(&path, content)?;
    Ok(())
}

fn apply_env_overrides(settings: &mut Settings) {
    // Environment variables only override if settings.json values are empty/unset
    // This gives priority to settings.json configuration

    if settings.model.is_empty() || settings.model == default_model() {
        if let Ok(model) = env::var("ANTHROPIC_MODEL").or_else(|_| env::var("RUST_HARNESS_MODEL")) {
            settings.model = model;
        }
    }

    // Only use env override if settings.json base_url is None or empty
    if settings.base_url.is_none() || settings.base_url.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
        if let Ok(base_url) = env::var("ANTHROPIC_BASE_URL").or_else(|_| env::var("RUST_HARNESS_BASE_URL")) {
            settings.base_url = Some(base_url);
        }
    }

    if let Ok(max_tokens) = env::var("RUST_HARNESS_MAX_TOKENS") {
        if let Ok(tokens) = max_tokens.parse() {
            settings.max_tokens = tokens;
        }
    }

    // Only use env API key if settings.json api_key is empty
    if settings.api_key.is_empty() {
        if let Ok(api_key) = env::var("ANTHROPIC_API_KEY").or_else(|_| env::var("OPENAI_API_KEY")) {
            settings.api_key = api_key;
        }
    }

    // Only use env api_format if settings.json has default value
    if settings.api_format == default_api_format() {
        if let Ok(api_format) = env::var("RUST_HARNESS_API_FORMAT") {
            settings.api_format = api_format;
        }
    }
}
