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
pub fn is_binary(name: &str) -> bool {
    name.ends_with(".deb") || name.ends_with(".udeb")
}
