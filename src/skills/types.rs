//! Skill types

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub content: String,
    pub source: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_creation() {
        let skill = Skill {
            name: "test".to_string(),
            description: "A test skill".to_string(),
            content: "# Test".to_string(),
            source: "local".to_string(),
        };
        assert_eq!(skill.name, "test");
        assert_eq!(skill.description, "A test skill");
        assert_eq!(skill.source, "local");
    }

    #[test]
    fn test_skill_clone() {
        let skill = Skill {
            name: "test".to_string(),
            description: "desc".to_string(),
            content: "content".to_string(),
            source: "local".to_string(),
        };
        let cloned = skill.clone();
        assert_eq!(cloned.name, skill.name);
        assert_eq!(cloned.content, skill.content);
    }

    #[test]
    fn test_skill_serialization() {
        let skill = Skill {
            name: "test".to_string(),
            description: "desc".to_string(),
            content: "content".to_string(),
            source: "local".to_string(),
        };
        let json = serde_json::to_string(&skill).unwrap();
        assert!(json.contains("\"name\":\"test\""));
        assert!(json.contains("\"source\":\"local\""));
    }

    #[test]
    fn test_skill_deserialization() {
        let json = "{\"name\":\"my_skill\",\"description\":\"A skill\",\"content\":\"# Hello\",\"source\":\"github\"}";
        let skill: Skill = serde_json::from_str(json).unwrap();
        assert_eq!(skill.name, "my_skill");
        assert_eq!(skill.source, "github");
    }
}
