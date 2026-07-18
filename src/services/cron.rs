//! Cron scheduler service

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJob {
    pub name: String,
    pub schedule: String,
    pub command: String,
    pub enabled: bool,
}

/// Global cron manager instance
lazy_static::lazy_static! {
    pub static ref CRON_MANAGER: CronManager = CronManager::new();
}

/// Cron manager for managing scheduled jobs
pub struct CronManager {
    jobs: Arc<Mutex<HashMap<String, CronJob>>>,
}

impl CronManager {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create a new cron job
    pub async fn create_job(&self, name: &str, schedule: &str, command: &str) -> bool {
        let mut jobs = self.jobs.lock().await;
        if jobs.contains_key(name) {
            return false; // Job already exists
        }

        jobs.insert(name.to_string(), CronJob {
            name: name.to_string(),
            schedule: schedule.to_string(),
            command: command.to_string(),
            enabled: true,
        });
        true
    }

    /// Delete a cron job
    pub async fn delete_job(&self, name: &str) -> bool {
        let mut jobs = self.jobs.lock().await;
        jobs.remove(name).is_some()
    }

    /// Toggle job enabled status
    pub async fn set_job_enabled(&self, name: &str, enabled: bool) -> bool {
        let mut jobs = self.jobs.lock().await;
        if let Some(job) = jobs.get_mut(name) {
            job.enabled = enabled;
            true
        } else {
            false
        }
    }

    /// Get a job by name
    pub async fn get_job(&self, name: &str) -> Option<CronJob> {
        let jobs = self.jobs.lock().await;
        jobs.get(name).cloned()
    }

    /// List all jobs
    pub async fn list_jobs(&self) -> Vec<CronJob> {
        let jobs = self.jobs.lock().await;
        jobs.values().cloned().collect()
    }
}

impl Default for CronManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_and_get_job() {
        let manager = CronManager::new();
        let created = manager.create_job("test", "*/5 * * * *", "echo hi").await;
        assert!(created);

        let job = manager.get_job("test").await.unwrap();
        assert_eq!(job.name, "test");
        assert_eq!(job.schedule, "*/5 * * * *");
        assert!(job.enabled);
    }

    #[tokio::test]
    async fn test_create_duplicate_job() {
        let manager = CronManager::new();
        manager.create_job("test", "*/5 * * * *", "echo hi").await;
        let created = manager.create_job("test", "*/10 * * * *", "echo bye").await;
        assert!(!created);
    }

    #[tokio::test]
    async fn test_delete_job() {
        let manager = CronManager::new();
        manager.create_job("test", "*/5 * * * *", "echo hi").await;

        let deleted = manager.delete_job("test").await;
        assert!(deleted);

        let job = manager.get_job("test").await;
        assert!(job.is_none());
    }

    #[tokio::test]
    async fn test_delete_nonexistent_job() {
        let manager = CronManager::new();
        let deleted = manager.delete_job("nonexistent").await;
        assert!(!deleted);
    }

    #[tokio::test]
    async fn test_toggle_job() {
        let manager = CronManager::new();
        manager.create_job("test", "*/5 * * * *", "echo hi").await;

        let toggled = manager.set_job_enabled("test", false).await;
        assert!(toggled);

        let job = manager.get_job("test").await.unwrap();
        assert!(!job.enabled);
    }

    #[tokio::test]
    async fn test_toggle_nonexistent_job() {
        let manager = CronManager::new();
        let toggled = manager.set_job_enabled("nonexistent", true).await;
        assert!(!toggled);
    }

    #[tokio::test]
    async fn test_list_jobs() {
        let manager = CronManager::new();
        manager.create_job("job1", "*/5 * * * *", "echo 1").await;
        manager.create_job("job2", "*/10 * * * *", "echo 2").await;

        let jobs = manager.list_jobs().await;
        assert_eq!(jobs.len(), 2);
    }

    #[tokio::test]
    async fn test_list_empty() {
        let manager = CronManager::new();
        let jobs = manager.list_jobs().await;
        assert!(jobs.is_empty());
    }
}
