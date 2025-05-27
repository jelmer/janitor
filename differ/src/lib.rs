//! Differ crate for the Janitor project.
//!
//! This crate provides functionality for finding and comparing binary files.

#![deny(missing_docs)]

/// Module for interacting with diffoscope
pub mod diffoscope;
/// Error types and handling for the differ service
pub mod error;

/// Test utilities for integration testing
#[doc(hidden)]
pub mod test_utils;

use std::ffi::OsString;
use std::path::{Path, PathBuf};

pub use error::{DifferError, DifferResult};

/// Find binary files in a directory.
///
/// # Arguments
/// * `path` - The directory to search
///
/// # Returns
/// A result containing an iterator of (filename, path) pairs
///
/// # Errors
/// Returns `DifferError::IoError` if the directory cannot be read
pub fn find_binaries(path: &Path) -> DifferResult<impl Iterator<Item = (OsString, PathBuf)>> {
    let iter = std::fs::read_dir(path)
        .map_err(|e| DifferError::IoError {
            operation: "read_dir".to_string(),
            path: path.to_path_buf(),
            source: e,
        })?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            Some((entry.file_name(), path))
        });
    Ok(iter)
}

/// Check if a filename is a binary package.
///
/// # Arguments
/// * `name` - The filename to check
///
/// # Returns
/// `true` if the file is a binary package, `false` otherwise
pub fn is_binary(name: &str) -> bool {
    name.ends_with(".deb") || name.ends_with(".udeb")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::TempDir;

    #[test]
    fn test_is_binary() {
        assert!(is_binary("package.deb"));
        assert!(is_binary("package.udeb"));
        assert!(!is_binary("package.tar.gz"));
        assert!(!is_binary("file.txt"));
    }

    #[test]
    fn test_find_binaries_empty_dir() -> DifferResult<()> {
        let temp_dir = TempDir::new().unwrap();
        let binaries: Vec<_> = find_binaries(temp_dir.path())?.collect();
        assert_eq!(binaries.len(), 0);
        Ok(())
    }

    #[test]
    fn test_find_binaries_with_files() -> DifferResult<()> {
        let temp_dir = TempDir::new().unwrap();

        // Create some test files
        File::create(temp_dir.path().join("test.deb")).unwrap();
        File::create(temp_dir.path().join("other.txt")).unwrap();
        File::create(temp_dir.path().join("another.udeb")).unwrap();

        let binaries: Vec<_> = find_binaries(temp_dir.path())?.collect();
        assert_eq!(binaries.len(), 3);

        let filenames: Vec<String> = binaries
            .iter()
            .map(|(name, _)| name.to_string_lossy().to_string())
            .collect();

        assert!(filenames.contains(&"test.deb".to_string()));
        assert!(filenames.contains(&"other.txt".to_string()));
        assert!(filenames.contains(&"another.udeb".to_string()));

        Ok(())
    }

    #[test]
    fn test_find_binaries_nonexistent_dir() {
        let result = find_binaries(Path::new("/nonexistent/path"));
        assert!(result.is_err());
        if let Err(error) = result {
            match error {
                DifferError::IoError { .. } => {} // Expected
                _ => panic!("Expected IoError, got: {:?}", error),
            }
        }
    }
}
