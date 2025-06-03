//! Main upload processing logic

use std::path::Path;
use tracing::{error, info, warn};

use crate::artifacts::{ArtifactProcessor, ArtifactValidator};
use crate::error::{Result, UploadError};
use crate::upload::{sign_package, upload_package, UploadConfig};
use crate::utils::{find_changes_files, fix_file_permissions};

/// Process and upload a build result
pub async fn upload_build_result(
    run_id: &str,
    artifact_processor: &ArtifactProcessor,
    upload_config: &UploadConfig,
) -> Result<()> {
    info!(
        run_id = run_id,
        dput_host = upload_config.dput_host,
        "Processing upload for run {} to {}",
        run_id, upload_config.dput_host
    );
    
    // Retrieve artifacts
    let temp_dir = match artifact_processor.retrieve_artifacts(run_id).await {
        Ok(dir) => dir,
        Err(UploadError::ArtifactsMissing(_)) => {
            error!(
                run_id = run_id,
                "Artifacts for build {} are missing",
                run_id
            );
            return Err(UploadError::ArtifactsMissing(run_id.to_string()));
        }
        Err(e) => return Err(e),
    };
    
    let artifacts_path = temp_dir.path();
    
    // Validate artifacts
    ArtifactValidator::validate_artifacts(artifacts_path).await?;
    
    // Fix file permissions for signing (works around https://bugs.debian.org/389908)
    fix_file_permissions(artifacts_path).await?;
    
    // Find changes files
    let changes_files = find_changes_files(artifacts_path, upload_config.source_only).await?;
    
    if changes_files.is_empty() {
        error!(
            run_id = run_id,
            "No changes files found in build artifacts"
        );
        return Err(UploadError::NoChangesFiles);
    }
    
    info!(
        run_id = run_id,
        count = changes_files.len(),
        "Found {} changes files to process",
        changes_files.len()
    );
    
    let mut had_failures = false;
    let mut successful_uploads = 0;
    
    for changes_path in &changes_files {
        let changes_filename = changes_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        
        info!(
            run_id = run_id,
            changes_file = %changes_filename,
            "Processing {}",
            changes_filename
        );
        
        // Sign the package
        match sign_package(
            artifacts_path,
            &changes_filename,
            upload_config.debsign_keyid.as_deref(),
        )
        .await
        {
            Ok(_) => {
                info!(
                    run_id = run_id,
                    changes_file = %changes_filename,
                    "Successfully signed {} for {}",
                    changes_filename, run_id
                );
            }
            Err(e) => {
                error!(
                    run_id = run_id,
                    changes_file = %changes_filename,
                    error = %e,
                    "Failed to sign {} for {}: {}",
                    changes_filename, run_id, e
                );
                had_failures = true;
                continue; // Skip upload if signing failed
            }
        }
        
        // Upload the package
        match upload_package(artifacts_path, &changes_filename, &upload_config.dput_host).await {
            Ok(_) => {
                info!(
                    run_id = run_id,
                    changes_file = %changes_filename,
                    "Successfully uploaded {} for {}",
                    changes_filename, run_id
                );
                successful_uploads += 1;
            }
            Err(e) => {
                error!(
                    run_id = run_id,
                    changes_file = %changes_filename,
                    error = %e,
                    "Failed to upload {} for {}: {}",
                    changes_filename, run_id, e
                );
                had_failures = true;
            }
        }
    }
    
    if !had_failures {
        info!(
            run_id = run_id,
            successful_uploads = successful_uploads,
            "Successfully uploaded all {} packages for run {}",
            successful_uploads, run_id
        );
        Ok(())
    } else if successful_uploads > 0 {
        warn!(
            run_id = run_id,
            successful_uploads = successful_uploads,
            total = changes_files.len(),
            "Partially uploaded run {} ({}/{} packages successful)",
            run_id, successful_uploads, changes_files.len()
        );
        Err(UploadError::DputFailure(format!(
            "Failed to upload some packages ({}/{} successful)",
            successful_uploads,
            changes_files.len()
        )))
    } else {
        error!(
            run_id = run_id,
            "Failed to upload any packages for run {}",
            run_id
        );
        Err(UploadError::DputFailure(
            "Failed to upload any packages".to_string(),
        ))
    }
}

/// Extract distribution from changes file content
pub async fn extract_distribution_from_changes(changes_path: &Path) -> Result<String> {
    let content = tokio::fs::read_to_string(changes_path).await?;
    
    for line in content.lines() {
        if line.starts_with("Distribution:") {
            return Ok(line
                .trim_start_matches("Distribution:")
                .trim()
                .to_string());
        }
    }
    
    Err(UploadError::InvalidRequest {
        message: "No Distribution field found in changes file".to_string(),
    })
}

/// Check if a changes file should be uploaded based on configuration
pub async fn should_upload_changes(
    changes_path: &Path,
    upload_config: &UploadConfig,
) -> Result<bool> {
    // Check source-only filter
    if upload_config.source_only {
        let filename = changes_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        if !filename.ends_with("_source.changes") {
            return Ok(false);
        }
    }
    
    // Check distribution filter
    if !upload_config.distributions.is_empty() {
        let distribution = extract_distribution_from_changes(changes_path).await?;
        if !upload_config.should_upload_distribution(&distribution) {
            return Ok(false);
        }
    }
    
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;
    
    #[tokio::test]
    async fn test_extract_distribution_from_changes() {
        let temp_dir = TempDir::new().unwrap();
        let changes_path = temp_dir.path().join("test.changes");
        
        let content = r#"Format: 1.8
Source: test
Version: 1.0-1
Distribution: unstable
Maintainer: Test <test@example.com>
"#;
        
        fs::write(&changes_path, content).await.unwrap();
        
        let distribution = extract_distribution_from_changes(&changes_path)
            .await
            .unwrap();
        assert_eq!(distribution, "unstable");
    }
    
    #[tokio::test]
    async fn test_should_upload_changes_source_only() {
        let temp_dir = TempDir::new().unwrap();
        let source_changes = temp_dir.path().join("test_1.0-1_source.changes");
        let binary_changes = temp_dir.path().join("test_1.0-1_amd64.changes");
        
        let content = r#"Format: 1.8
Distribution: unstable
"#;
        
        fs::write(&source_changes, content).await.unwrap();
        fs::write(&binary_changes, content).await.unwrap();
        
        let config = UploadConfig::new(
            "test".to_string(),
            None,
            true, // source_only = true
            vec![],
        );
        
        assert!(should_upload_changes(&source_changes, &config).await.unwrap());
        assert!(!should_upload_changes(&binary_changes, &config).await.unwrap());
    }
}