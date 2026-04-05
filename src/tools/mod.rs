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

pub use base::ToolRegistry;
pub use init::init_tools;
