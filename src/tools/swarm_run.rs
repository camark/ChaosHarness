//! SwarmRun tool - Execute a task using a multi-agent swarm

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use crate::multi_agent::agent::AgentRole;
use super::swarm_create::SWARMS;
use anyhow::Result;
use serde_json::Value;

/// Input schema for swarm_run tool
pub fn swarm_run_input_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "swarm_name": {
                "type": "string",
                "description": "Name of the swarm to use"
            },
            "task": {
                "type": "string",
                "description": "Task description for the swarm"
            },
            "role": {
                "type": "string",
                "description": "Specific role to assign the task to (optional)",
                "enum": ["general", "reviewer", "tester", "documenter", "security", "architect", "debugger"]
            }
        },
        "required": ["swarm_name", "task"]
    })
}

/// SwarmRun tool
pub struct SwarmRunTool;

#[async_trait::async_trait]
impl Tool for SwarmRunTool {
    fn name(&self) -> &'static str {
        "swarm_run"
    }

    fn description(&self) -> &'static str {
        "Execute a task using a multi-agent swarm."
    }

    fn input_schema(&self) -> Value {
        swarm_run_input_schema()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        false
    }

    async fn execute(&self, input: Value, _context: ToolExecutionContext) -> Result<ToolResult> {
        let swarm_name = input["swarm_name"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'swarm_name' field"))?;

        let task = input["task"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'task' field"))?;

        let role = input["role"].as_str().map(|r| match r {
            "general" => AgentRole::General,
            "reviewer" => AgentRole::Reviewer,
            "tester" => AgentRole::Tester,
            "documenter" => AgentRole::Documenter,
            "security" => AgentRole::SecurityAnalyst,
            "architect" => AgentRole::Architect,
            "debugger" => AgentRole::Debugger,
            _ => AgentRole::General,
        });

        let swarms = SWARMS.lock().await;
        let swarm = swarms.get(swarm_name)
            .ok_or_else(|| anyhow::anyhow!("Swarm '{}' not found", swarm_name))?;

        // Create a task in the coordinator
        let task_role = role.unwrap_or(AgentRole::General);
        let task_id = swarm.coordinator.create_task(task.to_string(), task_role).await;

        // Execute the task
        let result = swarm.coordinator.execute_task(&task_id).await
            .map_err(|e| anyhow::anyhow!("Task execution failed: {}", e))?;

        Ok(ToolResult::success(format!(
            "Task completed by swarm '{}':\n\n{}",
            swarm_name, result
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_swarm_run_no_swarm() {
        let tool = SwarmRunTool;
        let input = serde_json::json!({
            "swarm_name": "nonexistent",
            "task": "test task"
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await;

        // Should return Err for non-existent swarm
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }
}
