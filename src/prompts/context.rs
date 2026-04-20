//! Context building for prompts

#![allow(dead_code)]

use std::path::Path;

pub fn build_context(_cwd: &str) -> String {
    // In a full implementation, this would gather:
    // - Current directory structure
    // - Relevant files
    // - Recent git history
    // - Project configuration

    String::from("Working directory context...")
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
