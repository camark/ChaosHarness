//! Memory module for persistent context

pub mod manager;
pub mod paths;
pub mod types;

pub use manager::MemoryManager;

/// List memory files for a project
pub fn list_memory_files(cwd: &str) -> Vec<std::path::PathBuf> {
    MemoryManager::list_memory_files(cwd)
}

/// Add a memory entry
pub fn add_memory_entry(cwd: &str, title: &str, content: &str) -> Result<std::path::PathBuf, String> {
    MemoryManager::add_memory_entry(cwd, title, content)
}

/// Remove a memory entry
pub fn remove_memory_entry(cwd: &str, name: &str) -> Result<bool, String> {
    MemoryManager::remove_memory_entry(cwd, name)
}

/// Get the memory entrypoint content
pub fn get_memory_entrypoint(cwd: &str) -> Option<String> {
    MemoryManager::get_memory_entrypoint_content(cwd)
}
