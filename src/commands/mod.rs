//! Commands module for slash command support

pub mod types;
pub mod registry;

pub use types::{CommandResult, CommandContext};
pub use registry::{CommandRegistry, create_default_command_registry};
