//! Lsp tool - Code intelligence operations

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use anyhow::Result;
use serde_json::Value;
use std::path::PathBuf;

/// Input schema for lsp tool
pub fn lsp_input_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "operation": {
                "type": "string",
                "description": "The code intelligence operation to perform",
                "enum": [
                    "document_symbol",
                    "workspace_symbol",
                    "go_to_definition",
                    "find_references",
                    "hover"
                ]
            },
            "file_path": {
                "type": "string",
                "description": "Path to the source file for file-based operations"
            },
            "symbol": {
                "type": "string",
                "description": "Explicit symbol name to look up"
            },
            "line": {
                "type": "integer",
                "description": "1-based line number for position-based lookups",
                "minimum": 1
            },
            "character": {
                "type": "integer",
                "description": "1-based character offset for position-based lookups",
                "minimum": 1
            },
            "query": {
                "type": "string",
                "description": "Substring query for workspace_symbol"
            }
        },
        "required": ["operation"]
    })
}

/// Lsp tool
pub struct LspTool;

#[async_trait::async_trait]
impl Tool for LspTool {
    fn name(&self) -> &'static str {
        "lsp"
    }

    fn description(&self) -> &'static str {
        "Inspect code symbols, definitions, references, and hover information across the workspace."
    }

    fn input_schema(&self) -> Value {
        lsp_input_schema()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        true
    }

    async fn execute(&self, input: Value, _context: ToolExecutionContext) -> Result<ToolResult> {
        let operation = input["operation"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'operation' field"))?;

        let file_path = input["file_path"].as_str();
        let _symbol = input["symbol"].as_str();
        let _line = input["line"].as_i64();
        let _character = input["character"].as_i64();
        let query = input["query"].as_str();

        // Validate operation-specific requirements
        match operation {
            "workspace_symbol" => {
                if query.is_none() {
                    return Ok(ToolResult::error(
                        "workspace_symbol requires query".to_string()
                    ));
                }
                // In a full implementation, this would search workspace symbols
                Ok(ToolResult::success(format!(
                    "Workspace symbol search for '{}' (not implemented)",
                    query.unwrap()
                )))
            }
            "document_symbol" | "go_to_definition" | "find_references" | "hover" => {
                if file_path.is_none() {
                    return Ok(ToolResult::error(format!(
                        "{} requires file_path",
                        operation
                    )));
                }

                let path = PathBuf::from(file_path.unwrap());
                if !path.exists() {
                    return Ok(ToolResult::error(format!(
                        "File not found: {}",
                        path.display()
                    )));
                }

                // In a full implementation, this would use LSP
                Ok(ToolResult::success(format!(
                    "LSP operation '{}' on file '{}' (not implemented)",
                    operation,
                    path.display()
                )))
            }
            _ => Ok(ToolResult::error(format!(
                "Unknown LSP operation: {}",
                operation
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_lsp_workspace_symbol_missing_query() {
        let tool = LspTool;
        let input = serde_json::json!({
            "operation": "workspace_symbol"
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(result.is_error);
        assert!(result.output.contains("requires query"));
    }

    #[tokio::test]
    async fn test_lsp_file_operation_missing_path() {
        let tool = LspTool;
        let input = serde_json::json!({
            "operation": "document_symbol"
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(result.is_error);
        assert!(result.output.contains("requires file_path"));
    }
}
