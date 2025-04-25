//! Auto-upload crate for the Janitor project.
//!
//! This crate provides functionality for automatically uploading Debian packages.

#![deny(missing_docs)]

/// Re-export for signing Debian packages
pub use silver_platter::debian::uploader::debsign;

/// Re-export for uploading Debian changes files
pub use silver_platter::debian::uploader::dput_changes;
