//! Slash command registry

use crate::commands::types::{CommandContext, CommandResult, SlashCommand};
use std::collections::HashMap;

/// Registry for slash commands
pub struct CommandRegistry {
    commands: HashMap<String, SlashCommand>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
        }
    }

    pub fn register(&mut self, command: SlashCommand) {
        self.commands.insert(command.name.to_string(), command);
    }

    pub fn lookup(&self, raw_input: &str) -> Option<&SlashCommand> {
        if !raw_input.starts_with('/') {
            return None;
        }
        let input = &raw_input[1..];
        let name = input.split_whitespace().next().unwrap_or(input);
        self.commands.get(name)
    }

    pub fn help_text(&self) -> String {
        let mut lines = vec!["Available commands:".to_string()];
        let mut commands: Vec<_> = self.commands.values().collect();
        commands.sort_by(|a, b| a.name.cmp(b.name));
        for command in commands {
            lines.push(format!("  /{:<12} {}", command.name, command.description));
        }
        lines.join("\n")
    }

    pub fn list_commands(&self) -> Vec<&SlashCommand> {
        self.commands.values().collect()
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Create the default command registry with all built-in commands
pub fn create_default_command_registry() -> CommandRegistry {
    let mut registry = CommandRegistry::new();

    registry.register(SlashCommand {
        name: "help",
        description: "Show available commands",
        handler: cmd_help,
    });

    registry.register(SlashCommand {
        name: "exit",
        description: "Exit the REPL",
        handler: cmd_exit,
    });

    registry.register(SlashCommand {
        name: "clear",
        description: "Clear conversation history",
        handler: cmd_clear,
    });

    registry.register(SlashCommand {
        name: "version",
        description: "Show version information",
        handler: cmd_version,
    });

    registry.register(SlashCommand {
        name: "status",
        description: "Show session status",
        handler: cmd_status,
    });

    registry.register(SlashCommand {
        name: "usage",
        description: "Show token usage",
        handler: cmd_usage,
    });

    registry.register(SlashCommand {
        name: "skills",
        description: "List or show available skills",
        handler: cmd_skills,
    });

    registry.register(SlashCommand {
        name: "plugin",
        description: "Manage plugins",
        handler: cmd_plugin,
    });

    registry.register(SlashCommand {
        name: "hooks",
        description: "Show configured hooks",
        handler: cmd_hooks,
    });

    registry.register(SlashCommand {
        name: "config",
        description: "Show or update configuration",
        handler: cmd_config,
    });

    registry
}

// Command handlers

fn cmd_help(_args: &str, _ctx: &CommandContext) -> CommandResult {
    CommandResult::message(_ctx.registry_help_text())
}

fn cmd_exit(_args: &str, _ctx: &CommandContext) -> CommandResult {
    CommandResult::exit()
}

fn cmd_clear(_args: &str, _ctx: &CommandContext) -> CommandResult {
    CommandResult::clear(Some("Conversation cleared."))
}

fn cmd_version(_args: &str, _ctx: &CommandContext) -> CommandResult {
    CommandResult::message("RustHarness v0.1.0")
}

fn cmd_status(_args: &str, ctx: &CommandContext) -> CommandResult {
    // For now, just show basic info
    CommandResult::message(format!(
        "Working directory: {}\nModel: {}",
        ctx.cwd, ctx.settings.model
    ))
}

fn cmd_usage(_args: &str, _ctx: &CommandContext) -> CommandResult {
    CommandResult::message("Token usage tracking not yet implemented in REPL")
}

fn cmd_skills(args: &str, ctx: &CommandContext) -> CommandResult {
    use crate::skills::loader::load_skill_registry;
    use std::path::Path;

    let registry = load_skill_registry(Path::new(&ctx.cwd));

    if args.is_empty() {
        let skills = registry.list();
        if skills.is_empty() {
            return CommandResult::message("No skills available.");
        }
        let lines: Vec<_> = skills
            .iter()
            .map(|s| format!("  {}: {}", s.name, s.description))
            .collect();
        CommandResult::message(format!("Available skills:\n{}", lines.join("\n")))
    } else {
        match registry.get(args) {
            Some(skill) => CommandResult::message(skill.content.clone()),
            None => CommandResult::message(format!("Skill not found: {}", args)),
        }
    }
}

fn cmd_plugin(args: &str, ctx: &CommandContext) -> CommandResult {
    use crate::plugins::loader::load_plugins;

    let tokens: Vec<&str> = args.split_whitespace().collect();

    if tokens.is_empty() || tokens[0] == "list" {
        let plugins = load_plugins(ctx.settings, &ctx.cwd);
        if plugins.is_empty() {
            return CommandResult::message("No plugins discovered.");
        }
        let lines: Vec<_> = plugins
            .iter()
            .map(|p| format!("  {} v{} - {}", p.name, p.version, p.description.as_deref().unwrap_or("")))
            .collect();
        CommandResult::message(format!("Installed plugins:\n{}", lines.join("\n")))
    } else {
        CommandResult::message("Usage: /plugin [list|enable NAME|disable NAME|install PATH|uninstall NAME]")
    }
}

fn cmd_hooks(_args: &str, ctx: &CommandContext) -> CommandResult {
    if ctx.settings.hooks.enabled && !ctx.settings.hooks.hooks.is_empty() {
        let lines: Vec<_> = ctx.settings.hooks.hooks
            .iter()
            .map(|h| format!("  {} ({})", h.name, h.event))
            .collect();
        CommandResult::message(format!("Configured hooks:\n{}", lines.join("\n")))
    } else {
        CommandResult::message("No hooks configured.")
    }
}

fn cmd_config(args: &str, ctx: &CommandContext) -> CommandResult {
    let tokens: Vec<&str> = args.split_whitespace().collect();

    if tokens.is_empty() || tokens[0] == "show" {
        CommandResult::message(format!(
            "Model: {}\nTheme: {}\nOutput style: {}\nVim mode: {}\nFast mode: {}",
            ctx.settings.model,
            ctx.settings.theme,
            ctx.settings.output_style,
            if ctx.settings.vim_mode { "on" } else { "off" },
            if ctx.settings.fast_mode { "on" } else { "off" }
        ))
    } else {
        CommandResult::message("Usage: /config [show|set KEY VALUE]")
    }
}
