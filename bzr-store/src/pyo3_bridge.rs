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

/// High-level convenience functions for Breezy operations
pub struct BreezyOperations;

impl BreezyOperations {
    /// Initialize a shared repository
    pub async fn init_shared_repository(path: &Path) -> Result<()> {
        ensure_python_initialized();
        
        Python::with_gil(|py| {
            debug!("Creating shared repository at: {}", path.display());
            
            // Import bzrdir module
            let bzrdir = py.import_bound("breezy.bzr.bzrdir")
                .map_err(|e| BzrError::Python(format!("Failed to import bzrdir: {}", e)))?;
            
            let path_str = path.to_string_lossy();
            
            // Create shared repository
            bzrdir.call_method1("BzrDir.create_repository", (path_str.as_ref(), true))
                .map_err(|e| BzrError::Python(format!("Failed to create shared repository: {}", e)))?;
            
            info!("Successfully created shared repository at: {}", path.display());
            Ok(())
        })
    }
    
    /// Initialize a branch
    pub async fn init_branch(path: &Path, _repository_path: Option<&Path>) -> Result<()> {
        ensure_python_initialized();
        
        Python::with_gil(|py| {
            debug!("Initializing branch at: {}", path.display());
            
            // Import bzrdir module
            let bzrdir = py.import_bound("breezy.bzr.bzrdir")
                .map_err(|e| BzrError::Python(format!("Failed to import bzrdir: {}", e)))?;
            
            let path_str = path.to_string_lossy();
            
            // Create bzrdir
            let bzr_dir = bzrdir.call_method1("BzrDir.create", (path_str.as_ref(),))
                .map_err(|e| BzrError::Python(format!("Failed to create bzrdir: {}", e)))?;
            
            // Create branch
            bzr_dir.call_method0("create_branch")
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
            let repository = py.import_bound("breezy.repository")
                .map_err(|e| BzrError::Python(format!("Failed to import repository: {}", e)))?;
            
            let path_str = path.to_string_lossy();
            
            // Open repository
            let repo = repository.call_method1("Repository.open", (path_str.as_ref(),))
                .map_err(|e| BzrError::Python(format!("Failed to open repository: {}", e)))?;
            
            // Get repository information
            let is_shared = repo.call_method0("is_shared")
                .and_then(|r| r.extract::<bool>())
                .unwrap_or(false);
            
            let format = repo.getattr("_format")
                .and_then(|f| f.call_method0("get_format_string"))
                .and_then(|s| s.extract::<String>())
                .unwrap_or_else(|_| "unknown".to_string());
            
            // Try to get revision count (might fail for some repository types)
            let revision_count = repo.call_method0("revision_count")
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
            let branch = py.import_bound("breezy.branch")
                .map_err(|e| BzrError::Python(format!("Failed to import branch: {}", e)))?;
            
            let path_str = path.to_string_lossy();
            
            // Open branch
            let branch_obj = branch.call_method1("Branch.open", (path_str.as_ref(),))
                .map_err(|e| BzrError::Python(format!("Failed to open branch: {}", e)))?;
            
            // Get branch information
            let name = branch_obj.call_method0("get_user_name")
                .and_then(|n| n.extract::<String>())
                .unwrap_or_else(|_| "main".to_string());
            
            let last_revision = branch_obj.call_method0("last_revision")
                .and_then(|rev| rev.extract::<String>())
                .ok();
            
            let revision_count = branch_obj.call_method0("revision_count")
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
            let bzr_dir = bzrdir.call_method1("BzrDir.open", (path_str.as_ref(),)).ok()?;
            
            // Check if it has a repository
            Some(bzr_dir.call_method0("has_repository")
                .and_then(|has_repo| has_repo.extract::<bool>())
                .unwrap_or(false))
        }).unwrap_or(false)
    }
    
    /// Check if path is a branch
    pub async fn is_branch(path: &Path) -> bool {
        ensure_python_initialized();
        
        Python::with_gil(|py| {
            let bzrdir = py.import_bound("breezy.bzr.bzrdir").ok()?;
            let path_str = path.to_string_lossy();
            
            // Try to open as bzrdir
            let bzr_dir = bzrdir.call_method1("BzrDir.open", (path_str.as_ref(),)).ok()?;
            
            // Check if it has a branch
            Some(bzr_dir.call_method0("has_branch")
                .and_then(|has_branch| has_branch.extract::<bool>())
                .unwrap_or(false))
        }).unwrap_or(false)
    }
    
    /// Get Breezy version
    pub async fn get_version() -> Result<String> {
        ensure_python_initialized();
        
        Python::with_gil(|py| {
            let breezy = py.import_bound("breezy")
                .map_err(|e| BzrError::Python(format!("Failed to import breezy: {}", e)))?;
            
            let version = breezy.getattr("version_string")
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
        let breezy = py.import_bound("breezy")
            .map_err(|e| BzrError::Python(format!("Failed to import breezy: {}", e)))?;
        
        breezy.call_method0("initialize")
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
            return;
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
}