use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Standard API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination: Option<PaginationInfo>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            data: Some(data),
            error: None,
            reason: None,
            details: None,
            pagination: None,
        }
    }

    pub fn success_with_pagination(data: T, pagination: PaginationInfo) -> Self {
        Self {
            data: Some(data),
            error: None,
            reason: None,
            details: None,
            pagination: Some(pagination),
        }
    }
}

impl ApiResponse<()> {
    pub fn error(error: String, reason: Option<String>) -> Self {
        Self {
            data: None,
            error: Some(error),
            reason,
            details: None,
            pagination: None,
        }
    }

    pub fn error_with_details(
        error: String,
        reason: Option<String>,
        details: serde_json::Value,
    ) -> Self {
        Self {
            data: None,
            error: Some(error),
            reason,
            details: Some(details),
            pagination: None,
        }
    }
}

/// API error type for consistent error handling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub error: String,
    pub reason: Option<String>,
    pub details: Option<serde_json::Value>,
    pub status: u16,
}

impl ApiError {
    pub fn new(error: String, status: StatusCode) -> Self {
        Self {
            error,
            reason: None,
            details: None,
            status: status.as_u16(),
        }
    }

    pub fn with_reason(mut self, reason: String) -> Self {
        self.reason = Some(reason);
        self
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    pub fn bad_request(reason: String) -> Self {
        Self::new("bad_request".to_string(), StatusCode::BAD_REQUEST).with_reason(reason)
    }

    pub fn unauthorized() -> Self {
        Self::new("unauthorized".to_string(), StatusCode::UNAUTHORIZED)
    }

    pub fn forbidden() -> Self {
        Self::new("forbidden".to_string(), StatusCode::FORBIDDEN)
    }

    pub fn not_found(resource: String) -> Self {
        Self::new("not_found".to_string(), StatusCode::NOT_FOUND)
            .with_reason(format!("{} not found", resource))
    }

    pub fn internal_error(reason: String) -> Self {
        Self::new("internal_error".to_string(), StatusCode::INTERNAL_SERVER_ERROR)
            .with_reason(reason)
    }

    pub fn service_unavailable(service: String) -> Self {
        Self::new("service_unavailable".to_string(), StatusCode::SERVICE_UNAVAILABLE)
            .with_reason(format!("Unable to contact {}", service))
    }

    pub fn gateway_timeout(service: String) -> Self {
        Self::new("gateway_timeout".to_string(), StatusCode::GATEWAY_TIMEOUT)
            .with_reason(format!("{} service timeout", service))
    }
}

/// Result type for API operations
pub type ApiResult<T> = Result<T, ApiError>;

/// Pagination parameters from query string
#[derive(Debug, Clone, Deserialize)]
pub struct PaginationParams {
    #[serde(default)]
    pub offset: Option<i64>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub page: Option<i64>,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            offset: None,
            limit: Some(50), // Default page size
            page: None,
        }
    }
}

impl PaginationParams {
    pub fn get_offset(&self) -> i64 {
        if let Some(page) = self.page {
            let limit = self.limit.unwrap_or(50);
            (page - 1).max(0) * limit
        } else {
            self.offset.unwrap_or(0)
        }
    }

    pub fn get_limit(&self) -> i64 {
        self.limit.unwrap_or(50).min(1000) // Cap at 1000
    }
}

/// Pagination information in response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationInfo {
    pub total: Option<i64>,
    pub offset: i64,
    pub limit: i64,
    pub has_more: bool,
}

impl PaginationInfo {
    pub fn new(total: Option<i64>, offset: i64, limit: i64, returned_count: usize) -> Self {
        let has_more = if let Some(total) = total {
            offset + limit < total
        } else {
            returned_count as i64 >= limit
        };

        Self {
            total,
            offset,
            limit,
            has_more,
        }
    }
}

/// Common query parameters for API endpoints
#[derive(Debug, Clone, Deserialize)]
pub struct CommonQuery {
    #[serde(flatten)]
    pub pagination: PaginationParams,
    
    /// Search query
    pub search: Option<String>,
    
    /// Sorting parameters
    pub sort: Option<String>,
    pub order: Option<SortOrder>,
    
    /// Filtering
    pub filter: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    Asc,
    Desc,
}

impl Default for SortOrder {
    fn default() -> Self {
        Self::Asc
    }
}

/// Run-related API types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunInfo {
    pub id: String,
    pub codebase: String,
    pub campaign: String,
    pub command: Vec<String>,
    pub result_code: Option<String>,
    pub description: Option<String>,
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    pub finish_time: Option<chrono::DateTime<chrono::Utc>>,
    pub worker: Option<String>,
    pub branch_name: Option<String>,
    pub revision: Option<String>,
    pub suite: Option<String>,
    pub result_branches: Vec<String>,
    pub main_branch_revision: Option<String>,
    pub failure_transient: Option<bool>,
    pub failure_stage: Option<String>,
}

/// Queue status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStatus {
    pub total_candidates: i64,
    pub pending_candidates: i64,
    pub active_runs: i64,
    pub campaigns: Vec<CampaignStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignStatus {
    pub name: String,
    pub total_candidates: i64,
    pub pending_candidates: i64,
    pub active_runs: i64,
    pub success_rate: Option<f64>,
}

/// Merge proposal information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeProposalInfo {
    pub url: String,
    pub status: String,
    pub codebase: String,
    pub campaign: String,
    pub description: Option<String>,
    pub created_time: chrono::DateTime<chrono::Utc>,
    pub updated_time: chrono::DateTime<chrono::Utc>,
    pub merged_at: Option<chrono::DateTime<chrono::Utc>>,
    pub run_id: Option<String>,
}

/// Publish request
#[derive(Debug, Clone, Deserialize)]
pub struct PublishRequest {
    pub mode: PublishMode,
    pub requester: Option<String>,
    pub branch_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PublishMode {
    Propose,
    Push,
    Attempt,
    Bts,
}

/// Reschedule request
#[derive(Debug, Clone, Deserialize)]
pub struct RescheduleRequest {
    pub offset: Option<i32>,
    pub requester: Option<String>,
}

/// Mass reschedule request (admin only)
#[derive(Debug, Clone, Deserialize)]
pub struct MassRescheduleRequest {
    pub campaign: Option<String>,
    pub suite: Option<String>,
    pub result_code: Option<String>,
    pub limit: Option<i64>,
    pub requester: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_response_serialization() {
        let response = ApiResponse::success("test_data".to_string());
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"data\":\"test_data\""));
        assert!(!json.contains("\"error\""));
    }

    #[test]
    fn test_api_error_creation() {
        let error = ApiError::bad_request("Invalid input".to_string());
        assert_eq!(error.error, "bad_request");
        assert_eq!(error.status, 400);
        assert_eq!(error.reason, Some("Invalid input".to_string()));
    }

    #[test]
    fn test_pagination_params() {
        let params = PaginationParams {
            page: Some(3),
            limit: Some(25),
            offset: None,
        };
        
        assert_eq!(params.get_offset(), 50); // (3-1) * 25
        assert_eq!(params.get_limit(), 25);
    }

    #[test]
    fn test_pagination_info() {
        let info = PaginationInfo::new(Some(100), 50, 25, 25);
        assert_eq!(info.total, Some(100));
        assert_eq!(info.offset, 50);
        assert_eq!(info.limit, 25);
        assert!(info.has_more);
    }
}