//! AgentCard implementation
//!
//! The AgentCard is the core metadata document that describes an agent's
//! capabilities, endpoints, and other information needed for discovery
//! and interoperability.

use crate::acp::types::*;
use serde_json::json;

/// Builder for creating AgentCard instances
pub struct AgentCardBuilder {
    name: String,
    description: String,
    version: Option<String>,
    provider: Option<ProviderInfo>,
    capabilities: Option<AgentCapabilities>,
    authentication: Option<AuthenticationInfo>,
    skills: Vec<Skill>,
    endpoint: String,
    website: Option<String>,
    contact: Option<ContactInfo>,
    metadata: Option<serde_json::Value>,
    supported_languages: Vec<String>,
}

impl AgentCardBuilder {
    pub fn new(name: &str, description: &str, endpoint: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            version: None,
            provider: None,
            capabilities: None,
            authentication: None,
            skills: Vec::new(),
            endpoint: endpoint.to_string(),
            website: None,
            contact: None,
            metadata: None,
            supported_languages: vec!["en-US".to_string()],
        }
    }

    pub fn version(mut self, version: &str) -> Self {
        self.version = Some(version.to_string());
        self
    }

    pub fn provider(mut self, org: &str, url: Option<&str>) -> Self {
        self.provider = Some(ProviderInfo {
            organization: org.to_string(),
            url: url.map(|s| s.to_string()),
        });
        self
    }

    pub fn capabilities(mut self, caps: AgentCapabilities) -> Self {
        self.capabilities = Some(caps);
        self
    }

    pub fn add_skill(mut self, skill: Skill) -> Self {
        self.skills.push(skill);
        self
    }

    pub fn website(mut self, url: &str) -> Self {
        self.website = Some(url.to_string());
        self
    }

    pub fn contact(mut self, contact: ContactInfo) -> Self {
        self.contact = Some(contact);
        self
    }

    pub fn metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn supported_languages(mut self, langs: Vec<String>) -> Self {
        self.supported_languages = langs;
        self
    }

    pub fn skills(mut self, skills: Vec<Skill>) -> Self {
        self.skills = skills;
        self
    }

    pub fn build(self) -> AgentCard {
        AgentCard {
            name: self.name,
            description: self.description,
            version: self.version,
            provider: self.provider,
            capabilities: self.capabilities,
            authentication: self.authentication,
            input_modalities: Some(vec![Modality::Text, Modality::Code, Modality::File]),
            output_modalities: Some(vec![Modality::Text, Modality::Code, Modality::File]),
            skills: self.skills,
            default_language: Some("en-US".to_string()),
            supported_languages: self.supported_languages,
            endpoint: self.endpoint,
            website: self.website,
            contact: self.contact,
            metadata: self.metadata,
        }
    }
}

impl AgentCard {
    /// Create an AgentCard for RustHarness
    pub fn for_rust_harness(base_url: &str) -> Self {
        let endpoint = format!("{}/acp", base_url.trim_end_matches('/'));

        // Build skills from available tools
        let skills = vec![
            Skill {
                id: "bash".to_string(),
                name: "Bash".to_string(),
                description: Some("Execute shell commands".to_string()),
                category: Some("system".to_string()),
                tags: vec!["shell".to_string(), "command".to_string(), "execute".to_string()],
                input_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "command": {"type": "string", "description": "The command to execute"},
                        "timeout": {"type": "number", "description": "Timeout in seconds"}
                    },
                    "required": ["command"]
                })),
                output_schema: None,
                requires_confirmation: Some(true),
            },
            Skill {
                id: "read_file".to_string(),
                name: "Read File".to_string(),
                description: Some("Read content of text files".to_string()),
                category: Some("file_io".to_string()),
                tags: vec!["file".to_string(), "read".to_string()],
                input_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "File path to read"},
                        "offset": {"type": "number", "description": "Line offset"},
                        "limit": {"type": "number", "description": "Max lines to read"}
                    },
                    "required": ["path"]
                })),
                output_schema: None,
                requires_confirmation: None,
            },
            Skill {
                id: "write_file".to_string(),
                name: "Write File".to_string(),
                description: Some("Create or overwrite files".to_string()),
                category: Some("file_io".to_string()),
                tags: vec!["file".to_string(), "write".to_string(), "create".to_string()],
                input_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "File path"},
                        "content": {"type": "string", "description": "File content"}
                    },
                    "required": ["path", "content"]
                })),
                output_schema: None,
                requires_confirmation: Some(true),
            },
            Skill {
                id: "edit_file".to_string(),
                name: "Edit File".to_string(),
                description: Some("Replace text in existing files".to_string()),
                category: Some("file_io".to_string()),
                tags: vec!["file".to_string(), "edit".to_string(), "modify".to_string()],
                input_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "File path"},
                        "old_string": {"type": "string", "description": "Text to replace"},
                        "new_string": {"type": "string", "description": "Replacement text"},
                        "replace_all": {"type": "boolean", "description": "Replace all occurrences"}
                    },
                    "required": ["path", "old_string", "new_string"]
                })),
                output_schema: None,
                requires_confirmation: Some(true),
            },
            Skill {
                id: "glob".to_string(),
                name: "Glob".to_string(),
                description: Some("List files matching glob patterns".to_string()),
                category: Some("file_io".to_string()),
                tags: vec!["file".to_string(), "search".to_string(), "pattern".to_string()],
                input_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "pattern": {"type": "string", "description": "Glob pattern"},
                        "path": {"type": "string", "description": "Search path"}
                    },
                    "required": ["pattern"]
                })),
                output_schema: None,
                requires_confirmation: None,
            },
            Skill {
                id: "grep".to_string(),
                name: "Grep".to_string(),
                description: Some("Search file contents with regex".to_string()),
                category: Some("file_io".to_string()),
                tags: vec!["file".to_string(), "search".to_string(), "regex".to_string()],
                input_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "pattern": {"type": "string", "description": "Regex pattern"},
                        "path": {"type": "string", "description": "File or directory to search"},
                        "case_sensitive": {"type": "boolean", "description": "Case sensitive search"}
                    },
                    "required": ["pattern"]
                })),
                output_schema: None,
                requires_confirmation: None,
            },
            Skill {
                id: "web_fetch".to_string(),
                name: "Web Fetch".to_string(),
                description: Some("Fetch content from URLs".to_string()),
                category: Some("web".to_string()),
                tags: vec!["web".to_string(), "fetch".to_string(), "http".to_string()],
                input_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "url": {"type": "string", "description": "URL to fetch"}
                    },
                    "required": ["url"]
                })),
                output_schema: None,
                requires_confirmation: None,
            },
            Skill {
                id: "web_search".to_string(),
                name: "Web Search".to_string(),
                description: Some("Search the web".to_string()),
                category: Some("web".to_string()),
                tags: vec!["web".to_string(), "search".to_string()],
                input_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string", "description": "Search query"}
                    },
                    "required": ["query"]
                })),
                output_schema: None,
                requires_confirmation: None,
            },
            Skill {
                id: "ask_user".to_string(),
                name: "Ask User".to_string(),
                description: Some("Interactive user prompts".to_string()),
                category: Some("interaction".to_string()),
                tags: vec!["user".to_string(), "prompt".to_string(), "interactive".to_string()],
                input_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "question": {"type": "string", "description": "Question to ask"},
                        "options": {"type": "array", "items": {"type": "string"}, "description": "Optional choices"}
                    },
                    "required": ["question"]
                })),
                output_schema: None,
                requires_confirmation: None,
            },
        ];

        AgentCardBuilder::new(
            "RustHarness",
            "AI-powered coding assistant harness with tool-use, skills, memory, and multi-agent coordination",
            &endpoint,
        )
        .version(env!("CARGO_PKG_VERSION"))
        .provider("Open Source", Some("https://github.com/RustHarnetss"))
        .capabilities(AgentCapabilities {
            streaming: Some(false),
            tool_use: Some(true),
            memory: Some(true),
            multi_turn: Some(true),
            file_io: Some(true),
            web_access: Some(true),
            code_execution: Some(true),
            vision: Some(false),
        })
        .skills(skills)
        .supported_languages(vec!["en-US".to_string(), "zh-CN".to_string()])
        .website("https://github.com/RustHarnetss")
        .contact(ContactInfo {
            name: Some("RustHarness Team".to_string()),
            email: None,
            url: Some("https://github.com/RustHarnetss".to_string()),
        })
        .build()
    }

    /// Serialize AgentCard to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Get the agent endpoint URL
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Find a skill by ID
    pub fn find_skill(&self, skill_id: &str) -> Option<&Skill> {
        self.skills.iter().find(|s| s.id == skill_id)
    }

    /// Check if the agent has a specific capability
    pub fn has_capability(&self, capability: &str) -> bool {
        match self.capabilities {
            Some(ref caps) => match capability {
                "streaming" => caps.streaming.unwrap_or(false),
                "tool_use" => caps.tool_use.unwrap_or(false),
                "memory" => caps.memory.unwrap_or(false),
                "multi_turn" => caps.multi_turn.unwrap_or(false),
                "file_io" => caps.file_io.unwrap_or(false),
                "web_access" => caps.web_access.unwrap_or(false),
                "code_execution" => caps.code_execution.unwrap_or(false),
                "vision" => caps.vision.unwrap_or(false),
                _ => false,
            },
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_card_builder() {
        let card = AgentCardBuilder::new("TestAgent", "A test agent", "http://localhost:8080")
            .version("1.0.0")
            .provider("Test Org", Some("https://test.org"))
            .build();

        assert_eq!(card.name, "TestAgent");
        assert_eq!(card.version, Some("1.0.0".to_string()));
        assert!(card.provider.is_some());
    }

    #[test]
    fn test_agent_card_serialization() {
        let card = AgentCard::for_rust_harness("http://localhost:3000");
        let json = card.to_json().expect("Failed to serialize");

        assert!(json.contains("RustHarness"));
        assert!(json.contains("endpoint"));
    }
}
