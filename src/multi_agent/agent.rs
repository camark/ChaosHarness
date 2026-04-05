//! Agent definition and state management

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::config::Settings;
use crate::engine::messages::ConversationMessage;

/// Agent roles for specialized tasks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    /// General purpose assistant
    General,
    /// Code reviewer
    Reviewer,
    /// Test writer
    Tester,
    /// Documentation writer
    Documenter,
    /// Security analyst
    SecurityAnalyst,
    /// Architecture planner
    Architect,
    /// Debugging specialist
    Debugger,
}

impl AgentRole {
    pub fn system_prompt(&self) -> &'static str {
        match self {
            AgentRole::General => "You are a helpful AI assistant.",
            AgentRole::Reviewer => "You are an expert code reviewer. Analyze code for quality, security, and best practices.",
            AgentRole::Tester => "You are a test engineering specialist. Write comprehensive tests and identify edge cases.",
            AgentRole::Documenter => "You are a technical writer. Create clear, concise documentation.",
            AgentRole::SecurityAnalyst => "You are a security expert. Identify vulnerabilities and suggest fixes.",
            AgentRole::Architect => "You are a software architect. Design scalable, maintainable systems.",
            AgentRole::Debugger => "You are a debugging specialist. Find and fix bugs systematically.",
        }
    }
}

/// Agent state
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
pub enum AgentState {
    #[default]
    Idle,
    Thinking,
    Executing,
    Waiting,
    Completed,
}

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub role: AgentRole,
    pub model: Option<String>,
    pub system_prompt: Option<String>,
    pub max_tokens: Option<u32>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            name: "assistant".to_string(),
            role: AgentRole::General,
            model: None,
            system_prompt: None,
            max_tokens: None,
        }
    }
}

/// Agent instance
#[derive(Clone, Debug)]
pub struct Agent {
    pub config: AgentConfig,
    pub state: Arc<Mutex<AgentState>>,
    pub message_history: Arc<Mutex<Vec<ConversationMessage>>>,
    pub settings: Settings,
}

impl Agent {
    pub fn new(config: AgentConfig, settings: Settings) -> Self {
        Self {
            config,
            state: Arc::new(Mutex::new(AgentState::Idle)),
            message_history: Arc::new(Mutex::new(Vec::new())),
            settings,
        }
    }

    pub fn get_effective_system_prompt(&self) -> String {
        self.config.system_prompt
            .clone()
            .unwrap_or_else(|| self.config.role.system_prompt().to_string())
    }

    pub fn get_model(&self) -> String {
        self.config.model
            .clone()
            .unwrap_or_else(|| self.settings.model.clone())
    }

    pub fn get_max_tokens(&self) -> u32 {
        self.config.max_tokens
            .unwrap_or(self.settings.max_tokens)
    }

    pub async fn get_state(&self) -> AgentState {
        *self.state.lock().await
    }

    pub async fn set_state(&self, state: AgentState) {
        *self.state.lock().await = state;
    }

    pub async fn add_message(&self, message: ConversationMessage) {
        let mut history = self.message_history.lock().await;
        history.push(message);
    }

    pub async fn get_history(&self) -> Vec<ConversationMessage> {
        self.message_history.lock().await.clone()
    }

    pub async fn clear_history(&self) {
        let mut history = self.message_history.lock().await;
        history.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_role_prompts() {
        assert!(AgentRole::Reviewer.system_prompt().contains("reviewer"));
        assert!(AgentRole::Tester.system_prompt().contains("test"));
        assert!(AgentRole::Documenter.system_prompt().contains("documentation"));
    }

    #[tokio::test]
    async fn test_agent_state() {
        let config = AgentConfig::default();
        let settings = Settings::default();
        let agent = Agent::new(config, settings);

        assert_eq!(agent.get_state().await, AgentState::Idle);

        agent.set_state(AgentState::Thinking).await;
        assert_eq!(agent.get_state().await, AgentState::Thinking);
    }
}
