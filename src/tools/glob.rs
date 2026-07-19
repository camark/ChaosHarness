//! Glob tool - List files matching a glob pattern

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use anyhow::Result;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Input schema for glob tool
pub fn glob_input_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "pattern": {
                "type": "string",
                "description": "Glob pattern to match files, e.g. \"*.docx\", \"**/*.xls\""
            },
            "root": {
                "type": "string",
                "description": "Root directory to search from, e.g. \".\", \"~/Desktop\", \"/home/user\""
            },
            "recursive": {
                "type": "boolean",
                "description": "Search recursively in subdirectories",
                "default": false
            },
            "limit": {
                "type": "integer",
                "description": "Maximum number of results to return",
                "default": 200,
                "minimum": 1,
                "maximum": 5000
            }
        },
        "required": ["pattern"]
    })
}

/// Glob tool
pub struct GlobTool;

#[async_trait::async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &'static str {
        "glob"
    }

    fn description(&self) -> &'static str {
        "List files matching a glob pattern. Use root parameter to specify the search directory, e.g. root: \"~/Desktop\" for user's desktop."
    }

    fn input_schema(&self) -> Value {
        glob_input_schema()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        true
    }

    async fn execute(&self, input: Value, context: ToolExecutionContext) -> Result<ToolResult> {
        let pattern = input["pattern"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'pattern' field"))?;

        let root = input["root"]
            .as_str()
            .map(|s| resolve_path(&context.cwd, s))
            .unwrap_or_else(|| context.cwd.clone());

        let recursive = input["recursive"].as_bool().unwrap_or(false);
        let limit = input["limit"].as_u64().unwrap_or(200).min(5000) as usize;

        let mut matches: Vec<String> = Vec::new();

        // For non-recursive search, filter out files in subdirectories
        let search_pattern = root.join(pattern);

        if let Ok(entries) = glob::glob(&search_pattern.to_string_lossy()) {
            for entry in entries.take(limit).flatten() {
                let path = entry;
                if path.is_file() {
                    // For non-recursive, check that the file is directly in root
                    if !recursive {
                        if let Some(parent) = path.parent() {
                            if parent != root {
                                continue;
                            }
                        }
                    }

                    let relative = path
                        .strip_prefix(&root)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .to_string();
                    matches.push(relative);
                }
            }
        }

        // Fallback to std::fs read_dir for simple patterns
        if matches.is_empty() && !pattern.contains('*') {
            let candidate = root.join(pattern);
            if candidate.exists() && candidate.is_file() {
                let relative = candidate
                    .strip_prefix(&root)
                    .unwrap_or(&candidate)
                    .to_string_lossy()
                    .to_string();
                matches.push(relative);
            }
        }

        if matches.is_empty() {
            return Ok(ToolResult::success("(no matches)".to_string()));
        }

        matches.sort();
        Ok(ToolResult::success(matches.join("\n")))
    }
}

fn resolve_path(base: &Path, candidate: &str) -> PathBuf {
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
                // Fallback if desktop_dir not available
                return home_dir.join("Desktop");
            }

            return home_dir.join(remainder);
        }
    }

    // Auto-detect Desktop directory if candidate is "Desktop" or ends with "/Desktop"
    if candidate == "Desktop" || candidate.ends_with("/Desktop") {
        if let Some(desktop_dir) = dirs::desktop_dir() {
            return desktop_dir;
        }
        // Fallback to ~/Desktop if desktop_dir not available
        if let Some(home_dir) = dirs::home_dir() {
            return home_dir.join("Desktop");
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
    use std::fs::File;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_glob_rust_files() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("test1.rs")).unwrap();
        File::create(dir.path().join("test2.rs")).unwrap();
        File::create(dir.path().join("test.txt")).unwrap();

        let tool = GlobTool;
        let input = json!({
            "pattern": "*.rs",
            "root": dir.path().to_str().unwrap()
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("test1.rs"));
        assert!(result.output.contains("test2.rs"));
        assert!(!result.output.contains("test.txt"));
    }

    #[tokio::test]
    async fn test_glob_no_matches() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("test.txt")).unwrap();

        let tool = GlobTool;
        let input = json!({
            "pattern": "*.nonexistent",
            "root": dir.path().to_str().unwrap()
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("(no matches)"));
    }
}
