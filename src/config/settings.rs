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

impl Default for PermissionSettings {
    fn default() -> Self {
        Self {
            mode: PermissionMode::default(),
            allowed_tools: Vec::new(),
            denied_tools: Vec::new(),
            path_rules: Vec::new(),
            denied_commands: Vec::new(),
        }
    }
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
pub struct HooksSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub hooks: Vec<HookDefinition>,
}

impl Default for HooksSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            hooks: Vec::new(),
        }
    }
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
    pub enabled_plugins: HashMap<String, bool>,
    #[serde(default)]
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
    if let Ok(model) = env::var("ANTHROPIC_MODEL").or_else(|_| env::var("RUST_HARNESS_MODEL")) {
        settings.model = model;
    }

    if let Ok(base_url) = env::var("ANTHROPIC_BASE_URL").or_else(|_| env::var("RUST_HARNESS_BASE_URL")) {
        settings.base_url = Some(base_url);
    }

    if let Ok(max_tokens) = env::var("RUST_HARNESS_MAX_TOKENS") {
        if let Ok(tokens) = max_tokens.parse() {
            settings.max_tokens = tokens;
        }
    }

    if let Ok(api_key) = env::var("ANTHROPIC_API_KEY").or_else(|_| env::var("OPENAI_API_KEY")) {
        settings.api_key = api_key;
    }

    if let Ok(api_format) = env::var("RUST_HARNESS_API_FORMAT") {
        settings.api_format = api_format;
    }
}
