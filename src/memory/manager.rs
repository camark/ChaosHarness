//! Memory manager

use crate::config::Settings;
use crate::memory::types::MemoryEntry;
use std::path::Path;
use std::fs;

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

    #[allow(dead_code)]
    pub fn save_memory(&self, path: &Path, entry: &MemoryEntry) -> Result<(), String> {
        if !self.enabled {
            return Err("Memory is disabled".to_string());
        }

        let content = serde_json::to_string_pretty(entry)
            .map_err(|e| format!("Failed to serialize memory entry: {}", e))?;

        fs::write(path, content)
            .map_err(|e| format!("Failed to write memory file: {}", e))?;

        Ok(())
    }

    #[allow(dead_code)]
    pub fn load_memory(&self, path: &Path) -> Result<MemoryEntry, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read memory file: {}", e))?;

        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse memory entry: {}", e))
    }
}
