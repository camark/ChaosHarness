//! Path resolution for configuration and data directories

#![allow(dead_code)]

use std::path::PathBuf;
use std::env;
use std::fs;

const DEFAULT_BASE_DIR: &str = ".rust_harness";

fn get_base_dir() -> PathBuf {
    if let Ok(dir) = env::var("RUST_HARNESS_CONFIG_DIR") {
        PathBuf::from(dir)
    } else {
        dirs::home_dir()
            .expect("Failed to get home directory")
            .join(DEFAULT_BASE_DIR)
    }
}

pub fn get_config_dir() -> PathBuf {
    let dir = get_base_dir();
    fs::create_dir_all(&dir).expect("Failed to create config directory");
    dir
}

pub fn get_config_file_path() -> PathBuf {
    get_config_dir().join("settings.json")
}

pub fn get_data_dir() -> PathBuf {
    let dir = if let Ok(dir) = env::var("RUST_HARNESS_DATA_DIR") {
        PathBuf::from(dir)
    } else {
        get_config_dir().join("data")
    };
    fs::create_dir_all(&dir).expect("Failed to create data directory");
    dir
}

pub fn get_logs_dir() -> PathBuf {
    let dir = if let Ok(dir) = env::var("RUST_HARNESS_LOGS_DIR") {
        PathBuf::from(dir)
    } else {
        get_config_dir().join("logs")
    };
    fs::create_dir_all(&dir).expect("Failed to create logs directory");
    dir
}

pub fn get_sessions_dir() -> PathBuf {
    let dir = get_data_dir().join("sessions");
    fs::create_dir_all(&dir).expect("Failed to create sessions directory");
    dir
}

pub fn get_tasks_dir() -> PathBuf {
    let dir = get_data_dir().join("tasks");
    fs::create_dir_all(&dir).expect("Failed to create tasks directory");
    dir
}

pub fn get_feedback_dir() -> PathBuf {
    let dir = get_data_dir().join("feedback");
    fs::create_dir_all(&dir).expect("Failed to create feedback directory");
    dir
}

pub fn get_feedback_log_path() -> PathBuf {
    get_feedback_dir().join("feedback.log")
}

pub fn get_cron_registry_path() -> PathBuf {
    get_data_dir().join("cron_jobs.json")
}

pub fn get_project_config_dir(cwd: &str) -> PathBuf {
    let dir = PathBuf::from(cwd).join(".rust_harness");
    fs::create_dir_all(&dir).expect("Failed to create project config directory");
    dir
}

pub fn get_project_issue_file(cwd: &str) -> PathBuf {
    get_project_config_dir(cwd).join("issue.md")
}

pub fn get_project_pr_comments_file(cwd: &str) -> PathBuf {
    get_project_config_dir(cwd).join("pr_comments.md")
}
