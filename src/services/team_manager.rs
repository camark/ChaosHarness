//! Team manager service

#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct Team {
    pub name: String,
    pub description: String,
    pub created_at: std::time::Instant,
}

lazy_static::lazy_static! {
    /// Global team manager instance
    pub static ref GLOBAL_TEAM_MANAGER: TeamManager = TeamManager::new();
}

/// Team manager for managing in-memory teams
pub struct TeamManager {
    teams: Arc<Mutex<HashMap<String, Team>>>,
}

impl TeamManager {
    pub fn new() -> Self {
        Self {
            teams: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create a new team
    pub async fn create_team(&self, name: &str, description: &str) -> bool {
        let mut teams = self.teams.lock().await;
        if teams.contains_key(name) {
            return false; // Team already exists
        }

        teams.insert(name.to_string(), Team {
            name: name.to_string(),
            description: description.to_string(),
            created_at: std::time::Instant::now(),
        });
        true
    }

    /// Delete a team
    pub async fn delete_team(&self, name: &str) -> bool {
        let mut teams = self.teams.lock().await;
        teams.remove(name).is_some()
    }

    /// Get a team by name
    pub async fn get_team(&self, name: &str) -> Option<Team> {
        let teams = self.teams.lock().await;
        teams.get(name).cloned()
    }

    /// List all teams
    pub async fn list_teams(&self) -> Vec<Team> {
        let teams = self.teams.lock().await;
        teams.values().cloned().collect()
    }
}

impl Default for TeamManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_and_get_team() {
        let manager = TeamManager::new();
        let created = manager.create_team("test-team", "A test team").await;
        assert!(created);

        let team = manager.get_team("test-team").await.unwrap();
        assert_eq!(team.name, "test-team");
        assert_eq!(team.description, "A test team");
    }

    #[tokio::test]
    async fn test_create_duplicate_team() {
        let manager = TeamManager::new();
        manager.create_team("test", "first").await;
        let created = manager.create_team("test", "second").await;
        assert!(!created);
    }

    #[tokio::test]
    async fn test_delete_team() {
        let manager = TeamManager::new();
        manager.create_team("test", "desc").await;

        let deleted = manager.delete_team("test").await;
        assert!(deleted);

        let team = manager.get_team("test").await;
        assert!(team.is_none());
    }

    #[tokio::test]
    async fn test_delete_nonexistent_team() {
        let manager = TeamManager::new();
        let deleted = manager.delete_team("nonexistent").await;
        assert!(!deleted);
    }

    #[tokio::test]
    async fn test_list_teams() {
        let manager = TeamManager::new();
        manager.create_team("team1", "first").await;
        manager.create_team("team2", "second").await;

        let teams = manager.list_teams().await;
        assert_eq!(teams.len(), 2);
    }

    #[tokio::test]
    async fn test_list_empty_teams() {
        let manager = TeamManager::new();
        let teams = manager.list_teams().await;
        assert!(teams.is_empty());
    }
}
