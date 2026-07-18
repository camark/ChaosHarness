//! Context building for prompts

use std::path::Path;
use std::fs;

pub fn build_context(cwd: &str) -> String {
    let path = Path::new(cwd);
    let mut context = String::new();

    // Add working directory info
    context.push_str(&format!("Working directory: {}\n\n", cwd));

    // Add CLAUDE.md content if it exists
    let claude_md = path.join("CLAUDE.md");
    if claude_md.exists() {
        if let Ok(content) = fs::read_to_string(&claude_md) {
            context.push_str("# Project Instructions (CLAUDE.md)\n\n");
            context.push_str(&content);
            context.push_str("\n\n");
        }
    }

    // Add directory structure (top level)
    context.push_str("## Directory Structure\n\n");
    if let Ok(entries) = fs::read_dir(path) {
        let mut dirs = Vec::new();
        let mut files = Vec::new();

        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            if entry.path().is_dir() {
                dirs.push(name);
            } else {
                files.push(name);
            }
        }

        dirs.sort();
        files.sort();

        for dir in &dirs {
            context.push_str(&format!("{}/\n", dir));
        }
        for file in &files {
            context.push_str(&format!("{}\n", file));
        }
    }

    // Add git info if available
    let git_dir = path.join(".git");
    if git_dir.exists() {
        context.push_str("\n## Git Info\n\n");

        // Get current branch
        if let Ok(head) = fs::read_to_string(path.join(".git/HEAD")) {
            if let Some(branch) = head.strip_prefix("ref: refs/heads/") {
                context.push_str(&format!("Branch: {}\n", branch.trim()));
            }
        }
    }

    context
}

pub fn read_claude_md(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

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
        "The following skills are available via the `skill` tool. ".to_string(),
        "When a user's request matches a skill, invoke it with `skill(name=\"<skill_name>\")` ".to_string(),
        "to load detailed instructions before proceeding.".to_string(),
        "".to_string(),
    ];

    for skill in skills {
        lines.push(format!("- **{}**: {}", skill.name, skill.description));
    }

    Some(lines.join("\n"))
}
