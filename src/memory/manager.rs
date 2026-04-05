//! Memory manager

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
