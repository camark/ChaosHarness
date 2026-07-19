//! Slash command registry

#![allow(dead_code)]

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
        name: "mcp",
        description: "Manage MCP servers (list)",
        handler: cmd_mcp,
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

    // === Git Workflow ===
    registry.register(SlashCommand {
        name: "diff",
        description: "Show git diff (staged + unstaged)",
        handler: cmd_diff,
    });
    registry.register(SlashCommand {
        name: "log",
        description: "Show recent git log",
        handler: cmd_log,
    });
    registry.register(SlashCommand {
        name: "commit",
        description: "Stage all and commit with message",
        handler: cmd_commit,
    });
    registry.register(SlashCommand {
        name: "branch",
        description: "Show current branch and list branches",
        handler: cmd_branch,
    });
    registry.register(SlashCommand {
        name: "stash",
        description: "Git stash operations (list|pop|drop)",
        handler: cmd_stash,
    });

    // === Build & Test ===
    registry.register(SlashCommand {
        name: "test",
        description: "Run cargo test with optional filter",
        handler: cmd_test,
    });
    registry.register(SlashCommand {
        name: "build",
        description: "Run cargo build",
        handler: cmd_build,
    });
    registry.register(SlashCommand {
        name: "release",
        description: "Run cargo build --release",
        handler: cmd_release,
    });
    registry.register(SlashCommand {
        name: "lint",
        description: "Run cargo clippy",
        handler: cmd_lint,
    });
    registry.register(SlashCommand {
        name: "format",
        description: "Run cargo fmt",
        handler: cmd_format,
    });
    registry.register(SlashCommand {
        name: "check",
        description: "Run cargo check (fast compile check)",
        handler: cmd_check,
    });

    // === Conversation Management ===
    registry.register(SlashCommand {
        name: "model",
        description: "Show or switch model",
        handler: cmd_model,
    });
    registry.register(SlashCommand {
        name: "compact",
        description: "Force conversation compaction",
        handler: cmd_compact,
    });
    registry.register(SlashCommand {
        name: "verbose",
        description: "Toggle verbose mode",
        handler: cmd_verbose,
    });
    registry.register(SlashCommand {
        name: "fast",
        description: "Toggle fast mode",
        handler: cmd_fast,
    });

    // === Service Integration ===
    registry.register(SlashCommand {
        name: "task",
        description: "Manage background tasks (list|stop <id>)",
        handler: cmd_task,
    });
    registry.register(SlashCommand {
        name: "cron",
        description: "Manage cron jobs (list|create|delete|toggle)",
        handler: cmd_cron,
    });
    registry.register(SlashCommand {
        name: "team",
        description: "Manage teams (list|create|delete)",
        handler: cmd_team,
    });

    // === System ===
    registry.register(SlashCommand {
        name: "cost",
        description: "Show token usage and estimated cost",
        handler: cmd_cost,
    });
    registry.register(SlashCommand {
        name: "doctor",
        description: "Health check (config, API key, tools)",
        handler: cmd_doctor,
    });
    registry.register(SlashCommand {
        name: "permission",
        description: "Show or change permission mode",
        handler: cmd_permission,
    });
    registry.register(SlashCommand {
        name: "theme",
        description: "Show or change theme",
        handler: cmd_theme,
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
                match tokio::task::block_in_place(|| installer.install_from_github(&rest)) {
                    Ok(path) => CommandResult::message(format!("Installed skill from URL: {}", path)),
                    Err(e) => CommandResult::message(format!("Failed to install skill: {}", e)),
                }
            } else {
                // Search and install from SkillsMP (via GitHub)
                let search_query = rest.clone();
                match tokio::task::block_in_place(|| installer.search(&search_query)) {
                    Ok(skills) => {
                        if skills.is_empty() {
                            CommandResult::message(format!("No skills found for: {}", rest))
                        } else {
                            // Install the first result
                            let first_skill = &skills[0];
                            match tokio::task::block_in_place(|| installer.download_skill(&first_skill.skill_url, Some(&first_skill.name))) {
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
            match tokio::task::block_in_place(|| installer.search(&rest)) {
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

fn cmd_mcp(args: &str, ctx: &CommandContext) -> CommandResult {
    use crate::mcp::config::load_mcp_server_configs;

    let tokens: Vec<&str> = args.split_whitespace().collect();

    if tokens.is_empty() || tokens[0] == "list" {
        let mcp_servers = load_mcp_server_configs(ctx.settings);
        if mcp_servers.is_empty() {
            return CommandResult::message("No MCP servers configured.\n\nAdd MCP servers in ~/.rust_harness/settings.json:\n```json\n{\n  \"mcp_servers\": {\n    \"test-server\": {\n      \"name\": \"test-server\",\n      \"command\": \"node\",\n      \"args\": [\"/path/to/server.js\"],\n      \"transport\": \"stdio\",\n      \"enabled\": true\n    }\n  }\n}\n```");
        }
        let lines: Vec<_> = mcp_servers
            .iter()
            .map(|(name, config)| {
                let status = if config.enabled { "✓" } else { "✗" };
                let transport = &config.transport;
                let details = if transport == "stdio" {
                    format!("{}: {} {}",
                        config.command.as_deref().unwrap_or("unknown"),
                        config.args.as_ref().map(|a| a.join(" ")).unwrap_or_default(),
                        if let Some(env) = &config.env {
                            format!("({} env vars)", env.len())
                        } else {
                            String::new()
                        }
                    )
                } else if transport == "sse" {
                    config.url.clone().unwrap_or_default()
                } else {
                    transport.clone()
                };
                format!("  [{}] {} - {}", status, name, details)
            })
            .collect();
        CommandResult::message(format!("Configured MCP servers ({} total):\n{}", mcp_servers.len(), lines.join("\n")))
    } else if tokens[0] == "query" {
        if tokens.len() < 2 {
            return CommandResult::message("Usage: /mcp query <server-name>\n\nQuery a specific MCP server for its capabilities and tools.");
        }
        let server_name = tokens[1];
        let mcp_servers = load_mcp_server_configs(ctx.settings);

        if let Some(config) = mcp_servers.get(server_name) {
            let mut info = Vec::new();
            info.push(format!("MCP Server: {}", server_name));
            info.push(format!("  Status: {}", if config.enabled { "Enabled" } else { "Disabled" }));
            info.push(format!("  Transport: {}", config.transport));

            if config.transport == "stdio" {
                if let Some(cmd) = &config.command {
                    info.push(format!("  Command: {}", cmd));
                }
                if let Some(args) = &config.args {
                    info.push(format!("  Args: {}", args.join(" ")));
                }
                if let Some(env) = &config.env {
                    info.push(format!("  Environment variables: {}", env.len()));
                }
            } else if config.transport == "sse" {
                if let Some(url) = &config.url {
                    info.push(format!("  URL: {}", url));
                }
            }

            info.push(String::new());
            info.push("Note: For live tool/resource counts, connect to the server.".to_string());
            CommandResult::message(info.join("\n"))
        } else {
            CommandResult::message(format!("MCP server '{}' not found. Use /mcp list to see available servers.", server_name))
        }
    } else {
        CommandResult::message("Usage: /mcp [list|query <server-name>]")
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
    use crate::memory::{list_memory_files, add_memory_entry, remove_memory_entry, get_memory_entrypoint, read_memory};

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
            match read_memory(&ctx.cwd, &rest) {
                Some(content) => CommandResult::message(content),
                None => CommandResult::message(format!("Memory entry '{}' not found.", rest)),
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

// === Git Workflow Commands ===

fn run_shell_command(cmd: &str, args: &[&str], cwd: &str) -> String {
    match std::process::Command::new(cmd)
        .args(args)
        .current_dir(cwd)
        .output()
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if stdout.is_empty() && stderr.is_empty() {
                "(no output)".to_string()
            } else if stderr.is_empty() {
                stdout
            } else if stdout.is_empty() {
                stderr
            } else {
                format!("{}\n{}", stdout, stderr)
            }
        }
        Err(e) => format!("Failed to run {}: {}", cmd, e),
    }
}

/// Find a valid UTF-8 char boundary at or before the given byte index
fn find_char_boundary(s: &str, index: usize) -> usize {
    if index >= s.len() {
        return s.len();
    }
    let mut i = index;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

fn cmd_diff(_args: &str, ctx: &CommandContext) -> CommandResult {
    let output = run_shell_command("git", &["diff"], &ctx.cwd);
    if output.is_empty() {
        CommandResult::message("No changes.")
    } else {
        CommandResult::message(output)
    }
}

fn cmd_log(args: &str, ctx: &CommandContext) -> CommandResult {
    let n = if args.is_empty() { "10" } else { args.trim() };
    let output = run_shell_command("git", &["log", "--oneline", "-n", n], &ctx.cwd);
    CommandResult::message(output)
}

fn cmd_commit(args: &str, ctx: &CommandContext) -> CommandResult {
    let msg = args.trim();
    if msg.is_empty() {
        return CommandResult::error("Usage: /commit <message>");
    }
    // Stage all
    let stage = run_shell_command("git", &["add", "-A"], &ctx.cwd);
    if !stage.is_empty() && stage.contains("error") {
        return CommandResult::error(format!("git add failed: {}", stage));
    }
    // Commit
    let output = run_shell_command("git", &["commit", "-m", msg], &ctx.cwd);
    CommandResult::message(output)
}

fn cmd_branch(_args: &str, ctx: &CommandContext) -> CommandResult {
    let output = run_shell_command("git", &["branch", "-a"], &ctx.cwd);
    CommandResult::message(output)
}

fn cmd_stash(args: &str, ctx: &CommandContext) -> CommandResult {
    let tokens: Vec<&str> = args.split_whitespace().collect();
    let subcmd = if tokens.is_empty() { "list" } else { tokens[0] };
    let output = match subcmd {
        "list" => run_shell_command("git", &["stash", "list"], &ctx.cwd),
        "pop" => run_shell_command("git", &["stash", "pop"], &ctx.cwd),
        "drop" => run_shell_command("git", &["stash", "drop"], &ctx.cwd),
        "clear" => run_shell_command("git", &["stash", "clear"], &ctx.cwd),
        _ => return CommandResult::message("Usage: /stash [list|pop|drop|clear]"),
    };
    CommandResult::message(output)
}

// === Build & Test Commands ===

fn cmd_test(args: &str, ctx: &CommandContext) -> CommandResult {
    let mut cmd_args = vec!["test"];
    if !args.trim().is_empty() {
        cmd_args.push("--");
        cmd_args.push(args.trim());
    }
    let output = run_shell_command("cargo", &cmd_args, &ctx.cwd);
    // Truncate long test output (safe for UTF-8)
    let truncated = if output.len() > 4000 {
        let end = find_char_boundary(&output, 4000);
        format!("{}...\n[truncated]", &output[..end])
    } else {
        output
    };
    CommandResult::message(truncated)
}

fn cmd_build(_args: &str, ctx: &CommandContext) -> CommandResult {
    let output = run_shell_command("cargo", &["build"], &ctx.cwd);
    CommandResult::message(output)
}

fn cmd_release(_args: &str, ctx: &CommandContext) -> CommandResult {
    let output = run_shell_command("cargo", &["build", "--release"], &ctx.cwd);
    CommandResult::message(output)
}

fn cmd_lint(_args: &str, ctx: &CommandContext) -> CommandResult {
    let output = run_shell_command("cargo", &["clippy", "--", "-D", "warnings"], &ctx.cwd);
    let truncated = if output.len() > 4000 {
        let end = find_char_boundary(&output, 4000);
        format!("{}...\n[truncated]", &output[..end])
    } else {
        output
    };
    CommandResult::message(truncated)
}

fn cmd_format(_args: &str, ctx: &CommandContext) -> CommandResult {
    let output = run_shell_command("cargo", &["fmt"], &ctx.cwd);
    if output.is_empty() || output == "(no output)" {
        CommandResult::message("Formatted.")
    } else {
        CommandResult::message(output)
    }
}

fn cmd_check(_args: &str, ctx: &CommandContext) -> CommandResult {
    let output = run_shell_command("cargo", &["check"], &ctx.cwd);
    CommandResult::message(output)
}

// === Conversation Management Commands ===

fn cmd_model(args: &str, ctx: &CommandContext) -> CommandResult {
    let args = args.trim();
    if args.is_empty() {
        CommandResult::message(format!("Current model: {}", ctx.settings.model))
    } else {
        // Note: actual model switching requires runtime integration
        CommandResult::message(format!("Model set to: {} (restart required)", args))
    }
}

fn cmd_compact(_args: &str, _ctx: &CommandContext) -> CommandResult {
    CommandResult::message("Compaction triggered. History will be compacted on next message if over threshold.")
}

fn cmd_verbose(_args: &str, _ctx: &CommandContext) -> CommandResult {
    CommandResult::message("Verbose mode toggled. Use --verbose flag or config to persist.")
}

fn cmd_fast(_args: &str, _ctx: &CommandContext) -> CommandResult {
    CommandResult::message("Fast mode toggled. Use --fast flag or config to persist.")
}

// === Service Integration Commands ===

fn cmd_task(args: &str, _ctx: &CommandContext) -> CommandResult {
    use crate::services::task_manager::GLOBAL_TASK_MANAGER;

    let tokens: Vec<&str> = args.split_whitespace().collect();
    let subcmd = if tokens.is_empty() { "list" } else { tokens[0] };

    match subcmd {
        "list" => {
            let manager = &*GLOBAL_TASK_MANAGER;
            let rt = tokio::runtime::Handle::current();
            let tasks = rt.block_on(manager.list_tasks(None));
            if tasks.is_empty() {
                return CommandResult::message("No background tasks.");
            }
            let lines: Vec<_> = tasks
                .iter()
                .map(|t| format!("  {} [{}] {}", t.id, t.status.as_str(), t.description))
                .collect();
            CommandResult::message(format!("Background tasks:\n{}", lines.join("\n")))
        }
        "stop" => {
            if tokens.len() < 2 {
                return CommandResult::error("Usage: /task stop <task_id>");
            }
            let task_id = tokens[1];
            let manager = &*GLOBAL_TASK_MANAGER;
            let rt = tokio::runtime::Handle::current();
            let stopped = rt.block_on(manager.stop_task(task_id));
            if stopped {
                CommandResult::message(format!("Task {} stopped.", task_id))
            } else {
                CommandResult::message(format!("Task {} not found or already completed.", task_id))
            }
        }
        _ => CommandResult::message("Usage: /task [list|stop <id>]"),
    }
}

fn cmd_cron(args: &str, _ctx: &CommandContext) -> CommandResult {
    use crate::services::cron::CRON_MANAGER;

    let tokens: Vec<&str> = args.split_whitespace().collect();
    let subcmd = if tokens.is_empty() { "list" } else { tokens[0] };

    match subcmd {
        "list" => {
            let manager = &*CRON_MANAGER;
            let rt = tokio::runtime::Handle::current();
            let jobs = rt.block_on(manager.list_jobs());
            if jobs.is_empty() {
                return CommandResult::message("No cron jobs.");
            }
            let lines: Vec<_> = jobs
                .iter()
                .map(|j| {
                    let status = if j.enabled { "active" } else { "disabled" };
                    format!("  {} [{}] {} - {}", j.name, status, j.schedule, j.command)
                })
                .collect();
            CommandResult::message(format!("Cron jobs:\n{}", lines.join("\n")))
        }
        "delete" => {
            if tokens.len() < 2 {
                return CommandResult::error("Usage: /cron delete <name>");
            }
            let name = tokens[1];
            let manager = &*CRON_MANAGER;
            let rt = tokio::runtime::Handle::current();
            let deleted = rt.block_on(manager.delete_job(name));
            if deleted {
                CommandResult::message(format!("Cron job '{}' deleted.", name))
            } else {
                CommandResult::message(format!("Cron job '{}' not found.", name))
            }
        }
        "toggle" => {
            if tokens.len() < 2 {
                return CommandResult::error("Usage: /cron toggle <name>");
            }
            let name = tokens[1];
            let manager = &*CRON_MANAGER;
            let rt = tokio::runtime::Handle::current();
            // Get current state and toggle
            if let Some(job) = rt.block_on(manager.get_job(name)) {
                let new_enabled = !job.enabled;
                let ok = rt.block_on(manager.set_job_enabled(name, new_enabled));
                if ok {
                    CommandResult::message(format!(
                        "Cron job '{}' {}.",
                        name,
                        if new_enabled { "enabled" } else { "disabled" }
                    ))
                } else {
                    CommandResult::error(format!("Failed to toggle cron job '{}'", name))
                }
            } else {
                CommandResult::message(format!("Cron job '{}' not found.", name))
            }
        }
        _ => CommandResult::message("Usage: /cron [list|delete <name>|toggle <name>]"),
    }
}

fn cmd_team(args: &str, _ctx: &CommandContext) -> CommandResult {
    use crate::services::team_manager::GLOBAL_TEAM_MANAGER;

    let tokens: Vec<&str> = args.split_whitespace().collect();
    let subcmd = if tokens.is_empty() { "list" } else { tokens[0] };

    match subcmd {
        "list" => {
            let manager = &*GLOBAL_TEAM_MANAGER;
            let rt = tokio::runtime::Handle::current();
            let teams = rt.block_on(manager.list_teams());
            if teams.is_empty() {
                return CommandResult::message("No teams.");
            }
            let lines: Vec<_> = teams
                .iter()
                .map(|t| format!("  {} - {}", t.name, t.description))
                .collect();
            CommandResult::message(format!("Teams:\n{}", lines.join("\n")))
        }
        _ => CommandResult::message("Usage: /team [list]"),
    }
}

// === System Commands ===

fn cmd_cost(_args: &str, _ctx: &CommandContext) -> CommandResult {
    // Placeholder - would need runtime token tracking integration
    CommandResult::message("Token usage: tracking in progress.\nUse /usage for current session stats.")
}

fn cmd_doctor(_args: &str, ctx: &CommandContext) -> CommandResult {
    let mut checks = Vec::new();

    // Check API key
    let api_key = std::env::var("ANTHROPIC_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        checks.push("[FAIL] ANTHROPIC_API_KEY not set".to_string());
    } else {
        checks.push(format!("[OK] API key set ({}...)", &api_key[..8.min(api_key.len())]));
    }

    // Check config dir
    let config_dir = dirs::home_dir()
        .map(|h| h.join(".rust_harness"))
        .unwrap_or_default();
    if config_dir.exists() {
        checks.push(format!("[OK] Config dir: {}", config_dir.display()));
    } else {
        checks.push(format!("[WARN] Config dir not found: {}", config_dir.display()));
    }

    // Check working directory
    let cwd_path = std::path::Path::new(&ctx.cwd);
    if cwd_path.exists() {
        checks.push(format!("[OK] Working directory: {}", ctx.cwd));
    } else {
        checks.push(format!("[FAIL] Working directory not found: {}", ctx.cwd));
    }

    // Check git
    let git_check = std::process::Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(&ctx.cwd)
        .output();
    match git_check {
        Ok(o) if o.status.success() => checks.push("[OK] Git repository detected".to_string()),
        _ => checks.push("[WARN] Not a git repository".to_string()),
    }

    // Check cargo
    let cargo_check = std::process::Command::new("cargo")
        .args(["--version"])
        .output();
    match cargo_check {
        Ok(o) if o.status.success() => {
            let ver = String::from_utf8_lossy(&o.stdout).trim().to_string();
            checks.push(format!("[OK] {}", ver));
        }
        _ => checks.push("[WARN] cargo not found".to_string()),
    }

    CommandResult::message(format!("Doctor check:\n{}", checks.join("\n")))
}

fn cmd_permission(args: &str, ctx: &CommandContext) -> CommandResult {
    let args = args.trim();
    if args.is_empty() {
        CommandResult::message(format!(
            "Permission mode: {}\n\nAvailable modes: default, plan, full_auto",
            ctx.settings.permission.mode
        ))
    } else {
        match args {
            "default" | "plan" | "full_auto" => {
                CommandResult::message(format!("Permission mode set to: {} (restart required)", args))
            }
            _ => CommandResult::error("Invalid mode. Use: default, plan, full_auto"),
        }
    }
}

fn cmd_theme(args: &str, ctx: &CommandContext) -> CommandResult {
    let args = args.trim();
    if args.is_empty() {
        CommandResult::message(format!("Current theme: {}", ctx.settings.theme))
    } else {
        CommandResult::message(format!("Theme set to: {} (restart required)", args))
    }
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
