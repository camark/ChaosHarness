//! Grep tool - Search file contents with regular expressions

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use anyhow::Result;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::fs::File;
use std::io::{BufRead, BufReader};

/// Input schema for grep tool
pub fn grep_input_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "pattern": {
                "type": "string",
                "description": "Regular expression to search for"
            },
            "root": {
                "type": "string",
                "description": "Search root directory"
            },
            "file_glob": {
                "type": "string",
                "description": "File pattern to search",
                "default": "**/*"
            },
            "case_sensitive": {
                "type": "boolean",
                "description": "Case-sensitive search",
                "default": true
            },
            "limit": {
                "type": "integer",
                "description": "Maximum number of results to return",
                "default": 200,
                "minimum": 1,
                "maximum": 2000
            }
        },
        "required": ["pattern"]
    })
}

/// Grep tool
pub struct GrepTool;

#[async_trait::async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &'static str {
        "grep"
    }

    fn description(&self) -> &'static str {
        "Search file contents with a regular expression."
    }

    fn input_schema(&self) -> Value {
        grep_input_schema()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        true
    }

    async fn execute(&self, input: Value, context: ToolExecutionContext) -> Result<ToolResult> {
        let pattern_str = input["pattern"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'pattern' field"))?;

        let root = input["root"]
            .as_str()
            .map(|s| resolve_path(&context.cwd, s))
            .unwrap_or_else(|| context.cwd.clone());

        let file_glob = input["file_glob"].as_str().unwrap_or("**/*");
        let case_sensitive = input["case_sensitive"].as_bool().unwrap_or(true);
        let limit = input["limit"].as_u64().unwrap_or(200).min(2000) as usize;

        // Compile regex
        let regex_result = if case_sensitive {
            regex::Regex::new(pattern_str)
        } else {
            regex::RegexBuilder::new(pattern_str)
                .case_insensitive(true)
                .build()
        };

        let re = match regex_result {
            Ok(re) => re,
            Err(e) => return Ok(ToolResult::error(format!("Invalid regex pattern: {}", e))),
        };

        let mut matches: Vec<String> = Vec::new();

        // Search files matching glob pattern
        search_files(&root, file_glob, &re, limit, &mut matches).await;

        if matches.is_empty() {
            return Ok(ToolResult::success("(no matches)".to_string()));
        }

        Ok(ToolResult::success(matches.join("\n")))
    }
}

async fn search_files(
    root: &PathBuf,
    glob_pattern: &str,
    re: &regex::Regex,
    limit: usize,
    matches: &mut Vec<String>,
) {
    // Use glob to find matching files
    let pattern = root.join(glob_pattern).to_string_lossy().to_string();

    if let Ok(entries) = glob::glob(&pattern) {
        for entry in entries {
            if matches.len() >= limit {
                break;
            }

            if let Ok(path) = entry {
                if !path.is_file() {
                    continue;
                }

                // Try to read the file
                let file = match File::open(&path) {
                    Ok(f) => f,
                    Err(_) => continue,
                };

                let reader = BufReader::new(file);

                for (line_no, line_result) in reader.lines().enumerate() {
                    if matches.len() >= limit {
                        break;
                    }

                    if let Ok(line) = line_result {
                        if re.is_match(&line) {
                            if let Ok(relative) = path.strip_prefix(root) {
                                matches.push(format!(
                                    "{}:{}:{}",
                                    relative.to_string_lossy(),
                                    line_no + 1,
                                    line
                                ));
                            }
                        }
                    }
                }
            }
        }
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
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_grep_pattern() {
        let dir = tempdir().unwrap();
        let mut file1 = File::create(dir.path().join("test.txt")).unwrap();
        writeln!(file1, "hello world").unwrap();
        writeln!(file1, "foo bar").unwrap();
        writeln!(file1, "hello rust").unwrap();

        let tool = GrepTool;
        let input = json!({
            "pattern": "hello",
            "root": dir.path().to_str().unwrap()
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("test.txt:1:hello world"));
        assert!(result.output.contains("test.txt:3:hello rust"));
    }

    #[tokio::test]
    async fn test_grep_case_insensitive() {
        let dir = tempdir().unwrap();
        let mut file1 = File::create(dir.path().join("test.txt")).unwrap();
        writeln!(file1, "Hello World").unwrap();

        let tool = GrepTool;
        let input = json!({
            "pattern": "HELLO",
            "root": dir.path().to_str().unwrap(),
            "case_sensitive": false
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("Hello World"));
    }

    #[tokio::test]
    async fn test_grep_no_matches() {
        let dir = tempdir().unwrap();
        let mut file1 = File::create(dir.path().join("test.txt")).unwrap();
        writeln!(file1, "hello world").unwrap();

        let tool = GrepTool;
        let input = json!({
            "pattern": "nonexistent",
            "root": dir.path().to_str().unwrap()
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("(no matches)"));
    }

    #[tokio::test]
    async fn test_grep_invalid_regex() {
        let tool = GrepTool;
        let input = json!({
            "pattern": "[invalid(regex"
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(result.is_error);
        assert!(result.output.contains("Invalid regex"));
    }
}
