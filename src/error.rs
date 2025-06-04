//! Shared error types and utilities for the Janitor platform

/// Common error type for all Janitor services
#[derive(Debug)]
pub enum JanitorError {
    /// Database-related errors
    Database(sqlx::Error),

    /// I/O errors
    Io(std::io::Error),

    /// JSON parsing errors
    Json(serde_json::Error),

    /// HTTP client errors
    Http(reqwest::Error),

    /// Configuration errors
    Config(String),

    /// Authentication/authorization errors
    Auth(String),

    /// Validation errors for user input
    Validation(String),

    /// External service errors (VCS, build tools, etc.)
    ExternalService { service: String, message: String },

    /// Rate limiting errors
    RateLimit(String),

    /// Timeout errors
    Timeout(String),

    /// Resource not found
    NotFound { resource: String, id: String },

    /// Resource already exists
    AlreadyExists { resource: String, id: String },

    /// Permission denied
    PermissionDenied(String),

    /// Internal server errors
    Internal(String),
}

impl std::fmt::Display for JanitorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database(e) => write!(f, "Database error: {}", e),
            Self::Io(e) => write!(f, "I/O error: {}", e),
            Self::Json(e) => write!(f, "JSON error: {}", e),
            Self::Http(e) => write!(f, "HTTP client error: {}", e),
            Self::Config(msg) => write!(f, "Configuration error: {}", msg),
            Self::Auth(msg) => write!(f, "Authentication error: {}", msg),
            Self::Validation(msg) => write!(f, "Validation error: {}", msg),
            Self::ExternalService { service, message } => {
                write!(f, "External service error: {}: {}", service, message)
            }
            Self::RateLimit(msg) => write!(f, "Rate limited: {}", msg),
            Self::Timeout(msg) => write!(f, "Operation timed out: {}", msg),
            Self::NotFound { resource, id } => write!(f, "Not found: {} '{}'", resource, id),
            Self::AlreadyExists { resource, id } => {
                write!(f, "Already exists: {} '{}'", resource, id)
            }
            Self::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            Self::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for JanitorError {}

impl From<sqlx::Error> for JanitorError {
    fn from(e: sqlx::Error) -> Self {
        Self::Database(e)
    }
}

impl From<std::io::Error> for JanitorError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<serde_json::Error> for JanitorError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

impl From<reqwest::Error> for JanitorError {
    fn from(e: reqwest::Error) -> Self {
        Self::Http(e)
    }
}

impl JanitorError {
    /// Create a configuration error
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    /// Create an authentication error
    pub fn auth(msg: impl Into<String>) -> Self {
        Self::Auth(msg.into())
    }

    /// Create a validation error
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }

    /// Create an external service error
    pub fn external_service(service: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ExternalService {
            service: service.into(),
            message: message.into(),
        }
    }

    /// Create a not found error
    pub fn not_found(resource: impl Into<String>, id: impl Into<String>) -> Self {
        Self::NotFound {
            resource: resource.into(),
            id: id.into(),
        }
    }

    /// Create an already exists error
    pub fn already_exists(resource: impl Into<String>, id: impl Into<String>) -> Self {
        Self::AlreadyExists {
            resource: resource.into(),
            id: id.into(),
        }
    }

    /// Create a permission denied error
    pub fn permission_denied(msg: impl Into<String>) -> Self {
        Self::PermissionDenied(msg.into())
    }

    /// Create an internal error
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }

    /// Check if the error is transient (worth retrying)
    pub fn is_transient(&self) -> bool {
        match self {
            Self::Database(sqlx::Error::Io(_)) => true,
            Self::Database(sqlx::Error::PoolTimedOut) => true,
            Self::Http(e) => e.is_timeout() || e.is_connect(),
            Self::RateLimit(_) => true,
            Self::Timeout(_) => true,
            Self::ExternalService { .. } => true, // Often network-related
            _ => false,
        }
    }

    /// Get the appropriate HTTP status code for this error
    pub fn http_status(&self) -> u16 {
        match self {
            Self::NotFound { .. } => 404,
            Self::AlreadyExists { .. } => 409,
            Self::PermissionDenied(_) => 403,
            Self::Auth(_) => 401,
            Self::Validation(_) => 400,
            Self::RateLimit(_) => 429,
            Self::Timeout(_) => 408,
            _ => 500,
        }
    }
}

/// Result type alias using JanitorError
pub type Result<T> = std::result::Result<T, JanitorError>;

/// Convert common error types to JanitorError
impl From<url::ParseError> for JanitorError {
    fn from(e: url::ParseError) -> Self {
        Self::Validation(format!("Invalid URL: {}", e))
    }
}

/// Error conversion utilities
pub trait ErrorContext<T> {
    /// Add context to an error
    fn with_context<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> String;

    /// Add simple string context to an error
    fn context(self, msg: &str) -> Result<T>;
}

impl<T, E> ErrorContext<T> for std::result::Result<T, E>
where
    E: Into<JanitorError>,
{
    fn with_context<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|e| {
            let base_error = e.into();
            match base_error {
                JanitorError::Internal(msg) => JanitorError::Internal(format!("{}: {}", f(), msg)),
                other => JanitorError::Internal(format!("{}: {}", f(), other)),
            }
        })
    }

    fn context(self, msg: &str) -> Result<T> {
        self.with_context(|| msg.to_string())
    }
}

/// Helper for creating validation errors
pub fn validation_error(msg: impl Into<String>) -> JanitorError {
    JanitorError::Validation(msg.into())
}

/// Helper for creating not found errors
pub fn not_found(resource: impl Into<String>, id: impl Into<String>) -> JanitorError {
    JanitorError::not_found(resource, id)
}

/// Helper for creating internal errors
pub fn internal_error(msg: impl Into<String>) -> JanitorError {
    JanitorError::Internal(msg.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = JanitorError::not_found("run", "test-123");
        assert_eq!(err.to_string(), "Not found: run 'test-123'");
        assert_eq!(err.http_status(), 404);
    }

    #[test]
    fn test_transient_errors() {
        assert!(JanitorError::RateLimit("test".to_string()).is_transient());
        assert!(JanitorError::Timeout("test".to_string()).is_transient());
        assert!(!JanitorError::NotFound {
            resource: "test".to_string(),
            id: "123".to_string()
        }
        .is_transient());
    }

    #[test]
    fn test_http_status_codes() {
        assert_eq!(JanitorError::not_found("test", "123").http_status(), 404);
        assert_eq!(JanitorError::validation("test").http_status(), 400);
        assert_eq!(JanitorError::permission_denied("test").http_status(), 403);
    }

    #[test]
    fn test_error_context() {
        let result: std::result::Result<(), std::io::Error> = Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file not found",
        ));

        let err = result.context("Failed to read config").unwrap_err();
        assert!(err.to_string().contains("Failed to read config"));
    }
}
