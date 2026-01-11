//! Session management for user authentication

use async_trait::async_trait;
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres, Row};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use super::{AuthError, AuthSession, UserContext};

/// Session management trait
#[async_trait]
pub trait SessionManager: Send + Sync {
    /// Create a new session
    async fn create_session(
        &self,
        user: UserContext,
        duration: Duration,
    ) -> Result<String, AuthError>;

    /// Get session by token
    async fn get_session(&self, token: &str) -> Result<Option<AuthSession>, AuthError>;

    /// Update session last accessed time
    async fn touch_session(&self, token: &str) -> Result<(), AuthError>;

    /// Delete session
    async fn delete_session(&self, token: &str) -> Result<(), AuthError>;

    /// Cleanup expired sessions
    async fn cleanup_expired(&self) -> Result<u64, AuthError>;

    /// List sessions for a user
    async fn list_user_sessions(&self, user_id: &str) -> Result<Vec<AuthSession>, AuthError>;
}

/// PostgreSQL-based session manager
pub struct DatabaseSessionManager {
    database: Pool<Postgres>,
}

impl DatabaseSessionManager {
    /// Create a new database session manager
    pub fn new(database: Pool<Postgres>) -> Self {
        Self { database }
    }

    /// Generate a new session token
    fn generate_token(&self) -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                                abcdefghijklmnopqrstuvwxyz\
                                0123456789";
        const TOKEN_LEN: usize = 32;

        let mut rng = rand::rng();
        (0..TOKEN_LEN)
            .map(|_| {
                let idx = rng.random_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }
}

#[async_trait]
impl SessionManager for DatabaseSessionManager {
    async fn create_session(
        &self,
        user: UserContext,
        duration: Duration,
    ) -> Result<String, AuthError> {
        let token = self.generate_token();
        let now = Utc::now();
        let expires_at = now + duration;

        // Store session in database
        sqlx::query(
            r#"
            INSERT INTO user_sessions (id, user_id, user_email, user_name, user_roles, created_at, last_accessed_at, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#
        )
        .bind(&token)
        .bind(&user.id)
        .bind(&user.email)
        .bind(&user.name)
        .bind(serde_json::to_string(&user.roles).map_err(|e| AuthError::Session(e.to_string()))?)
        .bind(now)
        .bind(now)
        .bind(expires_at)
        .execute(&self.database)
        .await?;

        log::info!("Created session for user: {}", user.id);
        Ok(token)
    }

    async fn get_session(&self, token: &str) -> Result<Option<AuthSession>, AuthError> {
        let row = sqlx::query(
            r#"
            SELECT id, user_id, user_email, user_name, user_roles, created_at, last_accessed_at, expires_at
            FROM user_sessions 
            WHERE id = $1 AND expires_at > NOW()
            "#
        )
        .bind(token)
        .fetch_optional(&self.database)
        .await?;

        if let Some(row) = row {
            let user_roles: Vec<super::UserRole> =
                serde_json::from_str(&row.get::<String, _>("user_roles"))
                    .map_err(|e| AuthError::Session(e.to_string()))?;

            let user = UserContext {
                id: row.get("user_id"),
                email: row.get("user_email"),
                name: row.get("user_name"),
                roles: user_roles,
                expires_at: Some(row.get("expires_at")),
            };

            let session = AuthSession {
                id: row.get("id"),
                user,
                created_at: row.get("created_at"),
                last_accessed_at: row.get("last_accessed_at"),
                expires_at: row.get("expires_at"),
            };

            Ok(Some(session))
        } else {
            Ok(None)
        }
    }

    async fn touch_session(&self, token: &str) -> Result<(), AuthError> {
        let result = sqlx::query("UPDATE user_sessions SET last_accessed_at = NOW() WHERE id = $1")
            .bind(token)
            .execute(&self.database)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AuthError::SessionExpired);
        }

        Ok(())
    }

    async fn delete_session(&self, token: &str) -> Result<(), AuthError> {
        sqlx::query("DELETE FROM user_sessions WHERE id = $1")
            .bind(token)
            .execute(&self.database)
            .await?;

        log::info!("Deleted session: {}", token);
        Ok(())
    }

    async fn cleanup_expired(&self) -> Result<u64, AuthError> {
        let result = sqlx::query("DELETE FROM user_sessions WHERE expires_at <= NOW()")
            .execute(&self.database)
            .await?;

        let cleaned = result.rows_affected();
        if cleaned > 0 {
            log::info!("Cleaned up {} expired sessions", cleaned);
        }

        Ok(cleaned)
    }

    async fn list_user_sessions(&self, user_id: &str) -> Result<Vec<AuthSession>, AuthError> {
        let rows = sqlx::query(
            r#"
            SELECT id, user_id, user_email, user_name, user_roles, created_at, last_accessed_at, expires_at
            FROM user_sessions 
            WHERE user_id = $1 AND expires_at > NOW()
            ORDER BY last_accessed_at DESC
            "#
        )
        .bind(user_id)
        .fetch_all(&self.database)
        .await?;

        let mut sessions = Vec::new();
        for row in rows {
            let user_roles: Vec<super::UserRole> =
                serde_json::from_str(&row.get::<String, _>("user_roles"))
                    .map_err(|e| AuthError::Session(e.to_string()))?;

            let user = UserContext {
                id: row.get("user_id"),
                email: row.get("user_email"),
                name: row.get("user_name"),
                roles: user_roles,
                expires_at: Some(row.get("expires_at")),
            };

            let session = AuthSession {
                id: row.get("id"),
                user,
                created_at: row.get("created_at"),
                last_accessed_at: row.get("last_accessed_at"),
                expires_at: row.get("expires_at"),
            };

            sessions.push(session);
        }

        Ok(sessions)
    }
}

/// In-memory session manager for testing
pub struct MemorySessionManager {
    sessions: Arc<RwLock<HashMap<String, AuthSession>>>,
}

impl MemorySessionManager {
    /// Create a new memory session manager
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Generate a new session token
    fn generate_token(&self) -> String {
        use rand::Rng;
        let mut rng = rand::rng();
        format!("test_session_{}", rng.random::<u64>())
    }
}

impl Default for MemorySessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SessionManager for MemorySessionManager {
    async fn create_session(
        &self,
        user: UserContext,
        duration: Duration,
    ) -> Result<String, AuthError> {
        let token = self.generate_token();
        let now = Utc::now();

        let session = AuthSession {
            id: token.clone(),
            user,
            created_at: now,
            last_accessed_at: now,
            expires_at: now + duration,
        };

        let mut sessions = self
            .sessions
            .write()
            .map_err(|_| AuthError::Session("Lock poisoned".to_string()))?;
        sessions.insert(token.clone(), session);

        Ok(token)
    }

    async fn get_session(&self, token: &str) -> Result<Option<AuthSession>, AuthError> {
        let sessions = self
            .sessions
            .read()
            .map_err(|_| AuthError::Session("Lock poisoned".to_string()))?;

        if let Some(session) = sessions.get(token) {
            if session.is_expired() {
                Ok(None)
            } else {
                Ok(Some(session.clone()))
            }
        } else {
            Ok(None)
        }
    }

    async fn touch_session(&self, token: &str) -> Result<(), AuthError> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|_| AuthError::Session("Lock poisoned".to_string()))?;

        if let Some(session) = sessions.get_mut(token) {
            if session.is_expired() {
                return Err(AuthError::SessionExpired);
            }
            session.last_accessed_at = Utc::now();
            Ok(())
        } else {
            Err(AuthError::SessionExpired)
        }
    }

    async fn delete_session(&self, token: &str) -> Result<(), AuthError> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|_| AuthError::Session("Lock poisoned".to_string()))?;
        sessions.remove(token);
        Ok(())
    }

    async fn cleanup_expired(&self) -> Result<u64, AuthError> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|_| AuthError::Session("Lock poisoned".to_string()))?;

        let now = Utc::now();
        let before_count = sessions.len();

        sessions.retain(|_, session| session.expires_at > now);

        let cleaned = (before_count - sessions.len()) as u64;
        Ok(cleaned)
    }

    async fn list_user_sessions(&self, user_id: &str) -> Result<Vec<AuthSession>, AuthError> {
        let sessions = self
            .sessions
            .read()
            .map_err(|_| AuthError::Session("Lock poisoned".to_string()))?;

        let mut user_sessions: Vec<AuthSession> = sessions
            .values()
            .filter(|session| session.user.id == user_id && !session.is_expired())
            .cloned()
            .collect();

        // Sort by last accessed time (most recent first)
        user_sessions.sort_by(|a, b| b.last_accessed_at.cmp(&a.last_accessed_at));

        Ok(user_sessions)
    }
}

/// Session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Default session duration
    pub default_duration: Duration,
    /// Maximum session duration
    pub max_duration: Duration,
    /// Session cookie name
    pub cookie_name: String,
    /// Session cookie domain
    pub cookie_domain: Option<String>,
    /// Session cookie secure flag
    pub cookie_secure: bool,
    /// Session cookie httponly flag
    pub cookie_httponly: bool,
    /// Cleanup interval for expired sessions
    pub cleanup_interval: Duration,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            default_duration: Duration::hours(24),
            max_duration: Duration::days(30),
            cookie_name: "session_id".to_string(),
            cookie_domain: None,
            cookie_secure: true,
            cookie_httponly: true,
            cleanup_interval: Duration::hours(1),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::UserRole;

    #[tokio::test]
    async fn test_memory_session_manager() {
        let manager = MemorySessionManager::new();

        let user = UserContext {
            id: "user123".to_string(),
            email: Some("test@example.com".to_string()),
            name: Some("Test User".to_string()),
            roles: vec![UserRole::User],
            expires_at: None,
        };

        // Create session
        let token = manager
            .create_session(user.clone(), Duration::hours(1))
            .await
            .unwrap();
        assert!(!token.is_empty());

        // Get session
        let session = manager.get_session(&token).await.unwrap().unwrap();
        assert_eq!(session.user.id, "user123");

        // Touch session
        manager.touch_session(&token).await.unwrap();

        // List user sessions
        let sessions = manager.list_user_sessions("user123").await.unwrap();
        assert_eq!(sessions.len(), 1);

        // Delete session
        manager.delete_session(&token).await.unwrap();

        // Verify session is gone
        let session = manager.get_session(&token).await.unwrap();
        assert!(session.is_none());
    }

    #[tokio::test]
    async fn test_memory_session_expiration() {
        let manager = MemorySessionManager::new();

        let user = UserContext {
            id: "user123".to_string(),
            email: Some("test@example.com".to_string()),
            name: Some("Test User".to_string()),
            roles: vec![UserRole::User],
            expires_at: None,
        };

        // Create expired session (negative duration)
        let token = manager
            .create_session(user, Duration::seconds(-1))
            .await
            .unwrap();

        // Should not return expired session
        let session = manager.get_session(&token).await.unwrap();
        assert!(session.is_none());

        // Cleanup should remove expired session
        let cleaned = manager.cleanup_expired().await.unwrap();
        assert_eq!(cleaned, 1);
    }
}
