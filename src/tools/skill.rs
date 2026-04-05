//! Skill tool - loads and returns skill content

use crate::tools::base::{Tool, ToolResult, ToolExecutionContext};
use crate::skills::loader::load_skill_registry;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
struct SkillInput {
    /// Name of the skill to load
    name: String,
}

pub struct SkillTool {
    cwd: PathBuf,
}

impl SkillTool {
    pub fn new(cwd: PathBuf) -> Self {
        Self { cwd }
    }
}

#[async_trait::async_trait]
impl Tool for SkillTool {
    fn name(&self) -> &'static str {
        "skill"
    }

    fn description(&self) -> &'static str {
        "Load a skill's content by name. Use this when the user's request matches a skill's description."
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Name of the skill to load"
                }
            },
            "required": ["name"]
        })
    }

    fn is_read_only(&self, _input: &Value) -> bool {
        true
    }

    async fn execute(&self, input: Value, _context: ToolExecutionContext) -> Result<ToolResult> {
        let input: SkillInput = match serde_json::from_value(input) {
            Ok(i) => i,
            Err(e) => {
                return Ok(ToolResult {
                    output: format!("Invalid input: {}", e),
                    is_error: true,
                    metadata: serde_json::Map::new().into_iter().collect(),
                });
            }
        };

        let registry = load_skill_registry(&self.cwd);

        match registry.get(&input.name) {
            Some(skill) => {
                Ok(ToolResult {
                    output: format!("Skill '{}':\n\n{}", skill.name, skill.content),
                    is_error: false,
                    metadata: serde_json::Map::new().into_iter().collect(),
                })
            }
            None => {
                let available = registry.list()
                    .iter()
                    .map(|s| s.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");

                Ok(ToolResult {
                    output: format!("Skill '{}' not found. Available skills: {}", input.name, available),
                    is_error: true,
                    metadata: serde_json::Map::new().into_iter().collect(),
                })
            }
        }
    }
}
