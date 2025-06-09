use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;
use validator::{Validate, ValidationError};

/// Schedule result response
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct ScheduleResult {
    /// Codebase name
    #[validate(length(min = 1, max = 255))]
    pub codebase: String,

    /// Campaign name
    #[validate(length(min = 1, max = 255))]
    pub campaign: String,

    /// Offset from top of queue
    pub offset: Option<i32>,

    /// Estimated duration in seconds
    #[validate(range(min = 0))]
    pub estimated_duration_seconds: Option<i32>,

    /// New position in the queue
    #[validate(range(min = 0))]
    pub queue_position: i32,

    /// New delay until run, in seconds
    #[validate(range(min = 0))]
    pub queue_wait_time: i32,
}

/// Merge proposal information
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema, sqlx::FromRow)]
pub struct MergeProposal {
    /// Merge proposal URL
    #[validate(url)]
    pub url: String,

    /// Status of the merge proposal
    #[validate(length(min = 1))]
    pub status: String,

    /// Associated codebase
    pub codebase: Option<String>,

    /// Associated campaign
    pub campaign: Option<String>,

    /// Description of the merge proposal
    pub description: Option<String>,

    /// When the proposal was created
    pub created_time: Option<DateTime<Utc>>,

    /// When the proposal was last updated
    pub updated_time: Option<DateTime<Utc>>,

    /// When the proposal was merged (if applicable)
    pub merged_at: Option<DateTime<Utc>>,

    /// Associated run ID
    pub run_id: Option<String>,
}

/// Queue item representation
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct QueueItem {
    /// Queue identifier
    pub queue_id: i64,

    /// Branch URL
    #[validate(url)]
    pub branch_url: String,

    /// Run context
    pub context: serde_json::Value,

    /// Command to execute
    #[validate(length(min = 1))]
    pub command: String,

    /// Campaign this item belongs to
    #[validate(length(min = 1))]
    pub campaign: String,

    /// Estimated duration in seconds
    #[validate(range(min = 0))]
    pub estimated_duration_seconds: Option<i32>,

    /// Whether this is a refresh run
    pub refresh: bool,

    /// Who requested this run
    pub requester: Option<String>,

    /// Change set identifier
    pub change_set: Option<String>,

    /// Codebase name
    #[validate(length(min = 1))]
    pub codebase: String,
}

/// Build information
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct BuildInfo {
    /// Build version
    pub version: Option<String>,

    /// Distribution name
    pub distribution: Option<String>,

    /// Architecture
    pub architecture: Option<String>,

    /// Build status
    pub status: Option<String>,
}

/// Comprehensive run information
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct Run {
    /// Run identifier
    #[validate(length(min = 1))]
    pub run_id: String,

    /// Start time
    pub start_time: Option<DateTime<Utc>>,

    /// Finish time
    pub finish_time: Option<DateTime<Utc>>,

    /// Command executed
    #[validate(length(min = 1))]
    pub command: String,

    /// Build result description
    pub description: Option<String>,

    /// Build information
    pub build_info: Option<BuildInfo>,

    /// Result code
    pub result_code: Option<String>,

    /// Main branch revision
    pub main_branch_revision: Option<String>,

    /// Current revision
    pub revision: Option<String>,

    /// Run context
    pub context: Option<serde_json::Value>,

    /// Suite name
    pub suite: Option<String>,

    /// VCS type (git, bzr, etc.)
    pub vcs_type: Option<String>,

    /// Branch URL
    pub branch_url: Option<String>,

    /// Log filenames
    pub logfilenames: Vec<String>,

    /// Worker name that executed this run
    pub worker_name: Option<String>,

    /// Result branches
    pub result_branches: Vec<ResultBranch>,

    /// Result tags
    pub result_tags: Vec<ResultTag>,

    /// Target branch URL
    pub target_branch_url: Option<String>,

    /// Change set identifier
    pub change_set: Option<String>,

    /// Whether the failure is transient
    pub failure_transient: Option<bool>,

    /// Stage where failure occurred
    pub failure_stage: Option<String>,

    /// Codebase name
    #[validate(length(min = 1))]
    pub codebase: String,

    /// Campaign name
    #[validate(length(min = 1))]
    pub campaign: String,

    /// Subpath within the repository
    pub subpath: Option<String>,
}

/// Result branch information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ResultBranch {
    pub name: String,
    pub role: Option<String>,
    pub base_revision: Option<String>,
    pub revision: Option<String>,
}

/// Result tag information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ResultTag {
    pub name: String,
    pub revision: Option<String>,
}

/// Worker result information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WorkerResult {
    /// Result code
    pub code: String,

    /// Description of the result
    pub description: Option<String>,

    /// Context information
    pub context: serde_json::Value,

    /// Codemod information
    pub codemod: Option<serde_json::Value>,

    /// Main branch revision
    pub main_branch_revision: Option<String>,

    /// Current revision
    pub revision: Option<String>,

    /// Numeric value associated with result
    pub value: Option<i32>,

    /// Result branches
    pub branches: Vec<ResultBranch>,

    /// Result tags
    pub tags: Vec<ResultTag>,

    /// Remote information
    pub remotes: HashMap<String, serde_json::Value>,

    /// Additional details
    pub details: Option<serde_json::Value>,

    /// Stage information
    pub stage: Option<String>,

    /// Builder result
    pub builder_result: Option<serde_json::Value>,

    /// Start time
    pub start_time: Option<DateTime<Utc>>,

    /// Finish time
    pub finish_time: Option<DateTime<Utc>>,

    /// Queue ID
    pub queue_id: Option<i64>,

    /// Worker name
    pub worker_name: Option<String>,

    /// Whether result was refreshed
    pub refreshed: bool,

    /// Target branch URL
    pub target_branch_url: Option<String>,

    /// Branch URL
    pub branch_url: Option<String>,

    /// VCS type
    pub vcs_type: Option<String>,

    /// Subpath
    pub subpath: Option<String>,

    /// Whether failure is transient
    pub transient: Option<bool>,

    /// Codebase name
    pub codebase: Option<String>,
}

/// Publish modes matching Python implementation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum PublishMode {
    #[serde(rename = "push-derived")]
    PushDerived,
    Push,
    Propose,
    #[serde(rename = "attempt-push")]
    AttemptPush,
    Bts,
}

impl PublishMode {
    /// Validate publish mode string
    pub fn from_str(s: &str) -> Result<Self, ValidationError> {
        match s {
            "push-derived" => Ok(Self::PushDerived),
            "push" => Ok(Self::Push),
            "propose" => Ok(Self::Propose),
            "attempt-push" => Ok(Self::AttemptPush),
            "bts" => Ok(Self::Bts),
            _ => Err(ValidationError::new("invalid_publish_mode")),
        }
    }
}

/// Publish request with validation
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct PublishRequest {
    /// Publish mode
    pub mode: PublishMode,

    /// Requester information
    pub requester: Option<String>,

    /// Specific branch name to publish
    pub branch_name: Option<String>,

    /// Whether to refresh before publishing
    pub refresh: Option<bool>,
}

/// Reschedule request with validation
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct RescheduleRequest {
    /// Offset from current position (can be negative)
    pub offset: Option<i32>,

    /// Requester information
    pub requester: Option<String>,

    /// Whether to refresh
    pub refresh: Option<bool>,
}

/// Mass reschedule request (admin only)
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct MassRescheduleRequest {
    /// Campaign filter
    pub campaign: Option<String>,

    /// Suite filter
    pub suite: Option<String>,

    /// Result code filter
    pub result_code: Option<String>,

    /// Maximum number of items to reschedule
    #[validate(range(min = 1, max = 10000))]
    pub limit: Option<i64>,

    /// Requester information
    pub requester: Option<String>,

    /// Offset for rescheduled items
    pub offset: Option<i32>,
}

/// Log file information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LogFile {
    /// Filename
    pub name: String,

    /// File size in bytes
    pub size: Option<i64>,

    /// Content type
    pub content_type: Option<String>,

    /// Last modified time
    pub modified_time: Option<DateTime<Utc>>,

    /// Whether file is compressed
    pub compressed: bool,
}

/// Diff information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DiffInfo {
    /// Run ID
    pub run_id: String,

    /// Diff type (vcs, debdiff, diffoscope)
    pub diff_type: String,

    /// Diff content (if small enough)
    pub content: Option<String>,

    /// URL to full diff content
    pub url: Option<String>,

    /// Size of diff in bytes
    pub size: Option<i64>,

    /// Whether diff is binary
    pub is_binary: bool,
}

/// User information for API responses
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserInfo {
    /// User email
    pub email: String,

    /// Display name
    pub name: Option<String>,

    /// Username
    pub username: Option<String>,

    /// User roles
    pub roles: Vec<String>,

    /// Whether user is admin
    pub is_admin: bool,

    /// Whether user is QA reviewer
    pub is_qa_reviewer: bool,
}

/// Campaign status information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CampaignStatus {
    /// Campaign name
    pub name: String,

    /// Total candidates
    pub total_candidates: i64,

    /// Pending candidates
    pub pending_candidates: i64,

    /// Active runs
    pub active_runs: i64,

    /// Success rate (0.0 to 1.0)
    pub success_rate: Option<f64>,

    /// Last update time
    pub last_updated: Option<DateTime<Utc>>,

    /// Campaign description
    pub description: Option<String>,
}

/// System health information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HealthStatus {
    /// Overall status
    pub status: String,

    /// Timestamp of check
    pub timestamp: DateTime<Utc>,

    /// Individual service statuses
    pub services: HashMap<String, ServiceHealth>,

    /// System version
    pub version: Option<String>,
}

/// Individual service health
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ServiceHealth {
    /// Service status
    pub status: String,

    /// Optional error message
    pub error: Option<String>,

    /// Response time in milliseconds
    pub response_time_ms: Option<i64>,

    /// Last check time
    pub last_check: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn test_schedule_result_validation() {
        let valid_result = ScheduleResult {
            codebase: "test-package".to_string(),
            campaign: "lintian-fixes".to_string(),
            offset: Some(10),
            estimated_duration_seconds: Some(300),
            queue_position: 5,
            queue_wait_time: 1200,
        };

        assert!(valid_result.validate().is_ok());

        let invalid_result = ScheduleResult {
            codebase: "".to_string(), // Empty codebase should fail
            campaign: "lintian-fixes".to_string(),
            offset: Some(10),
            estimated_duration_seconds: Some(-100), // Negative duration should fail
            queue_position: 5,
            queue_wait_time: 1200,
        };

        assert!(invalid_result.validate().is_err());
    }

    #[test]
    fn test_publish_mode_serialization() {
        assert_eq!(
            serde_json::to_string(&PublishMode::PushDerived).unwrap(),
            "\"push-derived\""
        );

        assert_eq!(
            serde_json::to_string(&PublishMode::AttemptPush).unwrap(),
            "\"attempt-push\""
        );

        let mode: PublishMode = serde_json::from_str("\"propose\"").unwrap();
        assert!(matches!(mode, PublishMode::Propose));
    }

    #[test]
    fn test_merge_proposal_validation() {
        let valid_proposal = MergeProposal {
            url: "https://github.com/owner/repo/pull/123".to_string(),
            status: "open".to_string(),
            codebase: Some("test-package".to_string()),
            campaign: Some("lintian-fixes".to_string()),
            description: Some("Fix lintian warnings".to_string()),
            created_time: Some(Utc::now()),
            updated_time: Some(Utc::now()),
            merged_at: None,
            run_id: Some("run-123".to_string()),
        };

        assert!(valid_proposal.validate().is_ok());

        let invalid_proposal = MergeProposal {
            url: "not-a-url".to_string(), // Invalid URL should fail
            status: "".to_string(),       // Empty status should fail
            codebase: None,
            campaign: None,
            description: None,
            created_time: None,
            updated_time: None,
            merged_at: None,
            run_id: None,
        };

        assert!(invalid_proposal.validate().is_err());
    }

    #[test]
    fn test_mass_reschedule_validation() {
        let valid_request = MassRescheduleRequest {
            campaign: Some("lintian-fixes".to_string()),
            suite: Some("unstable".to_string()),
            result_code: Some("success".to_string()),
            limit: Some(100),
            requester: Some("admin@example.com".to_string()),
            offset: Some(10),
        };

        assert!(valid_request.validate().is_ok());

        let invalid_request = MassRescheduleRequest {
            campaign: None,
            suite: None,
            result_code: None,
            limit: Some(50000), // Too high limit should fail
            requester: None,
            offset: None,
        };

        assert!(invalid_request.validate().is_err());
    }
}

// ============================================================================
// Admin User Management Schemas
// ============================================================================

/// Request for updating user role
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateUserRoleRequest {
    /// New role for the user (Admin, QaReviewer, User)
    #[validate(custom(function = "validate_user_role"))]
    pub role: String,
}

/// User information for admin endpoints
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AdminUserInfo {
    /// User ID (subject identifier)
    pub user_id: String,
    
    /// Email address
    pub email: String,
    
    /// Display name
    pub name: Option<String>,
    
    /// Preferred username
    pub preferred_username: Option<String>,
    
    /// User groups
    pub groups: Vec<String>,
    
    /// Current role
    pub role: String,
    
    /// Last activity timestamp
    pub last_activity: Option<DateTime<Utc>>,
    
    /// Session count
    pub active_sessions: i32,
}

/// Session information for admin endpoints
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AdminSessionInfo {
    /// Session ID
    pub session_id: String,
    
    /// User ID
    pub user_id: String,
    
    /// User email
    pub user_email: String,
    
    /// Session creation time
    pub created_at: DateTime<Utc>,
    
    /// Last activity time
    pub last_activity: DateTime<Utc>,
    
    /// IP address
    pub ip_address: Option<String>,
    
    /// User agent
    pub user_agent: Option<String>,
}

/// Bulk user operation request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct BulkUserOperationRequest {
    /// User IDs to operate on
    #[validate(length(min = 1, max = 100))]
    pub user_ids: Vec<String>,
    
    /// Operation type (revoke_sessions, update_role, etc.)
    #[validate(length(min = 1))]
    pub operation: String,
    
    /// Additional parameters for the operation
    pub parameters: Option<serde_json::Value>,
    
    /// Requester information
    pub requester: Option<String>,
}

/// Custom validator for user roles
fn validate_user_role(role: &str) -> Result<(), ValidationError> {
    match role {
        "Admin" | "QaReviewer" | "User" => Ok(()),
        _ => Err(ValidationError::new("invalid_user_role")),
    }
}

#[cfg(test)]
mod admin_tests {
    use super::*;

    #[test]
    fn test_update_user_role_validation() {
        let valid_request = UpdateUserRoleRequest {
            role: "Admin".to_string(),
        };
        assert!(valid_request.validate().is_ok());

        let invalid_request = UpdateUserRoleRequest {
            role: "SuperUser".to_string(),
        };
        assert!(invalid_request.validate().is_err());
    }

    #[test]
    fn test_bulk_operation_validation() {
        let valid_request = BulkUserOperationRequest {
            user_ids: vec!["user1".to_string(), "user2".to_string()],
            operation: "revoke_sessions".to_string(),
            parameters: None,
            requester: Some("admin@example.com".to_string()),
        };
        assert!(valid_request.validate().is_ok());

        let invalid_request = BulkUserOperationRequest {
            user_ids: vec![], // Empty list should fail
            operation: "".to_string(),
            parameters: None,
            requester: None,
        };
        assert!(invalid_request.validate().is_err());
    }
}
