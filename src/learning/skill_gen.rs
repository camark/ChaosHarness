//! Skill generator — auto-generates .skill files from observed patterns.

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use super::types::Pattern;

/// Generates `.skill` files from patterns that meet a frequency threshold.
pub struct SkillGenerator {
    skills_dir: PathBuf,
    promotion_threshold: i64,
}

impl SkillGenerator {
    /// Create a new generator rooted at `project_dir/.rust_harness/skills/auto_generated/`.
    pub fn new(project_dir: &Path, promotion_threshold: i64) -> Self {
        let skills_dir = project_dir
            .join(".rust_harness")
            .join("skills")
            .join("auto_generated");
        Self {
            skills_dir,
            promotion_threshold,
        }
    }

    /// Returns `true` when the pattern's frequency meets or exceeds the threshold.
    pub fn should_generate(&self, pattern: &Pattern) -> bool {
        pattern.frequency >= self.promotion_threshold
    }

    /// Generate a `.skill` file for `pattern` and return its path.
    pub fn generate_skill(&self, pattern: &Pattern) -> Result<PathBuf> {
        fs::create_dir_all(&self.skills_dir)
            .with_context(|| format!("creating skills dir {}", self.skills_dir.display()))?;

        let slug = slugify(&pattern.description);
        let filename = format!("auto_{}_{}.skill", pattern.pattern_type.as_str(), slug);
        let path = self.skills_dir.join(&filename);

        let mut content = String::new();

        // YAML frontmatter
        content.push_str("---\n");
        content.push_str(&format!(
            "name: auto_{}_{}\n",
            pattern.pattern_type.as_str(),
            slug
        ));
        content.push_str(&format!(
            "description: Auto-generated from observed pattern: {}\n",
            pattern.description
        ));
        content.push_str("auto_generated: true\n");
        content.push_str("---\n\n");

        // Markdown body
        content.push_str("# Auto-Generated Skill\n\n");
        content.push_str(
            "> This skill was automatically generated from observed conversation patterns.\n",
        );
        content.push_str("> Review and edit before relying on it.\n\n");

        content.push_str("## Pattern\n\n");
        content.push_str(&format!("**Type:** {}\n", pattern.pattern_type.as_str()));
        content.push_str(&format!("**Description:** {}\n", pattern.description));
        content.push_str(&format!(
            "**Frequency:** {} observations\n",
            pattern.frequency
        ));

        if let Some(ref example) = pattern.example {
            content.push_str(&format!("\n## Example\n\n{}\n", example));
        }

        content.push_str(&format!(
            "\n## When to Apply\n\nApply this pattern when the user's request involves {}.\n",
            pattern.description.to_lowercase()
        ));

        fs::write(&path, &content)
            .with_context(|| format!("writing skill file {}", path.display()))?;

        Ok(path)
    }
}

/// Lowercase `text`, replace every non-alphanumeric character with `_`,
/// and collapse consecutive underscores.
pub fn slugify(text: &str) -> String {
    let mut slug: String = text
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();

    // Collapse consecutive underscores
    while slug.contains("__") {
        slug = slug.replace("__", "_");
    }

    // Trim leading/trailing underscores
    slug.trim_matches('_').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::learning::types::PatternType;
    use tempfile::TempDir;

    fn make_pattern(description: &str, frequency: i64, example: Option<&str>) -> Pattern {
        Pattern {
            id: None,
            pattern_type: PatternType::Workflow,
            description: description.to_string(),
            example: example.map(|s| s.to_string()),
            frequency,
            created_at: None,
            last_seen: None,
        }
    }

    #[test]
    fn test_should_generate_above_threshold() {
        let tmp = TempDir::new().unwrap();
        let gen = SkillGenerator::new(tmp.path(), 5);
        let pattern = make_pattern("use TDD", 5, None);
        assert!(gen.should_generate(&pattern));

        let pattern_high = make_pattern("use TDD", 10, None);
        assert!(gen.should_generate(&pattern_high));
    }

    #[test]
    fn test_should_generate_below_threshold() {
        let tmp = TempDir::new().unwrap();
        let gen = SkillGenerator::new(tmp.path(), 5);
        let pattern = make_pattern("use TDD", 4, None);
        assert!(!gen.should_generate(&pattern));

        let pattern_zero = make_pattern("use TDD", 0, None);
        assert!(!gen.should_generate(&pattern_zero));
    }

    #[test]
    fn test_generate_skill() {
        let tmp = TempDir::new().unwrap();
        let gen = SkillGenerator::new(tmp.path(), 1);
        let pattern = make_pattern("TDD workflow", 3, Some("Write test first, then code."));
        let path = gen.generate_skill(&pattern).unwrap();

        assert!(path.exists());
        let content = fs::read_to_string(&path).unwrap();

        // Frontmatter
        assert!(content.contains("---\n"));
        assert!(content.contains("auto_generated: true"));
        assert!(content.contains("name: auto_workflow_tdd_workflow"));

        // Body
        assert!(content.contains("**Description:** TDD workflow"));
        assert!(content.contains("**Frequency:** 3 observations"));

        // Example section
        assert!(content.contains("## Example"));
        assert!(content.contains("Write test first, then code."));

        // When to Apply (lowercased description)
        assert!(content.contains("tdd workflow"));
    }

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Hello World"), "hello_world");
        assert_eq!(slugify("TDD workflow!"), "tdd_workflow");
        assert_eq!(slugify("  spaces  "), "spaces");
        assert_eq!(slugify("multiple---dashes"), "multiple_dashes");
        assert_eq!(slugify("already_ok"), "already_ok");
    }
}
