//! Permission checking logic

use crate::permissions::PermissionMode;
use crate::config::PermissionSettings;
use std::sync::Arc;
use tokio::sync::Mutex;
use serde_json::Value;

/// Permission decision
#[derive(Debug, Clone)]
pub struct PermissionDecision {
    pub allowed: bool,
    pub requires_confirmation: bool,
    pub reason: String,
}

/// Permission checker
#[derive(Clone)]
pub struct PermissionChecker {
    settings: Arc<Mutex<PermissionSettings>>,
}

impl PermissionChecker {
    pub fn new(settings: PermissionSettings) -> Self {
        Self {
            settings: Arc::new(Mutex::new(settings)),
        }
    }

    /// Extract file path from tool input if present
    pub fn extract_file_path(input: &Value) -> Option<String> {
        // Check common path fields in tool inputs
        let fields = ["path", "file_path", "filepath", "root", "directory"];
        for field in fields {
            if let Some(path) = input[field].as_str() {
                return Some(path.to_string());
            }
        }
        None
    }

    /// Evaluate permission for a tool invocation
    pub async fn evaluate(
        &self,
        tool_name: &str,
        is_read_only: bool,
        file_path: Option<&str>,
        _command: Option<&str>,
    ) -> PermissionDecision {
        let settings = self.settings.lock().await;

        match settings.mode {
            PermissionMode::FullAuto => {
                return PermissionDecision {
                    allowed: true,
                    requires_confirmation: false,
                    reason: String::new(),
                }
            }
            PermissionMode::Plan => {
                // In plan mode, read-only tools are allowed, write tools require confirmation
                if is_read_only {
                    return PermissionDecision {
                        allowed: true,
                        requires_confirmation: false,
                        reason: String::new(),
                    };
                } else {
                    return PermissionDecision {
                        allowed: false,
                        requires_confirmation: true,
                        reason: "Plan mode: write operations require explicit approval".to_string(),
                    };
                }
            }
            PermissionMode::Default => {
                // Check explicit denies first
                if settings.denied_tools.contains(&tool_name.to_string()) {
                    return PermissionDecision {
                        allowed: false,
                        requires_confirmation: false,
                        reason: format!("Tool '{}' is explicitly denied", tool_name),
                    };
                }

                // Check explicit allows
                if settings.allowed_tools.contains(&tool_name.to_string()) {
                    return PermissionDecision {
                        allowed: true,
                        requires_confirmation: false,
                        reason: String::new(),
                    };
                }

                // Check file path rules if a path is provided
                if let Some(path) = file_path {
                    // Check if path is under user's home directory
                    if let Some(home_dir) = dirs::home_dir() {
                        let path_buf = std::path::PathBuf::from(path);

                        // Expand ~ to home directory
                        let resolved_path = if path.starts_with("~/") || path == "~" {
                            home_dir.clone()
                        } else if path_buf.is_absolute() {
                            path_buf
                        } else {
                            return PermissionDecision {
                                allowed: false,
                                requires_confirmation: true,
                                reason: "Relative paths outside working directory not allowed".to_string(),
                            };
                        };

                        // Check if path is under home directory
                        if resolved_path.starts_with(&home_dir) {
                            // Home directory and subdirectories are allowed
                            if self.is_safe_tool(tool_name, is_read_only) {
                                return PermissionDecision {
                                    allowed: true,
                                    requires_confirmation: false,
                                    reason: String::new(),
                                };
                            }
                        }
                    }

                    // Check explicit path rules
                    for rule in &settings.path_rules {
                        if path.contains(&rule.pattern) {
                            return PermissionDecision {
                                allowed: rule.allow,
                                requires_confirmation: !rule.allow,
                                reason: if rule.allow {
                                    String::new()
                                } else {
                                    format!("Path '{}' is explicitly denied", rule.pattern)
                                },
                            };
                        }
                    }
                }

                // Default: safe tools only
                if self.is_safe_tool(tool_name, is_read_only) {
                    return PermissionDecision {
                        allowed: true,
                        requires_confirmation: false,
                        reason: String::new(),
                    };
                }

                // Requires confirmation
                return PermissionDecision {
                    allowed: false,
                    requires_confirmation: true,
                    reason: format!("Tool '{}' requires confirmation", tool_name),
                };
            }
        }
    }

    fn is_safe_tool(&self, tool_name: &str, is_read_only: bool) -> bool {
        // Read-only tools are generally safe
        if is_read_only {
            return true;
        }

        // List of tools considered safe by default
        let safe_tools = ["read_file", "glob", "grep", "write_file", "edit_file"];
        safe_tools.contains(&tool_name)
    }

    pub async fn is_tool_allowed(&self, tool_name: &str) -> bool {
        let settings = self.settings.lock().await;

        match settings.mode {
            PermissionMode::FullAuto => true,
            PermissionMode::Plan => true,
            PermissionMode::Default => {
                if settings.denied_tools.contains(&tool_name.to_string()) {
                    return false;
                }
                if settings.allowed_tools.contains(&tool_name.to_string()) {
                    return true;
                }
                self.is_safe_tool(tool_name, false)
            }
        }
    }

    pub async fn is_command_allowed(&self, command: &str) -> bool {
        let settings = self.settings.lock().await;
        !settings.denied_commands.contains(&command.to_string())
    }

    pub async fn requires_confirmation(&self, tool_name: &str) -> bool {
        let settings = self.settings.lock().await;

        match settings.mode {
            PermissionMode::FullAuto => false,
            PermissionMode::Plan => true,
            PermissionMode::Default => {
                if settings.allowed_tools.contains(&tool_name.to_string()) {
                    return false;
                }
                !self.is_safe_tool(tool_name, false)
            }
        }
    }

    pub async fn set_mode(&self, mode: PermissionMode) {
        let mut settings = self.settings.lock().await;
        settings.mode = mode;
    }
}
