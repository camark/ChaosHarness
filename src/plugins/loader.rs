//! Plugin loader

use crate::config::Settings;
use crate::plugins::types::Plugin;
use std::path::{Path, PathBuf};
use std::fs;

/// Get the plugins directory for the current project
pub fn get_plugins_dir(cwd: &str) -> PathBuf {
    Path::new(cwd).join(".rust_harness").join("plugins")
}

/// Get the user-level plugins directory
pub fn get_user_plugins_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".rust_harness").join("plugins"))
}

pub fn load_plugins(settings: &Settings, cwd: &str) -> Vec<Plugin> {
    let mut plugins = Vec::new();

    // Load project-level plugins
    let project_plugins_dir = get_plugins_dir(cwd);
    if project_plugins_dir.exists() {
        if let Ok(entries) = fs::read_dir(&project_plugins_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Ok(plugin) = load_plugin(&path) {
                        plugins.push(plugin);
                    }
                }
            }
        }
    }

    // Load user-level plugins
    if let Some(user_plugins_dir) = get_user_plugins_dir() {
        if user_plugins_dir.exists() {
            if let Ok(entries) = fs::read_dir(&user_plugins_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        if let Ok(plugin) = load_plugin(&path) {
                            // Avoid duplicates
                            if !plugins.iter().any(|p| p.name == plugin.name) {
                                plugins.push(plugin);
                            }
                        }
                    }
                }
            }
        }
    }

    plugins
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
