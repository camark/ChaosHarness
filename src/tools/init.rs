//! Tools initialization

use crate::tools::ToolRegistry;
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

/// Initialize all available tools
pub async fn init_tools() -> ToolRegistry {
    let registry = ToolRegistry::new();

    // Register Phase 1 tools
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

    registry
}
