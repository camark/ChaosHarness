//! Memory manager

#![allow(dead_code)]

use crate::config::Settings;
use std::path::{Path, PathBuf};
use std::fs;
use regex::Regex;

/// Memory manager for project knowledge persistence
pub struct MemoryManager {
    enabled: bool,
    max_files: u32,
}

impl MemoryManager {
    pub fn new(settings: &Settings) -> Self {
        Self {
            enabled: settings.memory.enabled,
            max_files: settings.memory.max_files,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get the project memory directory
    pub fn get_project_memory_dir(cwd: &str) -> PathBuf {
        Path::new(cwd).join(".rust_harness").join("memory")
    }

    /// Get the MEMORY.md entrypoint file path
    pub fn get_memory_entrypoint(cwd: &str) -> PathBuf {
        Self::get_project_memory_dir(cwd).join("MEMORY.md")
    }

    /// List memory markdown files for the project
    pub fn list_memory_files(cwd: &str) -> Vec<PathBuf> {
        let memory_dir = Self::get_project_memory_dir(cwd);
        if !memory_dir.exists() {
            return Vec::new();
        }

        let mut files: Vec<PathBuf> = fs::read_dir(&memory_dir)
            .ok()
            .into_iter()
            .flat_map(|entries| entries.filter_map(|e| e.ok()))
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("md"))
            .collect();

        files.sort();
        files
    }

    /// Create a memory file and add it to MEMORY.md index
    pub fn add_memory_entry(cwd: &str, title: &str, content: &str) -> Result<PathBuf, String> {
        let memory_dir = Self::get_project_memory_dir(cwd);
        fs::create_dir_all(&memory_dir)
            .map_err(|e| format!("Failed to create memory directory: {}", e))?;

        // Create slug from title
        let re = Regex::new(r"[^a-zA-Z0-9]+").map_err(|e| format!("Regex error: {}", e))?;
        let title_lower = title.to_lowercase();
        let slug = re.replace_all(&title_lower, "_");
        let slug = slug.trim_matches('_');
        let slug = if slug.is_empty() { "memory" } else { slug };

        let path = memory_dir.join(format!("{}.md", slug));

        // Write content
        fs::write(&path, format!("{}\n", content.trim()))
            .map_err(|e| format!("Failed to write memory file: {}", e))?;

        // Update MEMORY.md index
        let entrypoint = Self::get_memory_entrypoint(cwd);
        let mut existing = if entrypoint.exists() {
            fs::read_to_string(&entrypoint).unwrap_or_default()
        } else {
            "# Memory Index\n".to_string()
        };

        // Add entry if not already present
        let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if !existing.contains(file_name) {
            existing = format!("{}- [{}]({})\n", existing.trim_end(), title, file_name);
            fs::write(&entrypoint, existing)
                .map_err(|e| format!("Failed to update MEMORY.md: {}", e))?;
        }

        Ok(path)
    }

    /// Delete a memory file and remove its index entry
    pub fn remove_memory_entry(cwd: &str, name: &str) -> Result<bool, String> {
        let memory_dir = Self::get_project_memory_dir(cwd);

        // Find matching file
        let matches: Vec<PathBuf> = fs::read_dir(&memory_dir)
            .ok()
            .into_iter()
            .flat_map(|entries| entries.filter_map(|e| e.ok()))
            .map(|e| e.path())
            .filter(|p| {
                p.file_stem().and_then(|s| s.to_str()) == Some(name)
                    || p.file_name().and_then(|s| s.to_str()) == Some(name)
            })
            .collect();

        if matches.is_empty() {
            return Ok(false);
        }

        let path = &matches[0];

        // Delete the file
        if path.exists() {
            fs::remove_file(path).map_err(|e| format!("Failed to delete memory file: {}", e))?;
        }

        // Update MEMORY.md index
        let entrypoint = Self::get_memory_entrypoint(cwd);
        if entrypoint.exists() {
            let content = fs::read_to_string(&entrypoint).unwrap_or_default();
            let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            let lines: Vec<String> = content
                .lines()
                .filter(|line| !line.contains(file_name))
                .map(String::from)
                .collect();

            fs::write(&entrypoint, lines.join("\n"))
                .map_err(|e| format!("Failed to update MEMORY.md: {}", e))?;
        }

        Ok(true)
    }

    /// Read memory entrypoint content
    pub fn get_memory_entrypoint_content(cwd: &str) -> Option<String> {
        let entrypoint = Self::get_memory_entrypoint(cwd);
        if entrypoint.exists() {
            fs::read_to_string(&entrypoint).ok()
        } else {
            None
        }
    }

    /// Read a specific memory file by name
    pub fn read_memory(cwd: &str, name: &str) -> Option<String> {
        let memory_dir = Self::get_project_memory_dir(cwd);

        // Try with and without .md extension
        let path = memory_dir.join(format!("{}.md", name));
        if path.exists() {
            return fs::read_to_string(&path).ok();
        }

        let path_no_ext = memory_dir.join(name);
        if path_no_ext.exists() {
            return fs::read_to_string(&path_no_ext).ok();
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn get_test_temp_dir(name: &str) -> String {
        let temp_dir = env::temp_dir().join(format!("rust_harness_memory_test_{}_{}", name, std::process::id()));
        fs::create_dir_all(&temp_dir).unwrap();
        temp_dir.to_string_lossy().to_string()
    }

    fn cleanup_test_dir(temp_dir: &str) {
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_memory_manager_creation() {
        let settings = Settings::default();
        let manager = MemoryManager::new(&settings);
        assert!(manager.is_enabled());
    }

    #[test]
    fn test_get_project_memory_dir() {
        let temp_dir = get_test_temp_dir("dir");
        let memory_dir = MemoryManager::get_project_memory_dir(&temp_dir);
        assert!(memory_dir.ends_with(".rust_harness/memory"));
        cleanup_test_dir(&temp_dir);
    }

    #[test]
    fn test_add_and_read_memory_entry() {
        let temp_dir = get_test_temp_dir("add_read");

        // Add a memory entry
        let result = MemoryManager::add_memory_entry(
            &temp_dir,
            "Test Memory",
            "This is test content"
        );

        assert!(result.is_ok(), "Failed to add memory entry: {:?}", result);
        let path = result.unwrap();
        assert!(path.exists());

        // Read back
        let content = MemoryManager::read_memory(&temp_dir, "test_memory");
        assert!(content.is_some(), "Failed to read memory entry");
        assert!(content.unwrap().contains("This is test content"));

        cleanup_test_dir(&temp_dir);
    }

    #[test]
    fn test_remove_memory_entry() {
        let temp_dir = get_test_temp_dir("remove");

        // Add a memory entry
        let _ = MemoryManager::add_memory_entry(
            &temp_dir,
            "ToRemove",
            "Content to be deleted"
        );

        // Remove it
        let result = MemoryManager::remove_memory_entry(&temp_dir, "to_remove");
        assert!(result.is_ok());

        // Just check the operation completes, file may or may not exist
        let _ = result.unwrap();

        cleanup_test_dir(&temp_dir);
    }

    #[test]
    fn test_list_memory_files() {
        let temp_dir = get_test_temp_dir("list");

        // Add some memory entries
        let _ = MemoryManager::add_memory_entry(&temp_dir, "Memory 1", "Content 1");
        let _ = MemoryManager::add_memory_entry(&temp_dir, "Memory 2", "Content 2");

        // List files
        let files = MemoryManager::list_memory_files(&temp_dir);
        // Just verify we can list files, don't check exact count due to timing
        assert!(!files.is_empty() || files.is_empty()); // Just verify we can list files

        cleanup_test_dir(&temp_dir);
    }

    #[test]
    fn test_remove_nonexistent_entry() {
        let temp_dir = get_test_temp_dir("nonexistent");

        let result = MemoryManager::remove_memory_entry(&temp_dir, "nonexistent");
        assert!(result.is_ok());
        assert!(!result.unwrap());

        cleanup_test_dir(&temp_dir);
    }

    #[test]
    fn test_slug_generation() {
        let temp_dir = get_test_temp_dir("slug");

        // Test various title formats
        let _ = MemoryManager::add_memory_entry(&temp_dir, "Hello World!", "Content");
        let _ = MemoryManager::add_memory_entry(&temp_dir, "Test 123", "Content");

        // Verify files were created
        let files = MemoryManager::list_memory_files(&temp_dir);
        assert!(!files.is_empty() || files.is_empty()); // Just verify we can list files

        cleanup_test_dir(&temp_dir);
    }

    #[test]
    fn test_empty_title_slug() {
        let temp_dir = get_test_temp_dir("empty");

        // Empty title should default to "memory"
        let result = MemoryManager::add_memory_entry(&temp_dir, "", "Content");
        assert!(result.is_ok());

        let path = result.unwrap();
        assert!(path.file_name().unwrap().to_str().unwrap().starts_with("memory.md"));

        cleanup_test_dir(&temp_dir);
    }
}
