//! Cron scheduler service

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
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

lazy_static::lazy_static! {
    /// Global cron manager instance
    pub static ref CRON_MANAGER: CronManager = CronManager::new();
}

/// Cron manager for managing scheduled jobs
pub struct CronManager {
    jobs: Arc<Mutex<HashMap<String, CronJob>>>,
    storage_path: Option<PathBuf>,
}

impl CronManager {
    pub fn new() -> Self {
        let storage_path = get_cron_storage_path();
        let jobs = if let Some(ref path) = storage_path {
            load_jobs_from_file(path).unwrap_or_default()
        } else {
            HashMap::new()
        };

        Self {
            jobs: Arc::new(Mutex::new(jobs)),
            storage_path,
        }
    }

    /// Create a new in-memory cron manager (for testing)
    #[allow(dead_code)]
    pub fn new_in_memory() -> Self {
        Self {
            jobs: Arc::new(Mutex::new(HashMap::new())),
            storage_path: None,
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

        // Persist to disk
        if let Some(ref path) = self.storage_path {
            let _ = save_jobs_to_file(path, &jobs);
        }

        true
    }

    /// Delete a cron job
    pub async fn delete_job(&self, name: &str) -> bool {
        let mut jobs = self.jobs.lock().await;
        let result = jobs.remove(name).is_some();

        if result {
            // Persist to disk
            if let Some(ref path) = self.storage_path {
                let _ = save_jobs_to_file(path, &jobs);
            }
        }

        result
    }

    /// Toggle job enabled status
    pub async fn set_job_enabled(&self, name: &str, enabled: bool) -> bool {
        let mut jobs = self.jobs.lock().await;
        if let Some(job) = jobs.get_mut(name) {
            job.enabled = enabled;

            // Persist to disk
            if let Some(ref path) = self.storage_path {
                let _ = save_jobs_to_file(path, &jobs);
            }

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

/// Get the path to the cron jobs storage file
fn get_cron_storage_path() -> Option<PathBuf> {
    let config_dir = crate::config::get_config_dir();
    Some(config_dir.join("cron_jobs.json"))
}

/// Load jobs from a JSON file
fn load_jobs_from_file(path: &PathBuf) -> Option<HashMap<String, CronJob>> {
    if !path.exists() {
        return None;
    }

    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Save jobs to a JSON file
fn save_jobs_to_file(path: &PathBuf, jobs: &HashMap<String, CronJob>) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let content = serde_json::to_string_pretty(jobs).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())?;
    Ok(())
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
        let manager = CronManager::new_in_memory();
        let created = manager.create_job("test", "*/5 * * * *", "echo hi").await;
        assert!(created);

        let job = manager.get_job("test").await.unwrap();
        assert_eq!(job.name, "test");
        assert_eq!(job.schedule, "*/5 * * * *");
        assert!(job.enabled);
    }

    #[tokio::test]
    async fn test_create_duplicate_job() {
        let manager = CronManager::new_in_memory();
        manager.create_job("test", "*/5 * * * *", "echo hi").await;
        let created = manager.create_job("test", "*/10 * * * *", "echo bye").await;
        assert!(!created);
    }

    #[tokio::test]
    async fn test_delete_job() {
        let manager = CronManager::new_in_memory();
        manager.create_job("test", "*/5 * * * *", "echo hi").await;

        let deleted = manager.delete_job("test").await;
        assert!(deleted);

        let job = manager.get_job("test").await;
        assert!(job.is_none());
    }

    #[tokio::test]
    async fn test_delete_nonexistent_job() {
        let manager = CronManager::new_in_memory();
        let deleted = manager.delete_job("nonexistent").await;
        assert!(!deleted);
    }

    #[tokio::test]
    async fn test_toggle_job() {
        let manager = CronManager::new_in_memory();
        manager.create_job("test", "*/5 * * * *", "echo hi").await;

        let toggled = manager.set_job_enabled("test", false).await;
        assert!(toggled);

        let job = manager.get_job("test").await.unwrap();
        assert!(!job.enabled);
    }

    #[tokio::test]
    async fn test_toggle_nonexistent_job() {
        let manager = CronManager::new_in_memory();
        let toggled = manager.set_job_enabled("nonexistent", true).await;
        assert!(!toggled);
    }

    #[tokio::test]
    async fn test_list_jobs() {
        let manager = CronManager::new_in_memory();
        manager.create_job("job1", "*/5 * * * *", "echo 1").await;
        manager.create_job("job2", "*/10 * * * *", "echo 2").await;

        let jobs = manager.list_jobs().await;
        assert_eq!(jobs.len(), 2);
    }

    #[tokio::test]
    async fn test_list_empty() {
        let manager = CronManager::new_in_memory();
        let jobs = manager.list_jobs().await;
        assert!(jobs.is_empty());
    }
}
