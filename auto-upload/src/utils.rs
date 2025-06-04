//! Utility functions for the auto-upload service

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::debug;

use crate::error::{Result, UploadError};

/// Find all .changes files in a directory
pub async fn find_changes_files(dir: &Path, source_only: bool) -> Result<Vec<PathBuf>> {
    let mut changes_files = Vec::new();
    let mut entries = fs::read_dir(dir).await?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if let Some(filename) = path.file_name() {
            let filename_str = filename.to_string_lossy();

            if filename_str.ends_with(".changes") {
                if source_only && !filename_str.ends_with("_source.changes") {
                    continue;
                }
                changes_files.push(path);
            }
        }
    }

    if changes_files.is_empty() {
        return Err(UploadError::NoChangesFiles);
    }

    Ok(changes_files)
}

/// Fix file permissions for signing
///
/// Works around https://bugs.debian.org/389908 by ensuring
/// files have appropriate permissions for GPG signing
pub async fn fix_file_permissions(dir: &Path) -> Result<()> {
    // Get current umask (work around umask issue)
    let umask = unsafe {
        let old_umask = libc::umask(0);
        libc::umask(old_umask);
        old_umask
    };

    let mut entries = fs::read_dir(dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let metadata = entry.metadata().await?;

        // Set permissions to 0o644 & ~umask
        let new_mode = 0o644 & !umask;
        let mut permissions = metadata.permissions();
        permissions.set_mode(new_mode);

        fs::set_permissions(&path, permissions).await?;
        debug!("Set permissions on {} to {:o}", path.display(), new_mode);
    }

    Ok(())
}

/// Extract run ID from a path or string
pub fn extract_run_id(s: &str) -> &str {
    // Handle both full paths and just IDs
    s.split('/').last().unwrap_or(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_find_changes_files() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        // Create test files
        fs::write(dir_path.join("test_1.0-1_amd64.changes"), "")
            .await
            .unwrap();
        fs::write(dir_path.join("test_1.0-1_source.changes"), "")
            .await
            .unwrap();
        fs::write(dir_path.join("test_1.0-1.dsc"), "")
            .await
            .unwrap();

        // Test finding all changes files
        let changes_files = find_changes_files(dir_path, false).await.unwrap();
        assert_eq!(changes_files.len(), 2);

        // Test finding only source changes
        let source_changes = find_changes_files(dir_path, true).await.unwrap();
        assert_eq!(source_changes.len(), 1);
        assert!(source_changes[0]
            .to_string_lossy()
            .ends_with("_source.changes"));
    }

    #[test]
    fn test_extract_run_id() {
        assert_eq!(extract_run_id("12345"), "12345");
        assert_eq!(extract_run_id("/path/to/12345"), "12345");
        assert_eq!(extract_run_id("prefix/12345"), "12345");
    }
}
