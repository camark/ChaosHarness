//! Commands module for slash command support

pub mod types;
pub mod registry;

#[allow(unused_imports)]
pub use types::{CommandResult, CommandContext};
#[allow(unused_imports)]
pub use registry::{CommandRegistry, create_default_command_registry};
