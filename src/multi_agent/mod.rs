//! Multi-agent coordination system
//!
//! Allows multiple AI agents to collaborate on complex tasks

pub mod agent;
pub mod coordinator;
pub mod messages;
pub mod swarm;

pub use agent::{Agent, AgentConfig, AgentRole, AgentState};
pub use coordinator::{Coordinator, TaskAssignment, TaskResult};
pub use messages::{AgentMessage, MessageType};
pub use swarm::{Swarm, SwarmConfig};
