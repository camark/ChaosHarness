//! Background task manager

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Global task manager instance
lazy_static::lazy_static! {
    pub static ref GLOBAL_TASK_MANAGER: TaskManager = TaskManager::new();
}

/// Task status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Running,
    Completed,
    Failed,
    Stopped,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Stopped => "stopped",
        }
    }
}

/// Task type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    Bash,
    Agent,
}

impl TaskType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Bash => "local_bash",
            Self::Agent => "local_agent",
        }
    }
}

/// Background task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub task_type: TaskType,
    pub description: String,
    pub status: TaskStatus,
    pub command: Option<String>,
    pub prompt: Option<String>,
    pub model: Option<String>,
    pub output: String,
    pub exit_code: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Task manager for background tasks
pub struct TaskManager {
    tasks: Arc<Mutex<HashMap<String, Task>>>,
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create a bash task
    pub async fn create_bash_task(&self, description: &str, command: &str) -> String {
        let id = format!("task-{}", uuid::Uuid::new_v4().to_string()[..8].to_string());
        let now = Utc::now();

        // Spawn the background process
        let task_id = id.clone();
        let cmd = command.to_string();
        let tasks = self.tasks.clone();

        // Create task entry first
        let task = Task {
            id: id.clone(),
            task_type: TaskType::Bash,
            description: description.to_string(),
            status: TaskStatus::Running,
            command: Some(command.to_string()),
            prompt: None,
            model: None,
            output: String::new(),
            exit_code: None,
            created_at: now,
            updated_at: now,
            completed_at: None,
        };

        self.tasks.lock().await.insert(id.clone(), task);

        // Spawn background process
        tokio::spawn(async move {
            let result = tokio::process::Command::new("sh")
                .arg("-c")
                .arg(&cmd)
                .output()
                .await;

            let mut tasks = tasks.lock().await;
            if let Some(task) = tasks.get_mut(&task_id) {
                match result {
                    Ok(output) => {
                        task.output = String::from_utf8_lossy(&output.stdout).to_string();
                        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                        if !stderr.is_empty() {
                            task.output.push_str("\n--- STDERR ---\n");
                            task.output.push_str(&stderr);
                        }
                        task.exit_code = output.status.code();
                        task.status = if output.status.success() {
                            TaskStatus::Completed
                        } else {
                            TaskStatus::Failed
                        };
                    }
                    Err(e) => {
                        task.output = format!("Failed to execute: {}", e);
                        task.status = TaskStatus::Failed;
                        task.exit_code = Some(-1);
                    }
                }
                task.completed_at = Some(Utc::now());
                task.updated_at = Utc::now();
            }
        });

        id
    }

    /// Create an agent task (simplified - just stores the prompt)
    pub async fn create_agent_task(&self, description: &str, prompt: &str, model: &str) -> String {
        let id = format!("task-{}", uuid::Uuid::new_v4().to_string()[..8].to_string());
        let now = Utc::now();

        let task = Task {
            id: id.clone(),
            task_type: TaskType::Agent,
            description: description.to_string(),
            status: TaskStatus::Running,
            command: None,
            prompt: Some(prompt.to_string()),
            model: Some(model.to_string()),
            output: String::new(),
            exit_code: None,
            created_at: now,
            updated_at: now,
            completed_at: None,
        };

        self.tasks.lock().await.insert(id.clone(), task);
        id
    }

    /// List tasks with optional status filter
    pub async fn list_tasks(&self, status_filter: Option<&str>) -> Vec<Task> {
        let tasks = self.tasks.lock().await;
        tasks.values()
            .filter(|t| {
                if let Some(filter) = status_filter {
                    t.status.as_str() == filter
                } else {
                    true
                }
            })
            .cloned()
            .collect()
    }

    /// Get a task by ID
    pub async fn get_task(&self, task_id: &str) -> Option<Task> {
        let tasks = self.tasks.lock().await;
        tasks.get(task_id).cloned()
    }

    /// Get task output with optional byte limit
    pub async fn get_task_output(&self, task_id: &str, max_bytes: usize) -> Option<String> {
        let tasks = self.tasks.lock().await;
        tasks.get(task_id).map(|t| {
            let output = &t.output;
            if output.len() > max_bytes {
                format!("...(truncated)...\n{}", &output[output.len() - max_bytes..])
            } else {
                output.clone()
            }
        })
    }

    /// Update task status
    pub async fn update_task_status(&self, task_id: &str, status: TaskStatus) -> bool {
        let mut tasks = self.tasks.lock().await;
        if let Some(task) = tasks.get_mut(task_id) {
            task.status = status.clone();
            task.updated_at = Utc::now();
            if status == TaskStatus::Completed || status == TaskStatus::Failed || status == TaskStatus::Stopped {
                task.completed_at = Some(Utc::now());
            }
            true
        } else {
            false
        }
    }

    /// Stop a running task
    pub async fn stop_task(&self, task_id: &str) -> bool {
        let mut tasks = self.tasks.lock().await;
        if let Some(task) = tasks.get_mut(task_id) {
            if task.status == TaskStatus::Running {
                task.status = TaskStatus::Stopped;
                task.updated_at = Utc::now();
                task.completed_at = Some(Utc::now());
                task.output.push_str("\n[Task stopped by user]");
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Delete a task
    pub async fn delete_task(&self, task_id: &str) -> bool {
        let mut tasks = self.tasks.lock().await;
        tasks.remove(task_id).is_some()
    }
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_and_list_bash_task() {
        let manager = TaskManager::new();
        let id = manager.create_bash_task("test", "echo hello").await;

        let tasks = manager.list_tasks(None).await;
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, id);
        assert_eq!(tasks[0].description, "test");
        assert_eq!(tasks[0].status, TaskStatus::Running);
    }

    #[tokio::test]
    async fn test_bash_task_execution() {
        let manager = TaskManager::new();
        let id = manager.create_bash_task("echo test", "echo hello world").await;

        // Wait for task to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let task = manager.get_task(&id).await.unwrap();
        assert_eq!(task.status, TaskStatus::Completed);
        assert!(task.output.contains("hello world"));
        assert_eq!(task.exit_code, Some(0));
    }

    #[tokio::test]
    async fn test_bash_task_failure() {
        let manager = TaskManager::new();
        let id = manager.create_bash_task("fail", "exit 1").await;

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let task = manager.get_task(&id).await.unwrap();
        assert_eq!(task.status, TaskStatus::Failed);
        assert_eq!(task.exit_code, Some(1));
    }

    #[tokio::test]
    async fn test_create_agent_task() {
        let manager = TaskManager::new();
        let id = manager.create_agent_task("analyze", "do something", "test-model").await;

        let task = manager.get_task(&id).await.unwrap();
        assert_eq!(task.task_type.as_str(), "local_agent");
        assert_eq!(task.prompt, Some("do something".to_string()));
    }

    #[tokio::test]
    async fn test_list_with_status_filter() {
        let manager = TaskManager::new();
        manager.create_bash_task("task1", "echo 1").await;
        let _id2 = manager.create_bash_task("task2", "echo 2").await;

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let running = manager.list_tasks(Some("running")).await;
        let completed = manager.list_tasks(Some("completed")).await;

        assert_eq!(running.len() + completed.len(), 2);
    }

    #[tokio::test]
    async fn test_get_task_output() {
        let manager = TaskManager::new();
        let id = manager.create_bash_task("output test", "echo test_output_12345").await;

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let output = manager.get_task_output(&id, 1000).await.unwrap();
        assert!(output.contains("test_output_12345"));
    }

    #[tokio::test]
    async fn test_get_task_output_truncated() {
        let manager = TaskManager::new();
        let id = manager.create_bash_task("long output", "for i in $(seq 1 1000); do echo line_$i; done").await;

        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

        let output = manager.get_task_output(&id, 100).await.unwrap();
        assert!(output.len() <= 200); // Some overhead for truncation message
    }

    #[tokio::test]
    async fn test_stop_task() {
        let manager = TaskManager::new();
        let id = manager.create_bash_task("long task", "sleep 100").await;

        let stopped = manager.stop_task(&id).await;
        assert!(stopped);

        let task = manager.get_task(&id).await.unwrap();
        assert_eq!(task.status, TaskStatus::Stopped);
    }

    #[tokio::test]
    async fn test_stop_completed_task() {
        let manager = TaskManager::new();
        let id = manager.create_bash_task("quick", "echo done").await;

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let stopped = manager.stop_task(&id).await;
        assert!(!stopped); // Can't stop a completed task
    }

    #[tokio::test]
    async fn test_update_task_status() {
        let manager = TaskManager::new();
        let id = manager.create_bash_task("test", "echo hi").await;

        let updated = manager.update_task_status(&id, TaskStatus::Completed).await;
        assert!(updated);

        let task = manager.get_task(&id).await.unwrap();
        assert_eq!(task.status, TaskStatus::Completed);
    }

    #[tokio::test]
    async fn test_delete_task() {
        let manager = TaskManager::new();
        let id = manager.create_bash_task("to delete", "echo bye").await;

        let deleted = manager.delete_task(&id).await;
        assert!(deleted);

        let task = manager.get_task(&id).await;
        assert!(task.is_none());
    }

    #[tokio::test]
    async fn test_get_nonexistent_task() {
        let manager = TaskManager::new();
        let task = manager.get_task("nonexistent").await;
        assert!(task.is_none());
    }
}
