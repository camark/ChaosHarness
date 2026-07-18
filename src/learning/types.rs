//! Types for the self-learning system

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KnowledgeCategory {
    Fact,
    Decision,
    Solution,
    Preference,
}

impl KnowledgeCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Fact => "fact",
            Self::Decision => "decision",
            Self::Solution => "solution",
            Self::Preference => "preference",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "fact" => Self::Fact,
            "decision" => Self::Decision,
            "solution" => Self::Solution,
            "preference" => Self::Preference,
            _ => Self::Fact,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternType {
    CodingStyle,
    Workflow,
    ToolPreference,
}

impl PatternType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CodingStyle => "coding_style",
            Self::Workflow => "workflow",
            Self::ToolPreference => "tool_preference",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "coding_style" => Self::CodingStyle,
            "workflow" => Self::Workflow,
            "tool_preference" => Self::ToolPreference,
            _ => Self::Workflow,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEntry {
    pub id: Option<i64>,
    pub category: KnowledgeCategory,
    pub topic: String,
    pub content: String,
    pub source_session_id: Option<String>,
    pub confidence: f64,
    pub access_count: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub last_accessed: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub id: Option<i64>,
    pub pattern_type: PatternType,
    pub description: String,
    pub example: Option<String>,
    pub frequency: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub last_seen: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSummary {
    pub id: Option<i64>,
    pub session_id: String,
    pub summary: String,
    pub message_range_start: i64,
    pub message_range_end: i64,
    pub tokens_saved: i64,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub enum RetrievedContext {
    Summary {
        text: String,
        score: f64,
    },
    Knowledge {
        entry: KnowledgeEntry,
        score: f64,
    },
    Pattern {
        pattern: Pattern,
        score: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningMarker {
    pub category: String,
    pub topic: String,
    pub content: String,
}

#[derive(Debug, Default)]
pub struct LearningResult {
    pub knowledge_extracted: usize,
    pub patterns_extracted: usize,
    pub skills_generated: usize,
}
