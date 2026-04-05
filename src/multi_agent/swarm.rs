//! Agent swarm for collaborative task execution
//!
//! A swarm is a group of agents working together on a complex task

use std::sync::Arc;
use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};

use super::agent::{AgentConfig, AgentRole};
use super::coordinator::{Coordinator, TaskResult};
use super::messages::AgentMessage;
use crate::config::Settings;

/// Swarm configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmConfig {
    pub name: String,
    pub max_agents: usize,
    pub required_roles: Vec<AgentRole>,
    pub collaboration_mode: CollaborationMode,
}

impl Default for SwarmConfig {
    fn default() -> Self {
        Self {
            name: "default_swarm".to_string(),
            max_agents: 5,
            required_roles: vec![AgentRole::General],
            collaboration_mode: CollaborationMode::Sequential,
        }
    }
}

/// Collaboration mode for swarm
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CollaborationMode {
    /// Agents work one after another
    Sequential,
    /// Agents work simultaneously
    Parallel,
    /// Agents vote on decisions
    Democratic,
    /// Hierarchical with leader
    Hierarchical,
}

/// Swarm state
#[derive(Debug, Clone, Default)]
pub struct SwarmState {
    pub agents: Vec<String>,
    pub active_task: Option<String>,
    pub completed_tasks: Vec<TaskResult>,
    pub message_log: Vec<AgentMessage>,
    pub status: SwarmStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SwarmStatus {
    #[default]
    Idle,
    Initializing,
    Running,
    Paused,
    Completed,
    Error,
}

/// Agent swarm for collaborative execution
pub struct Swarm {
    pub config: SwarmConfig,
    pub state: Arc<Mutex<SwarmState>>,
    pub coordinator: Coordinator,
}

impl Swarm {
    pub fn new(config: SwarmConfig, settings: Settings) -> Self {
        Self {
            config,
            state: Arc::new(Mutex::new(SwarmState::default())),
            coordinator: Coordinator::new(settings),
        }
    }

    /// Initialize swarm with required agents
    pub async fn initialize(&self) -> anyhow::Result<()> {
        let mut state = self.state.lock().await;
        state.status = SwarmStatus::Initializing;

        // Create agents for each required role
        for role in &self.config.required_roles {
            let config = AgentConfig {
                name: format!("{:?}", role),
                role: role.clone(),
                ..Default::default()
            };

            let agent_id = self.coordinator.register_agent(config).await;
            state.agents.push(agent_id);
        }

        state.status = SwarmStatus::Idle;

        // Log initialization
        let msg = AgentMessage::status_update(
            "swarm",
            format!("Swarm '{}' initialized with {} agents", self.config.name, state.agents.len()),
        );
        state.message_log.push(msg);

        Ok(())
    }

    /// Execute a complex task using the swarm
    pub async fn execute(&self, task_description: &str) -> anyhow::Result<Vec<TaskResult>> {
        let mut state = self.state.lock().await;
        state.status = SwarmStatus::Running;
        state.active_task = Some(task_description.to_string());

        // Record start message
        let msg = AgentMessage::status_update(
            "swarm",
            format!("Starting task: {}", task_description),
        );
        state.message_log.push(msg);

        // Drop lock before awaiting
        drop(state);

        // Decompose task and assign to agents based on roles
        self.decompose_and_assign(task_description).await?;

        // Wait for completion (simplified - in production would use proper async coordination)
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let mut state = self.state.lock().await;
        state.status = SwarmStatus::Completed;
        state.active_task = None;

        Ok(state.completed_tasks.clone())
    }

    /// Decompose task and assign to appropriate agents
    async fn decompose_and_assign(&self, task: &str) -> anyhow::Result<()> {
        // Simple decomposition - in production would use AI to break down tasks
        let subtasks = self.decompose_task(task).await;

        for subtask in subtasks {
            self.coordinator.create_task(
                subtask,
                self.get_appropriate_role(task).await,
            ).await;
        }

        Ok(())
    }

    /// Decompose task into subtasks (simplified version)
    async fn decompose_task(&self, _task: &str) -> Vec<String> {
        // In production, this would use AI to intelligently decompose
        vec![
            "Analyze requirements".to_string(),
            "Plan approach".to_string(),
            "Execute task".to_string(),
            "Review results".to_string(),
        ]
    }

    /// Get appropriate agent role for a task
    async fn get_appropriate_role(&self, task: &str) -> AgentRole {
        let task_lower = task.to_lowercase();

        if task_lower.contains("review") || task_lower.contains("audit") {
            AgentRole::Reviewer
        } else if task_lower.contains("test") {
            AgentRole::Tester
        } else if task_lower.contains("document") || task_lower.contains("explain") {
            AgentRole::Documenter
        } else if task_lower.contains("security") || task_lower.contains("vulnerability") {
            AgentRole::SecurityAnalyst
        } else if task_lower.contains("design") || task_lower.contains("architecture") {
            AgentRole::Architect
        } else if task_lower.contains("debug") || task_lower.contains("fix") {
            AgentRole::Debugger
        } else {
            AgentRole::General
        }
    }

    /// Get swarm status
    pub async fn get_status(&self) -> SwarmStatus {
        self.state.lock().await.status.clone()
    }

    /// Get message log
    pub async fn get_messages(&self) -> Vec<AgentMessage> {
        self.state.lock().await.message_log.clone()
    }

    /// Get all agents
    pub async fn get_agents(&self) -> Vec<String> {
        self.state.lock().await.agents.clone()
    }

    /// Pause swarm execution
    pub async fn pause(&self) {
        let mut state = self.state.lock().await;
        state.status = SwarmStatus::Paused;
    }

    /// Resume swarm execution
    pub async fn resume(&self) {
        let mut state = self.state.lock().await;
        if state.status == SwarmStatus::Paused {
            state.status = SwarmStatus::Running;
        }
    }

    /// Stop swarm execution
    pub async fn stop(&self) {
        let mut state = self.state.lock().await;
        state.status = SwarmStatus::Idle;
        state.active_task = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_swarm_creation() {
        let config = SwarmConfig::default();
        let settings = Settings::default();
        let swarm = Swarm::new(config, settings);

        assert_eq!(swarm.get_status().await, SwarmStatus::Idle);
    }

    #[tokio::test]
    async fn test_swarm_initialization() {
        let config = SwarmConfig {
            name: "test_swarm".to_string(),
            required_roles: vec![AgentRole::General, AgentRole::Reviewer],
            ..Default::default()
        };
        let settings = Settings::default();
        let swarm = Swarm::new(config, settings);

        swarm.initialize().await.unwrap();

        let agents = swarm.get_agents().await;
        assert_eq!(agents.len(), 2);
    }

    #[tokio::test]
    async fn test_role_detection() {
        let config = SwarmConfig::default();
        let settings = Settings::default();
        let swarm = Swarm::new(config, settings);

        assert_eq!(
            swarm.get_appropriate_role("Review this code for bugs").await,
            AgentRole::Reviewer
        );
        assert_eq!(
            swarm.get_appropriate_role("Write unit tests").await,
            AgentRole::Tester
        );
        assert_eq!(
            swarm.get_appropriate_role("Design the system architecture").await,
            AgentRole::Architect
        );
    }

    #[tokio::test]
    async fn test_swarm_execute() {
        let config = SwarmConfig {
            name: "test_execute_swarm".to_string(),
            required_roles: vec![AgentRole::General],
            ..Default::default()
        };
        let settings = Settings::default();
        let swarm = Swarm::new(config, settings);

        // Initialize and execute a task
        swarm.initialize().await.unwrap();
        let results = swarm.execute("Test task").await;

        assert!(results.is_ok());
    }

    #[tokio::test]
    async fn test_swarm_pause_resume() {
        let config = SwarmConfig::default();
        let settings = Settings::default();
        let swarm = Swarm::new(config, settings);

        // Set to running first
        let mut state = swarm.state.lock().await;
        state.status = SwarmStatus::Running;
        drop(state);

        swarm.pause().await;
        assert_eq!(swarm.get_status().await, SwarmStatus::Paused);

        swarm.resume().await;
        assert_eq!(swarm.get_status().await, SwarmStatus::Running);
    }

    #[tokio::test]
    async fn test_swarm_stop() {
        let config = SwarmConfig::default();
        let settings = Settings::default();
        let swarm = Swarm::new(config, settings);

        // Set to running then stop
        let mut state = swarm.state.lock().await;
        state.status = SwarmStatus::Running;
        drop(state);

        swarm.stop().await;
        assert_eq!(swarm.get_status().await, SwarmStatus::Idle);
    }
}
