//! Bash tool - Execute shell commands

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use anyhow::Result;
use serde_json::{json, Value};
use std::time::Duration;
use tokio::process::Command;

/// Input schema for bash tool
pub fn bash_input_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "command": {
                "type": "string",
                "description": "Shell command to execute"
            },
            "cwd": {
                "type": "string",
                "description": "Working directory override"
            },
            "timeout_seconds": {
                "type": "integer",
                "description": "Command timeout in seconds",
                "default": 120,
                "minimum": 1,
                "maximum": 600
            }
        },
        "required": ["command"]
    })
}

/// Bash tool for executing shell commands
pub struct BashTool;

#[async_trait::async_trait]
impl Tool for BashTool {
    fn name(&self) -> &'static str {
        "bash"
    }

    fn description(&self) -> &'static str {
        "Run a shell command in the local repository and capture stdout/stderr."
    }

    fn input_schema(&self) -> Value {
        bash_input_schema()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        false // Bash commands can modify state
    }

    async fn execute(&self, input: Value, context: ToolExecutionContext) -> Result<ToolResult> {
        let command = input["command"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'command' field"))?;

        let cwd = input["cwd"]
            .as_str()
            .map(|s| s.to_string())
            .unwrap_or_else(|| context.cwd.to_string_lossy().to_string());

        let timeout_seconds = input["timeout_seconds"]
            .as_u64()
            .unwrap_or(120)
            .max(1)
            .min(600);

        // Determine shell based on platform
        #[cfg(windows)]
        let (shell, shell_arg) = ("cmd", "/C");
        #[cfg(unix)]
        let (shell, shell_arg) = ("/bin/bash", "-lc");

        let mut cmd = Command::new(shell);
        cmd.arg(shell_arg)
            .arg(command)
            .current_dir(&cwd)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        // Execute with timeout
        let result = tokio::time::timeout(
            Duration::from_secs(timeout_seconds),
            cmd.output(),
        )
        .await;

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

                let mut parts = Vec::new();
                if !stdout.is_empty() {
                    parts.push(stdout);
                }
                if !stderr.is_empty() {
                    parts.push(stderr);
                }

                let mut text = if parts.is_empty() {
                    "(no output)".to_string()
                } else {
                    parts.join("\n")
                };

                // Truncate very long output
                if text.len() > 12000 {
                    text = format!("{}...\n[truncated]", &text[..12000]);
                }

                let is_error = !output.status.success();

                let mut metadata = std::collections::HashMap::new();
                metadata.insert(
                    "returncode".to_string(),
                    json!(output.status.code().unwrap_or(-1)),
                );

                Ok(ToolResult {
                    output: text,
                    is_error,
                    metadata,
                })
            }
            Ok(Err(e)) => Ok(ToolResult::error(format!("Failed to execute command: {}", e))),
            Err(_) => Ok(ToolResult::error(format!(
                "Command timed out after {} seconds",
                timeout_seconds
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_bash_echo() {
        let tool = BashTool;
        let input = json!({"command": "echo hello"});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();
        assert!(result.output.contains("hello"));
        assert!(!result.is_error);
    }

    #[tokio::test]
    async fn test_bash_invalid_command() {
        let tool = BashTool;
        let input = json!({"command": "nonexistent_command_xyz"});
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();
        assert!(result.is_error);
    }
}
