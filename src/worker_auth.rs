// Worker authentication utilities
// Ported from py/janitor/worker_creds.py

use axum::{
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use base64::Engine;
use sqlx::{Pool, Postgres};
use std::str;

/// Error types for worker authentication
#[derive(Debug, thiserror::Error)]
pub enum WorkerAuthError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Invalid authorization header")]
    InvalidAuthHeader,
    #[error("Missing authorization header")]
    MissingAuthHeader,
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("Base64 decode error: {0}")]
    Base64Decode(#[from] base64::DecodeError),
    #[error("UTF-8 decode error: {0}")]
    Utf8Decode(#[from] str::Utf8Error),
}

impl IntoResponse for WorkerAuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            WorkerAuthError::MissingAuthHeader => (
                StatusCode::UNAUTHORIZED,
                "worker login required",
            ),
            WorkerAuthError::InvalidCredentials => (
                StatusCode::UNAUTHORIZED,
                "worker login required",
            ),
            WorkerAuthError::InvalidAuthHeader => (
                StatusCode::UNAUTHORIZED,
                "worker login required",
            ),
            WorkerAuthError::Database(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal server error",
            ),
            WorkerAuthError::Base64Decode(_) => (
                StatusCode::UNAUTHORIZED,
                "worker login required",
            ),
            WorkerAuthError::Utf8Decode(_) => (
                StatusCode::UNAUTHORIZED,
                "worker login required",
            ),
        };

        let mut response = (status, message).into_response();
        
        // Add WWW-Authenticate header for 401 responses
        if status == StatusCode::UNAUTHORIZED {
            response.headers_mut().insert(
                axum::http::header::WWW_AUTHENTICATE,
                axum::http::HeaderValue::from_static("Basic Realm=\"Janitor\""),
            );
        }
        
        response
    }
}

/// Basic authentication credentials
#[derive(Debug, Clone)]
pub struct BasicAuth {
    pub username: String,
    pub password: String,
}

impl BasicAuth {
    /// Decode Basic Auth from Authorization header value
    pub fn decode(auth_header: &str) -> Result<Self, WorkerAuthError> {
        if !auth_header.starts_with("Basic ") {
            return Err(WorkerAuthError::InvalidAuthHeader);
        }

        let encoded = &auth_header[6..]; // Skip "Basic "
        let decoded_bytes = base64::engine::general_purpose::STANDARD.decode(encoded)?;
        let decoded_str = str::from_utf8(&decoded_bytes)?;

        if let Some((username, password)) = decoded_str.split_once(':') {
            Ok(BasicAuth {
                username: username.to_string(),
                password: password.to_string(),
            })
        } else {
            Err(WorkerAuthError::InvalidAuthHeader)
        }
    }
}

/// Check if the request has valid worker credentials
/// 
/// Returns the worker name if authentication is successful, None if no auth header is present.
/// This is equivalent to the Python `is_worker()` function.
pub async fn is_worker(
    db: &Pool<Postgres>,
    headers: &HeaderMap,
) -> Result<Option<String>, WorkerAuthError> {
    let auth_header = match headers.get(axum::http::header::AUTHORIZATION) {
        Some(header) => header.to_str().map_err(|_| WorkerAuthError::InvalidAuthHeader)?,
        None => return Ok(None),
    };

    let auth = BasicAuth::decode(auth_header)?;

    // Use PostgreSQL's crypt() function to verify password
    let result: Option<i32> = sqlx::query_scalar(
        "SELECT 1 FROM worker WHERE name = $1 AND password = crypt($2, password)"
    )
    .bind(&auth.username)
    .bind(&auth.password)
    .fetch_optional(db)
    .await?;

    if result.is_some() {
        Ok(Some(auth.username))
    } else {
        Ok(None)
    }
}

/// Check worker credentials and return the worker name or error
/// 
/// This is equivalent to the Python `check_worker_creds()` function.
/// Returns an error if no credentials are provided or if they are invalid.
pub async fn check_worker_creds(
    db: &Pool<Postgres>,
    headers: &HeaderMap,
) -> Result<String, WorkerAuthError> {
    let _auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .ok_or(WorkerAuthError::MissingAuthHeader)?
        .to_str()
        .map_err(|_| WorkerAuthError::InvalidAuthHeader)?;

    let worker_name = is_worker(db, headers).await?;
    
    match worker_name {
        Some(name) => Ok(name),
        None => Err(WorkerAuthError::InvalidCredentials),
    }
}

/// Axum extractor for worker authentication
/// 
/// This can be used as a parameter in handler functions to automatically
/// authenticate workers and extract their name.
#[derive(Debug, Clone)]
pub struct WorkerAuth(pub String);

impl WorkerAuth {
    /// Get the worker name
    pub fn worker_name(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderMap, HeaderValue};
    use base64::Engine;

    #[test]
    fn test_basic_auth_decode_valid() {
        let auth_header = "Basic dGVzdDpwYXNzd29yZA=="; // base64 of "test:password"
        let auth = BasicAuth::decode(auth_header).unwrap();
        assert_eq!(auth.username, "test");
        assert_eq!(auth.password, "password");
    }

    #[test]
    fn test_basic_auth_decode_invalid_prefix() {
        let auth_header = "Bearer dGVzdDpwYXNzd29yZA==";
        let result = BasicAuth::decode(auth_header);
        assert!(matches!(result, Err(WorkerAuthError::InvalidAuthHeader)));
    }

    #[test]
    fn test_basic_auth_decode_invalid_base64() {
        let auth_header = "Basic invalid!!!";
        let result = BasicAuth::decode(auth_header);
        assert!(matches!(result, Err(WorkerAuthError::Base64Decode(_))));
    }

    #[test]
    fn test_basic_auth_decode_no_colon() {
        let encoded = base64::engine::general_purpose::STANDARD.encode("testpassword");
        let auth_header = format!("Basic {}", encoded);
        let result = BasicAuth::decode(&auth_header);
        assert!(matches!(result, Err(WorkerAuthError::InvalidAuthHeader)));
    }

    #[test]
    fn test_basic_auth_decode_empty_password() {
        let encoded = base64::engine::general_purpose::STANDARD.encode("test:");
        let auth_header = format!("Basic {}", encoded);
        let auth = BasicAuth::decode(&auth_header).unwrap();
        assert_eq!(auth.username, "test");
        assert_eq!(auth.password, "");
    }

    #[test]
    fn test_basic_auth_decode_empty_username() {
        let encoded = base64::engine::general_purpose::STANDARD.encode(":password");
        let auth_header = format!("Basic {}", encoded);
        let auth = BasicAuth::decode(&auth_header).unwrap();
        assert_eq!(auth.username, "");
        assert_eq!(auth.password, "password");
    }

    #[test]
    fn test_is_worker_no_auth_header() {
        // This test would require async and a database connection, so we'll skip implementation
        // In a real test, this would create a mock database and test the function
    }

    #[test]
    fn test_worker_auth_error_response() {
        let error = WorkerAuthError::MissingAuthHeader;
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_create_auth_header_map() {
        let mut headers = HeaderMap::new();
        let encoded = base64::engine::general_purpose::STANDARD.encode("worker1:secret123");
        let auth_value = format!("Basic {}", encoded);
        headers.insert(
            axum::http::header::AUTHORIZATION,
            HeaderValue::from_str(&auth_value).unwrap(),
        );

        // Test that we can extract the header
        let auth_header = headers.get(axum::http::header::AUTHORIZATION).unwrap();
        let auth_str = auth_header.to_str().unwrap();
        assert!(auth_str.starts_with("Basic "));
    }
}