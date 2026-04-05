//! Plugin installer

use std::path::Path;

pub fn install_plugin_from_path(source: &str) -> Result<String, String> {
    let source_path = Path::new(source);

    if !source_path.exists() {
        return Err(format!("Plugin source not found: {}", source));
    }

    // In a full implementation, this would copy the plugin to the plugins directory
    // and register it in the configuration

    Ok(source.to_string())
}

pub fn uninstall_plugin(_name: &str) -> Result<(), String> {
    // In a full implementation, this would remove the plugin
    Ok(())
}
