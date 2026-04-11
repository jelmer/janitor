//! Shared authentication module for Janitor services
//!
//! This module provides common authentication primitives that can be used
//! across all Janitor services to eliminate code duplication and ensure
//! consistent security practices.

pub mod basic;
pub mod extractors;
pub mod middleware;
pub mod session;
pub mod types;

pub use basic::*;
pub use extractors::*;
pub use middleware::*;
pub use session::*;
pub use types::*;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sqlx::{Pool, Postgres, Row};
use std::sync::Arc;

/// Unified authentication context for all services
#[derive(Debug, Clone)]
pub enum AuthContext {
    /// Worker authentication (Basic Auth)
    Worker(WorkerAuth),
    /// User session authentication (OIDC/OAuth2)
    User(UserContext),
    /// API key authentication
    ApiKey(String),
    /// Anonymous/unauthenticated request
    Anonymous,
}

impl AuthContext {
    /// Check if the context has write permissions
    pub fn can_write(&self) -> bool {
        match self {
            AuthContext::Worker(_) => true,
            AuthContext::User(user) => user.can_write(),
            AuthContext::ApiKey(_) => true, // API keys typically have write access
            AuthContext::Anonymous => false,
        }
    }

    /// Check if the context has admin permissions
    pub fn is_admin(&self) -> bool {
        match self {
            AuthContext::Worker(_) => false, // Workers are not admins
            AuthContext::User(user) => user.is_admin(),
            AuthContext::ApiKey(_) => false, // API keys are not admin by default
            AuthContext::Anonymous => false,
        }
    }

    /// Get the identity string for logging/auditing
    pub fn identity(&self) -> String {
        match self {
            AuthContext::Worker(worker) => format!("worker:{}", worker.name),
            AuthContext::User(user) => {
                format!("user:{}", user.email.as_deref().unwrap_or("unknown"))
            }
            AuthContext::ApiKey(key) => format!("api_key:{}", &key[..8.min(key.len())]),
            AuthContext::Anonymous => "anonymous".to_string(),
        }
    }
}

/// Common authentication errors across all services
#[derive(Debug)]
pub enum AuthError {
    /// Missing authorization header
    MissingAuth,
    /// Invalid authorization header format
    InvalidFormat,
    /// Invalid credentials provided
    InvalidCredentials,
    /// Session expired or invalid
    SessionExpired,
    /// Insufficient permissions for the requested operation
    Unauthorized,
    /// Database error during authentication
    Database(sqlx::Error),
    /// Base64 decode error
    Base64(base64::DecodeError),
    /// UTF-8 decode error
    Utf8(std::str::Utf8Error),
    /// Session management error
    Session(String),
    /// External authentication provider error
    Provider(String),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::MissingAuth => write!(f, "Missing authorization header"),
            AuthError::InvalidFormat => write!(f, "Invalid authorization header format"),
            AuthError::InvalidCredentials => write!(f, "Invalid credentials"),
            AuthError::SessionExpired => write!(f, "Session expired"),
            AuthError::Unauthorized => write!(f, "Insufficient permissions"),
            AuthError::Database(e) => write!(f, "Database error: {}", e),
            AuthError::Base64(e) => write!(f, "Base64 decode error: {}", e),
            AuthError::Utf8(e) => write!(f, "UTF-8 decode error: {}", e),
            AuthError::Session(e) => write!(f, "Session error: {}", e),
            AuthError::Provider(e) => write!(f, "Authentication provider error: {}", e),
        }
    }
}

impl std::error::Error for AuthError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AuthError::Database(e) => Some(e),
            AuthError::Base64(e) => Some(e),
            AuthError::Utf8(e) => Some(e),
            _ => None,
        }
    }
}

impl From<sqlx::Error> for AuthError {
    fn from(e: sqlx::Error) -> Self {
        AuthError::Database(e)
    }
}

impl From<base64::DecodeError> for AuthError {
    fn from(e: base64::DecodeError) -> Self {
        AuthError::Base64(e)
    }
}

impl From<std::str::Utf8Error> for AuthError {
    fn from(e: std::str::Utf8Error) -> Self {
        AuthError::Utf8(e)
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AuthError::MissingAuth => (StatusCode::UNAUTHORIZED, "Authentication required"),
            AuthError::InvalidFormat => (StatusCode::BAD_REQUEST, "Invalid authentication format"),
            AuthError::InvalidCredentials => (StatusCode::UNAUTHORIZED, "Invalid credentials"),
            AuthError::SessionExpired => (StatusCode::UNAUTHORIZED, "Session expired"),
            AuthError::Unauthorized => (StatusCode::FORBIDDEN, "Insufficient permissions"),
            AuthError::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error"),
            AuthError::Base64(_) => (StatusCode::BAD_REQUEST, "Invalid authentication format"),
            AuthError::Utf8(_) => (StatusCode::BAD_REQUEST, "Invalid authentication format"),
            AuthError::Session(_) => (StatusCode::UNAUTHORIZED, "Session error"),
            AuthError::Provider(_) => (StatusCode::BAD_GATEWAY, "Authentication provider error"),
        };

        let mut response = (status, message).into_response();

        // Add WWW-Authenticate header for 401 responses
        if status == StatusCode::UNAUTHORIZED {
            response.headers_mut().insert(
                axum::http::header::WWW_AUTHENTICATE,
                axum::http::HeaderValue::from_static("Basic realm=\"Janitor\""),
            );
        }

        response
    }
}

/// Authentication service trait for different auth backends
#[async_trait::async_trait]
pub trait AuthService: Send + Sync {
    /// Authenticate using basic auth credentials
    async fn authenticate_basic(
        &self,
        username: &str,
        password: &str,
    ) -> Result<AuthContext, AuthError>;

    /// Validate a session token
    async fn validate_session(&self, token: &str) -> Result<AuthContext, AuthError>;

    /// Validate an API key
    async fn validate_api_key(&self, key: &str) -> Result<AuthContext, AuthError>;
}

/// Shared authentication manager for all services
pub struct AuthManager {
    database: Pool<Postgres>,
    session_manager: Arc<dyn SessionManager>,
}

impl AuthManager {
    /// Create a new authentication manager
    pub fn new(database: Pool<Postgres>, session_manager: Arc<dyn SessionManager>) -> Self {
        Self {
            database,
            session_manager,
        }
    }

    /// Get database pool
    pub fn database(&self) -> &Pool<Postgres> {
        &self.database
    }

    /// Get session manager
    pub fn session_manager(&self) -> &Arc<dyn SessionManager> {
        &self.session_manager
    }
}

#[async_trait::async_trait]
impl AuthService for AuthManager {
    async fn authenticate_basic(
        &self,
        username: &str,
        password: &str,
    ) -> Result<AuthContext, AuthError> {
        // Try worker authentication
        let worker_result = authenticate_worker(&self.database, username, password).await?;
        if let Some(worker) = worker_result {
            return Ok(AuthContext::Worker(worker));
        }

        // Could add user basic auth here if needed
        Err(AuthError::InvalidCredentials)
    }

    async fn validate_session(&self, token: &str) -> Result<AuthContext, AuthError> {
        let session = self
            .session_manager
            .get_session(token)
            .await
            .map_err(|e| AuthError::Session(e.to_string()))?;

        if let Some(session) = session {
            Ok(AuthContext::User(session.user))
        } else {
            Err(AuthError::SessionExpired)
        }
    }

    async fn validate_api_key(&self, key: &str) -> Result<AuthContext, AuthError> {
        // Query the api_keys table
        let row = sqlx::query(
            r#"
            SELECT id, name, scopes, expires_at, is_active
            FROM api_keys
            WHERE key_hash = crypt($1, key_hash) AND is_active = true
            "#,
        )
        .bind(key)
        .fetch_optional(&self.database)
        .await?;

        if let Some(row) = row {
            let expires_at: Option<chrono::DateTime<chrono::Utc>> = row.get("expires_at");

            // Check if key is expired
            if let Some(exp) = expires_at {
                if chrono::Utc::now() > exp {
                    return Err(AuthError::SessionExpired);
                }
            }

            // Return API key context
            Ok(AuthContext::ApiKey(key.to_string()))
        } else {
            Err(AuthError::InvalidCredentials)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_context_permissions() {
        let worker = AuthContext::Worker(WorkerAuth {
            name: "test-worker".to_string(),
            link: None,
        });
        assert!(worker.can_write());
        assert!(!worker.is_admin());

        let anonymous = AuthContext::Anonymous;
        assert!(!anonymous.can_write());
        assert!(!anonymous.is_admin());
    }

    #[test]
    fn test_auth_context_identity() {
        let worker = AuthContext::Worker(WorkerAuth {
            name: "test-worker".to_string(),
            link: None,
        });
        assert_eq!(worker.identity(), "worker:test-worker");

        let api_key = AuthContext::ApiKey("secret123456789".to_string());
        assert_eq!(api_key.identity(), "api_key:secret12");
    }

    #[test]
    fn test_auth_error_response() {
        let error = AuthError::MissingAuth;
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
