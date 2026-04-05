//! Hook executor - runs hooks and processes results

use super::events::HookEvent;
use super::registry::HookRegistry;
use super::types::{HookContext, HookDecision, HookResult};
use super::schemas::HookDefinition;
use anyhow::{Result, Context};
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{timeout, Duration};
use tracing::{info, warn, error, debug};

/// Executor for running hooks
pub struct HookExecutor {
    registry: HookRegistry,
}

impl HookExecutor {
    pub fn new(registry: HookRegistry) -> Self {
        Self { registry }
    }

    /// Execute all hooks for an event and return combined result
    pub async fn execute(&self, event: &HookEvent, context: HookContext) -> HookResult {
        let hooks = self.registry.get_hooks(event);

        if hooks.is_empty() {
            return HookResult::continue_result(String::new());
        }

        let mut outputs = Vec::new();

        for hook in hooks {
            match self.execute_single(&hook, &context).await {
                Ok(output) => {
                    // If this is a blocking hook and output indicates failure, block
                    if hook.blocking && !output.trim().is_empty() {
                        // Non-empty output from blocking hook means block
                        return HookResult::block_result(
                            format!("Hook '{}' blocked the operation: {}", hook.name, output)
                        );
                    }
                    outputs.push(output);
                }
                Err(e) => {
                    error!("Hook '{}' failed: {}", hook.name, e);
                    if hook.blocking {
                        return HookResult::block_result(
                            format!("Hook '{}' failed: {}", hook.name, e)
                        );
                    }
                }
            }
        }

        HookResult::continue_result(outputs.join("\n"))
    }

    /// Execute a single hook and check for blocking decision
    pub async fn execute_for_decision(
        &self,
        event: &HookEvent,
        context: &HookContext,
    ) -> Option<HookDecision> {
        let hooks = self.registry.get_hooks(event);

        for hook in hooks {
            if !hook.blocking {
                continue;
            }

            match self.execute_single(&hook, context).await {
                Ok(output) => {
                    if !output.trim().is_empty() {
                        return Some(HookDecision::Block(
                            format!("Hook '{}' blocked: {}", hook.name, output)
                        ));
                    }
                }
                Err(e) => {
                    return Some(HookDecision::Block(
                        format!("Hook '{}' failed: {}", hook.name, e)
                    ));
                }
            }
        }

        None
    }

    /// Execute a single hook command
    async fn execute_single(&self, hook: &HookDefinition, context: &HookContext) -> Result<String> {
        info!("Executing hook: {} (event: {})", hook.name, hook.event);
        debug!("Hook command: {}", hook.command);

        let context_json = serde_json::to_string(context)
            .context("Failed to serialize hook context")?;

        let mut cmd = if cfg!(windows) {
            let mut c = Command::new("cmd");
            c.args(["/C", &hook.command]);
            c
        } else {
            let mut c = Command::new("sh");
            c.args(["-c", &hook.command]);
            c
        };

        // Set environment variables
        cmd.env("HOOK_CONTEXT", &context_json)
            .env("HOOK_NAME", &hook.name)
            .env("HOOK_EVENT", &hook.event);

        if let Some(cwd) = &hook.cwd {
            cmd.current_dir(cwd);
        }

        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Execute with timeout
        let result = timeout(
            Duration::from_secs(hook.timeout),
            cmd.output(),
        )
        .await
        .context("Hook timed out")?
        .context("Failed to execute hook command")?;

        let stdout = String::from_utf8_lossy(&result.stdout).to_string();
        let stderr = String::from_utf8_lossy(&result.stderr).to_string();

        if result.status.success() {
            if !stderr.is_empty() {
                warn!("Hook '{}' had stderr output: {}", hook.name, stderr);
            }
            Ok(stdout)
        } else {
            Err(anyhow::anyhow!(
                "Hook '{}' exited with code {:?}: {}",
                hook.name,
                result.status.code(),
                stderr
            ))
        }
    }

    /// Execute pre_tool_use hooks and check if tool should be blocked
    pub async fn check_pre_tool_use(
        &self,
        tool_name: &str,
        tool_input: &serde_json::Value,
    ) -> Option<String> {
        let context = HookContext::pre_tool_use(tool_name, tool_input);
        let result = self.execute(&HookEvent::PreToolUse, context).await;

        match result.decision {
            HookDecision::Block(reason) => Some(reason),
            _ => None,
        }
    }

    /// Execute post_tool_use hooks (for logging/modification)
    pub async fn notify_post_tool_use(
        &self,
        tool_name: &str,
        tool_input: &serde_json::Value,
        output: &str,
        is_error: bool,
    ) {
        let context = HookContext::post_tool_use(tool_name, tool_input, output, is_error);
        let _ = self.execute(&HookEvent::PostToolUse, context).await;
    }

    /// Execute on_error hooks
    pub async fn notify_error(&self, error: &str) {
        let context = HookContext::on_error(error);
        let _ = self.execute(&HookEvent::OnError, context).await;
    }

    /// Execute on_turn_complete hooks
    pub async fn notify_turn_complete(&self, usage: &serde_json::Value) {
        let context = HookContext::on_turn_complete(usage);
        let _ = self.execute(&HookEvent::OnTurnComplete, context).await;
    }
}

impl Clone for HookExecutor {
    fn clone(&self) -> Self {
        Self {
            registry: self.registry.clone(),
        }
    }
}
