//! TodoWrite tool - Maintain a project TODO file

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use anyhow::Result;
use serde_json::Value;
use std::path::Path;

/// Input schema for todo_write tool
pub fn todo_write_input_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "item": {
                "type": "string",
                "description": "TODO item text"
            },
            "checked": {
                "type": "boolean",
                "description": "Whether the item is completed",
                "default": false
            },
            "path": {
                "type": "string",
                "description": "Path to the TODO file",
                "default": "TODO.md"
            }
        },
        "required": ["item"]
    })
}

/// TodoWrite tool
pub struct TodoWriteTool;

#[async_trait::async_trait]
impl Tool for TodoWriteTool {
    fn name(&self) -> &'static str {
        "todo_write"
    }

    fn description(&self) -> &'static str {
        "Append a TODO item to a markdown checklist file."
    }

    fn input_schema(&self) -> Value {
        todo_write_input_schema()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        false
    }

    async fn execute(&self, input: Value, context: ToolExecutionContext) -> Result<ToolResult> {
        let item = input["item"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'item' field"))?;

        let checked = input["checked"].as_bool().unwrap_or(false);
        let path_str = input["path"].as_str().unwrap_or("TODO.md");

        let path = Path::new(&context.cwd).join(path_str);

        let prefix = if checked { "- [x]" } else { "- [ ]" };

        let existing = if path.exists() {
            tokio::fs::read_to_string(&path).await.unwrap_or_else(|_| "# TODO\n".to_string())
        } else {
            "# TODO\n".to_string()
        };

        let updated = format!("{}\n{} {}\n", existing.trim_end(), prefix, item);

        tokio::fs::write(&path, updated.as_bytes()).await?;

        Ok(ToolResult::success(format!("Updated {}", path.display())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_todo_write() {
        let temp_dir = TempDir::new().unwrap();
        let todo_path = temp_dir.path().join("TODO.md");

        let tool = TodoWriteTool;
        let input = serde_json::json!({
            "item": "Test the feature",
            "checked": false,
            "path": todo_path.to_string_lossy()
        });
        let context = ToolExecutionContext::new(temp_dir.path().to_path_buf());
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(todo_path.exists());

        let content = tokio::fs::read_to_string(&todo_path).await.unwrap();
        assert!(content.contains("- [ ] Test the feature"));
    }
}
