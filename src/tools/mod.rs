//! Tools module - AI agent toolkit

pub mod base;
pub mod bash;
pub mod file_read;
pub mod file_write;
pub mod file_edit;
pub mod glob;
pub mod grep;
pub mod web_fetch;
pub mod web_search;
pub mod notebook_edit;
pub mod ask_user;
pub mod directory_tree;
pub mod skill;
pub mod init;
pub mod mcp;
pub mod path_util;

// Cron tools
pub mod cron_create;
pub mod cron_list;
pub mod cron_delete;
pub mod cron_toggle;

// Task management tools
pub mod task_create;
pub mod task_list;
pub mod task_get;
pub mod task_update;
pub mod task_stop;
pub mod task_output;

// Plan mode tools
pub mod enter_plan_mode;
pub mod exit_plan_mode;

// Worktree tools
pub mod enter_worktree;
pub mod exit_worktree;

// Team tools
pub mod team_create;
pub mod team_delete;

// Utility tools
pub mod sleep;
pub mod todo_write;
pub mod config;
pub mod brief;
pub mod tool_search;
pub mod send_message;

// MCP tools
pub mod mcp_auth;
pub mod read_mcp_resource;
pub mod list_mcp_resources;
pub mod remote_trigger;

// LSP tool
pub mod lsp;

// Multi-agent tools
pub mod swarm_create;
pub mod swarm_run;

pub use base::ToolRegistry;
pub use init::init_tools;
