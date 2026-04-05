//! Multi-agent coordinator
//!
//! Coordinates task assignment and result aggregation across multiple agents

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast};
use serde::{Deserialize, Serialize};

use super::agent::{Agent, AgentConfig, AgentRole, AgentState};
use super::messages::{AgentMessage, Task, MessageType};
use crate::config::Settings;

/// Task assignment result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAssignment {
    pub task_id: String,
    pub assigned_to: String,
    pub description: String,
    pub status: TaskStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

/// Task execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub agent_id: String,
    pub success: bool,
    pub output: String,
    pub metadata: Option<serde_json::Value>,
}

/// Coordinator state
#[derive(Debug, Clone, Default)]
pub struct CoordinatorState {
    pub pending_tasks: Vec<Task>,
    pub active_tasks: HashMap<String, TaskAssignment>,
    pub completed_tasks: Vec<TaskResult>,
    pub agents: HashMap<String, Agent>,
}

/// Multi-agent coordinator
pub struct Coordinator {
    pub state: Arc<Mutex<CoordinatorState>>,
    pub message_tx: broadcast::Sender<AgentMessage>,
    pub message_rx: Arc<Mutex<broadcast::Receiver<AgentMessage>>>,
    pub settings: Settings,
}

impl Coordinator {
    pub fn new(settings: Settings) -> Self {
        let (message_tx, _) = broadcast::channel(100);
        let message_rx = Arc::new(Mutex::new(message_tx.subscribe()));

        Self {
            state: Arc::new(Mutex::new(CoordinatorState::default())),
            message_tx,
            message_rx,
            settings,
        }
    }

    /// Register a new agent
    pub async fn register_agent(&self, config: AgentConfig) -> String {
        let uuid = uuid::Uuid::new_v4().to_string();
        let agent_id = format!("{}_{}", config.role_to_string(), &uuid[..8]);
        let agent = Agent::new(config.clone(), self.settings.clone());

        let mut state = self.state.lock().await;
        state.agents.insert(agent_id.clone(), agent);

        // Broadcast agent registered
        let msg = AgentMessage::status_update(
            "coordinator",
            format!("Agent {} registered as {:?}", agent_id, config.role),
        );
        let _ = self.message_tx.send(msg);

        agent_id
    }

    /// Create a task and assign to appropriate agent
    pub async fn create_task(&self, description: String, role: AgentRole) -> String {
        let task = Task::new(description, role_to_string(&role));

        let mut state = self.state.lock().await;
        state.pending_tasks.push(task.clone());

        // Find available agent with matching role
        let mut target_agent_id: Option<String> = None;
        for (agent_id, agent) in &state.agents {
            if agent.config.role == role {
                target_agent_id = Some(agent_id.clone());
                break;
            }
        }

        if let Some(agent_id) = target_agent_id {
            let assignment = TaskAssignment {
                task_id: task.id.clone(),
                assigned_to: agent_id.clone(),
                description: task.description.clone(),
                status: TaskStatus::Pending,
            };
            state.active_tasks.insert(task.id.clone(), assignment);

            // Send task to agent
            let msg = AgentMessage::task_request("coordinator", &task.description)
                .with_target(&agent_id);

            let _ = self.message_tx.send(msg);
        }

        task.id
    }

    /// Get coordinator state
    pub async fn get_state(&self) -> CoordinatorState {
        self.state.lock().await.clone()
    }

    /// Get all registered agents
    pub async fn list_agents(&self) -> Vec<String> {
        let state = self.state.lock().await;
        state.agents.keys().cloned().collect()
    }

    /// Get pending tasks count
    pub async fn pending_tasks_count(&self) -> usize {
        let state = self.state.lock().await;
        state.pending_tasks.len()
    }

    /// Get completed tasks count
    pub async fn completed_tasks_count(&self) -> usize {
        let state = self.state.lock().await;
        state.completed_tasks.len()
    }

    /// Broadcast message to all agents
    pub async fn broadcast(&self, from: &str, content: &str) -> anyhow::Result<()> {
        let msg = AgentMessage::new(from, MessageType::StatusUpdate, content);
        self.message_tx.send(msg)?;
        Ok(())
    }

    /// Send message to specific agent
    pub async fn send_to(&self, from: &str, to: &str, content: &str) -> anyhow::Result<()> {
        let msg = AgentMessage::new(from, MessageType::StatusUpdate, content)
            .with_target(to);
        self.message_tx.send(msg)?;
        Ok(())
    }
}

fn role_to_string(role: &AgentRole) -> String {
    match role {
        AgentRole::General => "general".to_string(),
        AgentRole::Reviewer => "reviewer".to_string(),
        AgentRole::Tester => "tester".to_string(),
        AgentRole::Documenter => "documenter".to_string(),
        AgentRole::SecurityAnalyst => "security".to_string(),
        AgentRole::Architect => "architect".to_string(),
        AgentRole::Debugger => "debugger".to_string(),
    }
}

// Helper method for AgentConfig
impl AgentConfig {
    pub fn role_to_string(&self) -> &str {
        match self.role {
            AgentRole::General => "general",
            AgentRole::Reviewer => "reviewer",
            AgentRole::Tester => "tester",
            AgentRole::Documenter => "documenter",
            AgentRole::SecurityAnalyst => "security",
            AgentRole::Architect => "architect",
            AgentRole::Debugger => "debugger",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_coordinator_creation() {
        let settings = Settings::default();
        let coordinator = Coordinator::new(settings);

        assert_eq!(coordinator.pending_tasks_count().await, 0);
        assert_eq!(coordinator.completed_tasks_count().await, 0);
    }

    #[tokio::test]
    async fn test_agent_registration() {
        let settings = Settings::default();
        let coordinator = Coordinator::new(settings);

        let config = AgentConfig {
            name: "test".to_string(),
            role: AgentRole::Reviewer,
            ..Default::default()
        };

        let agent_id = coordinator.register_agent(config).await;
        assert!(!agent_id.is_empty());

        let agents = coordinator.list_agents().await;
        assert_eq!(agents.len(), 1);
    }

    #[tokio::test]
    async fn test_task_creation() {
        let settings = Settings::default();
        let coordinator = Coordinator::new(settings);

        // Register a reviewer agent
        let config = AgentConfig {
            name: "reviewer".to_string(),
            role: AgentRole::Reviewer,
            ..Default::default()
        };
        let _ = coordinator.register_agent(config).await;

        // Create a task
        let task_id = coordinator.create_task(
            "Review the code for security issues".to_string(),
            AgentRole::Reviewer,
        ).await;

        assert!(!task_id.is_empty());
        assert_eq!(coordinator.pending_tasks_count().await, 1);
    }
}
