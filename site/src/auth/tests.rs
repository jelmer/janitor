use crate::auth::types::{SessionInfo, User, UserRole};
use std::collections::HashSet;

/// Mock user for testing
fn create_test_user() -> User {
    let mut groups = std::collections::HashSet::new();
    groups.insert("users".to_string());

    User {
        email: "test@example.com".to_string(),
        name: Some("Test User".to_string()),
        preferred_username: Some("testuser".to_string()),
        groups,
        sub: "test-user-123".to_string(),
        additional_claims: serde_json::Map::new(),
    }
}

#[cfg(test)]
mod session_tests {
    use super::*;

    #[test]
    fn test_session_info_creation() {
        let user = create_test_user();
        let session_info = SessionInfo::new(user.clone());

        assert_eq!(session_info.user.email, user.email);
        assert_eq!(session_info.user.sub, user.sub);
        assert!(session_info.created_at <= chrono::Utc::now());
        assert!(session_info.last_activity <= chrono::Utc::now());
    }
}

#[cfg(test)]
mod user_role_tests {
    use super::*;

    #[test]
    fn test_user_role_equality() {
        assert_eq!(UserRole::Admin, UserRole::Admin);
        assert_eq!(UserRole::QaReviewer, UserRole::QaReviewer);
        assert_eq!(UserRole::User, UserRole::User);

        assert_ne!(UserRole::Admin, UserRole::User);
        assert_ne!(UserRole::QaReviewer, UserRole::Admin);
    }

    #[test]
    fn test_user_role_serialization() {
        let role = UserRole::Admin;
        let serialized = serde_json::to_string(&role).unwrap();
        let deserialized: UserRole = serde_json::from_str(&serialized).unwrap();

        assert_eq!(role, deserialized);
    }
}
