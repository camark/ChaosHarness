//! ExitWorktree tool - Remove a git worktree

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use anyhow::Result;
use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;

/// Input schema for exit_worktree tool
pub fn exit_worktree_input_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "path": {
                "type": "string",
                "description": "Worktree path to remove"
            }
        },
        "required": ["path"]
    })
}

/// ExitWorktree tool
pub struct ExitWorktreeTool;

#[async_trait::async_trait]
impl Tool for ExitWorktreeTool {
    fn name(&self) -> &'static str {
        "exit_worktree"
    }

    fn description(&self) -> &'static str {
        "Remove a git worktree by path."
    }

    fn input_schema(&self) -> Value {
        exit_worktree_input_schema()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        false
    }

    async fn execute(&self, input: Value, context: ToolExecutionContext) -> Result<ToolResult> {
        let path = input["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' field"))?;

        let worktree_path = PathBuf::from(path);
        let worktree_path = if worktree_path.is_absolute() {
            worktree_path
        } else {
            context.cwd.join(worktree_path)
        };

        // Run git worktree remove
        let result = Command::new("git")
            .args(["worktree", "remove", "--force"])
            .arg(&worktree_path)
            .current_dir(&context.cwd)
            .output();

        match result {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                let output = if stdout.is_empty() && stderr.is_empty() {
                    format!("Removed worktree {}", worktree_path.display())
                } else {
                    format!("{}{}", stdout.trim(), stderr.trim())
                };

                if out.status.success() {
                    Ok(ToolResult::success(output))
                } else {
                    Ok(ToolResult::error(output))
                }
            }
            Err(e) => Ok(ToolResult::error(format!("Failed to remove worktree: {}", e))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_exit_worktree_missing_path() {
        let tool = ExitWorktreeTool;
        let input = serde_json::json!({});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await;

        assert!(result.is_err());
    }
}
