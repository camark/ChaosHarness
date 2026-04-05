//! Configuration module

pub mod default_settings;
pub mod paths;
pub mod settings;

pub use paths::*;
pub use settings::{load_settings, save_settings, PermissionSettings, Settings};
