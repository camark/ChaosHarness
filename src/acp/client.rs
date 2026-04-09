//! ACP Client implementation
//!
//! Client for communicating with remote ACP agents.
//! Handles:
//! - AgentCard discovery and parsing
//! - Task creation and management
//! - Message sending and response handling
//! - Capability negotiation

use crate::acp::types::*;
use anyhow::{Result, anyhow, bail};
use reqwest::Client;
use tracing::{info, debug};

/// ACP Client for communicating with remote agents
pub struct AcpClient {
    /// HTTP client
    http: Client,
    /// Remote agent's base URL
    base_url: String,
    /// Cached AgentCard
    agent_card: Option<AgentCard>,
    /// API key for authentication (optional)
    api_key: Option<String>,
}

impl AcpClient {
    /// Create a new ACP client
    pub fn new(base_url: &str) -> Self {
        Self {
            http: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            agent_card: None,
            api_key: None,
        }
    }

    /// Create a new ACP client with authentication
    pub fn with_auth(base_url: &str, api_key: &str) -> Self {
        Self {
            http: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            agent_card: None,
            api_key: Some(api_key.to_string()),
        }
    }

    /// Build a request with authentication headers
    fn build_request(&self, method: reqwest::Method, url: &str) -> reqwest::RequestBuilder {
        let mut req = self.http.request(method, url);
        if let Some(ref key) = self.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }
        req
    }

    /// Discover and fetch the AgentCard from a remote agent
    pub async fn discover(&mut self) -> Result<&AgentCard> {
        // Try standard discovery endpoint first
        let discovery_url = format!("{}/.well-known/agent.json", self.base_url);

        let agent_card = match self.fetch_agent_card(&discovery_url).await {
            Ok(card) => card,
            Err(_) => {
                // Fallback to /acp endpoint
                let acp_url = format!("{}/acp", self.base_url);
                self.fetch_agent_card(&acp_url).await?
            }
        };

        info!("Discovered agent: {} v{}",
            agent_card.name,
            agent_card.version.as_deref().unwrap_or("unknown"));

        self.agent_card = Some(agent_card);
        Ok(self.agent_card.as_ref().unwrap())
    }

    /// Fetch AgentCard from a specific URL
    async fn fetch_agent_card(&self, url: &str) -> Result<AgentCard> {
        debug!("Fetching AgentCard from: {}", url);

        let response = self.build_request(reqwest::Method::GET, url)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to fetch AgentCard: {}", e))?;

        if !response.status().is_success() {
            bail!("AgentCard request failed with status: {}", response.status());
        }

        let card: AgentCard = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse AgentCard: {}", e))?;

        Ok(card)
    }

    /// Get the cached AgentCard
    pub fn agent_card(&self) -> Option<&AgentCard> {
        self.agent_card.as_ref()
    }

    /// Check if the remote agent supports a specific capability
    pub fn supports_capability(&self, capability: &str) -> bool {
        self.agent_card
            .as_ref()
            .map(|c| c.has_capability(capability))
            .unwrap_or(false)
    }

    /// Find a skill by ID in the remote agent
    pub fn find_skill(&self, skill_id: &str) -> Option<&Skill> {
        self.agent_card.as_ref().and_then(|c| c.find_skill(skill_id))
    }

    /// Create a new task on the remote agent
    pub async fn create_task(
        &self,
        description: &str,
        message: Option<Message>,
    ) -> Result<Task> {
        let url = format!("{}/tasks", self.base_url);

        let request = CreateTaskRequest {
            description: description.to_string(),
            message,
            config: None,
            metadata: None,
        };

        debug!("Creating task: {}", description);

        let response = self.build_request(reqwest::Method::POST, &url)
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to create task: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let error: Result<ErrorResponse, _> = response.json().await;
            match error {
                Ok(e) => bail!("Task creation failed: {}", e.message),
                Err(_) => bail!("Task creation failed with status: {}", status),
            }
        }

        let task: Task = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse task response: {}", e))?;

        info!("Created task: {}", task.id);
        Ok(task)
    }

    /// Get task status and details
    pub async fn get_task(&self, task_id: &str) -> Result<Task> {
        let url = format!("{}/tasks/{}", self.base_url, task_id);

        debug!("Getting task: {}", task_id);

        let response = self.build_request(reqwest::Method::GET, &url)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to get task: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            if status == reqwest::StatusCode::NOT_FOUND {
                bail!("Task '{}' not found", task_id);
            }
            bail!("Task request failed with status: {}", status);
        }

        let task: Task = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse task: {}", e))?;

        Ok(task)
    }

    /// Send a message to an existing task
    pub async fn send_message(
        &self,
        task_id: &str,
        message: Message,
    ) -> Result<TaskSendResponse> {
        let url = format!("{}/tasks/{}/send", self.base_url, task_id);

        let request = TaskSendRequest {
            message,
            stream: None,
        };

        debug!("Sending message to task: {}", task_id);

        let response = self.build_request(reqwest::Method::POST, &url)
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to send message: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            if status == reqwest::StatusCode::NOT_FOUND {
                bail!("Task '{}' not found", task_id);
            }
            let error: Result<ErrorResponse, _> = response.json().await;
            match error {
                Ok(e) => bail!("Message send failed: {}", e.message),
                Err(_) => bail!("Message send failed with status: {}", status),
            }
        }

        let result: TaskSendResponse = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse response: {}", e))?;

        Ok(result)
    }

    /// Get task artifacts
    pub async fn get_artifacts(&self, task_id: &str) -> Result<Vec<TaskArtifact>> {
        let url = format!("{}/tasks/{}/artifacts", self.base_url, task_id);

        debug!("Getting artifacts for task: {}", task_id);

        let response = self.build_request(reqwest::Method::GET, &url)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to get artifacts: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            if status == reqwest::StatusCode::NOT_FOUND {
                bail!("Task '{}' not found", task_id);
            }
            bail!("Artifacts request failed with status: {}", status);
        }

        let artifacts: Vec<TaskArtifact> = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse artifacts: {}", e))?;

        Ok(artifacts)
    }

    /// Cancel a task
    pub async fn cancel_task(&self, task_id: &str) -> Result<Task> {
        let url = format!("{}/tasks/{}/cancel", self.base_url, task_id);

        debug!("Canceling task: {}", task_id);

        let response = self.build_request(reqwest::Method::POST, &url)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to cancel task: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            if status == reqwest::StatusCode::NOT_FOUND {
                bail!("Task '{}' not found", task_id);
            }
            bail!("Task cancel failed with status: {}", status);
        }

        let task: Task = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse task: {}", e))?;

        info!("Canceled task: {}", task.id);
        Ok(task)
    }

    /// Submit input to a task awaiting user input
    pub async fn submit_input(&self, task_id: &str, message: Message) -> Result<TaskSendResponse> {
        let url = format!("{}/tasks/{}/input", self.base_url, task_id);

        let request = TaskSendRequest {
            message,
            stream: None,
        };

        debug!("Submitting input to task: {}", task_id);

        let response = self.build_request(reqwest::Method::POST, &url)
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to submit input: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let error: Result<ErrorResponse, _> = response.json().await;
            match error {
                Ok(e) => bail!("Input submission failed: {}", e.message),
                Err(_) => bail!("Input submission failed with status: {}", status),
            }
        }

        let result: TaskSendResponse = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse response: {}", e))?;

        Ok(result)
    }

    /// Get the base URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

/// Builder for creating Messages
pub struct MessageBuilder {
    role: MessageRole,
    content: Vec<MessagePart>,
    metadata: Option<serde_json::Value>,
}

impl MessageBuilder {
    pub fn new(role: MessageRole) -> Self {
        Self {
            role,
            content: Vec::new(),
            metadata: None,
        }
    }

    pub fn user() -> Self {
        Self::new(MessageRole::User)
    }

    pub fn agent() -> Self {
        Self::new(MessageRole::Agent)
    }

    pub fn system() -> Self {
        Self::new(MessageRole::System)
    }

    pub fn add_text(mut self, text: &str) -> Self {
        self.content.push(MessagePart::Text {
            text: text.to_string(),
        });
        self
    }

    pub fn add_data(mut self, data: serde_json::Value) -> Self {
        self.content.push(MessagePart::Data { data });
        self
    }

    pub fn add_file(mut self, name: &str, content: &str, mime_type: Option<&str>) -> Self {
        self.content.push(MessagePart::File {
            file: FileInfo {
                name: name.to_string(),
                content: content.to_string(),
                mime_type: mime_type.map(|s| s.to_string()),
            },
        });
        self
    }

    pub fn metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn build(self) -> Message {
        Message {
            role: self.role,
            content: self.content,
            metadata: self.metadata,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_message_builder() {
        let message = MessageBuilder::user()
            .add_text("Hello, world!")
            .add_data(json!({"key": "value"}))
            .build();

        assert_eq!(message.role, MessageRole::User);
        assert_eq!(message.content.len(), 2);
    }

    #[test]
    fn test_client_creation() {
        let client = AcpClient::new("http://localhost:8080");
        assert_eq!(client.base_url(), "http://localhost:8080");
        assert!(client.agent_card().is_none());
    }

    #[test]
    fn test_client_with_auth() {
        let client = AcpClient::with_auth("http://localhost:8080", "test-key");
        assert_eq!(client.api_key, Some("test-key".to_string()));
    }
}
