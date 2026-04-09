//! Skill installer for downloading skills from SkillsMP marketplace

use anyhow::{Result, anyhow, bail};
use reqwest::blocking::Client;
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

/// GitHub API file/directory response
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GitHubContent {
    name: String,
    path: String,
    #[serde(rename = "type")]
    file_type: String,
    download_url: Option<String>,
    #[serde(rename = "html_url")]
    html_url: Option<String>,
}

/// Skill installer
#[derive(Clone)]
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

    /// Search skills on SkillsMP (synchronous version)
    pub fn search(&self, query: &str) -> Result<Vec<SkillsMpSkill>> {
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
            .map_err(|e| anyhow!("Failed to search GitHub: {}", e))?;

        if !response.status().is_success() {
            bail!("GitHub API request failed: {}", response.status());
        }

        // Parse GitHub search results
        let data: serde_json::Value = response.json()
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

    /// Download a skill from a URL (synchronous version)
    pub fn download_skill(&self, url: &str, name: Option<&str>) -> Result<String> {
        // Ensure skills directory exists
        let skills_path = Path::new(&self.skills_dir);
        fs::create_dir_all(skills_path)
            .map_err(|e| anyhow!("Failed to create skills directory: {}", e))?;

        // Download the skill content
        let response = self.client.get(url)
            .send()
            .map_err(|e| anyhow!("Failed to download skill: {}", e))?;

        if !response.status().is_success() {
            bail!("Failed to download skill: {}", response.status());
        }

        let content = response.text()
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

    /// Install a skill from GitHub URL (synchronous version)
    /// Supports both file URLs (/blob/) and directory URLs (/tree/)
    pub fn install_from_github(&self, github_url: &str) -> Result<String> {
        tracing::info!("install_from_github: Starting with URL: {}", github_url);

        // Determine URL type: /blob/ for files, /tree/ for directories
        let url_type = if github_url.contains("/blob/") {
            "blob"
        } else if github_url.contains("/tree/") {
            "tree"
        } else {
            bail!("Invalid GitHub URL format. Expected: https://github.com/owner/repo/blob/path/file.md or https://github.com/owner/repo/tree/path/to/dir");
        };
        tracing::info!("install_from_github: URL type = {}", url_type);

        // Parse GitHub URL to get repo and path info
        let delimiter = format!("/{}/", url_type);
        let parts: Vec<&str> = github_url.split(&delimiter).collect();
        if parts.len() != 2 {
            bail!("Invalid GitHub URL format");
        }
        tracing::info!("install_from_github: Parsed URL parts OK");

        let repo_parts: Vec<&str> = parts[0].trim_end_matches('/').split('/').collect();
        if repo_parts.len() < 2 {
            bail!("Invalid GitHub repository URL");
        }

        let owner = repo_parts[repo_parts.len() - 2];
        let repo = repo_parts[repo_parts.len() - 1];
        tracing::info!("install_from_github: owner={}, repo={}", owner, repo);

        // path_after_type includes the branch name: branch/path/to/...
        let path_after_type = parts[1];

        // Split path into components
        let mut path_parts: Vec<&str> = path_after_type.split('/').collect();
        if path_parts.is_empty() {
            bail!("Invalid path in GitHub URL");
        }

        let _branch = path_parts[0]; // branch name (main or master)

        // Determine directory path and skill name based on URL type
        let dir_path: String;
        let skill_name: String;

        if url_type == "tree" {
            // Directory URL: use the full path as directory
            dir_path = if path_parts.len() > 1 {
                path_parts[1..].join("/")
            } else {
                "".to_string()
            };
            tracing::info!("install_from_github: dir_path={}", dir_path);

            // Skill name is the last component of the directory path
            skill_name = if !dir_path.is_empty() {
                dir_path.split('/').last().unwrap_or("skill").to_string()
            } else {
                repo.to_string()
            };
            tracing::info!("install_from_github: skill_name={}", skill_name);
        } else {
            // File URL (/blob/): extract directory from file path
            let file_name = path_parts.last().unwrap_or(&"SKILL.md");

            dir_path = if path_parts.len() > 1 {
                path_parts[1..path_parts.len()-1].join("/")
            } else {
                "".to_string()
            };

            // Skill name from directory name or file name
            skill_name = if !dir_path.is_empty() {
                dir_path.split('/').last().unwrap_or("skill").to_string()
            } else {
                file_name.trim_end_matches(".md").trim_end_matches(".skill").to_string()
            };
        }

        // Fetch directory contents from GitHub API
        let api_url = if dir_path.is_empty() {
            format!("https://api.github.com/repos/{}/{}/contents", owner, repo)
        } else {
            format!("https://api.github.com/repos/{}/{}/contents/{}", owner, repo, dir_path)
        };
        tracing::info!("install_from_github: Fetching directory from: {}", api_url);

        let request = self.client.get(&api_url);
        let request = if let Ok(token) = std::env::var("GITHUB_TOKEN") {
            request.header("Authorization", format!("Bearer {}", token))
        } else {
            request
        };

        tracing::info!("install_from_github: Sending API request...");
        let response = request
            .send()
            .map_err(|e| anyhow!("Failed to fetch directory listing: {}", e))?;
        tracing::info!("install_from_github: API response status = {}", response.status());

        if !response.status().is_success() {
            bail!("GitHub API request failed: {}", response.status());
        }

        tracing::info!("install_from_github: Parsing JSON response...");
        let contents: Vec<GitHubContent> = response.json()
            .map_err(|e| anyhow!("Failed to parse directory listing: {}", e))?;
        tracing::info!("install_from_github: Parsed {} items from directory listing", contents.len());

        // Create skill directory in skills directory
        let skill_dir = Path::new(&self.skills_dir).join(skill_name);
        tracing::info!("install_from_github: Creating skill directory: {:?}", skill_dir);
        fs::create_dir_all(&skill_dir)
            .map_err(|e| anyhow!("Failed to create skill directory: {}", e))?;

        // Download all files from the directory
        let mut downloaded_files = Vec::new();
        let mut main_skill_path: Option<String> = None;

        tracing::info!("install_from_github: Starting to download {} files", contents.iter().filter(|c| c.file_type == "file").count());
        for item in contents {
            if item.file_type == "file" {
                tracing::info!("install_from_github: Downloading file: {}", item.name);
                // Determine branch (try main first, then master)
                let download_url = if let Some(url) = item.download_url {
                    url
                } else {
                    // Construct URL for files without download_url
                    format!(
                        "https://raw.githubusercontent.com/{}/{}/main/{}",
                        owner, repo, item.path
                    )
                };
                tracing::info!("install_from_github: Download URL: {}", download_url);

                // Try to download the file
                let content = match self.download_from_url(&download_url) {
                    Ok(c) => c,
                    Err(_) => {
                        // Try master branch
                        let master_url = format!(
                            "https://raw.githubusercontent.com/{}/{}/master/{}",
                            owner, repo, item.path
                        );
                        tracing::info!("install_from_github: Trying master branch: {}", master_url);
                        self.download_from_url(&master_url)?
                    }
                };
                tracing::info!("install_from_github: Downloaded {} bytes", content.len());

                // Determine local file path
                let file_name = item.name;
                let local_path = skill_dir.join(&file_name);

                // Save the file
                fs::write(&local_path, &content)
                    .map_err(|e| anyhow!("Failed to save file {}: {}", file_name, e))?;

                downloaded_files.push(file_name.clone());

                if file_name.to_lowercase().contains("skill") || file_name.to_lowercase().contains("prompt") {
                    main_skill_path = Some(local_path.to_string_lossy().to_string());
                }
            }
        }

        if downloaded_files.is_empty() {
            bail!("No files found in the skill directory");
        }

        let result_path = main_skill_path.unwrap_or_else(|| skill_dir.join(downloaded_files[0].clone()).to_string_lossy().to_string());

        Ok(result_path)
    }

    /// Download content from a URL (synchronous version)
    fn download_from_url(&self, url: &str) -> Result<String> {
        let response = self.client.get(url)
            .send()
            .map_err(|e| anyhow!("Failed to download: {}", e))?;

        if !response.status().is_success() {
            bail!("Download failed: {}", response.status());
        }

        let content = response.text()
            .map_err(|e| anyhow!("Failed to read content: {}", e))?;

        Ok(content)
    }

    /// Remove a skill by name
    pub fn remove_skill(&self, name: &str) -> Result<bool> {
        let skills_path = Path::new(&self.skills_dir);

        // Try to remove as directory first
        let skill_dir = skills_path.join(name);
        if skill_dir.exists() && skill_dir.is_dir() {
            fs::remove_dir_all(&skill_dir)
                .map_err(|e| anyhow!("Failed to remove skill directory: {}", e))?;
            return Ok(true);
        }

        // Try to remove as .md file
        let skill_file = skills_path.join(format!("{}.md", name));
        if skill_file.exists() {
            fs::remove_file(&skill_file)
                .map_err(|e| anyhow!("Failed to remove skill: {}", e))?;
            return Ok(true);
        }

        Ok(false)
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
                if path.is_dir() {
                    // Skill directory - use directory name
                    if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                        skills.push(name.to_string());
                    }
                } else if path.extension().and_then(|s| s.to_str()) == Some("md") {
                    // Single .md file skill
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

    #[test]
    fn test_installer_creation() {
        let installer = SkillInstaller::new("/tmp/test_skills");
        assert_eq!(installer.skills_dir, "/tmp/test_skills");
    }
}
