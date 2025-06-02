//! Error types for the archive service.

use thiserror::Error;

/// Errors that can occur during archive operations.
#[derive(Error, Debug)]
pub enum ArchiveError {
    /// Database operation failed.
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// I/O operation failed.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Package scanning failed.
    #[error("Package scanning failed: {0}")]
    PackageScanning(String),

    /// Source scanning failed.
    #[error("Source scanning failed: {0}")]
    SourceScanning(String),

    /// Artifact retrieval failed.
    #[error("Artifact retrieval failed: {0}")]
    ArtifactRetrieval(String),

    /// Missing artifacts for build.
    #[error("Missing artifacts for build {build_id}: {message}")]
    ArtifactsMissing {
        /// Build ID for which artifacts are missing.
        build_id: String,
        /// Error message describing the issue.
        message: String,
    },

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Temporary directory creation failed.
    #[error("Failed to create temporary directory: {0}")]
    TempDir(#[from] tempfile::PersistError),

    /// Repository generation failed.
    #[error("Repository generation failed: {0}")]
    RepositoryGeneration(String),

    /// GPG operation failed.
    #[error("GPG operation failed: {0}")]
    Gpg(String),

    /// Compression operation failed.
    #[error("Compression failed: {0}")]
    Compression(String),

    /// Invalid archive configuration.
    #[error("Invalid archive configuration: {0}")]
    InvalidConfiguration(String),

    /// Redis operation error.
    #[error("Redis error: {0}")]
    Redis(String),

    /// Serialization error.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Resource limit exceeded.
    #[error("Resource limit exceeded: {0}")]
    ResourceLimit(String),

    /// Resource not found.
    #[error("Resource not found: {0}")]
    NotFound(String),
}

/// Result type for archive operations.
pub type ArchiveResult<T> = Result<T, ArchiveError>;
