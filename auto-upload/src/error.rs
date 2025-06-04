//! Error handling for the auto-upload service

use thiserror::Error;

/// Main error type for the auto-upload service
#[derive(Debug, Error)]
pub enum UploadError {
    /// Failed to sign package with debsign
    #[error("Failed to sign package: {0}")]
    DebsignFailure(String),

    /// Failed to upload package with dput
    #[error("Failed to upload package: {0}")]
    DputFailure(String),

    /// Artifacts are missing or inaccessible
    #[error("Artifacts missing for run {0}")]
    ArtifactsMissing(String),

    /// No changes files found in artifacts
    #[error("No changes files found in artifacts")]
    NoChangesFiles,

    /// Database error
    #[error("Database error: {0}")]
    Database(String),

    /// Redis error
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON parsing error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Artifact manager error
    #[error("Artifact manager error: {0}")]
    ArtifactManager(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Invalid request error
    #[error("Invalid request: {message}")]
    InvalidRequest {
        /// Error message
        message: String,
    },
}

/// Result type alias for upload operations
pub type Result<T> = std::result::Result<T, UploadError>;

/// Represents a failure from the debsign process
#[derive(Debug)]
pub struct DebsignFailureInfo {
    /// Exit code from debsign
    pub exit_code: i32,
    /// Error message or stderr output
    pub reason: String,
}

/// Represents a failure from the dput process
#[derive(Debug)]
pub struct DputFailureInfo {
    /// Exit code from dput
    pub exit_code: i32,
    /// Error message or stderr output
    pub reason: String,
}
