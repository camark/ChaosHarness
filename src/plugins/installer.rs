//! Plugin installer

use std::path::{Path, PathBuf};
use std::fs;

/// Get the user plugins directory
fn get_user_plugins_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".rust_harness").join("plugins"))
}

/// Get the project plugins directory
fn get_project_plugins_dir(cwd: &str) -> PathBuf {
    Path::new(cwd).join(".rust_harness").join("plugins")
}

pub fn install_plugin_from_path(source: &str, cwd: &str) -> Result<String, String> {
    let source_path = Path::new(source);

    if !source_path.exists() {
        return Err(format!("Plugin source not found: {}", source));
    }

    // Validate it's a plugin (has plugin.json)
    let manifest_path = source_path.join("plugin.json");
    if !manifest_path.exists() {
        return Err("No plugin.json manifest found".to_string());
    }

    // Read plugin name from manifest
    let content = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read manifest: {}", e))?;
    let manifest: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    let plugin_name = manifest["name"]
        .as_str()
        .ok_or("Plugin name is required")?;

    // Install to project plugins directory
    let target_dir = get_project_plugins_dir(cwd);
    fs::create_dir_all(&target_dir)
        .map_err(|e| format!("Failed to create plugins directory: {}", e))?;

    let target_path = target_dir.join(plugin_name);

    // Copy plugin directory
    if source_path.is_dir() {
        copy_dir_all(source_path, &target_path)?;
    }

    Ok(format!("Plugin '{}' installed to {}", plugin_name, target_path.display()))
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst)
        .map_err(|e| format!("Failed to create directory: {}", e))?;

    for entry in fs::read_dir(src).map_err(|e| format!("Failed to read directory: {}", e))? {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let ty = entry.file_type().map_err(|e| format!("Failed to get file type: {}", e))?;

        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.join(entry.file_name()))
                .map_err(|e| format!("Failed to copy file: {}", e))?;
        }
    }

    Ok(())
}

pub fn uninstall_plugin(name: &str, cwd: &str) -> Result<String, String> {
    // Try project plugins first
    let project_plugin_path = get_project_plugins_dir(cwd).join(name);
    if project_plugin_path.exists() {
        fs::remove_dir_all(&project_plugin_path)
            .map_err(|e| format!("Failed to remove plugin: {}", e))?;
        return Ok(format!("Plugin '{}' uninstalled from project", name));
    }

    // Try user plugins
    if let Some(user_plugins_dir) = get_user_plugins_dir() {
        let user_plugin_path = user_plugins_dir.join(name);
        if user_plugin_path.exists() {
            fs::remove_dir_all(&user_plugin_path)
                .map_err(|e| format!("Failed to remove plugin: {}", e))?;
            return Ok(format!("Plugin '{}' uninstalled from user plugins", name));
        }
    }

    Err(format!("Plugin '{}' not found", name))
}

pub fn enable_plugin(name: &str, cwd: &str) -> Result<String, String> {
    let plugin_path = get_project_plugins_dir(cwd).join(name);
    if !plugin_path.exists() {
        return Err(format!("Plugin '{}' not found", name));
    }

    let manifest_path = plugin_path.join("plugin.json");
    if !manifest_path.exists() {
        return Err("No plugin.json manifest found".to_string());
    }

    // Update manifest to enable plugin
    let content = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read manifest: {}", e))?;
    let mut manifest: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    manifest["enabled"] = serde_json::json!(true);

    let new_content = serde_json::to_string_pretty(&manifest)
        .map_err(|e| format!("Failed to serialize manifest: {}", e))?;

    fs::write(&manifest_path, new_content)
        .map_err(|e| format!("Failed to write manifest: {}", e))?;

    Ok(format!("Plugin '{}' enabled", name))
}

pub fn disable_plugin(name: &str, cwd: &str) -> Result<String, String> {
    let plugin_path = get_project_plugins_dir(cwd).join(name);
    if !plugin_path.exists() {
        return Err(format!("Plugin '{}' not found", name));
    }

    let manifest_path = plugin_path.join("plugin.json");
    if !manifest_path.exists() {
        return Err("No plugin.json manifest found".to_string());
    }

    // Update manifest to disable plugin
    let content = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read manifest: {}", e))?;
    let mut manifest: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    manifest["enabled"] = serde_json::json!(false);

    let new_content = serde_json::to_string_pretty(&manifest)
        .map_err(|e| format!("Failed to serialize manifest: {}", e))?;

    fs::write(&manifest_path, new_content)
        .map_err(|e| format!("Failed to write manifest: {}", e))?;

    Ok(format!("Plugin '{}' disabled", name))
}
