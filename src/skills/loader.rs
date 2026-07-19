//! Skill loader

use crate::skills::types::Skill;
use crate::skills::registry::SkillRegistry;
use std::path::Path;
use std::fs;

pub fn load_skill(path: &Path) -> Result<Skill, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read skill file: {}", e))?;

    // Try to parse YAML frontmatter first
    let (name, description) = parse_frontmatter(&content, path)
        .unwrap_or_else(|| {
            // Fallback to old format
            let name = path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            let description = content.lines()
                .skip(1)
                .find(|l| !l.is_empty() && !l.starts_with('#'))
                .unwrap_or("A skill")
                .trim()
                .to_string();
            (name, description)
        });

    Ok(Skill {
        name,
        description,
        content,
        source: "local".to_string(),
    })
}

fn parse_frontmatter(content: &str, path: &Path) -> Option<(String, String)> {
    // Check for YAML frontmatter (--- at start)
    if !content.starts_with("---") {
        return None;
    }

    // Find the end of frontmatter (second ---)
    let lines: Vec<&str> = content.lines().collect();
    let mut end_index = None;
    for (i, line) in lines.iter().enumerate().skip(1) {
        if *line == "---" {
            end_index = Some(i);
            break;
        }
    }

    let end = end_index?;

    // Parse name and description from frontmatter
    let mut name = None;
    let mut description = None;

    for line in lines.iter().skip(1).take(end - 1) {
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim().trim_matches('"');
            if key == "name" {
                name = Some(value.to_string());
            } else if key == "description" {
                description = Some(value.to_string());
            }
        }
    }

    // Use directory name as fallback for name
    let name = name.or_else(|| {
        path.parent()?.file_name()?.to_str().map(|s| s.to_string())
    })?;

    let description = description.unwrap_or_else(|| "A skill".to_string());

    Some((name, description))
}

pub fn load_skill_registry(cwd: &Path) -> SkillRegistry {
    let mut registry = SkillRegistry::new();

    // Load bundled skills first
    let bundled_dir = cwd.join(".rust_harness").join("skills");
    if bundled_dir.exists() {
        load_skills_from_dir(&bundled_dir, &mut registry);
    }

    // Load user skills from ~/.rust_harness/skills/
    if let Some(home) = dirs::home_dir() {
        let user_skills_dir = home.join(".rust_harness").join("skills");
        if user_skills_dir.exists() {
            load_skills_from_dir(&user_skills_dir, &mut registry);
        }
    }

    registry
}

fn load_skills_from_dir(skills_dir: &Path, registry: &mut SkillRegistry) {
    // First, try to load .md and .skill files directly in the skills directory
    if let Ok(entries) = fs::read_dir(skills_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let ext = path.extension().and_then(|s| s.to_str());
            if ext == Some("md") || ext == Some("skill") {
                if let Ok(skill) = load_skill(&path) {
                    registry.register(skill);
                }
            }
        }
    }

    // Second, try to load skills from subdirectories
    // Support both SKILL.md, <dirname>.md, and <dirname>.skill formats
    if let Ok(entries) = fs::read_dir(skills_dir) {
        for entry in entries.flatten() {
            let dir_path = entry.path();
            if dir_path.is_dir() {
                // Use directory name as the skill name
                let dir_name = dir_path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                // Try SKILL.md first (OpenHarness format)
                let skill_file = dir_path.join("SKILL.md");
                if skill_file.exists() {
                    if let Ok(mut skill) = load_skill(&skill_file) {
                        // Override name with directory name
                        skill.name = dir_name.clone();
                        registry.register(skill);
                        continue;
                    }
                }

                // Try <dirname>.skill (RustHarness format)
                let skill_file = dir_path.join(format!("{}.skill", dir_name));
                if skill_file.exists() {
                    if let Ok(mut skill) = load_skill(&skill_file) {
                        // Override name with directory name
                        skill.name = dir_name.clone();
                        registry.register(skill);
                        continue;
                    }
                }

                // Try <dirname>.md (simple format)
                let simple_file = dir_path.join(format!("{}.md", dir_name));
                if simple_file.exists() && simple_file != skill_file {
                    if let Ok(skill) = load_skill(&simple_file) {
                        registry.register(skill);
                    }
                }

                // Try any .md or .skill file in the directory as fallback
                if let Ok(sub_entries) = fs::read_dir(&dir_path) {
                    for sub_entry in sub_entries.flatten() {
                        let sub_path = sub_entry.path();
                        let ext = sub_path.extension().and_then(|s| s.to_str());
                        let is_readme = sub_path.file_name()
                            .and_then(|s| s.to_str())
                            .map(|s| s.eq_ignore_ascii_case("README.md"))
                            .unwrap_or(false);
                        if (ext == Some("md") || ext == Some("skill")) && !is_readme {
                            if let Ok(mut skill) = load_skill(&sub_path) {
                                // Override name with directory name
                                skill.name = dir_name.clone();
                                registry.register(skill);
                                break; // Only load one skill per directory
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_load_skill_with_frontmatter() {
        let dir = TempDir::new().unwrap();
        let skill_path = dir.path().join("test.skill");
        fs::write(&skill_path, "---\nname: my_skill\ndescription: A test skill\n---\n# Content\nSome content here").unwrap();

        let skill = load_skill(&skill_path).unwrap();
        assert_eq!(skill.name, "my_skill");
        assert_eq!(skill.description, "A test skill");
        assert!(skill.content.contains("# Content"));
        assert_eq!(skill.source, "local");
    }

    #[test]
    fn test_load_skill_without_frontmatter() {
        let dir = TempDir::new().unwrap();
        let skill_path = dir.path().join("simple.md");
        fs::write(&skill_path, "# Simple Skill\nThis is a simple skill").unwrap();

        let skill = load_skill(&skill_path).unwrap();
        assert_eq!(skill.name, "simple");
        assert_eq!(skill.source, "local");
    }

    #[test]
    fn test_load_skill_nonexistent() {
        let result = load_skill(Path::new("/nonexistent/path/skill.md"));
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_frontmatter_valid() {
        let content = "---\nname: test\ndescription: desc\n---\nbody";
        let result = parse_frontmatter(content, Path::new("/tmp/test.md"));
        assert!(result.is_some());
        let (name, desc) = result.unwrap();
        assert_eq!(name, "test");
        assert_eq!(desc, "desc");
    }

    #[test]
    fn test_parse_frontmatter_no_frontmatter() {
        let content = "# No frontmatter\nJust content";
        let result = parse_frontmatter(content, Path::new("/tmp/test.md"));
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_frontmatter_missing_name() {
        let content = "---\ndescription: desc\n---\nbody";
        let path = Path::new("/tmp/fallback_name.md");
        let result = parse_frontmatter(content, path);
        // Should use directory name or filename as fallback
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_frontmatter_quoted_values() {
        let content = "---\nname: \"quoted_name\"\ndescription: \"quoted desc\"\n---\nbody";
        let result = parse_frontmatter(content, Path::new("/tmp/test.md"));
        assert!(result.is_some());
        let (name, desc) = result.unwrap();
        assert_eq!(name, "quoted_name");
        assert_eq!(desc, "quoted desc");
    }

    #[test]
    fn test_load_skill_registry_empty_dirs() {
        // Use a temp dir that has no .rust_harness/skills subdirectory
        let dir = TempDir::new().unwrap();
        // Ensure no .rust_harness directory exists
        let harness_dir = dir.path().join(".rust_harness").join("skills");
        assert!(!harness_dir.exists());

        // The registry should be empty (though user home may have skills)
        // We just verify it doesn't crash on empty project dir
        let _registry = load_skill_registry(dir.path());
        // Note: may find skills from user's home directory, so we don't assert count
    }

    #[test]
    fn test_load_skills_from_dir() {
        let dir = TempDir::new().unwrap();
        let skills_dir = dir.path().join("skills");
        fs::create_dir_all(&skills_dir).unwrap();

        fs::write(skills_dir.join("skill1.md"), "---\nname: skill1\ndescription: first\n---\n# Skill 1").unwrap();
        fs::write(skills_dir.join("skill2.md"), "---\nname: skill2\ndescription: second\n---\n# Skill 2").unwrap();

        let mut registry = SkillRegistry::new();
        load_skills_from_dir(&skills_dir, &mut registry);

        assert_eq!(registry.count(), 2);
        assert!(registry.has("skill1"));
        assert!(registry.has("skill2"));
    }

    #[test]
    fn test_load_skills_from_subdirectory() {
        let dir = TempDir::new().unwrap();
        let skills_dir = dir.path().join("skills");
        let sub_dir = skills_dir.join("my_skill");
        fs::create_dir_all(&sub_dir).unwrap();

        fs::write(sub_dir.join("SKILL.md"), "---\ndescription: sub skill\n---\n# Sub Skill").unwrap();

        let mut registry = SkillRegistry::new();
        load_skills_from_dir(&skills_dir, &mut registry);

        assert_eq!(registry.count(), 1);
        assert!(registry.has("my_skill"));
    }

    #[test]
    fn test_load_skill_fallback_description() {
        let dir = TempDir::new().unwrap();
        let skill_path = dir.path().join("no_desc.md");
        fs::write(&skill_path, "# Title\nFirst non-empty line").unwrap();

        let skill = load_skill(&skill_path).unwrap();
        assert_eq!(skill.name, "no_desc");
        // Should extract description from first non-empty, non-header line
        assert!(!skill.description.is_empty());
    }
}
