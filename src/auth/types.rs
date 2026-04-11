//! Common authentication types

use serde::{Deserialize, Serialize};

/// Worker authentication information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerAuth {
    /// Worker name/username
    pub name: String,
    /// Optional worker link/URL
    pub link: Option<String>,
}

impl WorkerAuth {
    /// Get the worker name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the worker link if available
    pub fn link(&self) -> Option<&str> {
        self.link.as_deref()
    }
}

/// User authentication context for session-based auth
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContext {
    /// User ID
    pub id: String,
    /// User email (if available)
    pub email: Option<String>,
    /// User display name
    pub name: Option<String>,
    /// User roles
    pub roles: Vec<UserRole>,
    /// Session expiration timestamp
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl UserContext {
    /// Check if user has admin role
    pub fn is_admin(&self) -> bool {
        self.roles.contains(&UserRole::Admin)
    }

    /// Check if user has QA reviewer role
    pub fn is_qa_reviewer(&self) -> bool {
        self.roles.contains(&UserRole::QaReviewer)
    }

    /// Check if user can write/modify resources
    pub fn can_write(&self) -> bool {
        self.is_admin() || self.is_qa_reviewer()
    }

    /// Check if session is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            chrono::Utc::now() > expires_at
        } else {
            false // No expiration set
        }
    }
}

/// User roles in the system
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserRole {
    /// Regular user
    User,
    /// Quality assurance reviewer
    QaReviewer,
    /// Administrator
    Admin,
}

impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserRole::User => write!(f, "user"),
            UserRole::QaReviewer => write!(f, "qa_reviewer"),
            UserRole::Admin => write!(f, "admin"),
        }
    }
}

impl std::str::FromStr for UserRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "user" => Ok(UserRole::User),
            "qa_reviewer" | "qareviewr" => Ok(UserRole::QaReviewer),
            "admin" | "administrator" => Ok(UserRole::Admin),
            _ => Err(format!("Unknown role: {}", s)),
        }
    }
}

/// API key information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    /// Key identifier
    pub id: String,
    /// Key name/description
    pub name: String,
    /// Associated permissions/scopes
    pub scopes: Vec<String>,
    /// Expiration timestamp
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Whether the key is active
    pub is_active: bool,
}

impl ApiKey {
    /// Check if the API key is valid (active and not expired)
    pub fn is_valid(&self) -> bool {
        if !self.is_active {
            return false;
        }

        if let Some(expires_at) = self.expires_at {
            chrono::Utc::now() <= expires_at
        } else {
            true // No expiration set
        }
    }

    /// Check if the API key has a specific scope
    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.contains(&scope.to_string())
    }
}

/// Authentication session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSession {
    /// Session ID
    pub id: String,
    /// User context
    pub user: UserContext,
    /// Session creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last access timestamp
    pub last_accessed_at: chrono::DateTime<chrono::Utc>,
    /// Session expiration timestamp
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

impl AuthSession {
    /// Check if the session is expired
    pub fn is_expired(&self) -> bool {
        chrono::Utc::now() > self.expires_at
    }

    /// Check if the session needs refresh (close to expiration)
    pub fn needs_refresh(&self, refresh_threshold: chrono::Duration) -> bool {
        chrono::Utc::now() + refresh_threshold > self.expires_at
    }

    /// Extend the session expiration
    pub fn extend(&mut self, duration: chrono::Duration) {
        self.expires_at = chrono::Utc::now() + duration;
        self.last_accessed_at = chrono::Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    #[test]
    fn test_worker_auth() {
        let worker = WorkerAuth {
            name: "test-worker".to_string(),
            link: Some("https://example.com".to_string()),
        };

        assert_eq!(worker.name(), "test-worker");
        assert_eq!(worker.link(), Some("https://example.com"));
    }

    #[test]
    fn test_user_context_roles() {
        let mut user = UserContext {
            id: "123".to_string(),
            email: Some("test@example.com".to_string()),
            name: Some("Test User".to_string()),
            roles: vec![UserRole::User],
            expires_at: None,
        };

        assert!(!user.is_admin());
        assert!(!user.is_qa_reviewer());
        assert!(!user.can_write());

        user.roles.push(UserRole::QaReviewer);
        assert!(user.is_qa_reviewer());
        assert!(user.can_write());

        user.roles.push(UserRole::Admin);
        assert!(user.is_admin());
        assert!(user.can_write());
    }

    #[test]
    fn test_user_context_expiration() {
        let expired_user = UserContext {
            id: "123".to_string(),
            email: None,
            name: None,
            roles: vec![UserRole::User],
            expires_at: Some(Utc::now() - Duration::hours(1)),
        };

        assert!(expired_user.is_expired());

        let valid_user = UserContext {
            id: "456".to_string(),
            email: None,
            name: None,
            roles: vec![UserRole::User],
            expires_at: Some(Utc::now() + Duration::hours(1)),
        };

        assert!(!valid_user.is_expired());
    }

    #[test]
    fn test_user_role_from_str() {
        assert_eq!("user".parse::<UserRole>().unwrap(), UserRole::User);
        assert_eq!(
            "qa_reviewer".parse::<UserRole>().unwrap(),
            UserRole::QaReviewer
        );
        assert_eq!("admin".parse::<UserRole>().unwrap(), UserRole::Admin);
        assert!("invalid".parse::<UserRole>().is_err());
    }

    #[test]
    fn test_api_key_validation() {
        let valid_key = ApiKey {
            id: "key1".to_string(),
            name: "Test Key".to_string(),
            scopes: vec!["read".to_string(), "write".to_string()],
            expires_at: Some(Utc::now() + Duration::hours(1)),
            is_active: true,
        };

        assert!(valid_key.is_valid());
        assert!(valid_key.has_scope("read"));
        assert!(!valid_key.has_scope("admin"));

        let expired_key = ApiKey {
            is_active: true,
            expires_at: Some(Utc::now() - Duration::hours(1)),
            ..valid_key.clone()
        };

        assert!(!expired_key.is_valid());

        let inactive_key = ApiKey {
            is_active: false,
            expires_at: Some(Utc::now() + Duration::hours(1)),
            ..valid_key
        };

        assert!(!inactive_key.is_valid());
    }

    #[test]
    fn test_auth_session() {
        let mut session = AuthSession {
            id: "session1".to_string(),
            user: UserContext {
                id: "user1".to_string(),
                email: Some("test@example.com".to_string()),
                name: Some("Test User".to_string()),
                roles: vec![UserRole::User],
                expires_at: None,
            },
            created_at: Utc::now(),
            last_accessed_at: Utc::now(),
            expires_at: Utc::now() + Duration::hours(1),
        };

        assert!(!session.is_expired());
        assert!(session.needs_refresh(Duration::minutes(90)));
        assert!(!session.needs_refresh(Duration::minutes(30)));

        session.extend(Duration::hours(2));
        assert!(!session.needs_refresh(Duration::minutes(90)));
    }
}
