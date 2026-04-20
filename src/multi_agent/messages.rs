//! Inter-agent communication messages

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// Message types for agent communication
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    /// Task request from coordinator
    TaskRequest,
    /// Task response from agent
    TaskResponse,
    /// Request for help from another agent
    HelpRequest,
    /// Help response
    HelpResponse,
    /// Status update
    StatusUpdate,
    /// Final result
    FinalResult,
    /// Error message
    Error,
}

/// Message exchanged between agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub id: String,
    pub from: String,
    pub to: Option<String>, // None = broadcast
    pub message_type: MessageType,
    pub content: String,
    pub timestamp: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
}

impl AgentMessage {
    pub fn new(from: impl Into<String>, message_type: MessageType, content: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            from: from.into(),
            to: None,
            message_type,
            content: content.into(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            context: None,
        }
    }

    pub fn with_target(mut self, to: impl Into<String>) -> Self {
        self.to = Some(to.into());
        self
    }

    pub fn with_context(mut self, context: serde_json::Value) -> Self {
        self.context = Some(context);
        self
    }

    pub fn task_request(from: impl Into<String>, task: impl Into<String>) -> Self {
        Self::new(from, MessageType::TaskRequest, task)
    }

    pub fn task_response(from: impl Into<String>, result: impl Into<String>) -> Self {
        Self::new(from, MessageType::TaskResponse, result)
    }

    pub fn help_request(from: impl Into<String>, question: impl Into<String>) -> Self {
        Self::new(from, MessageType::HelpRequest, question)
    }

    pub fn status_update(from: impl Into<String>, status: impl Into<String>) -> Self {
        Self::new(from, MessageType::StatusUpdate, status)
    }

    pub fn error(from: impl Into<String>, error: impl Into<String>) -> Self {
        Self::new(from, MessageType::Error, error)
    }
}

/// Task assignment from coordinator to agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub description: String,
    pub assigned_to: String,
    pub priority: u32,
    pub dependencies: Vec<String>,
    pub context: Option<serde_json::Value>,
}

impl Task {
    pub fn new(description: impl Into<String>, assigned_to: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            description: description.into(),
            assigned_to: assigned_to.into(),
            priority: 1,
            dependencies: Vec::new(),
            context: None,
        }
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_dependencies(mut self, deps: Vec<String>) -> Self {
        self.dependencies = deps;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_message_creation() {
        let msg = AgentMessage::task_request("agent1", "Review this code");
        assert_eq!(msg.message_type, MessageType::TaskRequest);
        assert_eq!(msg.from, "agent1");
        assert!(msg.content.contains("Review this code"));
    }

    #[test]
    fn test_agent_message_with_target() {
        let msg = AgentMessage::new("agent1", MessageType::HelpRequest, "Help!")
            .with_target("agent2");
        assert_eq!(msg.to, Some("agent2".to_string()));
    }
}
