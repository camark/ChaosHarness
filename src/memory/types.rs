//! Memory types

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub name: String,
    pub description: String,
    pub content: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_entry_creation() {
        let now = chrono::Utc::now();
        let entry = MemoryEntry {
            name: "test".to_string(),
            description: "A test entry".to_string(),
            content: "Hello".to_string(),
            created_at: now,
            updated_at: now,
        };
        assert_eq!(entry.name, "test");
        assert_eq!(entry.description, "A test entry");
        assert_eq!(entry.content, "Hello");
    }

    #[test]
    fn test_memory_entry_clone() {
        let now = chrono::Utc::now();
        let entry = MemoryEntry {
            name: "test".to_string(),
            description: "desc".to_string(),
            content: "content".to_string(),
            created_at: now,
            updated_at: now,
        };
        let cloned = entry.clone();
        assert_eq!(cloned.name, entry.name);
        assert_eq!(cloned.created_at, entry.created_at);
    }

    #[test]
    fn test_memory_entry_serialization() {
        let now = chrono::Utc::now();
        let entry = MemoryEntry {
            name: "test".to_string(),
            description: "desc".to_string(),
            content: "content".to_string(),
            created_at: now,
            updated_at: now,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"name\":\"test\""));
        assert!(json.contains("\"content\":\"content\""));
    }

    #[test]
    fn test_memory_entry_deserialization() {
        let now = chrono::Utc::now();
        let entry = MemoryEntry {
            name: "test".to_string(),
            description: "desc".to_string(),
            content: "content".to_string(),
            created_at: now,
            updated_at: now,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: MemoryEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "test");
        assert_eq!(deserialized.content, "content");
    }

    #[test]
    fn test_memory_entry_debug() {
        let now = chrono::Utc::now();
        let entry = MemoryEntry {
            name: "test".to_string(),
            description: "desc".to_string(),
            content: "content".to_string(),
            created_at: now,
            updated_at: now,
        };
        let debug = format!("{:?}", entry);
        assert!(debug.contains("MemoryEntry"));
        assert!(debug.contains("test"));
    }
}
