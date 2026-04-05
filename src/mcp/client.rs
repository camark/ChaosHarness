//! MCP client

use crate::mcp::types::McpServerConfig;

pub struct McpClient {
    #[allow(dead_code)]
    config: McpServerConfig,
}

impl McpClient {
    pub fn new(config: McpServerConfig) -> Self {
        Self { config }
    }
}
