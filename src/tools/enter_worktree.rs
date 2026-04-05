//! EnterWorktree tool - Create a git worktree

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use anyhow::Result;
use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;

/// Input schema for enter_worktree tool
pub fn enter_worktree_input_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "branch": {
                "type": "string",
                "description": "Target branch name for the worktree"
            },
            "path": {
                "type": "string",
                "description": "Optional worktree path"
            },
            "create_branch": {
                "type": "boolean",
                "description": "Create a new branch",
                "default": true
            },
            "base_ref": {
                "type": "string",
                "description": "Base ref when creating a new branch",
                "default": "HEAD"
            }
        },
        "required": ["branch"]
    })
}

/// EnterWorktree tool
pub struct EnterWorktreeTool;

#[async_trait::async_trait]
impl Tool for EnterWorktreeTool {
    fn name(&self) -> &'static str {
        "enter_worktree"
    }

    fn description(&self) -> &'static str {
        "Create a git worktree and return its path."
    }

    fn input_schema(&self) -> Value {
        enter_worktree_input_schema()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        false
    }

    async fn execute(&self, input: Value, context: ToolExecutionContext) -> Result<ToolResult> {
        let branch = input["branch"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'branch' field"))?;

        let path = input["path"].as_str();
        let create_branch = input["create_branch"].as_bool().unwrap_or(true);
        let base_ref = input["base_ref"].as_str().unwrap_or("HEAD");

        // Get git repo root
        let output = Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .current_dir(&context.cwd)
            .output();

        let repo_root = match output {
            Ok(out) => String::from_utf8_lossy(&out.stdout).trim().to_string(),
            Err(_) => {
                return Ok(ToolResult::error(
                    "enter_worktree requires a git repository".to_string()
                ));
            }
        };

        let repo_root_path = PathBuf::from(repo_root);

        // Resolve worktree path
        let worktree_path = match path {
            Some(p) => {
                let resolved = PathBuf::from(p);
                if resolved.is_absolute() {
                    resolved
                } else {
                    repo_root_path.join(resolved)
                }
            }
            None => {
                // Generate path from branch name
                let slug = slugify_branch(branch);
                repo_root_path.join(".rust_harness").join("worktrees").join(slug)
            }
        };

        // Create parent directories
        if let Some(parent) = worktree_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Run git worktree add
        let mut cmd = Command::new("git");
        cmd.args(["worktree", "add"]);

        if create_branch {
            cmd.args(["-b", branch]);
            cmd.arg(&worktree_path);
            cmd.arg(base_ref);
        } else {
            cmd.arg(&worktree_path);
            cmd.arg(branch);
        }

        cmd.current_dir(&repo_root_path);

        let result = cmd.output();

        match result {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                let output = if stdout.is_empty() && stderr.is_empty() {
                    format!("Created worktree {}", worktree_path.display())
                } else {
                    format!("{}\nPath: {}", stdout.trim(), worktree_path.display())
                };

                if out.status.success() {
                    Ok(ToolResult::success(output))
                } else {
                    Ok(ToolResult::error(stderr.trim().to_string()))
                }
            }
            Err(e) => Ok(ToolResult::error(format!("Failed to create worktree: {}", e))),
        }
    }
}

fn slugify_branch(branch: &str) -> String {
    branch
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '.' || c == '_' || c == '-' { c } else { '-' })
        .collect::<String>()
        .trim_end_matches('-')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_enter_worktree_no_git() {
        let tool = EnterWorktreeTool;
        let input = serde_json::json!({"branch": "test-branch"});
        let context = ToolExecutionContext::new(PathBuf::from("C:\\"));
        let result = tool.execute(input, context).await;

        // Test should handle the error properly
        assert!(result.is_err() || result.unwrap().is_error);
    }

    #[tokio::test]
    async fn test_enter_worktree_missing_branch() {
        let tool = EnterWorktreeTool;
        let input = serde_json::json!({});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await;

        assert!(result.is_err());
    }
}
