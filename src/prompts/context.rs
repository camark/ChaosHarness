//! Context building for prompts

use std::path::Path;

pub fn build_context(_cwd: &str) -> String {
    // In a full implementation, this would gather:
    // - Current directory structure
    // - Relevant files
    // - Recent git history
    // - Project configuration

    String::from("Working directory context...")
}

pub fn read_claude_md(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}
