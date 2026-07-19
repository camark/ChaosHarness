//! User feedback system for learning
//!
//! Allows users to provide feedback on responses (thumbs up/down)
//! and uses this to improve knowledge extraction and preferences.

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Feedback rating
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum FeedbackRating {
    /// Positive feedback (thumbs up)
    Positive,
    /// Negative feedback (thumbs down)
    Negative,
    /// Neutral / no feedback
    Neutral,
}

impl FeedbackRating {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Positive => "positive",
            Self::Negative => "negative",
            Self::Neutral => "neutral",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "positive" => Self::Positive,
            "negative" => Self::Negative,
            _ => Self::Neutral,
        }
    }

    /// Convert to numeric score (-1.0 to 1.0)
    pub fn to_score(self) -> f64 {
        match self {
            Self::Positive => 1.0,
            Self::Negative => -1.0,
            Self::Neutral => 0.0,
        }
    }
}

/// A feedback entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackEntry {
    pub id: String,
    pub message_id: Option<String>,
    pub turn_number: usize,
    pub rating: FeedbackRating,
    pub comment: Option<String>,
    pub tool_used: Option<String>,
    pub topic: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Aggregated feedback statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackStats {
    pub total_feedback: usize,
    pub positive_count: usize,
    pub negative_count: usize,
    pub neutral_count: usize,
    pub average_score: f64,
    pub tool_ratings: HashMap<String, f64>,
    pub topic_ratings: HashMap<String, f64>,
}

/// User feedback manager
pub struct FeedbackManager {
    feedbacks: Arc<Mutex<Vec<FeedbackEntry>>>,
    storage_path: Option<PathBuf>,
}

impl FeedbackManager {
    pub fn new() -> Self {
        let storage_path = get_feedback_storage_path();
        let feedbacks = if let Some(ref path) = storage_path {
            load_feedbacks_from_file(path).unwrap_or_default()
        } else {
            Vec::new()
        };

        Self {
            feedbacks: Arc::new(Mutex::new(feedbacks)),
            storage_path,
        }
    }

    /// Record user feedback
    pub async fn record_feedback(
        &self,
        turn_number: usize,
        rating: FeedbackRating,
        comment: Option<String>,
        tool_used: Option<String>,
        topic: Option<String>,
    ) -> String {
        let id = format!("fb_{}", Utc::now().timestamp_millis());
        let entry = FeedbackEntry {
            id: id.clone(),
            message_id: None,
            turn_number,
            rating,
            comment,
            tool_used,
            topic,
            created_at: Utc::now(),
        };

        let mut feedbacks = self.feedbacks.lock().await;
        feedbacks.push(entry);

        // Persist to disk
        if let Some(ref path) = self.storage_path {
            if let Err(e) = save_feedbacks_to_file(path, &feedbacks) {
                tracing::warn!("Failed to save feedback: {}", e);
            }
        }

        id
    }

    /// Get feedback statistics
    pub async fn get_stats(&self) -> FeedbackStats {
        let feedbacks = self.feedbacks.lock().await;
        let total = feedbacks.len();

        if total == 0 {
            return FeedbackStats {
                total_feedback: 0,
                positive_count: 0,
                negative_count: 0,
                neutral_count: 0,
                average_score: 0.0,
                tool_ratings: HashMap::new(),
                topic_ratings: HashMap::new(),
            };
        }

        let mut positive = 0;
        let mut negative = 0;
        let mut neutral = 0;
        let mut score_sum = 0.0;
        let mut tool_scores: HashMap<String, Vec<f64>> = HashMap::new();
        let mut topic_scores: HashMap<String, Vec<f64>> = HashMap::new();

        for entry in feedbacks.iter() {
            match entry.rating {
                FeedbackRating::Positive => positive += 1,
                FeedbackRating::Negative => negative += 1,
                FeedbackRating::Neutral => neutral += 1,
            }
            score_sum += entry.rating.to_score();

            if let Some(ref tool) = entry.tool_used {
                tool_scores.entry(tool.clone())
                    .or_default()
                    .push(entry.rating.to_score());
            }

            if let Some(ref topic) = entry.topic {
                topic_scores.entry(topic.clone())
                    .or_default()
                    .push(entry.rating.to_score());
            }
        }

        let tool_ratings = tool_scores.into_iter()
            .map(|(k, v)| (k, v.iter().sum::<f64>() / v.len() as f64))
            .collect();

        let topic_ratings = topic_scores.into_iter()
            .map(|(k, v)| (k, v.iter().sum::<f64>() / v.len() as f64))
            .collect();

        FeedbackStats {
            total_feedback: total,
            positive_count: positive,
            negative_count: negative,
            neutral_count: neutral,
            average_score: score_sum / total as f64,
            tool_ratings,
            topic_ratings,
        }
    }

    /// Get recent feedback entries
    pub async fn get_recent(&self, limit: usize) -> Vec<FeedbackEntry> {
        let feedbacks = self.feedbacks.lock().await;
        feedbacks.iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get feedback count
    pub async fn count(&self) -> usize {
        self.feedbacks.lock().await.len()
    }
}

impl Default for FeedbackManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Get the path to the feedback storage file
fn get_feedback_storage_path() -> Option<PathBuf> {
    let config_dir = crate::config::get_config_dir();
    Some(config_dir.join("feedback.json"))
}

/// Load feedbacks from a JSON file
fn load_feedbacks_from_file(path: &PathBuf) -> Option<Vec<FeedbackEntry>> {
    if !path.exists() {
        return None;
    }

    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Save feedbacks to a JSON file
fn save_feedbacks_to_file(path: &PathBuf, feedbacks: &[FeedbackEntry]) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_string_pretty(feedbacks)?;
    std::fs::write(path, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_record_feedback() {
        let manager = FeedbackManager {
            feedbacks: Arc::new(Mutex::new(Vec::new())),
            storage_path: None,
        };

        let id = manager.record_feedback(
            1,
            FeedbackRating::Positive,
            Some("Great response!".to_string()),
            Some("bash".to_string()),
            Some("coding".to_string()),
        ).await;

        assert!(!id.is_empty());
        assert_eq!(manager.count().await, 1);
    }

    #[tokio::test]
    async fn test_feedback_stats() {
        let manager = FeedbackManager {
            feedbacks: Arc::new(Mutex::new(Vec::new())),
            storage_path: None,
        };

        manager.record_feedback(1, FeedbackRating::Positive, None, Some("bash".to_string()), None).await;
        manager.record_feedback(2, FeedbackRating::Negative, None, Some("bash".to_string()), None).await;
        manager.record_feedback(3, FeedbackRating::Positive, None, Some("read_file".to_string()), None).await;

        let stats = manager.get_stats().await;
        assert_eq!(stats.total_feedback, 3);
        assert_eq!(stats.positive_count, 2);
        assert_eq!(stats.negative_count, 1);
        assert!(stats.tool_ratings.contains_key("bash"));
    }

    #[tokio::test]
    async fn test_get_recent() {
        let manager = FeedbackManager {
            feedbacks: Arc::new(Mutex::new(Vec::new())),
            storage_path: None,
        };

        for i in 0..5 {
            manager.record_feedback(i, FeedbackRating::Positive, None, None, None).await;
        }

        let recent = manager.get_recent(3).await;
        assert_eq!(recent.len(), 3);
    }
}
