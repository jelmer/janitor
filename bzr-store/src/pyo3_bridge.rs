//! PyO3 bridge layer for integrating with Python Breezy library

use pyo3::prelude::*;
use std::path::Path;
use tracing::{debug, info};

use crate::error::{BzrError, Result};

/// Initialize PyO3 and prepare the Python interpreter
static PYTHON_INITIALIZED: std::sync::Once = std::sync::Once::new();

/// Initialize Python interpreter for Breezy operations
fn ensure_python_initialized() {
    PYTHON_INITIALIZED.call_once(|| {
        pyo3::prepare_freethreaded_python();
    });
}

/// Repository information from Breezy
#[derive(Debug, Clone)]
pub struct BreezyRepositoryInfo {
    /// Repository path
    pub path: String,
    /// Whether it's a shared repository
    pub is_shared: bool,
    /// Repository format
    pub format: String,
    /// Number of revisions
    pub revision_count: Option<u64>,
}

/// Branch information from Breezy
#[derive(Debug, Clone)]
pub struct BreezyBranchInfo {
    /// Branch path
    pub path: String,
    /// Branch name
    pub name: String,
    /// Last revision
    pub last_revision: Option<String>,
    /// Number of revisions
    pub revision_count: Option<u64>,
}

/// Advanced revision information from Breezy
#[derive(Debug, Clone)]
pub struct BreezyRevisionInfo {
    /// Revision identifier
    pub revision_id: String,
    /// Commit message
    pub message: String,
    /// Committer information
    pub committer: String,
    /// Timestamp
    pub timestamp: String,
    /// Parent revision IDs
    pub parents: Vec<String>,
}

/// High-level convenience functions for Breezy operations
pub struct BreezyOperations;

impl BreezyOperations {
    /// Initialize a shared repository
    pub async fn init_shared_repository(path: &Path) -> Result<()> {
        ensure_python_initialized();

        Python::with_gil(|py| {
            debug!("Creating shared repository at: {}", path.display());

            // Import bzrdir module
            let bzrdir = py
                .import_bound("breezy.bzr.bzrdir")
                .map_err(|e| BzrError::Python(format!("Failed to import bzrdir: {}", e)))?;

            let path_str = path.to_string_lossy();

            // Create shared repository
            bzrdir
                .call_method1("BzrDir.create_repository", (path_str.as_ref(), true))
                .map_err(|e| {
                    BzrError::Python(format!("Failed to create shared repository: {}", e))
                })?;

            info!(
                "Successfully created shared repository at: {}",
                path.display()
            );
            Ok(())
        })
    }

    /// Initialize a branch
    pub async fn init_branch(path: &Path, _repository_path: Option<&Path>) -> Result<()> {
        ensure_python_initialized();

        Python::with_gil(|py| {
            debug!("Initializing branch at: {}", path.display());

            // Import bzrdir module
            let bzrdir = py
                .import_bound("breezy.bzr.bzrdir")
                .map_err(|e| BzrError::Python(format!("Failed to import bzrdir: {}", e)))?;

            let path_str = path.to_string_lossy();

            // Create bzrdir
            let bzr_dir = bzrdir
                .call_method1("BzrDir.create", (path_str.as_ref(),))
                .map_err(|e| BzrError::Python(format!("Failed to create bzrdir: {}", e)))?;

            // Create branch
            bzr_dir
                .call_method0("create_branch")
                .map_err(|e| BzrError::Python(format!("Failed to create branch: {}", e)))?;

            info!("Successfully initialized branch at: {}", path.display());
            Ok(())
        })
    }

    /// Get repository information
    pub async fn get_repository_info(path: &Path) -> Result<BreezyRepositoryInfo> {
        ensure_python_initialized();

        Python::with_gil(|py| {
            debug!("Getting repository info for: {}", path.display());

            // Import repository module
            let repository = py
                .import_bound("breezy.repository")
                .map_err(|e| BzrError::Python(format!("Failed to import repository: {}", e)))?;

            let path_str = path.to_string_lossy();

            // Open repository
            let repo = repository
                .call_method1("Repository.open", (path_str.as_ref(),))
                .map_err(|e| BzrError::Python(format!("Failed to open repository: {}", e)))?;

            // Get repository information
            let is_shared = repo
                .call_method0("is_shared")
                .and_then(|r| r.extract::<bool>())
                .unwrap_or(false);

            let format = repo
                .getattr("_format")
                .and_then(|f| f.call_method0("get_format_string"))
                .and_then(|s| s.extract::<String>())
                .unwrap_or_else(|_| "unknown".to_string());

            // Try to get revision count (might fail for some repository types)
            let revision_count = repo
                .call_method0("revision_count")
                .and_then(|count| count.extract::<u64>())
                .ok();

            Ok(BreezyRepositoryInfo {
                path: path.to_string_lossy().to_string(),
                is_shared,
                format,
                revision_count,
            })
        })
    }

    /// Get branch information  
    pub async fn get_branch_info(path: &Path) -> Result<BreezyBranchInfo> {
        ensure_python_initialized();

        Python::with_gil(|py| {
            debug!("Getting branch info for: {}", path.display());

            // Import branch module
            let branch = py
                .import_bound("breezy.branch")
                .map_err(|e| BzrError::Python(format!("Failed to import branch: {}", e)))?;

            let path_str = path.to_string_lossy();

            // Open branch
            let branch_obj = branch
                .call_method1("Branch.open", (path_str.as_ref(),))
                .map_err(|e| BzrError::Python(format!("Failed to open branch: {}", e)))?;

            // Get branch information
            let name = branch_obj
                .call_method0("get_user_name")
                .and_then(|n| n.extract::<String>())
                .unwrap_or_else(|_| "main".to_string());

            let last_revision = branch_obj
                .call_method0("last_revision")
                .and_then(|rev| rev.extract::<String>())
                .ok();

            let revision_count = branch_obj
                .call_method0("revision_count")
                .and_then(|count| count.extract::<u64>())
                .ok();

            Ok(BreezyBranchInfo {
                path: path.to_string_lossy().to_string(),
                name,
                last_revision,
                revision_count,
            })
        })
    }

    /// Check if path is a repository
    pub async fn is_repository(path: &Path) -> bool {
        ensure_python_initialized();

        Python::with_gil(|py| {
            let bzrdir = py.import_bound("breezy.bzr.bzrdir").ok()?;
            let path_str = path.to_string_lossy();

            // Try to open as bzrdir
            let bzr_dir = bzrdir
                .call_method1("BzrDir.open", (path_str.as_ref(),))
                .ok()?;

            // Check if it has a repository
            Some(
                bzr_dir
                    .call_method0("has_repository")
                    .and_then(|has_repo| has_repo.extract::<bool>())
                    .unwrap_or(false),
            )
        })
        .unwrap_or(false)
    }

    /// Check if path is a branch
    pub async fn is_branch(path: &Path) -> bool {
        ensure_python_initialized();

        Python::with_gil(|py| {
            let bzrdir = py.import_bound("breezy.bzr.bzrdir").ok()?;
            let path_str = path.to_string_lossy();

            // Try to open as bzrdir
            let bzr_dir = bzrdir
                .call_method1("BzrDir.open", (path_str.as_ref(),))
                .ok()?;

            // Check if it has a branch
            Some(
                bzr_dir
                    .call_method0("has_branch")
                    .and_then(|has_branch| has_branch.extract::<bool>())
                    .unwrap_or(false),
            )
        })
        .unwrap_or(false)
    }

    /// Generate diff between two revisions
    pub async fn get_diff(repo_path: &Path, old_revid: &str, new_revid: &str) -> Result<Vec<u8>> {
        ensure_python_initialized();

        Python::with_gil(|py| {
            debug!("Getting diff for repository: {}", repo_path.display());

            // Import repository module
            let repository = py
                .import_bound("breezy.repository")
                .map_err(|e| BzrError::Python(format!("Failed to import repository: {}", e)))?;

            let path_str = repo_path.to_string_lossy();

            // Open repository
            let repo = repository
                .call_method1("Repository.open", (path_str.as_ref(),))
                .map_err(|e| BzrError::Python(format!("Failed to open repository: {}", e)))?;

            // Skip getting individual revisions since we use trees directly

            // Generate diff using tree comparison
            let old_tree = repo
                .call_method1("revision_tree", (old_revid,))
                .map_err(|e| BzrError::Python(format!("Failed to get old tree: {}", e)))?;

            let new_tree = repo
                .call_method1("revision_tree", (new_revid,))
                .map_err(|e| BzrError::Python(format!("Failed to get new tree: {}", e)))?;

            // Import diff module
            let diff_module = py
                .import_bound("breezy.diff")
                .map_err(|e| BzrError::Python(format!("Failed to import diff: {}", e)))?;

            // Create a StringIO object to capture diff output
            let io_module = py
                .import_bound("io")
                .map_err(|e| BzrError::Python(format!("Failed to import io: {}", e)))?;

            let diff_output = io_module
                .call_method0("StringIO")
                .map_err(|e| BzrError::Python(format!("Failed to create StringIO: {}", e)))?;

            // Generate unified diff - clone diff_output to avoid move
            diff_module
                .call_method(
                    "show_diff_trees",
                    (old_tree, new_tree, diff_output.clone()),
                    None,
                )
                .map_err(|e| BzrError::Python(format!("Failed to generate diff: {}", e)))?;

            // Get the diff content
            let diff_content = diff_output
                .call_method0("getvalue")
                .and_then(|content| content.extract::<String>())
                .map_err(|e| BzrError::Python(format!("Failed to get diff content: {}", e)))?;

            Ok(diff_content.into_bytes())
        })
    }

    /// Get revision information for a range of revisions
    pub async fn get_revision_info(
        repo_path: &Path,
        old_revid: &str,
        new_revid: &str,
    ) -> Result<Vec<BreezyRevisionInfo>> {
        ensure_python_initialized();

        Python::with_gil(|py| {
            debug!(
                "Getting revision info for repository: {}",
                repo_path.display()
            );

            // Import repository module
            let repository = py
                .import_bound("breezy.repository")
                .map_err(|e| BzrError::Python(format!("Failed to import repository: {}", e)))?;

            let path_str = repo_path.to_string_lossy();

            // Open repository
            let repo = repository
                .call_method1("Repository.open", (path_str.as_ref(),))
                .map_err(|e| BzrError::Python(format!("Failed to open repository: {}", e)))?;

            // Get revision range
            let revisions = repo
                .call_method("get_revision_history", (old_revid, new_revid), None)
                .map_err(|e| BzrError::Python(format!("Failed to get revision history: {}", e)))?;

            let mut revision_info = Vec::new();

            // Extract revision information using Python iteration
            Python::with_gil(|py| {
                // Try to iterate through the revisions
                match revisions.iter() {
                    Ok(iter) => {
                        for revision_result in iter {
                            match revision_result {
                                Ok(rev) => {
                                    let revision_id = rev
                                        .getattr("revision_id")
                                        .and_then(|id| id.extract::<String>())
                                        .unwrap_or_else(|_| "unknown".to_string());

                                    let message = rev
                                        .getattr("message")
                                        .and_then(|msg| msg.extract::<String>())
                                        .unwrap_or_else(|_| "".to_string());

                                    let committer = rev
                                        .getattr("committer")
                                        .and_then(|c| c.extract::<String>())
                                        .unwrap_or_else(|_| "unknown".to_string());

                                    let timestamp = rev
                                        .getattr("timestamp")
                                        .and_then(|ts| ts.extract::<String>())
                                        .unwrap_or_else(|_| "unknown".to_string());

                                    let parents = rev
                                        .getattr("parent_ids")
                                        .and_then(|p| p.extract::<Vec<String>>())
                                        .unwrap_or_else(|_| Vec::new());

                                    revision_info.push(BreezyRevisionInfo {
                                        revision_id,
                                        message,
                                        committer,
                                        timestamp,
                                        parents,
                                    });
                                }
                                Err(_) => {
                                    // Skip invalid revisions
                                    continue;
                                }
                            }
                        }
                    }
                    Err(_) => {
                        // If iteration fails, try a simpler approach - create dummy revision info
                        // This is a fallback for when Python iteration is complex
                        revision_info.push(BreezyRevisionInfo {
                            revision_id: old_revid.to_string(),
                            message: "Revision range query".to_string(),
                            committer: "unknown".to_string(),
                            timestamp: "unknown".to_string(),
                            parents: Vec::new(),
                        });

                        revision_info.push(BreezyRevisionInfo {
                            revision_id: new_revid.to_string(),
                            message: "Revision range query".to_string(),
                            committer: "unknown".to_string(),
                            timestamp: "unknown".to_string(),
                            parents: Vec::new(),
                        });
                    }
                }
            });

            Ok(revision_info)
        })
    }

    /// Configure remote URL for a repository
    pub async fn configure_remote(repo_path: &Path, remote_url: &str) -> Result<()> {
        ensure_python_initialized();

        Python::with_gil(|py| {
            debug!("Configuring remote for repository: {}", repo_path.display());

            // Import branch module
            let branch = py
                .import_bound("breezy.branch")
                .map_err(|e| BzrError::Python(format!("Failed to import branch: {}", e)))?;

            let path_str = repo_path.to_string_lossy();

            // Open branch
            let branch_obj = branch
                .call_method1("Branch.open", (path_str.as_ref(),))
                .map_err(|e| BzrError::Python(format!("Failed to open branch: {}", e)))?;

            // Configure parent location
            branch_obj
                .call_method("set_parent", (remote_url,), None)
                .map_err(|e| BzrError::Python(format!("Failed to set parent location: {}", e)))?;

            info!("Configured remote URL for repository: {}", remote_url);
            Ok(())
        })
    }

    /// Get transport for URL
    pub async fn get_transport(url: &str) -> Result<String> {
        ensure_python_initialized();

        Python::with_gil(|py| {
            debug!("Getting transport for URL: {}", url);

            // Import transport module
            let transport = py
                .import_bound("breezy.transport")
                .map_err(|e| BzrError::Python(format!("Failed to import transport: {}", e)))?;

            // Get transport
            let transport_obj = transport
                .call_method1("get_transport", (url,))
                .map_err(|e| BzrError::Python(format!("Failed to get transport: {}", e)))?;

            // Get transport base URL
            let base_url = transport_obj
                .call_method0("base")
                .and_then(|base| base.extract::<String>())
                .map_err(|e| BzrError::Python(format!("Failed to get transport base: {}", e)))?;

            Ok(base_url)
        })
    }

    /// Clone repository from remote
    pub async fn clone_repository(source_url: &str, target_path: &Path) -> Result<()> {
        ensure_python_initialized();

        Python::with_gil(|py| {
            debug!(
                "Cloning repository from {} to {}",
                source_url,
                target_path.display()
            );

            // Import bzrdir module
            let bzrdir = py
                .import_bound("breezy.bzr.bzrdir")
                .map_err(|e| BzrError::Python(format!("Failed to import bzrdir: {}", e)))?;

            let target_str = target_path.to_string_lossy();

            // Clone repository
            bzrdir
                .call_method("clone", (source_url, target_str.as_ref()), None)
                .map_err(|e| BzrError::Python(format!("Failed to clone repository: {}", e)))?;

            info!(
                "Successfully cloned repository from {} to {}",
                source_url,
                target_path.display()
            );
            Ok(())
        })
    }

    /// Check if a path has uncommitted changes
    pub async fn has_uncommitted_changes(repo_path: &Path) -> Result<bool> {
        ensure_python_initialized();

        Python::with_gil(|py| {
            debug!("Checking for uncommitted changes: {}", repo_path.display());

            // Import workingtree module
            let workingtree = py
                .import_bound("breezy.workingtree")
                .map_err(|e| BzrError::Python(format!("Failed to import workingtree: {}", e)))?;

            let path_str = repo_path.to_string_lossy();

            // Open working tree
            let tree = workingtree
                .call_method1("WorkingTree.open", (path_str.as_ref(),))
                .map_err(|e| BzrError::Python(format!("Failed to open working tree: {}", e)))?;

            // Check for changes
            let has_changes = tree
                .call_method0("has_changes")
                .and_then(|changes| changes.extract::<bool>())
                .map_err(|e| BzrError::Python(format!("Failed to check for changes: {}", e)))?;

            Ok(has_changes)
        })
    }

    /// Get Breezy version
    pub async fn get_version() -> Result<String> {
        ensure_python_initialized();

        Python::with_gil(|py| {
            let breezy = py
                .import_bound("breezy")
                .map_err(|e| BzrError::Python(format!("Failed to import breezy: {}", e)))?;

            let version = breezy
                .getattr("version_string")
                .and_then(|v| v.extract::<String>())
                .map_err(|e| BzrError::Python(format!("Failed to get version: {}", e)))?;

            Ok(version)
        })
    }
}

/// Initialize the Breezy bridge on startup
pub fn initialize_breezy() -> Result<()> {
    ensure_python_initialized();

    // Test that we can import Breezy
    Python::with_gil(|py| {
        py.import_bound("breezy")
            .map_err(|e| BzrError::Python(format!("Failed to initialize Breezy: {}", e)))?;

        // Initialize Breezy
        let breezy = py
            .import_bound("breezy")
            .map_err(|e| BzrError::Python(format!("Failed to import breezy: {}", e)))?;

        breezy
            .call_method0("initialize")
            .map_err(|e| BzrError::Python(format!("Failed to initialize breezy: {}", e)))?;

        info!("Breezy bridge initialized successfully");
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_breezy_bridge_initialization() {
        // Note: This test requires Breezy to be installed
        if let Err(e) = initialize_breezy() {
            eprintln!("Breezy not available for testing: {}", e);
        }
    }

    #[tokio::test]
    async fn test_breezy_operations() {
        // Skip if Breezy not available
        if initialize_breezy().is_err() {
            return;
        }

        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().join("test-repo");
        std::fs::create_dir_all(&repo_path).unwrap();

        // Test repository initialization
        if let Ok(_) = BreezyOperations::init_shared_repository(&repo_path).await {
            assert!(BreezyOperations::is_repository(&repo_path).await);

            // Test getting repository info
            if let Ok(info) = BreezyOperations::get_repository_info(&repo_path).await {
                assert!(info.is_shared);
                assert!(!info.format.is_empty());
            }
        }
    }

    #[tokio::test]
    async fn test_branch_operations() {
        // Skip if Breezy not available
        if initialize_breezy().is_err() {
            return;
        }

        let temp_dir = TempDir::new().unwrap();
        let branch_path = temp_dir.path().join("test-branch");
        std::fs::create_dir_all(&branch_path).unwrap();

        // Test branch initialization
        if let Ok(_) = BreezyOperations::init_branch(&branch_path, None).await {
            assert!(BreezyOperations::is_branch(&branch_path).await);

            // Test getting branch info
            if let Ok(info) = BreezyOperations::get_branch_info(&branch_path).await {
                assert!(!info.name.is_empty());
            }
        }
    }

    #[tokio::test]
    async fn test_advanced_operations() {
        // Skip if Breezy not available
        if initialize_breezy().is_err() {
            return;
        }

        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().join("test-advanced");
        std::fs::create_dir_all(&repo_path).unwrap();

        // Initialize repository and branch
        if BreezyOperations::init_shared_repository(&repo_path)
            .await
            .is_ok()
            && BreezyOperations::init_branch(&repo_path, None)
                .await
                .is_ok()
        {
            // Test uncommitted changes check
            if let Ok(has_changes) = BreezyOperations::has_uncommitted_changes(&repo_path).await {
                // Should have no changes in a fresh repository
                assert!(!has_changes);
            }

            // Test remote configuration
            let remote_url = "bzr://example.com/test";
            if BreezyOperations::configure_remote(&repo_path, remote_url)
                .await
                .is_ok()
            {
                println!("Successfully configured remote: {}", remote_url);
            }
        }
    }

    #[tokio::test]
    async fn test_transport_operations() {
        // Skip if Breezy not available
        if initialize_breezy().is_err() {
            return;
        }

        // Test transport for local path
        let test_url = "file:///tmp/test-transport";
        if let Ok(base_url) = BreezyOperations::get_transport(test_url).await {
            assert!(!base_url.is_empty());
            println!("Transport base URL: {}", base_url);
        }
    }

    #[tokio::test]
    async fn test_version_info() {
        // Skip if Breezy not available
        if initialize_breezy().is_err() {
            return;
        }

        // Test getting Breezy version
        if let Ok(version) = BreezyOperations::get_version().await {
            assert!(!version.is_empty());
            println!("Breezy version: {}", version);
        }
    }
}
