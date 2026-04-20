//! Environment detection

#![allow(dead_code)]

use std::env;

pub fn detect_environment() -> String {
    let os = env::consts::OS;
    let arch = env::consts::ARCH;
    let shell = env::var("SHELL").unwrap_or_else(|_| "unknown".to_string());

    format!("OS: {}, Arch: {}, Shell: {}", os, arch, shell)
}
