//! Hook registry - manages registered hooks by event type

#![allow(dead_code)]

use super::events::HookEvent;
use super::schemas::HookDefinition;
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// Registry for all hooks, organized by event type
#[derive(Clone, Default)]
pub struct HookRegistry {
    hooks: Arc<RwLock<HashMap<HookEvent, Vec<HookDefinition>>>>,
}

impl HookRegistry {
    pub fn new() -> Self {
        Self {
            hooks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a hook (synchronous version)
    pub fn register(&self, hook: HookDefinition) -> Result<(), String> {
        let event = HookEvent::from_str(&hook.event)
            .ok_or_else(|| format!("Unknown hook event: {}", hook.event))?;

        let mut hooks = self.hooks.write();
        hooks.entry(event).or_default().push(hook);
        Ok(())
    }

    /// Register a hook, ignoring errors (convenience method)
    pub fn register_blocking(&self, hook: HookDefinition) {
        let _ = self.register(hook);
    }

    /// Get all hooks for an event
    pub fn get_hooks(&self, event: &HookEvent) -> Vec<HookDefinition> {
        let hooks = self.hooks.read();
        hooks.get(event).cloned().unwrap_or_default()
    }

    /// Get all hooks for an event type string
    pub fn get_hooks_by_str(&self, event_str: &str) -> Vec<HookDefinition> {
        if let Some(event) = HookEvent::from_str(event_str) {
            self.get_hooks(&event)
        } else {
            Vec::new()
        }
    }

    /// Remove all hooks for an event
    pub fn clear_event(&self, event: &HookEvent) {
        let mut hooks = self.hooks.write();
        if let Some(hooks_for_event) = hooks.get_mut(event) {
            hooks_for_event.clear();
        }
    }

    /// List all registered hooks
    pub fn list_all(&self) -> Vec<HookDefinition> {
        let hooks = self.hooks.read();
        hooks.values().flatten().cloned().collect()
    }

    /// Check if any hooks are registered for an event
    pub fn has_hooks(&self, event: &HookEvent) -> bool {
        let hooks = self.hooks.read();
        hooks.get(event).map(|h| !h.is_empty()).unwrap_or(false)
    }

    /// Load hooks from configuration
    pub fn load_from_config(hooks_config: &[HookDefinition]) -> Self {
        let registry = Self::new();
        for hook in hooks_config {
            let _ = registry.register(hook.clone());
        }
        registry
    }
}
