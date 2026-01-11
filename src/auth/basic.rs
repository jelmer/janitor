//! HTTP Basic Authentication utilities

use axum::http::HeaderMap;
use base64::{engine::general_purpose, Engine};
use sqlx::{Pool, Postgres, Row};
use std::str;

use super::{AuthError, WorkerAuth};

/// Basic authentication credentials
#[derive(Debug, Clone)]
pub struct BasicCredentials {
    pub username: String,
    pub password: String,
}

impl BasicCredentials {
    /// Parse Basic Auth from Authorization header value
    pub fn from_header(auth_header: &str) -> Result<Self, AuthError> {
        let encoded = auth_header
            .strip_prefix("Basic ")
            .ok_or(AuthError::InvalidFormat)?;

        let decoded_bytes = general_purpose::STANDARD.decode(encoded)?;
        let decoded_str = str::from_utf8(&decoded_bytes)?;

        if let Some((username, password)) = decoded_str.split_once(':') {
            Ok(BasicCredentials {
                username: username.to_string(),
                password: password.to_string(),
            })
        } else {
            Err(AuthError::InvalidFormat)
        }
    }

    /// Extract Basic Auth credentials from HTTP headers
    pub fn from_headers(headers: &HeaderMap) -> Result<Option<Self>, AuthError> {
        let auth_header = match headers.get(axum::http::header::AUTHORIZATION) {
            Some(header) => header.to_str().map_err(|_| AuthError::InvalidFormat)?,
            None => return Ok(None),
        };

        if auth_header.starts_with("Basic ") {
            Ok(Some(Self::from_header(auth_header)?))
        } else {
            Ok(None)
        }
    }
}

/// Authenticate a worker using basic credentials
///
/// This function checks worker credentials against the PostgreSQL database
/// using the same encryption method as the original Python implementation.
pub async fn authenticate_worker(
    db: &Pool<Postgres>,
    username: &str,
    password: &str,
) -> Result<Option<WorkerAuth>, AuthError> {
    // Query the worker table with encrypted password verification
    let row = sqlx::query(
        "SELECT name, link FROM worker WHERE name = $1 AND password = crypt($2, password)",
    )
    .bind(username)
    .bind(password)
    .fetch_optional(db)
    .await?;

    if let Some(row) = row {
        let name: String = row.get("name");
        let link: Option<String> = row.get("link");
        Ok(Some(WorkerAuth { name, link }))
    } else {
        Ok(None)
    }
}

/// Check if request has valid worker credentials
///
/// Returns the worker auth if authentication is successful, None if no auth header is present.
/// This is equivalent to the original `is_worker()` function.
pub async fn check_worker_auth(
    db: &Pool<Postgres>,
    headers: &HeaderMap,
) -> Result<Option<WorkerAuth>, AuthError> {
    let credentials = BasicCredentials::from_headers(headers)?;

    if let Some(creds) = credentials {
        authenticate_worker(db, &creds.username, &creds.password).await
    } else {
        Ok(None)
    }
}

/// Require worker credentials and return the worker auth or error
///
/// This is equivalent to the original `check_worker_creds()` function.
/// Returns an error if no credentials are provided or if they are invalid.
pub async fn require_worker_auth(
    db: &Pool<Postgres>,
    headers: &HeaderMap,
) -> Result<WorkerAuth, AuthError> {
    let worker = check_worker_auth(db, headers).await?;
    worker.ok_or(AuthError::InvalidCredentials)
}

/// Worker management operations
pub struct WorkerManager {
    database: Pool<Postgres>,
}

impl WorkerManager {
    /// Create a new worker manager
    pub fn new(database: Pool<Postgres>) -> Self {
        Self { database }
    }

    /// Create a new worker account
    pub async fn create_worker(
        &self,
        name: &str,
        password: &str,
        link: Option<&str>,
    ) -> Result<(), AuthError> {
        sqlx::query(
            "INSERT INTO worker (name, password, link) VALUES ($1, crypt($2, gen_salt('bf')), $3)",
        )
        .bind(name)
        .bind(password)
        .bind(link)
        .execute(&self.database)
        .await?;

        log::info!("Created worker account: {}", name);
        Ok(())
    }

    /// Update worker password
    pub async fn update_worker_password(
        &self,
        name: &str,
        new_password: &str,
    ) -> Result<bool, AuthError> {
        let result =
            sqlx::query("UPDATE worker SET password = crypt($2, gen_salt('bf')) WHERE name = $1")
                .bind(name)
                .bind(new_password)
                .execute(&self.database)
                .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete worker account
    pub async fn delete_worker(&self, name: &str) -> Result<bool, AuthError> {
        let result = sqlx::query("DELETE FROM worker WHERE name = $1")
            .bind(name)
            .execute(&self.database)
            .await?;

        if result.rows_affected() > 0 {
            log::info!("Deleted worker account: {}", name);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// List all worker accounts
    pub async fn list_workers(&self) -> Result<Vec<WorkerAuth>, AuthError> {
        let rows = sqlx::query("SELECT name, link FROM worker ORDER BY name")
            .fetch_all(&self.database)
            .await?;

        let mut workers = Vec::new();
        for row in rows {
            let name: String = row.get("name");
            let link: Option<String> = row.get("link");
            workers.push(WorkerAuth { name, link });
        }

        Ok(workers)
    }

    /// Check if a worker exists
    pub async fn worker_exists(&self, name: &str) -> Result<bool, AuthError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM worker WHERE name = $1")
            .bind(name)
            .fetch_one(&self.database)
            .await?;

        Ok(count > 0)
    }

    /// Get worker by name
    pub async fn get_worker(&self, name: &str) -> Result<Option<WorkerAuth>, AuthError> {
        let row = sqlx::query("SELECT name, link FROM worker WHERE name = $1")
            .bind(name)
            .fetch_optional(&self.database)
            .await?;

        if let Some(row) = row {
            let name: String = row.get("name");
            let link: Option<String> = row.get("link");
            Ok(Some(WorkerAuth { name, link }))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderMap, HeaderValue};
    use base64::Engine;

    #[test]
    fn test_basic_credentials_from_header() {
        let auth_header = "Basic dGVzdDpwYXNzd29yZA=="; // base64 of "test:password"
        let creds = BasicCredentials::from_header(auth_header).unwrap();
        assert_eq!(creds.username, "test");
        assert_eq!(creds.password, "password");
    }

    #[test]
    fn test_basic_credentials_invalid_prefix() {
        let auth_header = "Bearer dGVzdDpwYXNzd29yZA==";
        let result = BasicCredentials::from_header(auth_header);
        assert!(matches!(result, Err(AuthError::InvalidFormat)));
    }

    #[test]
    fn test_basic_credentials_no_colon() {
        let encoded = general_purpose::STANDARD.encode("testpassword");
        let auth_header = format!("Basic {}", encoded);
        let result = BasicCredentials::from_header(&auth_header);
        assert!(matches!(result, Err(AuthError::InvalidFormat)));
    }

    #[test]
    fn test_basic_credentials_from_headers() {
        let mut headers = HeaderMap::new();
        let encoded = general_purpose::STANDARD.encode("worker1:secret123");
        let auth_value = format!("Basic {}", encoded);
        headers.insert(
            axum::http::header::AUTHORIZATION,
            HeaderValue::from_str(&auth_value).unwrap(),
        );

        let result = BasicCredentials::from_headers(&headers).unwrap();
        assert!(result.is_some());
        let creds = result.unwrap();
        assert_eq!(creds.username, "worker1");
        assert_eq!(creds.password, "secret123");
    }

    #[test]
    fn test_basic_credentials_from_headers_no_auth() {
        let headers = HeaderMap::new();
        let result = BasicCredentials::from_headers(&headers).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_basic_credentials_from_headers_bearer_token() {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            HeaderValue::from_static("Bearer token123"),
        );

        let result = BasicCredentials::from_headers(&headers).unwrap();
        assert!(result.is_none());
    }
}
