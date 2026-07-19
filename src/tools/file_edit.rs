//! File edit tool - Replace text in existing files

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use crate::tools::path_util::resolve_path;
use anyhow::Result;
use serde_json::{json, Value};

/// Input schema for file edit tool
pub fn file_edit_input_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "path": {
                "type": "string",
                "description": "Path of the file to edit"
            },
            "old_str": {
                "type": "string",
                "description": "Existing text to replace"
            },
            "new_str": {
                "type": "string",
                "description": "Replacement text"
            },
            "replace_all": {
                "type": "boolean",
                "description": "Replace all occurrences instead of just the first",
                "default": false
            }
        },
        "required": ["path", "old_str", "new_str"]
    })
}

/// File edit tool
pub struct FileEditTool;

#[async_trait::async_trait]
impl Tool for FileEditTool {
    fn name(&self) -> &'static str {
        "edit_file"
    }

    fn description(&self) -> &'static str {
        "Edit an existing file by replacing a string with new text."
    }

    fn input_schema(&self) -> Value {
        file_edit_input_schema()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        false
    }

    async fn execute(&self, input: Value, context: ToolExecutionContext) -> Result<ToolResult> {
        let path_str = input["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' field"))?;

        let old_str = input["old_str"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'old_str' field"))?;

        let new_str = input["new_str"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'new_str' field"))?;

        let replace_all = input["replace_all"].as_bool().unwrap_or(false);

        let path = resolve_path(&context.cwd, path_str);

        if !path.exists() {
            return Ok(ToolResult::error(format!("File not found: {}", path.display())));
        }

        // Read original content
        let original = tokio::fs::read_to_string(&path).await?;

        // Find and replace
        let updated = if replace_all {
            if !original.contains(old_str) {
                return Ok(ToolResult::error(
                    "old_str was not found in the file".to_string(),
                ));
            }
            original.replace(old_str, new_str)
        } else {
            match original.find(old_str) {
                Some(_) => original.replacen(old_str, new_str, 1),
                None => {
                    return Ok(ToolResult::error(
                        "old_str was not found in the file".to_string(),
                    ))
                }
            }
        };

        // Write updated content
        tokio::fs::write(&path, updated.as_bytes()).await?;

        Ok(ToolResult::success(format!("Updated {}", path.display())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_edit_file() {
        let mut temp = NamedTempFile::new().unwrap();
        writeln!(temp, "hello world").unwrap();

        let tool = FileEditTool;
        let input = json!({
            "path": temp.path().to_str().unwrap(),
            "old_str": "world",
            "new_str": "rust"
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("Updated"));

        let content = tokio::fs::read_to_string(temp.path()).await.unwrap();
        assert!(content.contains("hello rust"));
    }

    #[tokio::test]
    async fn test_edit_file_not_found() {
        let tool = FileEditTool;
        let input = json!({
            "path": "/nonexistent/file.txt",
            "old_str": "foo",
            "new_str": "bar"
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(result.is_error);
        assert!(result.output.contains("File not found"));
    }

    #[tokio::test]
    async fn test_edit_file_string_not_found() {
        let mut temp = NamedTempFile::new().unwrap();
        writeln!(temp, "hello world").unwrap();

        let tool = FileEditTool;
        let input = json!({
            "path": temp.path().to_str().unwrap(),
            "old_str": "nonexistent",
            "new_str": "bar"
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(result.is_error);
        assert!(result.output.contains("not found"));
    }

    #[tokio::test]
    async fn test_edit_file_replace_all() {
        let mut temp = NamedTempFile::new().unwrap();
        writeln!(temp, "foo foo foo").unwrap();

        let tool = FileEditTool;
        let input = json!({
            "path": temp.path().to_str().unwrap(),
            "old_str": "foo",
            "new_str": "bar",
            "replace_all": true
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);

        let content = tokio::fs::read_to_string(temp.path()).await.unwrap();
        assert_eq!(content.trim(), "bar bar bar");
    }
}
