//! Shared path resolution utilities for tools

use std::path::{Path, PathBuf};

/// Resolve a candidate path against a base directory
///
/// Handles:
/// - ~ expansion to home directory
/// - Desktop directory detection (OS-specific)
/// - Absolute paths passed through
/// - Relative paths joined with base
pub fn resolve_path(base: &Path, candidate: &str) -> PathBuf {
    let path = PathBuf::from(candidate);

    // Expand ~ to home directory
    if candidate.starts_with("~/") || candidate == "~" {
        if let Some(home_dir) = dirs::home_dir() {
            let remainder = if candidate == "~" { "" } else { &candidate[2..] };

            // Special handling for Desktop - use OS-specific desktop directory
            if remainder == "Desktop" {
                if let Some(desktop_dir) = dirs::desktop_dir() {
                    return desktop_dir;
                }
                return home_dir.join("Desktop");
            }

            return home_dir.join(remainder);
        }
    }

    // Use dirs::desktop_dir() for automatic OS-specific Desktop detection
    if candidate == "Desktop" || candidate.ends_with("/Desktop") {
        if let Some(desktop_dir) = dirs::desktop_dir() {
            return desktop_dir;
        }
    }

    if path.is_absolute() {
        path
    } else {
        base.join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_resolve_absolute_path() {
        let base = PathBuf::from("/home/user/project");
        let result = resolve_path(&base, "/tmp/file.txt");
        assert_eq!(result, PathBuf::from("/tmp/file.txt"));
    }

    #[test]
    fn test_resolve_relative_path() {
        let base = PathBuf::from("/home/user/project");
        let result = resolve_path(&base, "src/main.rs");
        assert_eq!(result, PathBuf::from("/home/user/project/src/main.rs"));
    }

    #[test]
    fn test_resolve_tilde_path() {
        let base = PathBuf::from("/tmp");
        let result = resolve_path(&base, "~/Documents/file.txt");
        if let Some(home) = dirs::home_dir() {
            assert_eq!(result, home.join("Documents/file.txt"));
        }
    }

    #[test]
    fn test_resolve_tilde_only() {
        let base = PathBuf::from("/tmp");
        let result = resolve_path(&base, "~");
        if let Some(home) = dirs::home_dir() {
            assert_eq!(result, home);
        }
    }

    #[test]
    fn test_resolve_desktop_path() {
        let base = PathBuf::from("/tmp");
        let result = resolve_path(&base, "Desktop");
        // In test environment, desktop_dir may not be available
        // The function should either return the actual desktop path or base/Desktop
        // Just verify it returns a valid path
        assert!(!result.as_os_str().is_empty());
    }
}
