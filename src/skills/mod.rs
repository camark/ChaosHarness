//! Skills module

pub mod loader;
pub mod registry;
pub mod types;
pub mod installer;

pub use types::Skill;
pub use registry::SkillRegistry;
pub use loader::{load_skill, load_skill_registry};
pub use installer::{SkillInstaller, get_user_skills_dir};
