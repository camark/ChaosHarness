//! Slash command types

use crate::config::Settings;

/// Result returned by a slash command
#[derive(Debug, Clone)]
pub struct CommandResult {
    pub message: Option<String>,
    pub should_exit: bool,
    pub clear_screen: bool,
}

impl CommandResult {
    pub fn message(msg: impl Into<String>) -> Self {
        Self {
            message: Some(msg.into()),
            should_exit: false,
            clear_screen: false,
        }
    }

    pub fn exit() -> Self {
        Self {
            message: None,
            should_exit: true,
            clear_screen: false,
        }
    }

    pub fn clear(msg: Option<&str>) -> Self {
        Self {
            message: msg.map(String::from),
            should_exit: false,
            clear_screen: true,
        }
    }
}

/// Context available to command handlers
pub struct CommandContext<'a> {
    pub cwd: String,
    pub settings: &'a Settings,
    pub registry: &'a crate::commands::registry::CommandRegistry,
}

impl<'a> CommandContext<'a> {
    pub fn registry_help_text(&self) -> String {
        self.registry.help_text()
    }
}

/// Slash command definition
pub struct SlashCommand {
    pub name: &'static str,
    pub description: &'static str,
    pub handler: fn(&str, &CommandContext) -> CommandResult,
}
