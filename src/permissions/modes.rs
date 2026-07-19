//! Permission modes

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum PermissionMode {
    #[default]
    Default,
    Plan,
    FullAuto,
}


impl std::fmt::Display for PermissionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PermissionMode::Default => write!(f, "default"),
            PermissionMode::Plan => write!(f, "plan"),
            PermissionMode::FullAuto => write!(f, "full_auto"),
        }
    }
}

impl std::str::FromStr for PermissionMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "default" => Ok(PermissionMode::Default),
            "plan" => Ok(PermissionMode::Plan),
            "full_auto" => Ok(PermissionMode::FullAuto),
            _ => Err(format!("Unknown permission mode: {}", s)),
        }
    }
}
