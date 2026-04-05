//! Tools initialization

use crate::tools::ToolRegistry;

// Core tools
use crate::tools::bash::BashTool;
use crate::tools::file_read::FileReadTool;
use crate::tools::file_write::FileWriteTool;
use crate::tools::file_edit::FileEditTool;
use crate::tools::glob::GlobTool;
use crate::tools::grep::GrepTool;
use crate::tools::web_fetch::WebFetchTool;
use crate::tools::web_search::WebSearchTool;
use crate::tools::notebook_edit::NotebookEditTool;
use crate::tools::ask_user::AskUserTool;
use crate::tools::directory_tree::DirectoryTreeTool;
use crate::tools::skill::SkillTool;

// Cron tools
use crate::tools::cron_create::CronCreateTool;
use crate::tools::cron_list::CronListTool;
use crate::tools::cron_delete::CronDeleteTool;
use crate::tools::cron_toggle::CronToggleTool;

// Task management tools
use crate::tools::task_create::TaskCreateTool;
use crate::tools::task_list::TaskListTool;
use crate::tools::task_get::TaskGetTool;
use crate::tools::task_update::TaskUpdateTool;
use crate::tools::task_stop::TaskStopTool;
use crate::tools::task_output::TaskOutputTool;

// Plan mode tools
use crate::tools::enter_plan_mode::EnterPlanModeTool;
use crate::tools::exit_plan_mode::ExitPlanModeTool;

// Worktree tools
use crate::tools::enter_worktree::EnterWorktreeTool;
use crate::tools::exit_worktree::ExitWorktreeTool;

// Team tools
use crate::tools::team_create::TeamCreateTool;
use crate::tools::team_delete::TeamDeleteTool;

// Utility tools
use crate::tools::sleep::SleepTool;
use crate::tools::todo_write::TodoWriteTool;
use crate::tools::config::ConfigTool;
use crate::tools::brief::BriefTool;
use crate::tools::tool_search::ToolSearchTool;
use crate::tools::send_message::SendMessageTool;

// MCP tools
use crate::tools::mcp_auth::McpAuthTool;
use crate::tools::read_mcp_resource::ReadMcpResourceTool;
use crate::tools::list_mcp_resources::ListMcpResourcesTool;
use crate::tools::remote_trigger::RemoteTriggerTool;

// LSP tool
use crate::tools::lsp::LspTool;

use std::env;
use std::env::current_dir;

/// Initialize all available tools
pub async fn init_tools() -> ToolRegistry {
    let registry = ToolRegistry::new();

    // Register core tools
    registry.register(BashTool).await;
    registry.register(FileReadTool).await;
    registry.register(FileWriteTool).await;
    registry.register(FileEditTool).await;
    registry.register(GlobTool).await;
    registry.register(GrepTool).await;
    registry.register(WebFetchTool).await;
    registry.register(WebSearchTool).await;
    registry.register(NotebookEditTool).await;
    registry.register(AskUserTool::new(None)).await;
    registry.register(DirectoryTreeTool::new(env::current_dir().unwrap_or_default())).await;
    registry.register(SkillTool::new(current_dir().unwrap_or_default())).await;

    // Register cron tools
    registry.register(CronCreateTool).await;
    registry.register(CronListTool).await;
    registry.register(CronDeleteTool).await;
    registry.register(CronToggleTool).await;

    // Register task management tools
    registry.register(TaskCreateTool).await;
    registry.register(TaskListTool).await;
    registry.register(TaskGetTool).await;
    registry.register(TaskUpdateTool).await;
    registry.register(TaskStopTool).await;
    registry.register(TaskOutputTool).await;

    // Register plan mode tools
    registry.register(EnterPlanModeTool).await;
    registry.register(ExitPlanModeTool).await;

    // Register worktree tools
    registry.register(EnterWorktreeTool).await;
    registry.register(ExitWorktreeTool).await;

    // Register team tools
    registry.register(TeamCreateTool).await;
    registry.register(TeamDeleteTool).await;

    // Register utility tools
    registry.register(SleepTool).await;
    registry.register(TodoWriteTool).await;
    registry.register(ConfigTool).await;
    registry.register(BriefTool).await;
    registry.register(ToolSearchTool).await;
    registry.register(SendMessageTool).await;

    // Register MCP tools
    registry.register(McpAuthTool).await;
    registry.register(ReadMcpResourceTool).await;
    registry.register(ListMcpResourcesTool).await;
    registry.register(RemoteTriggerTool).await;

    // Register LSP tool
    registry.register(LspTool).await;

    registry
}
