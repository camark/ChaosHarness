//! MCP configuration loading

use crate::config::Settings;
use crate::mcp::types::McpServerConfig;
use std::collections::HashMap;

pub fn load_mcp_server_configs(
    settings: &Settings,
) -> HashMap<String, McpServerConfig> {
    settings
        .mcp_servers
        .iter()
        .filter_map(|(name, value)| {
            serde_json::from_value::<McpServerConfig>(value.clone())
                .ok()
                .map(|config| (name.clone(), config))
        })
        .collect()
}
