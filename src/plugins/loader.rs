//! Plugin loader

use crate::config::Settings;
use crate::plugins::types::Plugin;
use std::path::Path;
use std::fs;

pub fn load_plugins(_settings: &Settings, _cwd: &str) -> Vec<Plugin> {
    // In a full implementation, this would scan plugin directories
    // and load plugin manifests
    Vec::new()
}

pub fn load_plugin(path: &Path) -> Result<Plugin, String> {
    let manifest_path = path.join("plugin.json");

    if !manifest_path.exists() {
        return Err("No plugin.json manifest found".to_string());
    }

    let content = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read manifest: {}", e))?;

    let manifest: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    Ok(Plugin {
        name: manifest["name"].as_str().unwrap_or("unknown").to_string(),
        version: manifest["version"].as_str().unwrap_or("0.0.0").to_string(),
        description: manifest["description"].as_str().map(String::from),
        enabled: manifest["enabled"].as_bool().unwrap_or(true),
        path: path.to_string_lossy().to_string(),
    })
}
