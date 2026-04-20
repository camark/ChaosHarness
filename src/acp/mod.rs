//! ACP (Agent Communication Protocol) implementation
//!
//! ACP is a REST-based protocol for AI agent interoperability.
//! This module implements:
//! - AgentCard specification for agent metadata and capabilities
//! - REST API endpoints for task and message management
//! - ACP client for communicating with remote agents
//! - ACP server for exposing local agent capabilities
//!
//! Specification reference: https://github.com/i-am-bee/acp

pub mod types;
pub mod agent_card;
pub mod server;
pub mod client;
pub mod handlers;

// Re-exports for convenience
// Note: These are kept for potential external use even if not currently used internally
#[allow(unused_imports)]
pub use types::{AgentCard, AgentCapabilities, Skill, Task, Message, MessageRole};
#[allow(unused_imports)]
pub use server::AcpServerState;
#[allow(unused_imports)]
pub use client::{AcpClient, MessageBuilder};
