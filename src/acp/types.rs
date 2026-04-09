//! ACP type definitions
//!
//! Based on the ACP specification:
//! - AgentCard: Agent metadata and capabilities
//! - Task: Task representation for agent work
//! - Message: Message types for agent communication
//! - Part: Content parts in messages (text, data, file, resource)

use serde::{Deserialize, Serialize};

/// ACP Protocol version
pub const ACP_VERSION: &str = "0.1.0";

/// AgentCard - Describes an agent's capabilities and metadata
///
/// The AgentCard is like a "business card" for AI agents,
/// allowing discovery and capability negotiation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCard {
    /// Human-readable name of the agent
    pub name: String,
    /// Description of the agent's purpose
    pub description: String,
    /// Agent version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Provider/organization responsible for the agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<ProviderInfo>,
    /// Agent capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<AgentCapabilities>,
    /// Authentication methods supported
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<AuthenticationInfo>,
    /// Input modalities the agent accepts
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_modalities: Option<Vec<Modality>>,
    /// Output modalities the agent produces
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_modalities: Option<Vec<Modality>>,
    /// Agent's skills/abilities
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub skills: Vec<Skill>,
    /// Default language (RFC 5646)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_language: Option<String>,
    /// Supported languages
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub supported_languages: Vec<String>,
    /// Agent endpoint URL
    pub endpoint: String,
    /// Agent website/documentation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    /// Contact information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact: Option<ContactInfo>,
    /// Additional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Provider information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInfo {
    /// Organization name
    pub organization: String,
    /// Provider URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Agent capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCapabilities {
    /// Whether the agent supports streaming responses
    #[serde(skip_serializing_if = "Option::is_none")]
    pub streaming: Option<bool>,
    /// Whether the agent supports tool use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use: Option<bool>,
    /// Whether the agent supports memory/context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<bool>,
    /// Whether the agent supports multi-turn conversations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multi_turn: Option<bool>,
    /// Whether the agent supports file I/O
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_io: Option<bool>,
    /// Whether the agent supports web access
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_access: Option<bool>,
    /// Whether the agent supports code execution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_execution: Option<bool>,
    /// Whether the agent supports vision
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vision: Option<bool>,
}

/// Authentication information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticationInfo {
    /// Authentication schemes supported
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub schemes: Vec<AuthScheme>,
}

/// Authentication scheme types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthScheme {
    #[serde(rename = "api_key")]
    ApiKey,
    #[serde(rename = "bearer")]
    Bearer,
    #[serde(rename = "basic")]
    Basic,
    #[serde(rename = "oauth2")]
    OAuth2,
    #[serde(rename = "none")]
    None,
}

/// Content modality
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Modality {
    Text,
    Audio,
    Image,
    Video,
    File,
    Code,
}

/// Skill definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Skill {
    /// Skill name/identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Description of what the skill does
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Skill category
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// Tags for discovery
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tags: Vec<String>,
    /// Input schema (JSON Schema)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<serde_json::Value>,
    /// Output schema (JSON Schema)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<serde_json::Value>,
    /// Whether the skill requires confirmation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires_confirmation: Option<bool>,
}

/// Contact information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContactInfo {
    /// Contact name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Contact email
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Contact URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Task representation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    /// Unique task identifier
    pub id: String,
    /// Task status
    pub status: TaskStatus,
    /// Task description/goal
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Task artifacts (messages, results)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub artifacts: Vec<TaskArtifact>,
    /// Task history
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub history: Vec<TaskHistoryEntry>,
    /// Timestamp when task was created
    pub created_at: String,
    /// Timestamp when task was last updated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    /// Task metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Task status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    #[serde(rename = "submitted")]
    Submitted,
    #[serde(rename = "working")]
    Working,
    #[serde(rename = "input-required")]
    InputRequired,
    #[serde(rename = "paused")]
    Paused,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "canceled")]
    Canceled,
    #[serde(rename = "failed")]
    Failed,
}

/// Task artifact - output or intermediate result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskArtifact {
    /// Artifact identifier
    pub id: String,
    /// Artifact name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Artifact type
    #[serde(rename = "type")]
    pub artifact_type: ArtifactType,
    /// Artifact content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<serde_json::Value>,
    /// Artifact MIME type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// Whether this is the final artifact
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_output: Option<bool>,
}

/// Artifact type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArtifactType {
    Text,
    Image,
    Audio,
    Video,
    File,
    Code,
    Data,
}

/// Task history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskHistoryEntry {
    /// Entry type
    #[serde(rename = "type")]
    pub entry_type: HistoryEntryType,
    /// Timestamp
    pub timestamp: String,
    /// Entry details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// History entry type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HistoryEntryType {
    StatusChange,
    Message,
    ToolCall,
    Error,
    UserInput,
}

/// Message for agent communication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    /// Message role
    pub role: MessageRole,
    /// Message content parts
    pub content: Vec<MessagePart>,
    /// Message metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Message role
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Agent,
    System,
}

/// Message content part
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum MessagePart {
    Text { text: String },
    Data { data: serde_json::Value },
    File { file: FileInfo },
    Resource { resource: ResourceInfo },
}

/// File information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
    /// File name
    pub name: String,
    /// File content (base64 encoded for binary)
    pub content: String,
    /// MIME type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// Resource information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceInfo {
    /// Resource URI
    pub uri: String,
    /// Resource name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// MIME type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// Task creation request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskRequest {
    /// Task description
    pub description: String,
    /// Initial message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<Message>,
    /// Task configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<TaskConfig>,
    /// Additional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Task configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskConfig {
    /// Whether to stream responses
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// Preferred language
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Additional context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
}

/// Task send request (add message to existing task)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskSendRequest {
    /// Message to send
    pub message: Message,
    /// Whether to stream response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

/// Task send response (streaming)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskSendResponse {
    /// Task ID
    pub id: String,
    /// Response artifacts
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub artifacts: Vec<TaskArtifact>,
    /// Task status
    pub status: TaskStatus,
}

/// Error response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    /// Error code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// Error message
    pub message: String,
    /// Additional error details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}
