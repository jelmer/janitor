//! Error types and handling for the differ service.
//!
//! This module provides comprehensive error handling that matches the Python
//! implementation's error patterns and HTTP status code mappings.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use std::path::PathBuf;
use thiserror::Error;

/// Result type alias for differ operations
pub type DifferResult<T> = Result<T, DifferError>;

/// Comprehensive error types for the differ service matching Python implementation
#[derive(Error, Debug)]
pub enum DifferError {
    /// Database operation failed
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Artifacts are missing for a run
    #[error("Artifacts missing for run: {run_id}")]
    ArtifactsMissing { 
        /// The run ID for which artifacts are missing
        run_id: String 
    },

    /// Timeout while retrieving artifacts
    #[error("Timeout retrieving artifacts for run {run_id}: exceeded {timeout}s")]
    ArtifactRetrievalTimeout { 
        /// The run ID for which artifact retrieval timed out
        run_id: String, 
        /// The timeout value in seconds
        timeout: u64 
    },

    /// Generic artifact retrieval failure
    #[error("Failed to retrieve artifacts for run {run_id}: {reason}")]
    ArtifactRetrievalFailed { 
        /// The run ID for which artifact retrieval failed
        run_id: String, 
        /// The reason for the failure
        reason: String 
    },

    /// Timeout while running diff command
    #[error("Diff command '{command}' timed out after {timeout}s")]
    DiffCommandTimeout { 
        /// The command that timed out
        command: String, 
        /// The timeout value in seconds
        timeout: u64 
    },

    /// Memory error while running diff command
    #[error("Diff command '{command}' exceeded memory limit: {limit_mb}MB")]
    DiffCommandMemoryError { 
        /// The command that exceeded memory limit
        command: String, 
        /// The memory limit in megabytes
        limit_mb: usize 
    },

    /// Generic diff command error
    #[error("Diff command '{command}' failed: {reason}")]
    DiffCommandError { 
        /// The command that failed
        command: String, 
        /// The reason for the failure
        reason: String 
    },

    /// Run not found in database
    #[error("Run not found: {run_id}")]
    RunNotFound { 
        /// The run ID that was not found
        run_id: String 
    },

    /// Run exists but is not successful
    #[error("Run {run_id} is not successful (status: {status})")]
    RunNotSuccessful { 
        /// The run ID that is not successful
        run_id: String, 
        /// The actual status of the run
        status: String 
    },

    /// IO operation failed
    #[error("IO error during {operation} on {path:?}: {source}")]
    IoError {
        /// The operation that failed
        operation: String,
        /// The path where the operation failed
        path: PathBuf,
        /// The underlying IO error
        #[source]
        source: std::io::Error,
    },

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Content negotiation failed
    #[error("No acceptable content type found. Available: {available:?}, Requested: {requested}")]
    ContentNegotiationFailed {
        /// List of available content types
        available: Vec<String>,
        /// The requested content type that was not available
        requested: String,
    },

    /// Cache operation failed
    #[error("Cache operation failed for {operation}: {reason}")]
    CacheError { 
        /// The cache operation that failed
        operation: String, 
        /// The reason for the failure
        reason: String 
    },

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Redis connection or operation error
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    /// Janitor artifacts error
    #[error("Janitor artifacts error: {0}")]
    JanitorArtifacts(janitor::artifacts::Error),

    /// Janitor debdiff error
    #[error("Debdiff error: {0}")]
    JanitorDebdiff(String),

    /// Diffoscope error from crate diffoscope module
    #[error("Diffoscope error: {0}")]
    Diffoscope(String),

    /// Invalid header value
    #[error("Invalid header value: {0}")]
    InvalidHeaderValue(#[from] axum::http::header::InvalidHeaderValue),

    /// Accept header parsing error
    #[error("Accept header parse error: {0}")]
    AcceptHeaderError(String),
}

/// Error response structure for JSON responses
#[derive(serde::Serialize)]
pub struct ErrorResponse {
    /// The main error message
    pub error: String,
    /// Additional details about the error
    pub details: Option<String>,
    /// The run ID associated with the error, if applicable
    pub run_id: Option<String>,
    /// The command associated with the error, if applicable
    pub command: Option<String>,
}

impl DifferError {
    /// Convert error to appropriate HTTP status code
    pub fn status_code(&self) -> StatusCode {
        match self {
            // Client errors (4xx)
            DifferError::RunNotFound { .. } => StatusCode::NOT_FOUND,
            DifferError::ArtifactsMissing { .. } => StatusCode::NOT_FOUND,
            DifferError::ContentNegotiationFailed { .. } => StatusCode::NOT_ACCEPTABLE,
            DifferError::AcceptHeaderError(_) => StatusCode::BAD_REQUEST,
            DifferError::InvalidHeaderValue(_) => StatusCode::BAD_REQUEST,

            // Server errors (5xx)
            DifferError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            DifferError::ArtifactRetrievalTimeout { .. } => StatusCode::GATEWAY_TIMEOUT,
            DifferError::ArtifactRetrievalFailed { .. } => StatusCode::BAD_GATEWAY,
            DifferError::DiffCommandTimeout { .. } => StatusCode::GATEWAY_TIMEOUT,
            DifferError::DiffCommandMemoryError { .. } => StatusCode::INSUFFICIENT_STORAGE,
            DifferError::DiffCommandError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            DifferError::RunNotSuccessful { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            DifferError::IoError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            DifferError::JsonError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            DifferError::CacheError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            DifferError::ConfigError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            DifferError::Redis(_) => StatusCode::INTERNAL_SERVER_ERROR,
            DifferError::JanitorArtifacts(_) => StatusCode::INTERNAL_SERVER_ERROR,
            DifferError::JanitorDebdiff(_) => StatusCode::INTERNAL_SERVER_ERROR,
            DifferError::Diffoscope(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Create error response with optional additional headers
    pub fn to_response(&self) -> ErrorResponse {
        match self {
            DifferError::ArtifactsMissing { run_id } => ErrorResponse {
                error: "Artifacts missing".to_string(),
                details: Some(format!("No artifacts found for run {}", run_id)),
                run_id: Some(run_id.clone()),
                command: None,
            },
            DifferError::RunNotFound { run_id } => ErrorResponse {
                error: "Run not found".to_string(),
                details: Some(format!("Run {} does not exist", run_id)),
                run_id: Some(run_id.clone()),
                command: None,
            },
            DifferError::RunNotSuccessful { run_id, status } => ErrorResponse {
                error: "Run not successful".to_string(),
                details: Some(format!("Run {} has status '{}', not 'success'", run_id, status)),
                run_id: Some(run_id.clone()),
                command: None,
            },
            DifferError::DiffCommandTimeout { command, timeout } => ErrorResponse {
                error: "Command timeout".to_string(),
                details: Some(format!("Command '{}' timed out after {}s", command, timeout)),
                run_id: None,
                command: Some(command.clone()),
            },
            DifferError::DiffCommandMemoryError { command, limit_mb } => ErrorResponse {
                error: "Memory limit exceeded".to_string(),
                details: Some(format!("Command '{}' exceeded memory limit of {}MB", command, limit_mb)),
                run_id: None,
                command: Some(command.clone()),
            },
            DifferError::DiffCommandError { command, reason } => ErrorResponse {
                error: "Command failed".to_string(),
                details: Some(format!("Command '{}' failed: {}", command, reason)),
                run_id: None,
                command: Some(command.clone()),
            },
            DifferError::ArtifactRetrievalTimeout { run_id, timeout } => ErrorResponse {
                error: "Artifact retrieval timeout".to_string(),
                details: Some(format!("Timed out retrieving artifacts for run {} after {}s", run_id, timeout)),
                run_id: Some(run_id.clone()),
                command: None,
            },
            DifferError::ContentNegotiationFailed { available, requested } => ErrorResponse {
                error: "Content negotiation failed".to_string(),
                details: Some(format!("Requested '{}' not available. Available: {:?}", requested, available)),
                run_id: None,
                command: None,
            },
            _ => ErrorResponse {
                error: "Internal server error".to_string(),
                details: Some(self.to_string()),
                run_id: None,
                command: None,
            },
        }
    }

    /// Get additional headers for the response
    pub fn additional_headers(&self) -> Vec<(&'static str, String)> {
        match self {
            DifferError::ArtifactsMissing { run_id } | 
            DifferError::RunNotFound { run_id } |
            DifferError::RunNotSuccessful { run_id, .. } => {
                vec![("unavailable_run_id", run_id.clone())]
            },
            _ => vec![],
        }
    }
}

impl IntoResponse for DifferError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let headers = self.additional_headers();
        let error_response = self.to_response();

        let mut response = (status, Json(error_response)).into_response();
        
        // Add any additional headers
        for (key, value) in headers {
            if let Ok(header_value) = value.parse() {
                response.headers_mut().insert(key, header_value);
            }
        }

        response
    }
}

impl DifferError {
    /// Create a JanitorArtifacts error
    pub fn from_janitor_artifacts(error: janitor::artifacts::Error) -> Self {
        match error {
            janitor::artifacts::Error::ArtifactsMissing => {
                // Note: We lose the run_id context here, caller should provide it
                DifferError::ArtifactsMissing { 
                    run_id: "unknown".to_string() 
                }
            },
            _ => DifferError::JanitorArtifacts(error),
        }
    }

    /// Create a JanitorDebdiff error
    pub fn from_janitor_debdiff<E: std::fmt::Display>(error: E) -> Self {
        DifferError::JanitorDebdiff(error.to_string())
    }

    /// Create a Diffoscope error
    pub fn from_diffoscope<E: std::fmt::Display>(error: E) -> Self {
        DifferError::Diffoscope(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_status_codes() {
        assert_eq!(
            DifferError::RunNotFound { run_id: "test".to_string() }.status_code(),
            StatusCode::NOT_FOUND
        );
        
        assert_eq!(
            DifferError::ArtifactsMissing { run_id: "test".to_string() }.status_code(),
            StatusCode::NOT_FOUND
        );

        assert_eq!(
            DifferError::ContentNegotiationFailed { 
                available: vec!["text/html".to_string()], 
                requested: "application/json".to_string() 
            }.status_code(),
            StatusCode::NOT_ACCEPTABLE
        );

        assert_eq!(
            DifferError::DiffCommandTimeout { 
                command: "diffoscope".to_string(), 
                timeout: 60 
            }.status_code(),
            StatusCode::GATEWAY_TIMEOUT
        );

        assert_eq!(
            DifferError::DiffCommandMemoryError { 
                command: "diffoscope".to_string(), 
                limit_mb: 1500 
            }.status_code(),
            StatusCode::INSUFFICIENT_STORAGE
        );
    }

    #[test]
    fn test_error_responses() {
        let error = DifferError::ArtifactsMissing { run_id: "test123".to_string() };
        let response = error.to_response();
        
        assert_eq!(response.error, "Artifacts missing");
        assert_eq!(response.run_id, Some("test123".to_string()));
        assert!(response.details.is_some());
    }

    #[test]
    fn test_additional_headers() {
        let error = DifferError::ArtifactsMissing { run_id: "test123".to_string() };
        let headers = error.additional_headers();
        
        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].0, "unavailable_run_id");
        assert_eq!(headers[0].1, "test123");
    }
}