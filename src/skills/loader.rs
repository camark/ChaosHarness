//! Skill loader

use crate::skills::types::Skill;
use std::path::Path;
use std::fs;

pub fn load_skill(path: &Path) -> Result<Skill, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read skill file: {}", e))?;

    // Parse skill frontmatter (YAML) and content
    // Simplified for now
    Ok(Skill {
        name: path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string(),
        description: "A skill".to_string(),
        content,
    })
}
