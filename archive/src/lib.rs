//! Archive crate for the Janitor project.
//!
//! This crate provides functionality for working with package archives.

#![deny(missing_docs)]

use tracing::{debug, error, info};

/// Temporary prefix used for archive operations.
pub const TMP_PREFIX: &str = "janitor-apt";
/// Default timeout for Google Cloud Storage operations in seconds.
pub const DEFAULT_GCS_TIMEOUT: usize = 60 * 30;

/// Scanner module for archive operations.
pub mod scanner;

// TODO(jelmer): Generate contents file
