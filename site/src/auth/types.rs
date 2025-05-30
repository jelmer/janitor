use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// User information from OIDC provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub email: String,
    pub name: Option<String>,
    pub preferred_username: Option<String>,
    pub groups: HashSet<String>,
    pub sub: String,  // Subject identifier
    
    // Additional fields that might be present
    #[serde(flatten)]
    pub additional_claims: serde_json::Map<String, serde_json::Value>,
}

/// User roles based on group membership
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserRole {
    User,
    QaReviewer,
    Admin,
}

impl User {
    /// Check if user has a specific role based on configured groups
    pub fn has_role(&self, role: UserRole, admin_group: Option<&str>, qa_reviewer_group: Option<&str>) -> bool {
        match role {
            UserRole::User => true, // All authenticated users have user role
            UserRole::Admin => {
                if let Some(admin_group) = admin_group {
                    self.groups.contains(admin_group)
                } else {
                    // If no admin group configured, all users are admins
                    true
                }
            }
            UserRole::QaReviewer => {
                // Admin also has QA reviewer permissions
                if self.has_role(UserRole::Admin, admin_group, qa_reviewer_group) {
                    return true;
                }
                
                if let Some(qa_reviewer_group) = qa_reviewer_group {
                    self.groups.contains(qa_reviewer_group)
                } else {
                    // If no QA reviewer group configured, all users are QA reviewers
                    true
                }
            }
        }
    }
    
    /// Get the highest role this user has
    pub fn get_highest_role(&self, admin_group: Option<&str>, qa_reviewer_group: Option<&str>) -> UserRole {
        if self.has_role(UserRole::Admin, admin_group, qa_reviewer_group) {
            UserRole::Admin
        } else if self.has_role(UserRole::QaReviewer, admin_group, qa_reviewer_group) {
            UserRole::QaReviewer
        } else {
            UserRole::User
        }
    }
}

/// Session information stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub user: User,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
}

impl SessionInfo {
    pub fn new(user: User) -> Self {
        let now = chrono::Utc::now();
        Self {
            user,
            created_at: now,
            last_activity: now,
        }
    }
    
    pub fn update_activity(&mut self) {
        self.last_activity = chrono::Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_user_roles() {
        let mut groups = HashSet::new();
        groups.insert("developers".to_string());
        
        let user = User {
            email: "test@example.com".to_string(),
            name: Some("Test User".to_string()),
            preferred_username: Some("testuser".to_string()),
            groups,
            sub: "123456".to_string(),
            additional_claims: serde_json::Map::new(),
        };
        
        // No groups configured - everyone is admin/qa
        assert!(user.has_role(UserRole::Admin, None, None));
        assert!(user.has_role(UserRole::QaReviewer, None, None));
        assert!(user.has_role(UserRole::User, None, None));
        
        // Admin group configured, user not in it
        assert!(!user.has_role(UserRole::Admin, Some("admins"), None));
        assert!(user.has_role(UserRole::User, Some("admins"), None));
        
        // User in qa reviewer group
        assert!(user.has_role(UserRole::QaReviewer, Some("admins"), Some("developers")));
        assert!(!user.has_role(UserRole::Admin, Some("admins"), Some("developers")));
    }
    
    #[test]
    fn test_highest_role() {
        let mut groups = HashSet::new();
        groups.insert("qa".to_string());
        
        let user = User {
            email: "qa@example.com".to_string(),
            name: Some("QA User".to_string()),
            preferred_username: Some("qauser".to_string()),
            groups,
            sub: "789".to_string(),
            additional_claims: serde_json::Map::new(),
        };
        
        assert_eq!(
            user.get_highest_role(Some("admins"), Some("qa")),
            UserRole::QaReviewer
        );
    }
}