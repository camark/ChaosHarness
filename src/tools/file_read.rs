//! File read tool - Read UTF-8 text files with line numbers

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use anyhow::Result;
use serde_json::{json, Value};
use std::path::PathBuf;

/// Input schema for file read tool
pub fn file_read_input_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "path": {
                "type": "string",
                "description": "Path of the file to read"
            },
            "offset": {
                "type": "integer",
                "description": "Zero-based starting line number",
                "default": 0,
                "minimum": 0
            },
            "limit": {
                "type": "integer",
                "description": "Number of lines to return",
                "default": 200,
                "minimum": 1,
                "maximum": 2000
            }
        },
        "required": ["path"]
    })
}

/// File read tool
pub struct FileReadTool;

#[async_trait::async_trait]
impl Tool for FileReadTool {
    fn name(&self) -> &'static str {
        "read_file"
    }

    fn description(&self) -> &'static str {
        "Read a UTF-8 text file from the local repository with line numbers."
    }

    fn input_schema(&self) -> Value {
        file_read_input_schema()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        true
    }

    async fn execute(&self, input: Value, context: ToolExecutionContext) -> Result<ToolResult> {
        let path_str = input["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' field"))?;

        let offset = input["offset"].as_u64().unwrap_or(0) as usize;
        let limit = input["limit"].as_u64().unwrap_or(200).min(2000) as usize;

        let path = resolve_path(&context.cwd, path_str);

        if !path.exists() {
            return Ok(ToolResult::error(format!("File not found: {}", path.display())));
        }

        if path.is_dir() {
            return Ok(ToolResult::error(format!(
                "Cannot read directory: {}",
                path.display()
            )));
        }

        // Read file bytes
        let raw = match tokio::fs::read(&path).await {
            Ok(bytes) => bytes,
            Err(e) => return Ok(ToolResult::error(format!("Failed to read file: {}", e))),
        };

        // Check for binary file
        if raw.contains(&0u8) {
            return Ok(ToolResult::error(format!(
                "Binary file cannot be read as text: {}",
                path.display()
            )));
        }

        // Decode as UTF-8
        let text = String::from_utf8_lossy(&raw);
        let lines: Vec<&str> = text.lines().collect();

        // Get selected range
        let start = offset.min(lines.len());
        let end = (offset + limit).min(lines.len());
        let selected = &lines[start..end];

        if selected.is_empty() {
            return Ok(ToolResult::success(format!(
                "(no content in selected range for {})",
                path.display()
            )));
        }

        // Add line numbers
        let numbered: Vec<String> = selected
            .iter()
            .enumerate()
            .map(|(i, line)| format!("{:>6}\t{}", start + i + 1, line))
            .collect();

        Ok(ToolResult::success(numbered.join("\n")))
    }
}

fn resolve_path(base: &PathBuf, candidate: &str) -> PathBuf {
    let path = PathBuf::from(candidate);

    // Expand ~ to home directory
    if candidate.starts_with("~/") || candidate == "~" {
        if let Some(home_dir) = dirs::home_dir() {
            let remainder = if candidate == "~" { "" } else { &candidate[2..] };
            return home_dir.join(remainder);
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
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_read_file() {
        let mut temp = NamedTempFile::new().unwrap();
        writeln!(temp, "line 1").unwrap();
        writeln!(temp, "line 2").unwrap();
        writeln!(temp, "line 3").unwrap();

        let tool = FileReadTool;
        let input = json!({"path": temp.path().to_str().unwrap()});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("1\tline 1"));
        assert!(result.output.contains("2\tline 2"));
        assert!(result.output.contains("3\tline 3"));
    }

    #[tokio::test]
    async fn test_read_file_not_found() {
        let tool = FileReadTool;
        let input = json!({"path": "/nonexistent/file.txt"});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(result.is_error);
        assert!(result.output.contains("File not found"));
    }

    #[tokio::test]
    async fn test_read_file_with_offset() {
        let mut temp = NamedTempFile::new().unwrap();
        writeln!(temp, "line 1").unwrap();
        writeln!(temp, "line 2").unwrap();
        writeln!(temp, "line 3").unwrap();

        let tool = FileReadTool;
        let input = json!({"path": temp.path().to_str().unwrap(), "offset": 1, "limit": 1});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("2\tline 2"));
        assert!(!result.output.contains("line 1"));
        assert!(!result.output.contains("line 3"));
    }
}
