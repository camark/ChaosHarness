//! Plugin types

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plugin {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub path: String,
}
