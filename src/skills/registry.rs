//! Skill registry

#![allow(dead_code)]

use crate::skills::types::Skill;
use std::collections::HashMap;

pub struct SkillRegistry {
    skills: HashMap<String, Skill>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
        }
    }

    pub fn register(&mut self, skill: Skill) {
        self.skills.insert(skill.name.clone(), skill);
    }

    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.skills.get(name)
    }

    pub fn list(&self) -> Vec<&Skill> {
        self.skills.values().collect()
    }

    pub fn count(&self) -> usize {
        self.skills.len()
    }

    pub fn has(&self, name: &str) -> bool {
        self.skills.contains_key(name)
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_skill(name: &str) -> Skill {
        Skill {
            name: name.to_string(),
            description: format!("{} skill", name),
            content: format!("# {}", name),
            source: "test".to_string(),
        }
    }

    #[test]
    fn test_registry_new() {
        let registry = SkillRegistry::new();
        assert_eq!(registry.count(), 0);
        assert!(registry.list().is_empty());
    }

    #[test]
    fn test_registry_default() {
        let registry = SkillRegistry::default();
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_registry_register() {
        let mut registry = SkillRegistry::new();
        registry.register(make_skill("test_skill"));
        assert_eq!(registry.count(), 1);
        assert!(registry.has("test_skill"));
    }

    #[test]
    fn test_registry_get() {
        let mut registry = SkillRegistry::new();
        registry.register(make_skill("my_skill"));

        let skill = registry.get("my_skill");
        assert!(skill.is_some());
        assert_eq!(skill.unwrap().name, "my_skill");

        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_registry_list() {
        let mut registry = SkillRegistry::new();
        registry.register(make_skill("a"));
        registry.register(make_skill("b"));
        registry.register(make_skill("c"));

        let list = registry.list();
        assert_eq!(list.len(), 3);
    }

    #[test]
    fn test_registry_overwrite() {
        let mut registry = SkillRegistry::new();
        registry.register(make_skill("same_name"));

        let mut updated = make_skill("same_name");
        updated.description = "Updated description".to_string();
        registry.register(updated);

        assert_eq!(registry.count(), 1);
        assert_eq!(registry.get("same_name").unwrap().description, "Updated description");
    }

    #[test]
    fn test_registry_has() {
        let mut registry = SkillRegistry::new();
        registry.register(make_skill("exists"));

        assert!(registry.has("exists"));
        assert!(!registry.has("does_not_exist"));
    }
}
