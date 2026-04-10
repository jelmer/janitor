//! Differ crate for the Janitor project.
//!
//! This crate provides functionality for finding and comparing binary files.

#![deny(missing_docs)]

/// Module for interacting with diffoscope
pub mod diffoscope;

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

/// Find binary files in a directory.
///
/// # Arguments
/// * `path` - The directory to search
///
/// # Returns
/// An iterator of (filename, path) pairs
pub fn find_binaries(path: &Path) -> impl Iterator<Item = (OsString, PathBuf)> {
    std::fs::read_dir(path).unwrap().filter_map(|entry| {
        let entry = entry.ok()?;
        let path = entry.path();
        Some((entry.file_name(), path))
    })
}

/// Check if a filename is a binary package.
///
/// # Arguments
/// * `name` - The filename to check
///
/// # Returns
/// `true` if the file is a binary package, `false` otherwise
pub fn is_binary(name: &OsStr) -> bool {
    name.to_str().map_or(false, |name| {
        name.ends_with(".deb") || name.ends_with(".udeb")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;
    use tempfile::TempDir;

    #[test]
    fn test_is_binary_deb() {
        assert_eq!(is_binary(OsStr::new("package_1.0_amd64.deb")), true);
    }

    #[test]
    fn test_is_binary_udeb() {
        assert_eq!(is_binary(OsStr::new("package_1.0_amd64.udeb")), true);
    }

    #[test]
    fn test_is_binary_not_binary() {
        assert_eq!(is_binary(OsStr::new("package_1.0.dsc")), false);
        assert_eq!(is_binary(OsStr::new("package_1.0.tar.gz")), false);
        assert_eq!(is_binary(OsStr::new("package_1.0.changes")), false);
        assert_eq!(is_binary(OsStr::new("Makefile")), false);
    }

    #[test]
    fn test_find_binaries() {
        let td = TempDir::new().unwrap();
        std::fs::write(td.path().join("package.deb"), b"fake deb").unwrap();
        std::fs::write(td.path().join("source.dsc"), b"fake dsc").unwrap();
        std::fs::write(td.path().join("installer.udeb"), b"fake udeb").unwrap();

        let entries: Vec<(OsString, PathBuf)> = find_binaries(td.path()).collect();
        assert_eq!(entries.len(), 3);

        let mut names: Vec<String> = entries
            .iter()
            .map(|(name, _)| name.to_string_lossy().to_string())
            .collect();
        names.sort();
        assert_eq!(names, vec!["installer.udeb", "package.deb", "source.dsc"]);
    }

    #[test]
    fn test_find_binaries_empty_dir() {
        let td = TempDir::new().unwrap();
        let entries: Vec<(OsString, PathBuf)> = find_binaries(td.path()).collect();
        assert_eq!(entries.len(), 0);
    }
}
