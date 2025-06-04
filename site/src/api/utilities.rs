use axum::http::{HeaderMap, Uri};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

use super::{
    content_negotiation::{negotiate_response, ContentType},
    types::{ApiResponse, PaginationInfo, PaginationParams, SortOrder},
};

/// Pagination helper utilities
pub struct PaginationHelper;

impl PaginationHelper {
    /// Calculate pagination metadata from query parameters and total count
    pub fn calculate_pagination(
        params: &PaginationParams,
        total_count: Option<i64>,
        result_count: usize,
    ) -> PaginationInfo {
        let offset = params.get_offset();
        let limit = params.get_limit();

        PaginationInfo::new(total_count, offset, limit, result_count)
    }

    /// Generate pagination URLs for navigation
    pub fn generate_pagination_urls(
        base_url: &str,
        params: &PaginationParams,
        total_count: Option<i64>,
        result_count: usize,
    ) -> PaginationUrls {
        let offset = params.get_offset();
        let limit = params.get_limit();

        let mut urls = PaginationUrls {
            first: None,
            prev: None,
            next: None,
            last: None,
        };

        // First page URL
        if offset > 0 {
            urls.first = Some(format!("{}?offset=0&limit={}", base_url, limit));
        }

        // Previous page URL
        if offset > 0 {
            let prev_offset = (offset - limit).max(0);
            urls.prev = Some(format!(
                "{}?offset={}&limit={}",
                base_url, prev_offset, limit
            ));
        }

        // Next page URL
        if let Some(total) = total_count {
            if offset + limit < total {
                urls.next = Some(format!(
                    "{}?offset={}&limit={}",
                    base_url,
                    offset + limit,
                    limit
                ));
            }

            // Last page URL
            if total > limit {
                let last_offset = ((total - 1) / limit) * limit;
                if last_offset != offset {
                    urls.last = Some(format!(
                        "{}?offset={}&limit={}",
                        base_url, last_offset, limit
                    ));
                }
            }
        } else {
            // If we don't know the total, assume there might be a next page if we got a full page
            if result_count >= limit as usize {
                urls.next = Some(format!(
                    "{}?offset={}&limit={}",
                    base_url,
                    offset + limit,
                    limit
                ));
            }
        }

        urls
    }

    /// Convert page number to offset
    pub fn page_to_offset(page: i64, limit: i64) -> i64 {
        if page <= 1 {
            0
        } else {
            (page - 1) * limit
        }
    }

    /// Convert offset to page number
    pub fn offset_to_page(offset: i64, limit: i64) -> i64 {
        if limit <= 0 {
            1
        } else {
            (offset / limit) + 1
        }
    }
}

/// Pagination URLs for navigation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaginationUrls {
    pub first: Option<String>,
    pub prev: Option<String>,
    pub next: Option<String>,
    pub last: Option<String>,
}

/// Query parameter parsing utilities
pub struct QueryHelper;

impl QueryHelper {
    /// Parse boolean from string with support for multiple formats
    pub fn parse_bool(value: &str) -> Result<bool, String> {
        match value.to_lowercase().as_str() {
            "true" | "1" | "yes" | "on" | "y" => Ok(true),
            "false" | "0" | "no" | "off" | "n" | "" => Ok(false),
            _ => Err(format!("Invalid boolean value: {}", value)),
        }
    }

    /// Parse integer with validation
    pub fn parse_int<T>(value: &str, min: Option<T>, max: Option<T>) -> Result<T, String>
    where
        T: std::str::FromStr + PartialOrd + std::fmt::Display + Copy,
        T::Err: std::fmt::Display,
    {
        let parsed = value
            .parse::<T>()
            .map_err(|e| format!("Invalid integer value '{}': {}", value, e))?;

        if let Some(min_val) = min {
            if parsed < min_val {
                return Err(format!("Value {} is below minimum {}", parsed, min_val));
            }
        }

        if let Some(max_val) = max {
            if parsed > max_val {
                return Err(format!("Value {} is above maximum {}", parsed, max_val));
            }
        }

        Ok(parsed)
    }

    /// Parse comma-separated list
    pub fn parse_list(value: &str) -> Vec<String> {
        if value.trim().is_empty() {
            Vec::new()
        } else {
            value
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        }
    }

    /// Parse sort parameter with validation
    pub fn parse_sort(value: &str, valid_fields: &[&str]) -> Result<(String, SortOrder), String> {
        let (field, order) = if let Some(stripped) = value.strip_prefix('-') {
            (stripped, SortOrder::Desc)
        } else if let Some(stripped) = value.strip_prefix('+') {
            (stripped, SortOrder::Asc)
        } else {
            (value, SortOrder::Asc)
        };

        if !valid_fields.contains(&field) {
            return Err(format!(
                "Invalid sort field '{}'. Valid fields: {}",
                field,
                valid_fields.join(", ")
            ));
        }

        Ok((field.to_string(), order))
    }
}

/// Response formatting utilities
pub struct ResponseHelper;

impl ResponseHelper {
    /// Create success response with data
    pub fn success<T>(data: T) -> ApiResponse<T> {
        ApiResponse::success(data)
    }

    /// Create success response with pagination
    pub fn success_with_pagination<T>(data: T, pagination: PaginationInfo) -> ApiResponse<T> {
        ApiResponse::success_with_pagination(data, pagination)
    }

    /// Create error response
    pub fn error(code: &str, message: &str) -> ApiResponse<()> {
        ApiResponse::error(code.to_string(), Some(message.to_string()))
    }

    /// Create error response with details
    pub fn error_with_details(
        code: &str,
        message: &str,
        details: serde_json::Value,
    ) -> ApiResponse<()> {
        ApiResponse::error_with_details(code.to_string(), Some(message.to_string()), details)
    }

    /// Create not found response
    pub fn not_found(resource: &str) -> ApiResponse<()> {
        Self::error("not_found", &format!("{} not found", resource))
    }

    /// Create validation error response
    pub fn validation_error(field: &str, message: &str) -> ApiResponse<()> {
        let details = serde_json::json!({
            "field": field,
            "message": message
        });
        Self::error_with_details("validation_error", "Validation failed", details)
    }

    /// Format response with content negotiation
    pub fn negotiate<T: Serialize>(
        data: ApiResponse<T>,
        headers: &HeaderMap,
        path: &str,
    ) -> impl axum::response::IntoResponse {
        negotiate_response(data, headers, path)
    }
}

/// URL building utilities
pub struct UrlHelper;

impl UrlHelper {
    /// Build URL with query parameters
    pub fn build_url(base: &str, params: &HashMap<String, String>) -> String {
        if params.is_empty() {
            return base.to_string();
        }

        let query: Vec<String> = params
            .iter()
            .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
            .collect();

        format!("{}?{}", base, query.join("&"))
    }

    /// Extract base URL from request URI
    pub fn get_base_url(uri: &Uri) -> String {
        format!(
            "{}://{}{}",
            uri.scheme_str().unwrap_or("http"),
            uri.authority().map(|a| a.as_str()).unwrap_or("localhost"),
            uri.path()
        )
    }

    /// Build resource URL
    pub fn resource_url(base: &str, resource_type: &str, id: &str) -> String {
        format!("{}/{}/{}", base, resource_type, urlencoding::encode(id))
    }

    /// Build API endpoint URL
    pub fn api_url(base: &str, endpoint: &str) -> String {
        format!(
            "{}/api/{}",
            base.trim_end_matches('/'),
            endpoint.trim_start_matches('/')
        )
    }
}

/// Search and filtering utilities
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SearchParams {
    /// Search query string
    pub q: Option<String>,

    /// Filter by status
    pub status: Option<String>,

    /// Filter by campaign
    pub campaign: Option<String>,

    /// Filter by suite
    pub suite: Option<String>,

    /// Filter by codebase
    pub codebase: Option<String>,

    /// Date range filter (start)
    pub since: Option<chrono::DateTime<chrono::Utc>>,

    /// Date range filter (end)
    pub until: Option<chrono::DateTime<chrono::Utc>>,
}

impl SearchParams {
    /// Check if any search/filter parameters are set
    pub fn has_filters(&self) -> bool {
        self.q.is_some()
            || self.status.is_some()
            || self.campaign.is_some()
            || self.suite.is_some()
            || self.codebase.is_some()
            || self.since.is_some()
            || self.until.is_some()
    }

    /// Build SQL WHERE clause from filters
    pub fn to_sql_conditions(&self) -> (String, Vec<String>) {
        let mut conditions = Vec::new();
        let mut params = Vec::new();
        let mut param_index = 1;

        if let Some(ref query) = self.q {
            conditions.push(format!(
                "(codebase ILIKE ${} OR description ILIKE ${})",
                param_index,
                param_index + 1
            ));
            let search_pattern = format!("%{}%", query);
            params.push(search_pattern.clone());
            params.push(search_pattern);
            param_index += 2;
        }

        if let Some(ref status) = self.status {
            conditions.push(format!("status = ${}", param_index));
            params.push(status.clone());
            param_index += 1;
        }

        if let Some(ref campaign) = self.campaign {
            conditions.push(format!("campaign = ${}", param_index));
            params.push(campaign.clone());
            param_index += 1;
        }

        if let Some(ref suite) = self.suite {
            conditions.push(format!("suite = ${}", param_index));
            params.push(suite.clone());
            param_index += 1;
        }

        if let Some(ref codebase) = self.codebase {
            conditions.push(format!("codebase = ${}", param_index));
            params.push(codebase.clone());
            param_index += 1;
        }

        if let Some(since) = self.since {
            conditions.push(format!("created_time >= ${}", param_index));
            params.push(since.to_rfc3339());
            param_index += 1;
        }

        if let Some(until) = self.until {
            conditions.push(format!("created_time <= ${}", param_index));
            params.push(until.to_rfc3339());
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        (where_clause, params)
    }
}

/// Common API response helpers
pub struct ApiResponseHelper;

impl ApiResponseHelper {
    /// Create a standard list response with pagination
    pub fn paginated_list<T: Serialize>(
        items: Vec<T>,
        pagination: PaginationInfo,
        total: Option<i64>,
    ) -> ApiResponse<PaginatedListResponse<T>> {
        let response = PaginatedListResponse {
            items,
            pagination,
            total,
        };
        ApiResponse::success(response)
    }

    /// Create a standard item response
    pub fn item<T: Serialize>(item: T) -> ApiResponse<T> {
        ApiResponse::success(item)
    }

    /// Create a standard creation response
    pub fn created<T: Serialize>(item: T, location: Option<String>) -> CreatedResponse<T> {
        CreatedResponse {
            data: item,
            location,
        }
    }

    /// Create a standard update response
    pub fn updated<T: Serialize>(item: T) -> ApiResponse<T> {
        ApiResponse::success(item)
    }

    /// Create a standard deletion response
    pub fn deleted() -> ApiResponse<DeletedResponse> {
        ApiResponse::success(DeletedResponse {
            deleted: true,
            timestamp: chrono::Utc::now(),
        })
    }
}

/// Standard paginated list response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaginatedListResponse<T> {
    pub items: Vec<T>,
    pub pagination: PaginationInfo,
    pub total: Option<i64>,
}

/// Standard creation response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreatedResponse<T> {
    pub data: T,
    pub location: Option<String>,
}

/// Standard deletion response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeletedResponse {
    pub deleted: bool,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Rate limiting helpers
pub struct RateLimitHelper;

impl RateLimitHelper {
    /// Check if request should be rate limited
    pub fn should_limit(
        client_ip: &str,
        endpoint: &str,
        current_requests: u32,
        limit: u32,
        window_seconds: u64,
    ) -> bool {
        // This is a simplified rate limiting check
        // In a real implementation, you'd use Redis or in-memory cache
        current_requests >= limit
    }

    /// Get rate limit headers
    pub fn get_rate_limit_headers(
        limit: u32,
        remaining: u32,
        reset_time: chrono::DateTime<chrono::Utc>,
    ) -> Vec<(String, String)> {
        vec![
            ("X-RateLimit-Limit".to_string(), limit.to_string()),
            ("X-RateLimit-Remaining".to_string(), remaining.to_string()),
            (
                "X-RateLimit-Reset".to_string(),
                reset_time.timestamp().to_string(),
            ),
        ]
    }
}

/// Content type detection utilities
pub struct ContentTypeHelper;

impl ContentTypeHelper {
    /// Detect content type from file extension
    pub fn from_extension(filename: &str) -> ContentType {
        match filename
            .split('.')
            .next_back()
            .unwrap_or("")
            .to_lowercase()
            .as_str()
        {
            "json" => ContentType::Json,
            "html" | "htm" => ContentType::Html,
            "txt" | "log" => ContentType::TextPlain,
            "diff" | "patch" => ContentType::TextDiff,
            "xml" => ContentType::Xml,
            "csv" => ContentType::Csv,
            _ => ContentType::TextPlain,
        }
    }

    /// Get MIME type string for content type
    pub fn to_mime_type(content_type: &ContentType) -> &'static str {
        match content_type {
            ContentType::Json => "application/json",
            ContentType::Html => "text/html",
            ContentType::TextPlain => "text/plain",
            ContentType::TextDiff => "text/x-diff",
            ContentType::OctetStream => "application/octet-stream",
            ContentType::Xml => "application/xml",
            ContentType::Csv => "text/csv",
        }
    }
}

/// Validation utilities for common patterns
pub struct ValidationHelper;

impl ValidationHelper {
    /// Validate email format
    pub fn validate_email(email: &str) -> Result<(), String> {
        if email.is_empty() {
            return Err("Email cannot be empty".to_string());
        }

        if !email.contains('@') || !email.contains('.') {
            return Err("Invalid email format".to_string());
        }

        // Basic email validation - in production you'd use a proper regex
        let parts: Vec<&str> = email.split('@').collect();
        if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
            return Err("Invalid email format".to_string());
        }

        Ok(())
    }

    /// Validate URL format
    pub fn validate_url(url: &str) -> Result<(), String> {
        url.parse::<url::Url>()
            .map_err(|e| format!("Invalid URL: {}", e))?;
        Ok(())
    }

    /// Validate identifier (alphanumeric + hyphens/underscores)
    pub fn validate_identifier(id: &str, min_len: usize, max_len: usize) -> Result<(), String> {
        if id.len() < min_len {
            return Err(format!(
                "Identifier too short (minimum {} characters)",
                min_len
            ));
        }

        if id.len() > max_len {
            return Err(format!(
                "Identifier too long (maximum {} characters)",
                max_len
            ));
        }

        if !id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(
                "Identifier can only contain alphanumeric characters, hyphens, and underscores"
                    .to_string(),
            );
        }

        Ok(())
    }

    /// Validate required field
    pub fn validate_required(value: &Option<String>, field_name: &str) -> Result<String, String> {
        value
            .as_ref()
            .filter(|s| !s.trim().is_empty())
            .cloned()
            .ok_or_else(|| format!("{} is required", field_name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_helper() {
        let params = PaginationParams {
            offset: Some(20),
            limit: Some(10),
            page: None,
        };

        let pagination = PaginationHelper::calculate_pagination(&params, Some(100), 10);
        assert_eq!(pagination.offset, 20);
        assert_eq!(pagination.limit, 10);
        assert_eq!(pagination.total, Some(100));
        assert!(pagination.has_more);
    }

    #[test]
    fn test_query_helper_parse_bool() {
        assert_eq!(QueryHelper::parse_bool("true"), Ok(true));
        assert_eq!(QueryHelper::parse_bool("1"), Ok(true));
        assert_eq!(QueryHelper::parse_bool("false"), Ok(false));
        assert_eq!(QueryHelper::parse_bool("0"), Ok(false));
        assert!(QueryHelper::parse_bool("invalid").is_err());
    }

    #[test]
    fn test_query_helper_parse_list() {
        assert_eq!(QueryHelper::parse_list("a,b,c"), vec!["a", "b", "c"]);
        assert_eq!(QueryHelper::parse_list(" a , b , c "), vec!["a", "b", "c"]);
        assert_eq!(QueryHelper::parse_list(""), Vec::<String>::new());
    }

    #[test]
    fn test_query_helper_parse_sort() {
        let valid_fields = &["name", "created_time", "status"];

        assert_eq!(
            QueryHelper::parse_sort("name", valid_fields),
            Ok(("name".to_string(), SortOrder::Asc))
        );

        assert_eq!(
            QueryHelper::parse_sort("-created_time", valid_fields),
            Ok(("created_time".to_string(), SortOrder::Desc))
        );

        assert!(QueryHelper::parse_sort("invalid", valid_fields).is_err());
    }

    #[test]
    fn test_url_helper() {
        let mut params = HashMap::new();
        params.insert("q".to_string(), "test query".to_string());
        params.insert("limit".to_string(), "10".to_string());

        let url = UrlHelper::build_url("/api/search", &params);
        assert!(url.contains("q=test%20query"));
        assert!(url.contains("limit=10"));
    }

    #[test]
    fn test_validation_helper() {
        assert!(ValidationHelper::validate_email("test@example.com").is_ok());
        assert!(ValidationHelper::validate_email("invalid").is_err());

        assert!(ValidationHelper::validate_identifier("valid-id", 1, 20).is_ok());
        assert!(ValidationHelper::validate_identifier("invalid id", 1, 20).is_err());
    }

    #[test]
    fn test_content_type_helper() {
        assert_eq!(
            ContentTypeHelper::from_extension("test.json"),
            ContentType::Json
        );
        assert_eq!(
            ContentTypeHelper::from_extension("test.html"),
            ContentType::Html
        );
        assert_eq!(
            ContentTypeHelper::from_extension("test.diff"),
            ContentType::TextDiff
        );

        assert_eq!(
            ContentTypeHelper::to_mime_type(&ContentType::Json),
            "application/json"
        );
        assert_eq!(
            ContentTypeHelper::to_mime_type(&ContentType::Html),
            "text/html"
        );
    }
}
