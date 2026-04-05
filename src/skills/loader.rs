//! Skill loader

use crate::skills::types::Skill;
use crate::skills::registry::SkillRegistry;
use std::path::Path;
use std::fs;

pub fn load_skill(path: &Path) -> Result<Skill, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read skill file: {}", e))?;

    // Parse skill frontmatter and content
    // Format: # name\ndescription\n\n## content...
    let name = path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Extract description from first lines after header
    let description = content.lines()
        .skip(1)
        .find(|l| !l.is_empty() && !l.starts_with('#'))
        .unwrap_or("A skill")
        .trim()
        .to_string();

    Ok(Skill {
        name,
        description,
        content,
        source: "local".to_string(),
    })
}

pub fn load_skill_registry(cwd: &Path) -> SkillRegistry {
    let mut registry = SkillRegistry::new();

    // Load bundled skills first
    let bundled_dir = cwd.join(".rust_harness").join("skills");
    if bundled_dir.exists() {
        if let Ok(entries) = fs::read_dir(&bundled_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("md") {
                    if let Ok(skill) = load_skill(&path) {
                        registry.register(skill);
                    }
                }
            }
        }
    }

    // Load user skills from ~/.rust_harness/skills/
    if let Some(home) = dirs::home_dir() {
        let user_skills_dir = home.join(".rust_harness").join("skills");
        if user_skills_dir.exists() {
            if let Ok(entries) = fs::read_dir(&user_skills_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("md") {
                        if let Ok(skill) = load_skill(&path) {
                            registry.register(skill);
                        }
                    }
                }
            }
        }
    }

    registry
}
