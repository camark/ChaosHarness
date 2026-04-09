//! Slash command registry

use crate::commands::types::{CommandContext, CommandResult, SlashCommand};
use crate::memory::MemoryManager;
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

    registry.register(SlashCommand {
        name: "memory",
        description: "Manage project memory (list/show/add/remove)",
        handler: cmd_memory,
    });

    registry.register(SlashCommand {
        name: "resume",
        description: "Resume a previous session",
        handler: cmd_resume,
    });

    registry.register(SlashCommand {
        name: "sessions",
        description: "List all saved sessions",
        handler: cmd_sessions,
    });

    registry.register(SlashCommand {
        name: "export",
        description: "Export current session to markdown",
        handler: cmd_export_session,
    });

    registry.register(SlashCommand {
        name: "delete_session",
        description: "Delete a session by ID",
        handler: cmd_delete_session,
    });

    registry.register(SlashCommand {
        name: "init",
        description: "Initialize default configuration and project structure",
        handler: cmd_init,
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
    use crate::skills::{loader::load_skill_registry, installer::{SkillInstaller, get_user_skills_dir}};
    use std::path::Path;

    let tokens: Vec<&str> = args.split_whitespace().collect();

    // No arguments - list all skills
    if tokens.is_empty() {
        let registry = load_skill_registry(Path::new(&ctx.cwd));
        let skills = registry.list();
        if skills.is_empty() {
            return CommandResult::message("No skills available. Use /skills install <name> to install from SkillsMP.");
        }
        let lines: Vec<_> = skills
            .iter()
            .map(|s| format!("  {}: {}", s.name, s.description))
            .collect();
        return CommandResult::message(format!("Available skills:\n{}", lines.join("\n")));
    }

    // Parse subcommand
    let subcommand = tokens[0];
    let rest = if tokens.len() > 1 { tokens[1..].join(" ") } else { String::new() };

    match subcommand {
        "list" => {
            // /skills list - list all installed skills
            let registry = load_skill_registry(Path::new(&ctx.cwd));
            let skills = registry.list();
            if skills.is_empty() {
                CommandResult::message("No skills installed.")
            } else {
                let lines: Vec<_> = skills
                    .iter()
                    .map(|s| format!("  {}: {}", s.name, s.description))
                    .collect();
                CommandResult::message(format!("Installed skills ({} total):\n{}", skills.len(), lines.join("\n")))
            }
        }
        "show" | "view" => {
            // /skills show <name> - show skill content
            if rest.is_empty() {
                return CommandResult::message("Usage: /skills show <name>");
            }
            let registry = load_skill_registry(Path::new(&ctx.cwd));
            match registry.get(&rest) {
                Some(skill) => CommandResult::message(skill.content.clone()),
                None => CommandResult::message(format!("Skill not found: {}", rest)),
            }
        }
        "remove" | "delete" => {
            // /skills remove <name> - remove a skill
            if rest.is_empty() {
                return CommandResult::message("Usage: /skills remove <name>");
            }
            let installer = SkillInstaller::new(&get_user_skills_dir());
            match installer.remove_skill(&rest) {
                Ok(true) => CommandResult::message(format!("Removed skill: {}", rest)),
                Ok(false) => CommandResult::message(format!("Skill not found: {}", rest)),
                Err(e) => CommandResult::message(format!("Failed to remove skill: {}", e)),
            }
        }
        "install" => {
            // /skills install <name|url> - install a skill from SkillsMP or GitHub
            if rest.is_empty() {
                return CommandResult::message("Usage: /skills install <name|github-url>\n\nExamples:\n  /skills install claude-api\n  /skills install https://github.com/user/repo/blob/main/skill.md");
            }

            let installer = SkillInstaller::new(&get_user_skills_dir());

            // Check if it's a GitHub URL
            if rest.starts_with("http") {
                match tokio::runtime::Handle::current().block_on(installer.install_from_github(&rest)) {
                    Ok(path) => CommandResult::message(format!("Installed skill from URL: {}", path)),
                    Err(e) => CommandResult::message(format!("Failed to install skill: {}", e)),
                }
            } else {
                // Search and install from SkillsMP (via GitHub)
                let search_query = rest.clone();
                match tokio::runtime::Handle::current().block_on(installer.search(&search_query)) {
                    Ok(skills) => {
                        if skills.is_empty() {
                            CommandResult::message(format!("No skills found for: {}", rest))
                        } else {
                            // Install the first result
                            let first_skill = &skills[0];
                            match tokio::runtime::Handle::current().block_on(installer.download_skill(&first_skill.skill_url, Some(&first_skill.name))) {
                                Ok(path) => CommandResult::message(format!("Installed skill '{}' from {}:\n  {}", first_skill.name, first_skill.author, path)),
                                Err(e) => CommandResult::message(format!("Failed to download skill: {}", e)),
                            }
                        }
                    }
                    Err(e) => CommandResult::message(format!("Search failed: {}", e)),
                }
            }
        }
        "search" => {
            // /skills search <query> - search SkillsMP
            if rest.is_empty() {
                return CommandResult::message("Usage: /skills search <query>");
            }
            let installer = SkillInstaller::new(&get_user_skills_dir());
            match tokio::runtime::Handle::current().block_on(installer.search(&rest)) {
                Ok(skills) => {
                    if skills.is_empty() {
                        CommandResult::message(format!("No skills found for: {}", rest))
                    } else {
                        let lines: Vec<_> = skills
                            .iter()
                            .take(10)
                            .map(|s| format!("  {} by {} - {}", s.name, s.author, s.description))
                            .collect();
                        CommandResult::message(format!("Found {} skills for '{}':\n{}", skills.len(), rest, lines.join("\n")))
                    }
                }
                Err(e) => CommandResult::message(format!("Search failed: {}", e)),
            }
        }
        _ => {
            // Unknown subcommand - try to show skill content as fallback
            let registry = load_skill_registry(Path::new(&ctx.cwd));
            match registry.get(args) {
                Some(skill) => CommandResult::message(skill.content.clone()),
                None => CommandResult::message("Usage: /skills <list|show <name>|remove <name>|install <name|url>|search <query>>"),
            }
        }
    }
}

fn cmd_plugin(args: &str, ctx: &CommandContext) -> CommandResult {
    use crate::plugins::loader::load_plugins;
    use crate::plugins::installer::{install_plugin_from_path, uninstall_plugin, enable_plugin, disable_plugin};

    let tokens: Vec<&str> = args.split_whitespace().collect();

    if tokens.is_empty() || tokens[0] == "list" {
        let plugins = load_plugins(ctx.settings, &ctx.cwd);
        if plugins.is_empty() {
            return CommandResult::message("No plugins discovered.");
        }
        let lines: Vec<_> = plugins
            .iter()
            .map(|p| {
                let status = if p.enabled { "✓" } else { "✗" };
                format!("  [{}] {} v{} - {}", status, p.name, p.version, p.description.as_deref().unwrap_or(""))
            })
            .collect();
        CommandResult::message(format!("Installed plugins:\n{}", lines.join("\n")))
    } else if tokens[0] == "install" {
        if tokens.len() < 2 {
            return CommandResult::message("Usage: /plugin install <path>");
        }
        match install_plugin_from_path(tokens[1], &ctx.cwd) {
            Ok(msg) => CommandResult::message(msg),
            Err(e) => CommandResult::message(format!("Failed to install plugin: {}", e)),
        }
    } else if tokens[0] == "uninstall" {
        if tokens.len() < 2 {
            return CommandResult::message("Usage: /plugin uninstall <name>");
        }
        match uninstall_plugin(tokens[1], &ctx.cwd) {
            Ok(msg) => CommandResult::message(msg),
            Err(e) => CommandResult::message(format!("Failed to uninstall plugin: {}", e)),
        }
    } else if tokens[0] == "enable" {
        if tokens.len() < 2 {
            return CommandResult::message("Usage: /plugin enable <name>");
        }
        match enable_plugin(tokens[1], &ctx.cwd) {
            Ok(msg) => CommandResult::message(msg),
            Err(e) => CommandResult::message(format!("Failed to enable plugin: {}", e)),
        }
    } else if tokens[0] == "disable" {
        if tokens.len() < 2 {
            return CommandResult::message("Usage: /plugin disable <name>");
        }
        match disable_plugin(tokens[1], &ctx.cwd) {
            Ok(msg) => CommandResult::message(msg),
            Err(e) => CommandResult::message(format!("Failed to disable plugin: {}", e)),
        }
    } else {
        CommandResult::message("Usage: /plugin [list|install PATH|uninstall NAME|enable NAME|disable NAME]")
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

fn cmd_memory(args: &str, ctx: &CommandContext) -> CommandResult {
    use crate::memory::{list_memory_files, add_memory_entry, remove_memory_entry, get_memory_entrypoint};

    let tokens: Vec<&str> = args.split_whitespace().collect();

    if tokens.is_empty() {
        // Show memory summary
        let memory_dir = MemoryManager::get_project_memory_dir(&ctx.cwd);
        let entrypoint = get_memory_entrypoint(&ctx.cwd);
        return CommandResult::message(format!(
            "Memory directory: {}\nEntrypoint: {}",
            memory_dir.display(),
            entrypoint.unwrap_or_else(|| "Not initialized".to_string())
        ));
    }

    let action = tokens[0];
    let rest = if tokens.len() > 1 {
        tokens[1..].join(" ")
    } else {
        String::new()
    };

    match action {
        "list" => {
            let files = list_memory_files(&ctx.cwd);
            if files.is_empty() {
                CommandResult::message("No memory files.")
            } else {
                let lines: Vec<_> = files
                    .iter()
                    .filter_map(|p| p.file_name().and_then(|s| s.to_str()))
                    .collect();
                CommandResult::message(lines.join("\n"))
            }
        }
        "show" => {
            if rest.is_empty() {
                return CommandResult::message("Usage: /memory show <name>");
            }
            match get_memory_entrypoint(&ctx.cwd) {
                Some(content) => CommandResult::message(content),
                None => CommandResult::message("No MEMORY.md found."),
            }
        }
        "add" => {
            // Format: /memory add TITLE :: CONTENT
            if let Some(separator_pos) = rest.find("::") {
                let title = rest[..separator_pos].trim();
                let content = rest[separator_pos + 2..].trim();

                if title.is_empty() || content.is_empty() {
                    return CommandResult::message("Usage: /memory add TITLE :: CONTENT");
                }

                match add_memory_entry(&ctx.cwd, title, content) {
                    Ok(path) => CommandResult::message(format!(
                        "Added memory entry: {}",
                        path.file_name().and_then(|s| s.to_str()).unwrap_or("unknown")
                    )),
                    Err(e) => CommandResult::message(format!("Error: {}", e)),
                }
            } else {
                CommandResult::message("Usage: /memory add TITLE :: CONTENT")
            }
        }
        "remove" => {
            if rest.is_empty() {
                return CommandResult::message("Usage: /memory remove <name>");
            }
            match remove_memory_entry(&ctx.cwd, &rest) {
                Ok(true) => CommandResult::message(format!("Removed memory entry: {}", rest)),
                Ok(false) => CommandResult::message(format!("Memory entry not found: {}", rest)),
                Err(e) => CommandResult::message(format!("Error: {}", e)),
            }
        }
        _ => CommandResult::message("Usage: /memory [list|show|add TITLE :: CONTENT|remove NAME]"),
    }
}

fn cmd_resume(args: &str, ctx: &CommandContext) -> CommandResult {
    use crate::services::session_storage::{list_session_snapshots, load_session_by_id, load_session_snapshot};

    let tokens: Vec<&str> = args.split_whitespace().collect();

    // /resume <session_id> - load a specific session
    if !tokens.is_empty() {
        let sid = tokens[0];
        match load_session_by_id(&ctx.cwd, sid) {
            Some(snapshot) => {
                let summary = snapshot.summary.as_deref().unwrap_or("(no summary)").chars().take(60).collect::<String>();
                CommandResult::message(format!(
                    "Restored {} messages from session {} ({})",
                    snapshot.messages.len(),
                    sid,
                    summary
                ))
            }
            None => CommandResult::message(format!("Session not found: {}", sid)),
        }
    } else {
        // /resume - list sessions
        let sessions = list_session_snapshots(&ctx.cwd, 10);
        if sessions.is_empty() {
            // Fall back to latest.json
            match load_session_snapshot(&ctx.cwd) {
                Some(snapshot) => {
                    return CommandResult::message(format!(
                        "Restored {} messages from the latest session.",
                        snapshot.messages.len()
                    ));
                }
                None => {
                    return CommandResult::message("No saved sessions found for this project.");
                }
            }
        }

        let mut lines = vec!["Saved sessions:".to_string()];
        for s in &sessions {
            let ts = s.created_at.format("%m/%d %H:%M");
            let summary = if s.summary.is_empty() {
                "(no summary)".to_string()
            } else {
                s.summary.chars().take(50).collect::<String>()
            };
            lines.push(format!(
                "  {}  {}  {}msg  {}",
                s.session_id, ts, s.message_count, summary
            ));
        }
        lines.push(String::new());
        lines.push("Use /resume <session_id> to restore a specific session.".to_string());
        CommandResult::message(lines.join("\n"))
    }
}

fn cmd_sessions(_args: &str, ctx: &CommandContext) -> CommandResult {
    use crate::services::session_storage::list_session_snapshots;

    let sessions = list_session_snapshots(&ctx.cwd, 20);

    if sessions.is_empty() {
        return CommandResult::message("No saved sessions found for this project.");
    }

    let mut lines = vec![format!("Saved sessions ({} total):", sessions.len())];
    for s in &sessions {
        let ts = s.created_at.format("%Y-%m-%d %H:%M");
        let summary = if s.summary.is_empty() {
            "(no summary)".to_string()
        } else {
            s.summary.chars().take(40).collect::<String>()
        };
        lines.push(format!(
            "  {}  {}  {} messages  {}",
            s.session_id, ts, s.message_count, summary
        ));
    }
    lines.push(String::new());
    lines.push("Commands:".to_string());
    lines.push("  /resume <session_id>  - Load a specific session".to_string());
    lines.push("  /export               - Export current session to markdown".to_string());
    lines.push("  /delete_session <id>  - Delete a session".to_string());

    CommandResult::message(lines.join("\n"))
}

fn cmd_export_session(_args: &str, ctx: &CommandContext) -> CommandResult {
    use crate::services::session_storage::{export_session_markdown, load_session_snapshot};

    // Try to load current session
    match load_session_snapshot(&ctx.cwd) {
        Some(snapshot) => {
            match export_session_markdown(&ctx.cwd, &snapshot.messages) {
                Ok(path) => {
                    CommandResult::message(format!("Session exported to: {}", path.display()))
                }
                Err(e) => CommandResult::error(format!("Failed to export session: {}", e)),
            }
        }
        None => {
            CommandResult::message("No active session to export. Start a conversation first.")
        }
    }
}

fn cmd_delete_session(args: &str, ctx: &CommandContext) -> CommandResult {
    use std::fs;

    let session_id = args.trim();
    if session_id.is_empty() {
        return CommandResult::error("Usage: /delete_session <session_id>");
    }

    let session_path = crate::services::session_storage::get_project_session_dir(&ctx.cwd)
        .join(format!("{}.json", session_id));

    if !session_path.exists() {
        return CommandResult::message(format!("Session not found: {}", session_id));
    }

    match fs::remove_file(&session_path) {
        Ok(_) => CommandResult::message(format!("Session {} deleted.", session_id)),
        Err(e) => CommandResult::error(format!("Failed to delete session: {}", e)),
    }
}

fn cmd_init(args: &str, ctx: &CommandContext) -> CommandResult {
    use crate::config::default_settings::{initialize_defaults, initialize_project};

    let tokens: Vec<&str> = args.split_whitespace().collect();
    let mut results = Vec::new();

    // Initialize global config if --global flag is passed or no args
    if tokens.is_empty() || tokens.contains(&"--global") || tokens.contains(&"-g") {
        match initialize_defaults() {
            Ok(msg) => results.push(msg),
            Err(e) => return CommandResult::message(format!("Error initializing global config: {}", e)),
        }
    }

    // Initialize project config if --project flag is passed or no args
    if tokens.is_empty() || tokens.contains(&"--project") || tokens.contains(&"-p") {
        match initialize_project(&ctx.cwd) {
            Ok(msg) => results.push(msg),
            Err(e) => return CommandResult::message(format!("Error initializing project: {}", e)),
        }
    }

    // Create CLAUDE.md if not exists
    let claude_md_path = std::path::Path::new(&ctx.cwd).join("CLAUDE.md");
    if !claude_md_path.exists() {
        let content = "# CLAUDE.md\n\nThis file provides guidance to Claude Code when working with code in this repository.\n\n## Project Overview\n\nTODO: Describe your project\n\n## Build & Run\n\n```bash\n# Build\ncargo build\n\n# Run\ncargo run\n```\n\n## Development\n\n```bash\n# Run tests\ncargo test\n\n# Lint\ncargo clippy -- -D warnings\n\n# Format\ncargo fmt\n```\n";
        if let Err(e) = std::fs::write(&claude_md_path, content) {
            return CommandResult::message(format!("Error creating CLAUDE.md: {}", e));
        }
        results.push(format!("Created: {}", claude_md_path.display()));
    }

    CommandResult::message(results.join("\n\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Settings;

    fn create_test_context() -> CommandContext<'static> {
        let registry = Box::leak(Box::new(CommandRegistry::new()));
        let settings = Box::leak(Box::new(Settings::default()));
        CommandContext {
            cwd: ".".to_string(),
            settings,
            registry,
        }
    }

    #[test]
    fn test_cmd_help() {
        let ctx = create_test_context();
        let result = cmd_help("", &ctx);
        assert!(result.message.unwrap().contains("Available commands"));
    }

    #[test]
    fn test_cmd_exit() {
        let ctx = create_test_context();
        let result = cmd_exit("", &ctx);
        assert!(result.should_exit);
    }

    #[test]
    fn test_cmd_clear() {
        let ctx = create_test_context();
        let result = cmd_clear("", &ctx);
        assert!(result.clear_screen);
        assert!(result.message.unwrap().contains("cleared"));
    }

    #[test]
    fn test_cmd_version() {
        let ctx = create_test_context();
        let result = cmd_version("", &ctx);
        assert!(result.message.unwrap().contains("v0.1.0"));
    }

    #[test]
    fn test_cmd_plugin_list() {
        let ctx = create_test_context();
        let result = cmd_plugin("list", &ctx);
        assert!(result.message.unwrap().contains("plugins"));
    }

    #[test]
    fn test_cmd_plugin_install_missing_path() {
        let ctx = create_test_context();
        let result = cmd_plugin("install", &ctx);
        assert!(result.message.unwrap().contains("Usage"));
    }

    #[test]
    fn test_cmd_hooks() {
        let ctx = create_test_context();
        let result = cmd_hooks("", &ctx);
        assert!(result.message.unwrap().contains("hooks"));
    }

    #[test]
    fn test_cmd_config_show() {
        let ctx = create_test_context();
        let result = cmd_config("show", &ctx);
        assert!(result.message.unwrap().contains("Model"));
    }

    #[test]
    fn test_cmd_memory_list() {
        let ctx = create_test_context();
        let result = cmd_memory("list", &ctx);
        assert!(result.message.is_some());
    }

    #[test]
    fn test_cmd_sessions() {
        let ctx = create_test_context();
        let result = cmd_sessions("", &ctx);
        assert!(result.message.is_some());
    }
}
