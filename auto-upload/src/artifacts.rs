//! Artifact retrieval and processing for the auto-upload service

use std::path::{Path, PathBuf};
use tempfile::TempDir;
use tracing::{debug, info, warn};

use crate::error::{Result, UploadError};
use janitor::artifacts::{get_artifact_manager, ArtifactManager};

/// Artifact processor for handling build artifacts
pub struct ArtifactProcessor {
    /// Artifact manager for retrieving files
    artifact_manager: Box<dyn ArtifactManager>,
}

impl ArtifactProcessor {
    /// Create a new artifact processor
    pub async fn new(artifact_location: &str) -> Result<Self> {
        let artifact_manager = get_artifact_manager(artifact_location).await.map_err(|e| {
            UploadError::Config(format!("Failed to create artifact manager: {}", e))
        })?;

        Ok(Self { artifact_manager })
    }

    /// Retrieve artifacts for a build run
    pub async fn retrieve_artifacts(&self, run_id: &str) -> Result<TempDir> {
        info!("Retrieving artifacts for run {}", run_id);

        let temp_dir = TempDir::new().map_err(|e| UploadError::Io(e))?;

        match self
            .artifact_manager
            .retrieve_artifacts(run_id, temp_dir.path(), None)
            .await
        {
            Ok(_) => {
                debug!("Successfully retrieved artifacts to {:?}", temp_dir.path());
                Ok(temp_dir)
            }
            Err(e) => {
                warn!("Failed to retrieve artifacts for run {}: {:?}", run_id, e);
                Err(UploadError::ArtifactsMissing(run_id.to_string()))
            }
        }
    }

    /// Check if artifacts exist for a run
    pub async fn artifacts_exist(&self, run_id: &str) -> bool {
        // Try to retrieve artifacts to a temporary directory to check if they exist
        let temp_dir = match TempDir::new() {
            Ok(dir) => dir,
            Err(_) => return false,
        };

        match self
            .artifact_manager
            .retrieve_artifacts(run_id, temp_dir.path(), None)
            .await
        {
            Ok(_) => true,
            Err(_) => false,
        }
    }
}

/// Process and validate artifacts directory
pub struct ArtifactValidator;

impl ArtifactValidator {
    /// Validate that the artifacts directory contains expected files
    pub async fn validate_artifacts(dir: &Path) -> Result<()> {
        // Check if directory exists and is readable
        if !dir.exists() {
            return Err(UploadError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Artifacts directory does not exist: {:?}", dir),
            )));
        }

        // Check if we have at least one .changes file
        let has_changes = tokio::fs::read_dir(dir)
            .await?
            .next_entry()
            .await?
            .map(|entry| {
                entry
                    .path()
                    .extension()
                    .map(|ext| ext == "changes")
                    .unwrap_or(false)
            })
            .unwrap_or(false);

        if !has_changes {
            return Err(UploadError::NoChangesFiles);
        }

        Ok(())
    }

    /// Find all files referenced in a .changes file
    pub async fn find_referenced_files(changes_path: &Path) -> Result<Vec<PathBuf>> {
        let content = tokio::fs::read_to_string(changes_path).await?;
        let mut files = Vec::new();

        // Parse the Files section from the changes file
        let mut in_files_section = false;
        for line in content.lines() {
            if line.starts_with("Files:") {
                in_files_section = true;
                continue;
            }

            if in_files_section {
                if line.starts_with(' ') || line.starts_with('\t') {
                    // Format: <md5> <size> <section> <priority> <filename>
                    let parts: Vec<&str> = line.trim().split_whitespace().collect();
                    if parts.len() >= 5 {
                        let filename = parts[4];
                        let file_path = changes_path
                            .parent()
                            .unwrap_or(Path::new("."))
                            .join(filename);
                        files.push(file_path);
                    }
                } else if !line.is_empty() {
                    // End of Files section
                    break;
                }
            }
        }

        // Always include the changes file itself
        files.push(changes_path.to_path_buf());

        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::fs;

    #[tokio::test]
    async fn test_find_referenced_files() {
        let temp_dir = TempDir::new().unwrap();
        let changes_path = temp_dir.path().join("test_1.0-1_amd64.changes");

        // Create a sample .changes file
        let changes_content = r#"Format: 1.8
Date: Mon, 01 Jan 2024 00:00:00 +0000
Source: test
Binary: test
Architecture: amd64
Version: 1.0-1
Distribution: unstable
Urgency: medium
Maintainer: Test <test@example.com>
Changed-By: Test <test@example.com>
Description:
 test - Test package
Changes:
 test (1.0-1) unstable; urgency=medium
 .
   * Initial release.
Checksums-Sha1:
 1234567890abcdef 1000 test_1.0-1.dsc
 abcdef1234567890 2000 test_1.0.orig.tar.gz
 fedcba0987654321 3000 test_1.0-1.debian.tar.xz
 0123456789abcdef 4000 test_1.0-1_amd64.deb
Files:
 d41d8cd98f00b204e9800998ecf8427e 1000 misc optional test_1.0-1.dsc
 d41d8cd98f00b204e9800998ecf8427e 2000 misc optional test_1.0.orig.tar.gz
 d41d8cd98f00b204e9800998ecf8427e 3000 misc optional test_1.0-1.debian.tar.xz
 d41d8cd98f00b204e9800998ecf8427e 4000 misc optional test_1.0-1_amd64.deb
"#;

        fs::write(&changes_path, changes_content).await.unwrap();

        let files = ArtifactValidator::find_referenced_files(&changes_path)
            .await
            .unwrap();

        assert_eq!(files.len(), 5); // 4 referenced files + changes file itself
        assert!(files.iter().any(|f| f.ends_with("test_1.0-1.dsc")));
        assert!(files.iter().any(|f| f.ends_with("test_1.0.orig.tar.gz")));
        assert!(files
            .iter()
            .any(|f| f.ends_with("test_1.0-1.debian.tar.xz")));
        assert!(files.iter().any(|f| f.ends_with("test_1.0-1_amd64.deb")));
        assert!(files
            .iter()
            .any(|f| f.ends_with("test_1.0-1_amd64.changes")));
    }
}
