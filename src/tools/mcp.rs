//! MCP Tool Wrapper - exposes MCP server tools as native tools

use crate::mcp::client::McpManager;
use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use anyhow::Result;
use serde_json::Value;
use std::sync::Arc;

/// Wrapper for an MCP tool
pub struct McpToolWrapper {
    /// Full tool name: "server_name__tool_name"
    tool_name: String,
    /// MCP server name
    server_name: String,
    /// Actual tool name on the MCP server
    mcp_tool_name: String,
    /// Tool description
    description: String,
    /// Input schema
    input_schema: Value,
    /// Reference to MCP manager
    mcp_manager: Arc<McpManager>,
}

impl McpToolWrapper {
    pub fn new(
        server_name: String,
        mcp_tool_name: String,
        description: String,
        input_schema: Value,
        mcp_manager: Arc<McpManager>,
    ) -> Self {
        let tool_name = format!("{}__{}", server_name, mcp_tool_name);
        Self {
            tool_name,
            server_name,
            mcp_tool_name,
            description,
            input_schema,
            mcp_manager,
        }
    }
}

#[async_trait::async_trait]
impl Tool for McpToolWrapper {
    fn name(&self) -> &'static str {
        Box::leak(self.tool_name.clone().into_boxed_str())
    }

    fn description(&self) -> &'static str {
        Box::leak(format!("[MCP:{}] {}", self.server_name, self.description).into_boxed_str())
    }

    fn input_schema(&self) -> Value {
        self.input_schema.clone()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        // MCP tools are read-only by default unless their name suggests otherwise
        !self.mcp_tool_name.contains("write")
            && !self.mcp_tool_name.contains("create")
            && !self.mcp_tool_name.contains("delete")
            && !self.mcp_tool_name.contains("execute")
    }

    async fn execute(&self, input: Value, _context: ToolExecutionContext) -> Result<ToolResult> {
        tracing::info!("Calling MCP tool: {} on server: {}", self.mcp_tool_name, self.server_name);

        match self.mcp_manager.call_tool(&self.server_name, &self.mcp_tool_name, input).await {
            Ok(result) => {
                // Convert MCP content to string
                let mut output_parts = Vec::new();
                for content in &result.content {
                    match content {
                        crate::mcp::types::ContentBlock::Text { text } => {
                            output_parts.push(text.clone());
                        }
                        crate::mcp::types::ContentBlock::Image { data: _, mime_type } => {
                            output_parts.push(format!("[Image: {} data]", mime_type));
                        }
                        crate::mcp::types::ContentBlock::Resource { resource } => {
                            output_parts.push(format!(
                                "[Resource: {} - {}]",
                                resource.uri,
                                resource.text.as_ref().unwrap_or(&"binary".to_string())
                            ));
                        }
                    }
                }

                let output = output_parts.join("\n");
                let is_error = result.is_error.unwrap_or(false);

                Ok(ToolResult {
                    output,
                    is_error,
                    metadata: std::collections::HashMap::new(),
                })
            }
            Err(e) => Ok(ToolResult {
                output: format!("MCP tool error: {}", e),
                is_error: true,
                metadata: std::collections::HashMap::new(),
            }),
        }
    }
}

/// Helper to register all MCP tools from all connected servers
pub async fn register_mcp_tools(mcp_manager: Arc<McpManager>, registry: &crate::tools::base::ToolRegistry) {
    let all_tools = mcp_manager.list_all_tools().await;

    for (server_name, mcp_tool) in all_tools {
        let wrapper = McpToolWrapper::new(
            server_name.clone(),
            mcp_tool.name.clone(),
            mcp_tool.description.unwrap_or_else(|| format!("MCP tool from {}", server_name)),
            mcp_tool.input_schema,
            mcp_manager.clone(),
        );
        registry.register(wrapper).await;
        tracing::info!("Registered MCP tool: {}__{}", server_name, mcp_tool.name);
    }
}
