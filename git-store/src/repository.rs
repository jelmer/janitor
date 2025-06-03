//! Repository management functionality

use crate::error::{GitStoreError, Result};
use git2::{Repository, RepositoryInitOptions};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Repository manager handles Git repository operations
pub struct RepositoryManager {
    base_path: PathBuf,
}

impl RepositoryManager {
    /// Create a new repository manager
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    /// Get the path for a repository
    pub fn repo_path(&self, codebase: &str) -> PathBuf {
        self.base_path.join(codebase)
    }

    /// Open a repository, creating it if it doesn't exist
    pub fn open_or_create(&self, codebase: &str) -> Result<Repository> {
        let repo_path = self.repo_path(codebase);
        
        match Repository::open(&repo_path) {
            Ok(repo) => {
                debug!("Opened existing repository: {}", codebase);
                Ok(repo)
            }
            Err(_) => {
                info!("Creating new bare repository: {}", codebase);
                self.create_bare_repository(&repo_path)
            }
        }
    }

    /// Create a new bare repository
    fn create_bare_repository(&self, path: &Path) -> Result<Repository> {
        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut opts = RepositoryInitOptions::new();
        opts.bare(true);
        opts.mkdir(true);

        Repository::init_opts(path, &opts).map_err(GitStoreError::from)
    }

    /// Check if a repository exists
    pub fn exists(&self, codebase: &str) -> bool {
        let repo_path = self.repo_path(codebase);
        Repository::open(&repo_path).is_ok()
    }

    /// Validate a SHA
    pub fn validate_sha(sha: &str) -> Result<()> {
        if sha.len() != 40 {
            return Err(GitStoreError::InvalidSha(
                "SHA must be 40 characters".to_string(),
            ));
        }

        if !sha.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(GitStoreError::InvalidSha(
                "SHA must contain only hexadecimal characters".to_string(),
            ));
        }

        Ok(())
    }

    /// Get repository information
    pub fn get_repo_info(&self, codebase: &str) -> Result<RepoInfo> {
        let repo = Repository::open(self.repo_path(codebase))
            .map_err(|_| GitStoreError::RepositoryNotFound(codebase.to_string()))?;

        let head = repo.head().ok();
        let head_oid = head.as_ref().and_then(|h| h.target());
        let head_name = head.as_ref().and_then(|h| h.shorthand()).map(String::from);

        let branches = repo
            .branches(None)?
            .filter_map(|b| b.ok())
            .filter_map(|(branch, _)| branch.name().ok().flatten().map(String::from))
            .collect();

        let tags = repo
            .tag_names(None)?
            .iter()
            .flatten()
            .map(String::from)
            .collect();

        Ok(RepoInfo {
            codebase: codebase.to_string(),
            head_oid: head_oid.map(|oid| oid.to_string()),
            head_name,
            branches,
            tags,
        })
    }

    /// Set repository remote
    pub fn set_remote(&self, codebase: &str, name: &str, url: &str) -> Result<()> {
        let repo = self.open_or_create(codebase)?;
        
        // Remove existing remote if it exists
        if repo.find_remote(name).is_ok() {
            repo.remote_delete(name)?;
        }

        repo.remote(name, url)?;
        info!("Set remote '{}' to '{}' for {}", name, url, codebase);
        
        Ok(())
    }

    /// List all repositories
    pub fn list_repositories(&self) -> Result<Vec<String>> {
        let mut repos = Vec::new();

        if !self.base_path.exists() {
            return Ok(repos);
        }

        for entry in std::fs::read_dir(&self.base_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                // Check if it's a git repository
                if Repository::open(&path).is_ok() {
                    if let Some(name) = path.file_name() {
                        if let Some(name_str) = name.to_str() {
                            repos.push(name_str.to_string());
                        }
                    }
                }
            }
        }

        repos.sort();
        Ok(repos)
    }
}

/// Repository information
#[derive(Debug, Clone, serde::Serialize)]
pub struct RepoInfo {
    /// Codebase name
    pub codebase: String,
    /// Current HEAD OID
    pub head_oid: Option<String>,
    /// Current HEAD reference name
    pub head_name: Option<String>,
    /// List of branches
    pub branches: Vec<String>,
    /// List of tags
    pub tags: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_validate_sha() {
        // Valid SHA
        assert!(RepositoryManager::validate_sha(
            "1234567890abcdef1234567890abcdef12345678"
        )
        .is_ok());

        // Too short
        assert!(RepositoryManager::validate_sha("123456").is_err());

        // Too long
        assert!(RepositoryManager::validate_sha(
            "1234567890abcdef1234567890abcdef123456789"
        )
        .is_err());

        // Invalid characters
        assert!(RepositoryManager::validate_sha(
            "1234567890abcdef1234567890abcdef1234567g"
        )
        .is_err());
    }

    #[test]
    fn test_repository_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = RepositoryManager::new(temp_dir.path().to_path_buf());

        // Repository doesn't exist initially
        assert!(!manager.exists("test-repo"));

        // Open or create should create it
        let repo = manager.open_or_create("test-repo").unwrap();
        assert!(repo.is_bare());

        // Now it should exist
        assert!(manager.exists("test-repo"));

        // Opening again should work
        let _repo2 = manager.open_or_create("test-repo").unwrap();
    }

    #[test]
    fn test_list_repositories() {
        let temp_dir = TempDir::new().unwrap();
        let manager = RepositoryManager::new(temp_dir.path().to_path_buf());

        // Initially empty
        assert_eq!(manager.list_repositories().unwrap(), Vec::<String>::new());

        // Create some repositories
        manager.open_or_create("repo1").unwrap();
        manager.open_or_create("repo2").unwrap();
        manager.open_or_create("repo3").unwrap();

        // List should contain all repos
        let repos = manager.list_repositories().unwrap();
        assert_eq!(repos.len(), 3);
        assert!(repos.contains(&"repo1".to_string()));
        assert!(repos.contains(&"repo2".to_string()));
        assert!(repos.contains(&"repo3".to_string()));
    }
}