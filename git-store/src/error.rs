//! Error types for the git-store service

use thiserror::Error;

/// Main error type for git-store operations
#[derive(Debug, Error)]
pub enum GitStoreError {
    /// Git operation failed
    #[error("Git operation failed: {0}")]
    GitError(#[from] git2::Error),

    /// IO operation failed
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Database operation failed
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    /// HTTP error
    #[error("HTTP error: {0}")]
    HttpError(String),

    /// Repository not found
    #[error("Repository not found: {0}")]
    RepositoryNotFound(String),

    /// Invalid SHA
    #[error("Invalid SHA: {0}")]
    InvalidSha(String),

    /// Timeout error
    #[error("Operation timed out")]
    Timeout,

    /// Authentication failed
    #[error("Authentication failed")]
    AuthenticationFailed,

    /// Permission denied
    #[error("Permission denied")]
    PermissionDenied,

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    ConfigError(String),

    /// Template error
    #[error("Template error: {0}")]
    TemplateError(#[from] tera::Error),

    /// HTTP error
    #[error("HTTP error: {0}")]
    HttpLibError(#[from] http::Error),

    /// Other error
    #[error("Other error: {0}")]
    Other(#[from] anyhow::Error),
}

/// Result type alias using GitStoreError
pub type Result<T> = std::result::Result<T, GitStoreError>;

impl axum::response::IntoResponse for GitStoreError {
    fn into_response(self) -> axum::response::Response {
        use axum::http::StatusCode;
        use axum::response::Response;

        let (status, message) = match &self {
            GitStoreError::GitError(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            GitStoreError::IoError(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            GitStoreError::DatabaseError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string())
            }
            GitStoreError::HttpError(_) => (StatusCode::BAD_GATEWAY, self.to_string()),
            GitStoreError::RepositoryNotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            GitStoreError::InvalidSha(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            GitStoreError::Timeout => (StatusCode::REQUEST_TIMEOUT, self.to_string()),
            GitStoreError::AuthenticationFailed => (StatusCode::UNAUTHORIZED, self.to_string()),
            GitStoreError::PermissionDenied => (StatusCode::FORBIDDEN, self.to_string()),
            GitStoreError::ConfigError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Configuration error".to_string())
            }
            GitStoreError::TemplateError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Template error".to_string())
            }
            GitStoreError::HttpLibError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "HTTP library error".to_string())
            }
            GitStoreError::Other(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
            }
        };

        Response::builder()
            .status(status)
            .body(message.into())
            .unwrap()
    }
}