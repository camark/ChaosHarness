//! Default settings generator

use std::fs;
use std::path::{Path, PathBuf};

/// Generate a default settings.json file
pub fn generate_default_settings(path: &Path) -> Result<(), String> {
    if path.exists() {
        return Err(format!(
            "Settings file already exists at: {}",
            path.display()
        ));
    }

    // Create parent directories if needed
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory: {}", e))?;
    }

    let content = r##"{
  "api_key": "",
  "model": "claude-sonnet-4-20250514",
  "max_tokens": 16384,
  "base_url": null,
  "api_format": "anthropic",

  "system_prompt": null,

  "permission": {
    "mode": "default",
    "allowed_tools": [],
    "denied_tools": [],
    "path_rules": [],
    "denied_commands": []
  },

  "memory": {
    "enabled": true,
    "max_files": 5,
    "max_entrypoint_lines": 200
  },

  "hooks": {
    "enabled": false,
    "hooks": []
  },

  "enabled_plugins": {},
  "mcp_servers": {},

  "theme": "default",
  "output_style": "default",
  "vim_mode": false,
  "voice_mode": false,
  "fast_mode": false,
  "effort": "medium",
  "passes": 1,
  "verbose": false
}
"##;

    fs::write(path, content)
        .map_err(|e| format!("Failed to write settings file: {}", e))?;

    Ok(())
}

/// Initialize all default directories and files
pub fn initialize_defaults() -> Result<String, String> {
    use crate::config::paths::{
        get_config_dir, get_config_file_path, get_data_dir, get_logs_dir,
        get_project_config_dir,
    };

    let mut created = Vec::new();

    // Global config directory
    let config_dir = get_config_dir();
    created.push(format!("Config directory: {}", config_dir.display()));

    // Settings file (if not exists)
    let settings_path = get_config_file_path();
    if !settings_path.exists() {
        generate_default_settings(&settings_path)?;
        created.push(format!("Settings file: {}", settings_path.display()));
    }

    // Data directory
    let data_dir = get_data_dir();
    created.push(format!("Data directory: {}", data_dir.display()));

    // Logs directory
    let logs_dir = get_logs_dir();
    created.push(format!("Logs directory: {}", logs_dir.display()));

    Ok(format!("Initialized:\n{}", created.join("\n")))
}

/// Create project-specific .rust_harness directory structure
pub fn initialize_project(cwd: &str) -> Result<String, String> {
    use std::path::PathBuf;

    let project_dir = PathBuf::from(cwd).join(".rust_harness");
    let mut created = Vec::new();

    // Create project directory
    fs::create_dir_all(&project_dir)
        .map_err(|e| format!("Failed to create project directory: {}", e))?;
    created.push(format!("Project directory: {}", project_dir.display()));

    // Create memory directory
    let memory_dir = project_dir.join("memory");
    fs::create_dir_all(&memory_dir)
        .map_err(|e| format!("Failed to create memory directory: {}", e))?;
    created.push(format!("Memory directory: {}", memory_dir.display()));

    // Create MEMORY.md if not exists
    let memory_md = memory_dir.join("MEMORY.md");
    if !memory_md.exists() {
        fs::write(&memory_md, "# Project Memory\n\nAdd reusable project knowledge here.\n")
            .map_err(|e| format!("Failed to create MEMORY.md: {}", e))?;
        created.push(format!("MEMORY.md: {}", memory_md.display()));
    }

    // Create skills directory
    let skills_dir = project_dir.join("skills");
    fs::create_dir_all(&skills_dir)
        .map_err(|e| format!("Failed to create skills directory: {}", e))?;
    created.push(format!("Skills directory: {}", skills_dir.display()));

    // Create plugins directory
    let plugins_dir = project_dir.join("plugins");
    fs::create_dir_all(&plugins_dir)
        .map_err(|e| format!("Failed to create plugins directory: {}", e))?;
    created.push(format!("Plugins directory: {}", plugins_dir.display()));

    // Create sessions directory
    let sessions_dir = project_dir.join("sessions");
    fs::create_dir_all(&sessions_dir)
        .map_err(|e| format!("Failed to create sessions directory: {}", e))?;
    created.push(format!("Sessions directory: {}", sessions_dir.display()));

    Ok(format!("Initialized project:\n{}", created.join("\n")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;

    #[test]
    fn test_generate_default_settings() {
        let temp_dir = TempDir::new().unwrap();
        let settings_path = temp_dir.path().join("settings.json");

        let result = generate_default_settings(&settings_path);
        assert!(result.is_ok());
        assert!(settings_path.exists());

        // Verify content is valid JSON
        let content = fs::read_to_string(&settings_path).unwrap();
        let _: serde_json::Value = serde_json::from_str(&content).unwrap();
    }

    #[test]
    fn test_generate_default_settings_already_exists() {
        let temp_dir = TempDir::new().unwrap();
        let settings_path = temp_dir.path().join("settings.json");

        // Create the file first
        generate_default_settings(&settings_path).unwrap();

        // Try to create again
        let result = generate_default_settings(&settings_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already exists"));
    }

    #[test]
    fn test_initialize_project() {
        let temp_dir = TempDir::new().unwrap();
        let cwd = temp_dir.path().to_str().unwrap();

        let result = initialize_project(cwd);
        assert!(result.is_ok());

        let project_dir = PathBuf::from(cwd).join(".rust_harness");
        assert!(project_dir.exists());
        assert!(project_dir.join("memory").exists());
        assert!(project_dir.join("memory").join("MEMORY.md").exists());
        assert!(project_dir.join("skills").exists());
        assert!(project_dir.join("plugins").exists());
        assert!(project_dir.join("sessions").exists());
    }
}
