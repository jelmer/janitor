use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

use crate::auth::types::{SessionInfo, User};

/// Session management for storing and retrieving user sessions
#[derive(Debug, Clone)]
pub struct SessionManager {
    pool: PgPool,
    session_duration: Duration,
}

/// Session storage errors
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Session not found")]
    NotFound,

    #[error("Session expired")]
    Expired,

    #[error("Invalid session data: {0}")]
    InvalidData(String),
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            session_duration: Duration::weeks(1), // Sessions expire after 1 week
        }
    }

    /// Create a new session for a user
    pub async fn create_session(&self, user: User) -> Result<String, SessionError> {
        let session_id = Uuid::new_v4().to_string();
        let session_info = SessionInfo::new(user);

        let userinfo_json = serde_json::to_value(&session_info).map_err(|e| {
            SessionError::InvalidData(format!("Failed to serialize session: {}", e))
        })?;

        sqlx::query("INSERT INTO site_session (id, timestamp, userinfo) VALUES ($1, $2, $3)")
            .bind(&session_id)
            .bind(session_info.created_at)
            .bind(&userinfo_json)
            .execute(&self.pool)
            .await?;

        Ok(session_id)
    }

    /// Retrieve a session by ID
    pub async fn get_session(&self, session_id: &str) -> Result<SessionInfo, SessionError> {
        let row = sqlx::query_as::<_, (serde_json::Value, DateTime<Utc>)>(
            "SELECT userinfo, timestamp FROM site_session WHERE id = $1",
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?;

        let (userinfo, timestamp) = row.ok_or(SessionError::NotFound)?;

        // Check if session is expired
        let session_age = Utc::now().signed_duration_since(timestamp);
        if session_age > self.session_duration {
            // Clean up expired session
            self.delete_session(session_id).await?;
            return Err(SessionError::Expired);
        }

        let session_info: SessionInfo = serde_json::from_value(userinfo).map_err(|e| {
            SessionError::InvalidData(format!("Failed to deserialize session: {}", e))
        })?;

        Ok(session_info)
    }

    /// Update session activity timestamp
    pub async fn update_activity(&self, session_id: &str) -> Result<(), SessionError> {
        let result = sqlx::query("UPDATE site_session SET timestamp = $1 WHERE id = $2")
            .bind(Utc::now())
            .bind(session_id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(SessionError::NotFound);
        }

        Ok(())
    }

    /// Delete a session
    pub async fn delete_session(&self, session_id: &str) -> Result<(), SessionError> {
        sqlx::query("DELETE FROM site_session WHERE id = $1")
            .bind(session_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Clean up expired sessions
    pub async fn cleanup_expired_sessions(&self) -> Result<u64, SessionError> {
        let cutoff = Utc::now() - self.session_duration;

        let result = sqlx::query("DELETE FROM site_session WHERE timestamp < $1")
            .bind(cutoff)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    /// Get all active sessions (for admin purposes)
    pub async fn get_active_sessions(&self) -> Result<HashMap<String, SessionInfo>, SessionError> {
        let rows = sqlx::query_as::<_, (String, serde_json::Value)>(
            "SELECT id, userinfo FROM site_session WHERE timestamp > $1",
        )
        .bind(Utc::now() - self.session_duration)
        .fetch_all(&self.pool)
        .await?;

        let mut sessions = HashMap::new();
        for (id, userinfo) in rows {
            match serde_json::from_value::<SessionInfo>(userinfo) {
                Ok(session_info) => {
                    sessions.insert(id.clone(), session_info);
                }
                Err(e) => {
                    tracing::warn!("Failed to deserialize session {}: {}", id, e);
                    // Optionally clean up corrupted session
                    let _ = self.delete_session(&id).await;
                }
            }
        }

        Ok(sessions)
    }

    /// Get session count by user email (for limiting concurrent sessions)
    pub async fn get_user_session_count(&self, email: &str) -> Result<i64, SessionError> {
        let count: Option<i64> = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM site_session 
            WHERE timestamp > $1 
            AND userinfo->>'user'->>'email' = $2
            "#,
        )
        .bind(Utc::now() - self.session_duration)
        .bind(email)
        .fetch_one(&self.pool)
        .await?;

        Ok(count.unwrap_or(0))
    }

    /// Initialize session table if it doesn't exist
    pub async fn ensure_table_exists(&self) -> Result<(), SessionError> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS site_session (
                id text PRIMARY KEY,
                timestamp timestamptz NOT NULL DEFAULT now(),
                userinfo jsonb NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create index for timestamp-based queries
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_site_session_timestamp ON site_session(timestamp)",
        )
        .execute(&self.pool)
        .await?;

        // Create index for user email queries
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_site_session_user_email 
            ON site_session USING gin ((userinfo->'user'->>'email'))
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Store temporary data with expiration (for OIDC state, CSRF tokens, etc.)
    pub async fn store_temporary_data<T: serde::Serialize>(
        &self,
        key: &str,
        data: &T,
        duration: std::time::Duration,
    ) -> Result<(), SessionError> {
        let expires_at = Utc::now() + Duration::from_std(duration).unwrap_or(Duration::hours(1));
        let data_json = serde_json::to_value(data).map_err(|e| {
            SessionError::InvalidData(format!("Failed to serialize temporary data: {}", e))
        })?;

        // Create temporary data table if it doesn't exist
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS site_temporary_data (
                key text PRIMARY KEY,
                data jsonb NOT NULL,
                expires_at timestamptz NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Insert or update the temporary data
        sqlx::query(
            "INSERT INTO site_temporary_data (key, data, expires_at) VALUES ($1, $2, $3)
             ON CONFLICT (key) DO UPDATE SET data = $2, expires_at = $3",
        )
        .bind(key)
        .bind(&data_json)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get temporary data by key
    pub async fn get_temporary_data<T: serde::de::DeserializeOwned>(
        &self,
        key: &str,
    ) -> Result<Option<T>, SessionError> {
        let row = sqlx::query_as::<_, (serde_json::Value, DateTime<Utc>)>(
            "SELECT data, expires_at FROM site_temporary_data WHERE key = $1",
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;

        let (data, expires_at) = match row {
            Some(row) => row,
            None => return Ok(None),
        };

        // Check if data has expired
        if Utc::now() > expires_at {
            // Clean up expired data
            self.delete_temporary_data(key).await?;
            return Ok(None);
        }

        let parsed_data: T = serde_json::from_value(data).map_err(|e| {
            SessionError::InvalidData(format!("Failed to deserialize temporary data: {}", e))
        })?;

        Ok(Some(parsed_data))
    }

    /// Delete temporary data by key
    pub async fn delete_temporary_data(&self, key: &str) -> Result<(), SessionError> {
        sqlx::query("DELETE FROM site_temporary_data WHERE key = $1")
            .bind(key)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Clean up expired temporary data
    pub async fn cleanup_expired_temporary_data(&self) -> Result<u64, SessionError> {
        let result = sqlx::query("DELETE FROM site_temporary_data WHERE expires_at < $1")
            .bind(Utc::now())
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }
}

/// Session cookie configuration
#[derive(Debug, Clone)]
pub struct SessionCookieConfig {
    pub name: String,
    pub domain: Option<String>,
    pub path: String,
    pub secure: bool,
    pub http_only: bool,
    pub same_site: SameSite,
    pub max_age: Option<Duration>,
}

#[derive(Debug, Clone, Copy)]
pub enum SameSite {
    Strict,
    Lax,
    None,
}

impl Default for SessionCookieConfig {
    fn default() -> Self {
        Self {
            name: "session_id".to_string(),
            domain: None,
            path: "/".to_string(),
            secure: true,
            http_only: true,
            same_site: SameSite::Lax,
            max_age: Some(Duration::weeks(1)),
        }
    }
}

impl SessionCookieConfig {
    pub fn for_development() -> Self {
        Self {
            secure: false, // Allow non-HTTPS in development
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    // Note: These tests would require a test database setup
    // For now, they're mostly structural tests

    #[test]
    fn test_session_cookie_config() {
        let config = SessionCookieConfig::default();
        assert_eq!(config.name, "session_id");
        assert!(config.secure);
        assert!(config.http_only);

        let dev_config = SessionCookieConfig::for_development();
        assert!(!dev_config.secure);
    }

    #[test]
    fn test_session_info() {
        let user = User {
            email: "test@example.com".to_string(),
            name: Some("Test User".to_string()),
            preferred_username: Some("testuser".to_string()),
            groups: HashSet::new(),
            sub: "123".to_string(),
            additional_claims: serde_json::Map::new(),
        };

        let session_info = SessionInfo::new(user.clone());
        assert_eq!(session_info.user.email, user.email);
        assert_eq!(session_info.created_at, session_info.last_activity);
    }
}
