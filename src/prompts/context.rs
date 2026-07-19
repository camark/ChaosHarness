//! Enhanced context building for prompts

use std::path::Path;
use std::fs;

/// Build comprehensive project context
pub fn build_context(cwd: &str) -> String {
    let path = Path::new(cwd);
    let mut context = String::new();

    // Project type detection
    if let Some(project_type) = detect_project_type(path) {
        context.push_str(&format!("**Project Type**: {}\n\n", project_type));
    }

    // CLAUDE.md content
    let claude_md = path.join("CLAUDE.md");
    if claude_md.exists() {
        if let Ok(content) = fs::read_to_string(&claude_md) {
            context.push_str("## Project Instructions (CLAUDE.md)\n\n");
            context.push_str(&content);
            context.push_str("\n\n");
        }
    }

    // Dependencies from Cargo.toml / package.json
    if let Some(deps) = extract_dependencies(path) {
        context.push_str("## Key Dependencies\n\n");
        context.push_str(&deps);
        context.push_str("\n\n");
    }

    // Directory structure (recursive with depth limit)
    context.push_str("## Directory Structure\n\n");
    context.push_str(&build_directory_tree(path, 2, ""));
    context.push('\n');

    // Git status
    if let Some(git_info) = build_git_context(path) {
        context.push_str("## Git Status\n\n");
        context.push_str(&git_info);
        context.push_str("\n\n");
    }

    // Recent modified files
    if let Some(recent) = get_recent_files(path) {
        context.push_str("## Recently Modified Files\n\n");
        context.push_str(&recent);
        context.push_str("\n\n");
    }

    context
}

/// Detect project type from files present
fn detect_project_type(path: &Path) -> Option<String> {
    if path.join("Cargo.toml").exists() {
        Some("Rust".to_string())
    } else if path.join("package.json").exists() {
        if path.join("tsconfig.json").exists() {
            Some("TypeScript".to_string())
        } else {
            Some("JavaScript/Node.js".to_string())
        }
    } else if path.join("pyproject.toml").exists() || path.join("setup.py").exists() {
        Some("Python".to_string())
    } else if path.join("go.mod").exists() {
        Some("Go".to_string())
    } else if path.join("pom.xml").exists() {
        Some("Java (Maven)".to_string())
    } else if path.join("build.gradle").exists() {
        Some("Java (Gradle)".to_string())
    } else if path.join("CMakeLists.txt").exists() {
        Some("C/C++ (CMake)".to_string())
    } else if path.join("Makefile").exists() {
        Some("C/C++ (Make)".to_string())
    } else {
        None
    }
}

/// Extract key dependencies from project files
fn extract_dependencies(path: &Path) -> Option<String> {
    // Try Cargo.toml
    let cargo_toml = path.join("Cargo.toml");
    if cargo_toml.exists() {
        if let Ok(content) = fs::read_to_string(&cargo_toml) {
            let mut deps = Vec::new();
            let mut in_deps = false;

            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed == "[dependencies]" || trimmed == "[dev-dependencies]" {
                    in_deps = true;
                    continue;
                }
                if in_deps {
                    if trimmed.starts_with('[') {
                        in_deps = false;
                        continue;
                    }
                    if !trimmed.is_empty() && !trimmed.starts_with('#') {
                        // Extract just the package name
                        let name = trimmed.split('=').next()
                            .map(|s| s.trim().to_string())
                            .unwrap_or_default();
                        if !name.is_empty() {
                            deps.push(name);
                        }
                    }
                }
            }

            if !deps.is_empty() {
                return Some(deps.join(", "));
            }
        }
    }

    // Try package.json
    let package_json = path.join("package.json");
    if package_json.exists() {
        if let Ok(content) = fs::read_to_string(&package_json) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                let mut deps = Vec::new();

                if let Some(deps_obj) = json.get("dependencies").and_then(|d| d.as_object()) {
                    deps.extend(deps_obj.keys().cloned());
                }
                if let Some(dev_deps) = json.get("devDependencies").and_then(|d| d.as_object()) {
                    deps.extend(dev_deps.keys().take(5).cloned());
                    if dev_deps.len() > 5 {
                        deps.push(format!("... (+{} more)", dev_deps.len() - 5));
                    }
                }

                if !deps.is_empty() {
                    return Some(deps.join(", "));
                }
            }
        }
    }

    None
}

/// Build directory tree with depth limit
fn build_directory_tree(path: &Path, max_depth: usize, prefix: &str) -> String {
    if max_depth == 0 {
        return String::new();
    }

    let mut result = String::new();
    let mut entries: Vec<_> = fs::read_dir(path)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            !name.starts_with('.') && name != "node_modules" && name != "target"
        })
        .collect();

    entries.sort_by(|a, b| {
        let a_is_dir = a.path().is_dir();
        let b_is_dir = b.path().is_dir();
        b_is_dir.cmp(&a_is_dir).then(a.file_name().cmp(&b.file_name()))
    });

    let len = entries.len();
    for (i, entry) in entries.iter().enumerate() {
        let name = entry.file_name().to_string_lossy().to_string();
        let is_last = i == len - 1;
        let connector = if is_last { "└── " } else { "├── " };
        let child_prefix = if is_last { "    " } else { "│   " };

        if entry.path().is_dir() {
            result.push_str(&format!("{}{}{}/\n", prefix, connector, name));
            result.push_str(&build_directory_tree(
                &entry.path(),
                max_depth - 1,
                &format!("{}{}", prefix, child_prefix),
            ));
        } else {
            result.push_str(&format!("{}{}{}\n", prefix, connector, name));
        }
    }

    result
}

/// Build git context information
fn build_git_context(path: &Path) -> Option<String> {
    let git_dir = path.join(".git");
    if !git_dir.exists() {
        return None;
    }

    let mut info = String::new();

    // Current branch
    if let Ok(head) = fs::read_to_string(git_dir.join("HEAD")) {
        if let Some(branch) = head.strip_prefix("ref: refs/heads/") {
            info.push_str(&format!("Branch: {}\n", branch.trim()));
        }
    }

    // Git status (modified files)
    let status_output = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(path)
        .output()
        .ok();

    if let Some(output) = status_output {
        let status = String::from_utf8_lossy(&output.stdout);
        let modified: Vec<_> = status.lines().take(10).collect();
        if !modified.is_empty() {
            info.push_str("Modified files:\n");
            for line in modified {
                info.push_str(&format!("  {}\n", line.trim()));
            }
            if status.lines().count() > 10 {
                info.push_str(&format!("  ... (+{} more)\n", status.lines().count() - 10));
            }
        }
    }

    // Recent commits
    let log_output = std::process::Command::new("git")
        .args(["log", "--oneline", "-5"])
        .current_dir(path)
        .output()
        .ok();

    if let Some(output) = log_output {
        let log = String::from_utf8_lossy(&output.stdout);
        if !log.trim().is_empty() {
            info.push_str("\nRecent commits:\n");
            for line in log.lines() {
                info.push_str(&format!("  {}\n", line));
            }
        }
    }

    if info.is_empty() {
        None
    } else {
        Some(info)
    }
}

/// Get recently modified files
fn get_recent_files(path: &Path) -> Option<String> {
    use std::time::SystemTime;

    let mut files: Vec<(String, SystemTime)> = Vec::new();

    fn collect_files(dir: &Path, files: &mut Vec<(String, SystemTime)>, depth: usize) {
        if depth > 2 {
            return;
        }
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') || name == "node_modules" || name == "target" {
                    continue;
                }
                let path = entry.path();
                if path.is_dir() {
                    collect_files(&path, files, depth + 1);
                } else if let Ok(metadata) = entry.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        files.push((path.to_string_lossy().to_string(), modified));
                    }
                }
            }
        }
    }

    collect_files(path, &mut files, 0);

    if files.is_empty() {
        return None;
    }

    files.sort_by_key(|b| std::cmp::Reverse(b.1));
    files.truncate(10);

    let mut result = String::new();
    for (file, _) in &files {
        // Make path relative to cwd
        if let Some(relative) = file.strip_prefix(&format!("{}/", path.display())) {
            result.push_str(&format!("  {}\n", relative));
        } else {
            result.push_str(&format!("  {}\n", file));
        }
    }

    Some(result)
}

/// Read CLAUDE.md from path
pub fn read_claude_md(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok()
}

/// Build skills section for system prompt
pub fn build_skills_section(cwd: &str) -> Option<String> {
    use crate::skills::loader::load_skill_registry;

    let registry = load_skill_registry(Path::new(cwd));
    let skills = registry.list();

    if skills.is_empty() {
        return None;
    }

    let mut lines = vec![
        "# Available Skills".to_string(),
        "".to_string(),
        "The following skills are available via the `skill` tool.".to_string(),
        "When a user's request matches a skill, invoke it with `skill(name=\"<skill_name>\")`".to_string(),
        "to load detailed instructions before proceeding.".to_string(),
        "".to_string(),
    ];

    for skill in skills {
        lines.push(format!("- **{}**: {}", skill.name, skill.description));
    }

    Some(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[test]
    fn test_detect_project_type_rust() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
        assert_eq!(detect_project_type(dir.path()), Some("Rust".to_string()));
    }

    #[test]
    fn test_detect_project_type_javascript() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();
        assert_eq!(detect_project_type(dir.path()), Some("JavaScript/Node.js".to_string()));
    }

    #[test]
    fn test_detect_project_type_typescript() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();
        fs::write(dir.path().join("tsconfig.json"), "{}").unwrap();
        assert_eq!(detect_project_type(dir.path()), Some("TypeScript".to_string()));
    }

    #[test]
    fn test_detect_project_type_python() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("pyproject.toml"), "[project]\nname = \"test\"").unwrap();
        assert_eq!(detect_project_type(dir.path()), Some("Python".to_string()));
    }

    #[test]
    fn test_detect_project_type_go() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("go.mod"), "module test").unwrap();
        assert_eq!(detect_project_type(dir.path()), Some("Go".to_string()));
    }

    #[test]
    fn test_detect_project_type_none() {
        let dir = tempdir().unwrap();
        assert_eq!(detect_project_type(dir.path()), None);
    }

    #[test]
    fn test_extract_dependencies_cargo() {
        let dir = tempdir().unwrap();
        let cargo_toml = r#"[package]
name = "test"

[dependencies]
serde = "1.0"
tokio = { version = "1.0", features = ["full"] }
"#;
        fs::write(dir.path().join("Cargo.toml"), cargo_toml).unwrap();
        let deps = extract_dependencies(dir.path());
        assert!(deps.is_some());
        let deps = deps.unwrap();
        assert!(deps.contains("serde"));
        assert!(deps.contains("tokio"));
    }

    #[test]
    fn test_extract_dependencies_none() {
        let dir = tempdir().unwrap();
        assert!(extract_dependencies(dir.path()).is_none());
    }

    #[test]
    fn test_build_directory_tree() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src").join("main.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]").unwrap();

        let tree = build_directory_tree(dir.path(), 2, "");
        assert!(tree.contains("src/"));
        assert!(tree.contains("Cargo.toml"));
    }

    #[test]
    fn test_build_context_empty_dir() {
        let dir = tempdir().unwrap();
        let context = build_context(dir.path().to_str().unwrap());
        // Should contain directory structure even for empty dir
        assert!(context.contains("Directory Structure"));
    }

    #[test]
    fn test_build_context_with_claude_md() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("CLAUDE.md"), "# Test Project").unwrap();
        let context = build_context(dir.path().to_str().unwrap());
        assert!(context.contains("Test Project"));
        assert!(context.contains("CLAUDE.md"));
    }

    #[test]
    fn test_read_claude_md() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("CLAUDE.md"), "# Test").unwrap();
        let content = read_claude_md(&dir.path().join("CLAUDE.md"));
        assert_eq!(content, Some("# Test".to_string()));
    }

    #[test]
    fn test_read_claude_md_not_found() {
        let dir = tempdir().unwrap();
        let content = read_claude_md(&dir.path().join("CLAUDE.md"));
        assert_eq!(content, None);
    }

    #[test]
    fn test_build_skills_section_empty() {
        let dir = tempdir().unwrap();
        // May return None if no skills directory exists
        let section = build_skills_section(dir.path().to_str().unwrap());
        // Either None or Some with content
        if let Some(s) = section {
            assert!(s.contains("Available Skills") || s.is_empty());
        }
    }
}
