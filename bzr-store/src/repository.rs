//! Repository management for BZR Store service

use std::path::{Path, PathBuf};
use std::process::Stdio;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::process::Command;
use tracing::{debug, info, warn};

use crate::database::DatabaseManager;
use crate::error::{BzrError, Result};

/// Repository path structure for campaign/codebase/role organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryPath {
    /// Campaign identifier for grouping related codebases
    pub campaign: String,
    /// Codebase identifier for the specific project
    pub codebase: String,
    /// Role identifier for the repository variant (e.g., 'main', 'dev')
    pub role: String,
}

impl RepositoryPath {
    /// Create a new repository path
    pub fn new(campaign: String, codebase: String, role: String) -> Self {
        Self {
            campaign,
            codebase,
            role,
        }
    }
    
    /// Get the relative path string
    pub fn relative_path(&self) -> String {
        format!("{}/{}/{}", self.campaign, self.codebase, self.role)
    }
}

/// Repository information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryInfo {
    /// Path structure identifying the repository
    pub path: RepositoryPath,
    /// Whether the repository exists on disk
    pub exists: bool,
    /// The most recent revision identifier, if available
    pub last_revision: Option<String>,
    /// Number of branches in the repository
    pub branch_count: u32,
}

/// Repository manager trait
#[async_trait]
pub trait RepositoryManager: Send + Sync {
    /// Ensure repository exists, creating it if necessary
    async fn ensure_repository(&self, path: &RepositoryPath) -> Result<PathBuf>;
    
    /// Get repository information
    async fn get_repository_info(&self, path: &RepositoryPath) -> Result<RepositoryInfo>;
    
    /// List all repositories
    async fn list_repositories(&self) -> Result<Vec<RepositoryInfo>>;
    
    /// Get diff between two revisions
    async fn get_diff(&self, path: &RepositoryPath, old_revid: &str, new_revid: &str) -> Result<Vec<u8>>;
    
    /// Get revision information
    async fn get_revision_info(&self, path: &RepositoryPath, old_revid: &str, new_revid: &str) -> Result<Vec<RevisionInfo>>;
    
    /// Configure remote URL for repository
    async fn configure_remote(&self, path: &RepositoryPath, remote_url: &str) -> Result<()>;
}

/// Revision information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevisionInfo {
    /// Unique identifier for the revision
    pub revision_id: String,
    /// Commit message associated with the revision
    pub message: String,
    /// Email or name of the person who made the commit
    pub committer: String,
    /// Timestamp when the revision was created
    pub timestamp: String,
}

/// Subprocess-based repository manager implementation
#[derive(Debug, Clone)]
pub struct SubprocessRepositoryManager {
    /// Base directory path where all repositories are stored
    base_path: PathBuf,
    /// Database manager for validation and authentication
    database: DatabaseManager,
}

impl SubprocessRepositoryManager {
    /// Create a new subprocess repository manager
    pub fn new(base_path: PathBuf, database: DatabaseManager) -> Self {
        Self { base_path, database }
    }
    
    /// Get the full path for a repository
    pub fn get_repository_path(&self, path: &RepositoryPath) -> PathBuf {
        self.base_path
            .join(&path.campaign)
            .join(&path.codebase)
            .join(&path.role)
    }
    
    /// Get the campaign path
    pub fn get_campaign_path(&self, campaign: &str) -> PathBuf {
        self.base_path.join(campaign)
    }
    
    /// Ensure campaign structure exists
    async fn ensure_campaign_structure(&self, campaign: &str) -> Result<()> {
        let campaign_path = self.get_campaign_path(campaign);
        
        if !campaign_path.exists() {
            info!("Creating campaign directory: {}", campaign_path.display());
            fs::create_dir_all(&campaign_path).await?;
            
            // Initialize shared repository for the campaign
            let output = Command::new("brz")
                .args(["init-shared-repository", "--format=2a"])
                .arg(&campaign_path)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await?;
            
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(BzrError::subprocess(format!(
                    "Failed to create shared repository: {}",
                    stderr
                )));
            }
            
            info!("Created shared repository for campaign: {}", campaign);
        }
        
        Ok(())
    }
    
    /// Check if path is a valid Bazaar repository
    async fn is_bzr_repository(&self, path: &Path) -> bool {
        path.join(".bzr").is_dir()
    }
    
    /// Run bzr command and return output
    async fn run_bzr_command(&self, args: &[&str], working_dir: Option<&Path>) -> Result<std::process::Output> {
        let mut cmd = Command::new("brz");
        cmd.args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        
        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }
        
        debug!("Running bzr command: brz {}", args.join(" "));
        
        let output = cmd.output().await?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BzrError::subprocess(format!(
                "bzr command failed: brz {} - {}",
                args.join(" "),
                stderr
            )));
        }
        
        Ok(output)
    }
}

#[async_trait]
impl RepositoryManager for SubprocessRepositoryManager {
    async fn ensure_repository(&self, path: &RepositoryPath) -> Result<PathBuf> {
        // Validate codebase exists in database
        if !self.database.validate_codebase(&path.codebase).await? {
            return Err(BzrError::invalid_request(format!(
                "Codebase '{}' not found in database",
                path.codebase
            )));
        }
        
        // Ensure campaign structure exists
        self.ensure_campaign_structure(&path.campaign).await?;
        
        let repo_path = self.get_repository_path(path);
        
        if !repo_path.exists() {
            info!("Creating repository: {}", repo_path.display());
            
            // Create the directory structure
            fs::create_dir_all(&repo_path).await?;
            
            // Initialize the branch in the shared repository
            let campaign_path = self.get_campaign_path(&path.campaign);
            self.run_bzr_command(
                &["init", "--format=2a", &repo_path.to_string_lossy()],
                Some(&campaign_path),
            ).await?;
            
            info!("Created Bazaar repository: {}", repo_path.display());
        } else if !self.is_bzr_repository(&repo_path).await {
            warn!("Directory exists but is not a Bazaar repository: {}", repo_path.display());
            return Err(BzrError::repository(format!(
                "Path exists but is not a Bazaar repository: {}",
                repo_path.display()
            )));
        }
        
        Ok(repo_path)
    }
    
    async fn get_repository_info(&self, path: &RepositoryPath) -> Result<RepositoryInfo> {
        let repo_path = self.get_repository_path(path);
        let exists = self.is_bzr_repository(&repo_path).await;
        
        let (last_revision, branch_count) = if exists {
            // Get last revision
            let last_rev = match self.run_bzr_command(
                &["log", "--limit=1", "--line"],
                Some(&repo_path),
            ).await {
                Ok(output) => {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    output_str.lines().next().map(|s| s.to_string())
                }
                Err(_) => None,
            };
            
            // Count branches (simplified - just return 1 if repository exists)
            let branch_count = if exists { 1 } else { 0 };
            
            (last_rev, branch_count)
        } else {
            (None, 0)
        };
        
        Ok(RepositoryInfo {
            path: path.clone(),
            exists,
            last_revision,
            branch_count,
        })
    }
    
    async fn list_repositories(&self) -> Result<Vec<RepositoryInfo>> {
        let mut repositories = Vec::new();
        
        if !self.base_path.exists() {
            return Ok(repositories);
        }
        
        // Walk through campaign directories
        let mut campaign_dir = fs::read_dir(&self.base_path).await?;
        while let Some(campaign_entry) = campaign_dir.next_entry().await? {
            if !campaign_entry.file_type().await?.is_dir() {
                continue;
            }
            
            let campaign_name = campaign_entry.file_name().to_string_lossy().to_string();
            let campaign_path = campaign_entry.path();
            
            // Walk through codebase directories
            let mut codebase_dir = fs::read_dir(&campaign_path).await?;
            while let Some(codebase_entry) = codebase_dir.next_entry().await? {
                if !codebase_entry.file_type().await?.is_dir() {
                    continue;
                }
                
                let codebase_name = codebase_entry.file_name().to_string_lossy().to_string();
                let codebase_path = codebase_entry.path();
                
                // Walk through role directories
                let mut role_dir = fs::read_dir(&codebase_path).await?;
                while let Some(role_entry) = role_dir.next_entry().await? {
                    if !role_entry.file_type().await?.is_dir() {
                        continue;
                    }
                    
                    let role_name = role_entry.file_name().to_string_lossy().to_string();
                    let repo_path = RepositoryPath::new(
                        campaign_name.clone(),
                        codebase_name.clone(),
                        role_name,
                    );
                    
                    let info = self.get_repository_info(&repo_path).await?;
                    repositories.push(info);
                }
            }
        }
        
        Ok(repositories)
    }
    
    async fn get_diff(&self, path: &RepositoryPath, old_revid: &str, new_revid: &str) -> Result<Vec<u8>> {
        let repo_path = self.get_repository_path(path);
        
        if !self.is_bzr_repository(&repo_path).await {
            return Err(BzrError::repository("Repository does not exist"));
        }
        
        let output = self.run_bzr_command(
            &["diff", "-r", &format!("{}..{}", old_revid, new_revid)],
            Some(&repo_path),
        ).await?;
        
        Ok(output.stdout)
    }
    
    async fn get_revision_info(&self, path: &RepositoryPath, old_revid: &str, new_revid: &str) -> Result<Vec<RevisionInfo>> {
        let repo_path = self.get_repository_path(path);
        
        if !self.is_bzr_repository(&repo_path).await {
            return Err(BzrError::repository("Repository does not exist"));
        }
        
        let output = self.run_bzr_command(
            &["log", "-r", &format!("{}..{}", old_revid, new_revid), "--show-ids"],
            Some(&repo_path),
        ).await?;
        
        // Parse the log output (simplified)
        let log_text = String::from_utf8_lossy(&output.stdout);
        let mut revisions = Vec::new();
        
        // This is a simplified parser - in a real implementation,
        // you'd want more robust parsing
        for line in log_text.lines() {
            if line.starts_with("revision-id:") {
                let revision_id = line.strip_prefix("revision-id:").unwrap_or("").trim().to_string();
                revisions.push(RevisionInfo {
                    revision_id,
                    message: "".to_string(),
                    committer: "".to_string(),
                    timestamp: "".to_string(),
                });
            }
        }
        
        Ok(revisions)
    }
    
    async fn configure_remote(&self, path: &RepositoryPath, remote_url: &str) -> Result<()> {
        let repo_path = self.get_repository_path(path);
        
        if !self.is_bzr_repository(&repo_path).await {
            return Err(BzrError::repository("Repository does not exist"));
        }
        
        self.run_bzr_command(
            &["config", "parent_location", remote_url],
            Some(&repo_path),
        ).await?;
        
        info!("Configured remote URL for {}: {}", path.relative_path(), remote_url);
        Ok(())
    }
}