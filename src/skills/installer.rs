//! Skill installer for downloading skills from SkillsMP marketplace

use anyhow::{Result, anyhow, bail};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::fs;

/// SkillsMP search response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsMpSearchResponse {
    pub skills: Vec<SkillsMpSkill>,
    pub total: u32,
}

/// SkillsMP skill info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsMpSkill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub github_url: String,
    pub skill_url: String,
    pub downloads: Option<u32>,
}

/// Skill installer
pub struct SkillInstaller {
    client: Client,
    skills_dir: String,
}

impl SkillInstaller {
    pub fn new(skills_dir: &str) -> Self {
        let client = Client::builder()
            .user_agent("RustHarness/0.1.0")
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            skills_dir: skills_dir.to_string(),
        }
    }

    /// Search skills on SkillsMP
    pub async fn search(&self, query: &str) -> Result<Vec<SkillsMpSkill>> {
        // SkillsMP aggregates skills from GitHub
        // We search GitHub for SKILL.md files
        let github_query = format!("{} SKILL.md in:path language:markdown", query);
        let url = format!(
            "https://api.github.com/search/code?q={}&per_page=10",
            urlencoding::encode(&github_query)
        );

        let request = self.client.get(&url);

        // Add GitHub token if available
        let request = if let Ok(token) = std::env::var("GITHUB_TOKEN") {
            request.header("Authorization", format!("Bearer {}", token))
        } else {
            request
        };

        let response = request
            .send()
            .await
            .map_err(|e| anyhow!("Failed to search GitHub: {}", e))?;

        if !response.status().is_success() {
            bail!("GitHub API request failed: {}", response.status());
        }

        // Parse GitHub search results
        let data: serde_json::Value = response.json().await
            .map_err(|e| anyhow!("Failed to parse GitHub response: {}", e))?;

        let items = data["items"]
            .as_array()
            .ok_or_else(|| anyhow!("Invalid GitHub response"))?;

        let skills: Vec<SkillsMpSkill> = items
            .iter()
            .filter_map(|item| {
                let name = item["name"].as_str()?;
                let path = item["path"].as_str()?;
                let repo = item["repository"].as_object()?;
                let repo_name = repo["full_name"].as_str()?;
                let html_url = repo["html_url"].as_str()?;

                Some(SkillsMpSkill {
                    id: format!("{}:{}", repo_name, path),
                    name: name.trim_end_matches(".md").to_string(),
                    description: format!("Skill from {}", repo_name),
                    author: repo_name.split('/').next().unwrap_or("unknown").to_string(),
                    github_url: html_url.to_string(),
                    skill_url: format!("https://raw.githubusercontent.com/{}/main/{}", repo_name, path),
                    downloads: None,
                })
            })
            .collect();

        Ok(skills)
    }

    /// Download a skill from a URL
    pub async fn download_skill(&self, url: &str, name: Option<&str>) -> Result<String> {
        // Ensure skills directory exists
        let skills_path = Path::new(&self.skills_dir);
        fs::create_dir_all(skills_path)
            .map_err(|e| anyhow!("Failed to create skills directory: {}", e))?;

        // Download the skill content
        let response = self.client.get(url)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to download skill: {}", e))?;

        if !response.status().is_success() {
            bail!("Failed to download skill: {}", response.status());
        }

        let content = response.text().await
            .map_err(|e| anyhow!("Failed to read skill content: {}", e))?;

        // Determine the skill name
        let skill_name = name.map(|s| s.to_string()).unwrap_or_else(|| {
            // Extract from URL
            url.split('/').last()
                .unwrap_or("skill")
                .trim_end_matches(".md")
                .to_string()
        });

        // Save the skill
        let skill_file = skills_path.join(format!("{}.md", skill_name));
        fs::write(&skill_file, &content)
            .map_err(|e| anyhow!("Failed to save skill: {}", e))?;

        Ok(skill_file.to_string_lossy().to_string())
    }

    /// Install a skill from GitHub URL
    pub async fn install_from_github(&self, github_url: &str) -> Result<String> {
        // Parse GitHub URL to get raw URL
        // Format: https://github.com/owner/repo/blob/path/to/file.md
        let parts: Vec<&str> = github_url.split("/blob/").collect();
        if parts.len() != 2 {
            bail!("Invalid GitHub URL format. Expected: https://github.com/owner/repo/blob/path/file.md");
        }

        let repo_parts: Vec<&str> = parts[0].trim_end_matches('/').split('/').collect();
        if repo_parts.len() < 2 {
            bail!("Invalid GitHub repository URL");
        }

        let owner = repo_parts[repo_parts.len() - 2];
        let repo = repo_parts[repo_parts.len() - 1];
        let file_path = parts[1];

        // Construct raw URL
        let raw_url = format!(
            "https://raw.githubusercontent.com/{}/{}/main/{}",
            owner, repo, file_path
        );

        // Try main branch first, then master
        let content = match self.download_from_url(&raw_url).await {
            Ok(c) => c,
            Err(_) => {
                let master_url = format!(
                    "https://raw.githubusercontent.com/{}/{}/master/{}",
                    owner, repo, file_path
                );
                self.download_from_url(&master_url).await?
            }
        };

        // Extract skill name from file path
        let skill_name = file_path
            .split('/')
            .last()
            .unwrap_or("skill")
            .trim_end_matches(".md")
            .trim_end_matches(".skill");

        // Save the skill
        let skill_file = Path::new(&self.skills_dir).join(format!("{}.md", skill_name));
        fs::write(&skill_file, &content)
            .map_err(|e| anyhow!("Failed to save skill: {}", e))?;

        Ok(skill_file.to_string_lossy().to_string())
    }

    /// Download content from a URL
    async fn download_from_url(&self, url: &str) -> Result<String> {
        let response = self.client.get(url)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to download: {}", e))?;

        if !response.status().is_success() {
            bail!("Download failed: {}", response.status());
        }

        let content = response.text().await
            .map_err(|e| anyhow!("Failed to read content: {}", e))?;

        Ok(content)
    }

    /// Remove a skill by name
    pub fn remove_skill(&self, name: &str) -> Result<bool> {
        let skill_file = Path::new(&self.skills_dir).join(format!("{}.md", name));

        if !skill_file.exists() {
            return Ok(false);
        }

        fs::remove_file(&skill_file)
            .map_err(|e| anyhow!("Failed to remove skill: {}", e))?;

        Ok(true)
    }

    /// List all installed skills
    pub fn list_installed_skills(&self) -> Result<Vec<String>> {
        let skills_path = Path::new(&self.skills_dir);
        let mut skills = Vec::new();

        if !skills_path.exists() {
            return Ok(skills);
        }

        if let Ok(entries) = fs::read_dir(skills_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("md") {
                    if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                        skills.push(name.to_string());
                    }
                }
            }
        }

        skills.sort();
        Ok(skills)
    }
}

/// Get the user skills directory
pub fn get_user_skills_dir() -> String {
    if let Some(home) = dirs::home_dir() {
        home.join(".rust_harness").join("skills").to_string_lossy().to_string()
    } else {
        "./skills".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_user_skills_dir() {
        let dir = get_user_skills_dir();
        assert!(!dir.is_empty());
    }

    #[tokio::test]
    async fn test_installer_creation() {
        let installer = SkillInstaller::new("/tmp/test_skills");
        assert_eq!(installer.skills_dir, "/tmp/test_skills");
    }
}
