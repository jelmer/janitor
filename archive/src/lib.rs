//! Archive crate for the Janitor project.
//!
//! This crate provides functionality for working with package archives,
//! including scanning Debian packages, generating APT repositories,
//! and providing HTTP access to repository metadata.

#![deny(missing_docs)]

// Re-export tracing for use by modules
pub use tracing;

/// Temporary prefix used for archive operations.
pub const TMP_PREFIX: &str = "janitor-apt";
/// Default timeout for Google Cloud Storage operations in seconds.
pub const DEFAULT_GCS_TIMEOUT: usize = 60 * 30;

/// Error types for archive operations.
pub mod error;

/// Enhanced scanner module for archive operations.
pub mod scanner;

/// Database integration for build queries.
pub mod database;

/// Archive configuration and setup.
pub mod config;

/// Repository generation engine.
pub mod repository;

/// Web service implementation.
pub mod web;

// Re-export commonly used types
pub use error::{ArchiveError, ArchiveResult};
pub use scanner::{BuildInfo, PackageScanner};
pub use repository::{RepositoryGenerator, RepositoryGenerationConfig};
pub use web::{ArchiveWebService, AppState, PublishRequest, PublishResponse};

// TODO(jelmer): Generate contents file
