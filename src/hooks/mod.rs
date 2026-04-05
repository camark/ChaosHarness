//! Hooks module for extensible event handling

pub mod builtins;
pub mod events;
pub mod executor;
pub mod registry;
pub mod schemas;
pub mod types;

pub use events::HookEvent;
pub use executor::HookExecutor;
pub use registry::HookRegistry;
pub use types::{HookContext, HookResult, HookDecision};
pub use builtins::{get_builtin_hooks, execute_builtin_hook};
