//! File write tool - Create or overwrite files

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use anyhow::Result;
use serde_json::{json, Value};
use std::path::PathBuf;

/// Input schema for file write tool
pub fn file_write_input_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "path": {
                "type": "string",
                "description": "Path of the file to write"
            },
            "content": {
                "type": "string",
                "description": "Full file contents"
            },
            "create_directories": {
                "type": "boolean",
                "description": "Create parent directories if they don't exist",
                "default": true
            }
        },
        "required": ["path", "content"]
    })
}

/// File write tool
pub struct FileWriteTool;

#[async_trait::async_trait]
impl Tool for FileWriteTool {
    fn name(&self) -> &'static str {
        "write_file"
    }

    fn description(&self) -> &'static str {
        "Create or overwrite a text file in the local repository."
    }

    fn input_schema(&self) -> Value {
        file_write_input_schema()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        false
    }

    async fn execute(&self, input: Value, context: ToolExecutionContext) -> Result<ToolResult> {
        let path_str = input["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' field"))?;

        let content = input["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'content' field"))?;

        let create_directories = input["create_directories"].as_bool().unwrap_or(true);

        let path = resolve_path(&context.cwd, path_str);

        if create_directories {
            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }

        tokio::fs::write(&path, content.as_bytes()).await?;

        Ok(ToolResult::success(format!("Wrote {}", path.display())))
    }
}

fn resolve_path(base: &PathBuf, candidate: &str) -> PathBuf {
    let path = PathBuf::from(candidate);

    // Expand ~ to home directory
    if candidate.starts_with("~/") || candidate == "~" {
        if let Some(home_dir) = dirs::home_dir() {
            let remainder = if candidate == "~" { "" } else { &candidate[2..] };

            // Special handling for Desktop - use OS-specific desktop directory
            if remainder == "Desktop" {
                if let Some(desktop_dir) = dirs::desktop_dir() {
                    return desktop_dir;
                }
                return home_dir.join("Desktop");
            }

            return home_dir.join(remainder);
        }
    }

    // Use dirs::desktop_dir() for automatic OS-specific Desktop detection
    if candidate == "Desktop" || candidate.ends_with("/Desktop") {
        if let Some(desktop_dir) = dirs::desktop_dir() {
            return desktop_dir;
        }
    }

    if path.is_absolute() {
        path
    } else {
        base.join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_write_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");

        let tool = FileWriteTool;
        let input = json!({
            "path": file_path.to_str().unwrap(),
            "content": "hello world"
        });
        let context = ToolExecutionContext::new(dir.path().to_path_buf());
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("Wrote"));

        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "hello world");
    }

    #[tokio::test]
    async fn test_write_file_creates_directories() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("subdir").join("nested").join("test.txt");

        let tool = FileWriteTool;
        let input = json!({
            "path": file_path.to_str().unwrap(),
            "content": "nested content"
        });
        let context = ToolExecutionContext::new(dir.path().to_path_buf());
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(file_path.exists());
    }
}
