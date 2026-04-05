//! Directory tree tool - List directory structure as a tree

use crate::tools::base::{Tool, ToolResult, ToolExecutionContext};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::fs;
use std::collections::VecDeque;

#[derive(Debug, Deserialize)]
pub struct DirectoryTreeInput {
    /// Directory path to list (default: current directory)
    #[serde(default)]
    pub path: Option<String>,
    /// Maximum depth to traverse (default: 3)
    #[serde(default = "default_depth")]
    pub max_depth: u32,
    /// Include hidden files (starting with .)
    #[serde(default)]
    pub include_hidden: bool,
    /// Glob pattern to filter files (e.g., "*.rs")
    #[serde(default)]
    pub pattern: Option<String>,
}

fn default_depth() -> u32 { 3 }

#[derive(Debug, Serialize)]
struct DirectoryTreeOutput {
    tree: String,
    total_files: usize,
    total_dirs: usize,
}

pub struct DirectoryTreeTool {
    cwd: PathBuf,
}

impl DirectoryTreeTool {
    pub fn new(cwd: PathBuf) -> Self {
        Self { cwd }
    }

    fn resolve_path(&self, input_path: Option<&str>) -> PathBuf {
        match input_path {
            Some(p) if !p.is_empty() => {
                // Expand ~ to home directory
                if p.starts_with("~/") || p == "~" {
                    if let Some(home_dir) = dirs::home_dir() {
                        let remainder = if p == "~" { "" } else { &p[2..] };
                        return home_dir.join(remainder);
                    }
                }

                let path = Path::new(p);
                if path.is_absolute() {
                    path.to_path_buf()
                } else {
                    self.cwd.join(path)
                }
            }
            _ => self.cwd.clone(),
        }
    }

    fn is_hidden(path: &Path) -> bool {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.starts_with('.'))
            .unwrap_or(false)
    }

    fn matches_pattern(file_name: &str, pattern: Option<&str>) -> bool {
        match pattern {
            Some(pat) => {
                if pat.starts_with('*') && pat.ends_with('*') {
                    file_name.contains(&pat[1..pat.len()-1])
                } else if pat.starts_with('*') {
                    file_name.ends_with(&pat[1..])
                } else if pat.ends_with('*') {
                    file_name.starts_with(&pat[..pat.len()-1])
                } else {
                    file_name == pat
                }
            }
            None => true,
        }
    }

    fn build_tree(
        &self,
        root: &Path,
        max_depth: u32,
        include_hidden: bool,
        pattern: Option<&str>,
    ) -> Result<DirectoryTreeOutput> {
        let mut tree = String::new();
        let mut total_files = 0;
        let mut total_dirs = 0;

        let mut queue: VecDeque<(PathBuf, u32, String)> = VecDeque::new();

        if !root.exists() {
            return Err(anyhow::anyhow!("Directory does not exist: {}", root.display()));
        }

        queue.push_back((root.to_path_buf(), 0, "".to_string()));

        while let Some((dir, depth, prefix)) = queue.pop_front() {
            let entries = match fs::read_dir(&dir) {
                Ok(e) => e.collect::<Result<Vec<_>, _>>()?,
                Err(e) => {
                    if depth == 0 {
                        return Err(anyhow::anyhow!("Cannot read directory: {}", e));
                    }
                    continue;
                }
            };

            let mut dirs: Vec<_> = Vec::new();
            let mut files: Vec<_> = Vec::new();

            for entry in entries {
                let path = entry.path();
                let file_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");

                if !include_hidden && Self::is_hidden(&path) {
                    continue;
                }

                if path.is_dir() {
                    dirs.push((file_name.to_string(), path));
                } else if Self::matches_pattern(file_name, pattern) {
                    files.push(file_name.to_string());
                }
            }

            dirs.sort_by(|a, b| a.0.cmp(&b.0));
            files.sort();

            if depth == 0 {
                let dir_name = root.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("root");
                tree.push_str(&format!("{}\n", dir_name));
                total_dirs += 1;
            }

            // Process files first
            for (i, file) in files.iter().enumerate() {
                let is_last = i == files.len() + dirs.len() - 1;
                let conn = if is_last { "└── " } else { "├── " };
                tree.push_str(&format!("{}{}{}\n", prefix, conn, file));
                total_files += 1;
            }

            // Process directories and queue children
            for (i, (dir_name, dir_path)) in dirs.iter().enumerate() {
                let is_last = i == dirs.len() - 1;
                let conn = if is_last { "└── " } else { "├── " };

                tree.push_str(&format!("{}{}{}/\n", prefix, conn, dir_name));
                total_dirs += 1;

                if depth + 1 < max_depth {
                    let child_prefix = format!("{}{}", prefix, if is_last { "    " } else { "│   " });
                    queue.push_back((dir_path.clone(), depth + 1, child_prefix));
                }
            }

            if depth + 1 == max_depth && !dirs.is_empty() {
                tree.push_str(&format!("{}    ...\n", prefix));
            }
        }

        Ok(DirectoryTreeOutput { tree, total_files, total_dirs })
    }
}

#[async_trait]
impl Tool for DirectoryTreeTool {
    fn name(&self) -> &'static str {
        "directory_tree"
    }

    fn description(&self) -> &'static str {
        "List directory structure as a tree. Use max_depth to control traversal depth, pattern to filter files, and include_hidden to show dotfiles."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory path to list (default: current directory)"
                },
                "max_depth": {
                    "type": "integer",
                    "description": "Maximum depth to traverse (default: 3)",
                    "default": 3
                },
                "include_hidden": {
                    "type": "boolean",
                    "description": "Include hidden files (starting with .)",
                    "default": false
                },
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern to filter files (e.g., \"*.rs\")"
                }
            },
            "required": []
        })
    }

    async fn execute(&self, input: Value, _ctx: ToolExecutionContext) -> Result<ToolResult> {
        let input: DirectoryTreeInput = match serde_json::from_value(input) {
            Ok(i) => i,
            Err(e) => {
                return Ok(ToolResult::error(format!("Invalid input: {}", e)));
            }
        };

        let path = self.resolve_path(input.path.as_deref());

        match self.build_tree(&path, input.max_depth, input.include_hidden, input.pattern.as_deref()) {
            Ok(output) => Ok(ToolResult::success(serde_json::to_string_pretty(&output)?)),
            Err(e) => Ok(ToolResult::error(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[tokio::test]
    async fn test_directory_tree_current_dir() {
        let tool = DirectoryTreeTool::new(env::current_dir().unwrap());
        let input = serde_json::json!({});
        let ctx = ToolExecutionContext::new(env::current_dir().unwrap());

        let result = tool.execute(input, ctx).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("src"));
    }

    #[tokio::test]
    async fn test_directory_tree_with_pattern() {
        let tool = DirectoryTreeTool::new(env::current_dir().unwrap());
        let input = serde_json::json!({
            "pattern": "*.rs",
            "max_depth": 2
        });
        let ctx = ToolExecutionContext::new(env::current_dir().unwrap());

        let result = tool.execute(input, ctx).await.unwrap();

        assert!(!result.is_error);
        let tree = &result.output;
        assert!(tree.contains(".rs") || !tree.contains("."));
    }

    #[tokio::test]
    async fn test_directory_tree_nonexistent_path() {
        let tool = DirectoryTreeTool::new(env::current_dir().unwrap());
        let input = serde_json::json!({
            "path": "/nonexistent/path/that/does/not/exist"
        });
        let ctx = ToolExecutionContext::new(env::current_dir().unwrap());

        let result = tool.execute(input, ctx).await.unwrap();

        assert!(result.is_error);
        assert!(result.output.contains("does not exist"));
    }
}
