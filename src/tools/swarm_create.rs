//! SwarmCreate tool - Create a multi-agent swarm

use crate::tools::base::{Tool, ToolExecutionContext, ToolResult};
use crate::multi_agent::swarm::{Swarm, SwarmConfig, CollaborationMode};
use crate::multi_agent::agent::AgentRole;
use anyhow::Result;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

lazy_static::lazy_static! {
    /// Global swarm instances
    pub static ref SWARMS: Arc<Mutex<std::collections::HashMap<String, Swarm>>> =
        Arc::new(Mutex::new(std::collections::HashMap::new()));
}

/// Input schema for swarm_create tool
pub fn swarm_create_input_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "name": {
                "type": "string",
                "description": "Swarm name"
            },
            "max_agents": {
                "type": "integer",
                "description": "Maximum number of agents",
                "default": 5,
                "minimum": 1,
                "maximum": 10
            },
            "roles": {
                "type": "array",
                "description": "Required agent roles",
                "items": {
                    "type": "string",
                    "enum": ["general", "reviewer", "tester", "documenter", "security", "architect", "debugger"]
                },
                "default": ["general"]
            },
            "collaboration_mode": {
                "type": "string",
                "description": "How agents collaborate",
                "enum": ["sequential", "parallel", "democratic", "hierarchical"],
                "default": "sequential"
            }
        },
        "required": ["name"]
    })
}

/// SwarmCreate tool
pub struct SwarmCreateTool;

#[async_trait::async_trait]
impl Tool for SwarmCreateTool {
    fn name(&self) -> &'static str {
        "swarm_create"
    }

    fn description(&self) -> &'static str {
        "Create a multi-agent swarm for collaborative task execution."
    }

    fn input_schema(&self) -> Value {
        swarm_create_input_schema()
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        false
    }

    async fn execute(&self, input: Value, _context: ToolExecutionContext) -> Result<ToolResult> {
        let name = input["name"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'name' field"))?;

        let max_agents = input["max_agents"].as_u64().unwrap_or(5) as usize;
        let collaboration_mode = match input["collaboration_mode"].as_str().unwrap_or("sequential") {
            "sequential" => CollaborationMode::Sequential,
            "parallel" => CollaborationMode::Parallel,
            "democratic" => CollaborationMode::Democratic,
            "hierarchical" => CollaborationMode::Hierarchical,
            _ => return Ok(ToolResult::error("Invalid collaboration mode".to_string())),
        };

        let roles: Vec<AgentRole> = if let Some(roles_arr) = input["roles"].as_array() {
            roles_arr.iter().filter_map(|r| match r.as_str()? {
                "general" => Some(AgentRole::General),
                "reviewer" => Some(AgentRole::Reviewer),
                "tester" => Some(AgentRole::Tester),
                "documenter" => Some(AgentRole::Documenter),
                "security" => Some(AgentRole::SecurityAnalyst),
                "architect" => Some(AgentRole::Architect),
                "debugger" => Some(AgentRole::Debugger),
                _ => None,
            }).collect()
        } else {
            vec![AgentRole::General]
        };

        let config = SwarmConfig {
            name: name.to_string(),
            max_agents,
            required_roles: roles.clone(),
            collaboration_mode: collaboration_mode.clone(),
        };

        let settings = crate::config::load_settings(None).unwrap_or_default();
        let swarm = Swarm::new(config, settings);

        // Initialize the swarm
        swarm.initialize().await.map_err(|e| anyhow::anyhow!("Failed to initialize swarm: {}", e))?;

        let mut swarms = SWARMS.lock().await;
        swarms.insert(name.to_string(), swarm);

        Ok(ToolResult::success(format!(
            "Created swarm '{}' with {} agents ({:?} mode, roles: {:?})",
            name,
            roles.len(),
            collaboration_mode,
            roles.iter().map(|r| format!("{:?}", r)).collect::<Vec<_>>()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_swarm_create() {
        let tool = SwarmCreateTool;
        let input = serde_json::json!({
            "name": "test-swarm",
            "roles": ["general", "reviewer"],
            "collaboration_mode": "sequential"
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
        assert!(result.output.contains("Created swarm"));
    }

    #[tokio::test]
    async fn test_swarm_create_default() {
        let tool = SwarmCreateTool;
        let input = serde_json::json!({
            "name": "default-swarm"
        });
        let context = ToolExecutionContext::new(PathBuf::from("."));
        let result = tool.execute(input, context).await.unwrap();

        assert!(!result.is_error);
    }
}
