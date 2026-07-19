//! Tool abstractions and registry

#![allow(dead_code)]

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Shared execution context for tool invocations
#[derive(Clone)]
pub struct ToolExecutionContext {
    /// Current working directory
    pub cwd: PathBuf,
    /// Additional metadata passed to tools
    pub metadata: HashMap<String, Arc<dyn Send + Sync>>,
}

impl ToolExecutionContext {
    pub fn new(cwd: PathBuf) -> Self {
        Self {
            cwd,
            metadata: HashMap::new(),
        }
    }
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub output: String,
    #[serde(default)]
    pub is_error: bool,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, Value>,
}

impl ToolResult {
    pub fn success(output: impl Into<String>) -> Self {
        Self {
            output: output.into(),
            is_error: false,
            metadata: HashMap::new(),
        }
    }

    pub fn error(output: impl Into<String>) -> Self {
        Self {
            output: output.into(),
            is_error: true,
            metadata: HashMap::new(),
        }
    }
}

/// Trait for all tools
#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    /// Tool name (must be unique)
    fn name(&self) -> &'static str;

    /// Tool description
    fn description(&self) -> &'static str;

    /// Input schema as JSON Schema
    fn input_schema(&self) -> Value;

    /// Execute the tool with the given input
    async fn execute(&self, input: Value, context: ToolExecutionContext) -> Result<ToolResult>;

    /// Whether the tool is read-only (for permission checking)
    fn is_read_only(&self, _input: &Value) -> bool {
        false
    }

    /// Return the tool schema for the Anthropic Messages API
    fn to_api_schema(&self) -> Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": self.name(),
                "description": self.description(),
                "parameters": self.input_schema()
            }
        })
    }
}

/// Registry for all available tools
#[derive(Clone)]
pub struct ToolRegistry {
    tools: Arc<Mutex<HashMap<String, Arc<dyn Tool>>>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a tool
    pub async fn register<T: Tool + 'static>(&self, tool: T) {
        let mut tools = self.tools.lock().await;
        tools.insert(tool.name().to_string(), Arc::new(tool));
    }

    /// Get a tool by name
    pub async fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        let tools = self.tools.lock().await;
        tools.get(name).cloned()
    }

    /// List all registered tools
    pub async fn list_tools(&self) -> Vec<Arc<dyn Tool>> {
        let tools = self.tools.lock().await;
        tools.values().cloned().collect()
    }

    /// Get all tool schemas in API format
    pub async fn to_api_schema(&self) -> Vec<Value> {
        let tools = self.tools.lock().await;
        tools.values().map(|t| t.to_api_schema()).collect()
    }

    /// Check if a tool exists
    pub async fn contains(&self, name: &str) -> bool {
        let tools = self.tools.lock().await;
        tools.contains_key(name)
    }

    /// Get the number of registered tools
    pub async fn len(&self) -> usize {
        let tools = self.tools.lock().await;
        tools.len()
    }

    /// Check if the registry is empty
    pub async fn is_empty(&self) -> bool {
        let tools = self.tools.lock().await;
        tools.is_empty()
    }

    /// List all tool names synchronously (blocking)
    pub fn list_names_sync(&self) -> Vec<String> {
        // Use try_lock to avoid blocking
        if let Ok(tools) = self.tools.try_lock() {
            tools.keys().cloned().collect()
        } else {
            Vec::new()
        }
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::tools::init_tools;

    #[tokio::test]
    async fn test_tool_schema_output() {
        let registry = init_tools().await;
        let schemas = registry.to_api_schema().await;

        for schema in schemas {
            println!("Tool schema: {:#?}", schema);
            println!();
        }
    }
}
