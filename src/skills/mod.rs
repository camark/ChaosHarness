//! Skills module

pub mod loader;
pub mod registry;
pub mod types;
pub mod installer;

// Re-exports for external use
pub use installer::{SkillInstaller, get_user_skills_dir};
