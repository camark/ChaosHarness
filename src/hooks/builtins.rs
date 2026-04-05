//! Built-in hooks for common functionality

use crate::hooks::schemas::HookDefinition;
use crate::hooks::types::HookContext;
use serde_json::Value;
use std::collections::HashSet;
use lazy_static::lazy_static;

lazy_static! {
    /// Dangerous command patterns that should be blocked
    static ref DANGEROUS_COMMANDS: HashSet<&'static str> = {
        let mut set = HashSet::new();
        set.insert("rm -rf /");
        set.insert("rm -rf *");
        set.insert("dd if=/dev/zero");
        set.insert(":(){:|:&};:");  // Fork bomb
        set.insert("mkfs");
        set.insert("> /dev/sda");
        set.insert("chmod -R 777 /");
        set.insert("curl .* | .*sh");
        set.insert("wget .* | .*sh");
        set
    };

    /// Dangerous file patterns
    static ref SENSITIVE_FILES: HashSet<&'static str> = {
        let mut set = HashSet::new();
        set.insert(".env");
        set.insert(".env.local");
        set.insert(".env.production");
        set.insert("id_rsa");
        set.insert("id_ed25519");
        set.insert("credentials.json");
        set.insert("secrets.yaml");
        set.insert("secrets.yml");
        set.insert("password.txt");
        set
    };
}

/// Security scanning hook for pre-tool-use checks
pub fn create_security_scanner_hook() -> HookDefinition {
    HookDefinition {
        name: "security_scanner".to_string(),
        command: "builtin:security_scan".to_string(),
        event: "pre_tool_use".to_string(),
        blocking: true,
        timeout: 5,
        cwd: None,
    }
}

/// Execute built-in security scan
pub fn execute_security_scan(context: &HookContext) -> Option<String> {
    let payload = context.payload.as_object()?;

    // Check tool name
    let tool_name = payload.get("tool_name")?.as_str()?;

    // Special handling for bash tool
    if tool_name == "bash" {
        if let Some(command) = payload.get("command").and_then(|v| v.as_str()) {
            // Check for dangerous commands
            for dangerous in DANGEROUS_COMMANDS.iter() {
                if command.contains(dangerous) {
                    return Some(format!("Blocked dangerous command pattern: {}", dangerous));
                }
            }

            // Check for sudo
            if command.starts_with("sudo") {
                return Some("Blocked: sudo commands require explicit approval".to_string());
            }

            // Check for curl/wget piped to shell
            if (command.contains("curl") || command.contains("wget"))
                && (command.contains("| sh") || command.contains("| bash")) {
                return Some("Blocked: piping remote scripts to shell is dangerous".to_string());
            }
        }
    }

    // Special handling for file write/edit tools
    if tool_name == "write_file" || tool_name == "edit_file" {
        if let Some(path) = payload.get("path").and_then(|v| v.as_str()) {
            let file_name = path.split('/').last().or_else(|| path.split('\\').last())?;

            for sensitive in SENSITIVE_FILES.iter() {
                if file_name == *sensitive || path.contains(sensitive) {
                    return Some(format!("Blocked: modifying sensitive file '{}'", sensitive));
                }
            }
        }
    }

    // All checks passed
    None
}

/// Code review hook - logs code changes for review
pub fn create_code_reviewer_hook() -> HookDefinition {
    HookDefinition {
        name: "code_reviewer".to_string(),
        command: "builtin:code_review".to_string(),
        event: "post_tool_use".to_string(),
        blocking: false,
        timeout: 10,
        cwd: None,
    }
}

/// Execute built-in code review
pub fn execute_code_review(context: &HookContext) -> Option<String> {
    let payload = context.payload.as_object()?;

    let tool_name = payload.get("tool_name")?.as_str()?;

    // Only review code-related tools
    if !["write_file", "edit_file"].contains(&tool_name) {
        return None;
    }

    let mut review_notes = Vec::new();

    // Check file extension
    if let Some(path) = payload.get("path").and_then(|v| v.as_str()) {
        if path.ends_with(".rs") {
            review_notes.push("Rust file modified - ensure proper error handling");
        } else if path.ends_with(".js") || path.ends_with(".ts") {
            review_notes.push("JavaScript/TypeScript file - check for type safety");
        } else if path.ends_with(".py") {
            review_notes.push("Python file - verify type hints and docstrings");
        }
    }

    // Check for TODO/FIXME comments in content
    if let Some(content) = payload.get("content").and_then(|v| v.as_str()) {
        if content.contains("TODO") || content.contains("FIXME") {
            review_notes.push("Note: Code contains TODO/FIXME comments");
        }
    }

    if review_notes.is_empty() {
        None
    } else {
        Some(format!("Code review notes: {}", review_notes.join("; ")))
    }
}

/// Logging hook - logs all tool executions
pub fn create_tool_logger_hook() -> HookDefinition {
    HookDefinition {
        name: "tool_logger".to_string(),
        command: "builtin:tool_log".to_string(),
        event: "post_tool_use".to_string(),
        blocking: false,
        timeout: 2,
        cwd: None,
    }
}

/// Execute built-in tool logging
pub fn execute_tool_log(context: &HookContext) -> Option<String> {
    let payload = context.payload.as_object()?;

    let tool_name = payload.get("tool_name")?.as_str()?;
    let success = payload.get("success").and_then(|v| v.as_bool()).unwrap_or(false);

    let status = if success { "SUCCESS" } else { "FAILED" };
    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S");

    Some(format!("[{}] Tool '{}' completed: {}", timestamp, tool_name, status))
}

/// Get all built-in hook definitions
pub fn get_builtin_hooks() -> Vec<HookDefinition> {
    vec![
        create_security_scanner_hook(),
        create_code_reviewer_hook(),
        create_tool_logger_hook(),
    ]
}

/// Execute a built-in hook by name
pub fn execute_builtin_hook(name: &str, context: &HookContext) -> Option<String> {
    match name {
        "builtin:security_scan" => execute_security_scan(context),
        "builtin:code_review" => execute_code_review(context),
        "builtin:tool_log" => execute_tool_log(context),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use crate::hooks::types::HookContext;

    #[test]
    fn test_dangerous_command_detection() {
        let context = HookContext {
            event: "pre_tool_use".to_string(),
            payload: json!({
                "tool_name": "bash",
                "command": "rm -rf /tmp/test"
            }),
        };

        let result = execute_security_scan(&context);
        assert!(result.is_some());
        assert!(result.unwrap().contains("Blocked"));
    }

    #[test]
    fn test_safe_command() {
        let context = HookContext {
            event: "pre_tool_use".to_string(),
            payload: json!({
                "tool_name": "bash",
                "command": "ls -la"
            }),
        };

        let result = execute_security_scan(&context);
        assert!(result.is_none());
    }

    #[test]
    fn test_sensitive_file_detection() {
        let context = HookContext {
            event: "pre_tool_use".to_string(),
            payload: json!({
                "tool_name": "write_file",
                "path": "/home/user/.env"
            }),
        };

        let result = execute_security_scan(&context);
        assert!(result.is_some());
    }
}
