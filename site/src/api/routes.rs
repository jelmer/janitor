use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tracing::{debug, info};
use utoipa::path;

use crate::{
    app::AppState,
    auth::{require_admin, require_login, require_qa_reviewer, UserContext, OptionalUser},
};
use super::{
    content_negotiation::{negotiate_response, ContentType, NegotiatedResponse},
    middleware::{content_negotiation_middleware, logging_middleware, metrics_middleware, cors_middleware},
    types::{
        ApiResponse, ApiResult, CommonQuery, QueueStatus,
    },
    schemas::{Run, MergeProposal},
    error::AppError,
};

/// Create the main API router
pub fn create_api_router() -> Router<Arc<AppState>> {
    Router::new()
        // Health and status endpoints
        .route("/health", get(health_check))
        .route("/status", get(api_status))
        
        // Queue management
        .route("/queue", get(get_queue_status))
        
        // Run management
        .route("/active-runs", get(get_active_runs))
        .route("/active-runs/:run_id", get(get_active_run))
        .route("/active-runs/:run_id/log", get(get_run_logs))
        .route("/active-runs/:run_id/log/:filename", get(get_run_log_file))
        .route("/run/:run_id/diff", get(get_run_diff))
        .route("/run/:run_id/debdiff", get(get_run_debdiff))
        .route("/run/:run_id/diffoscope", get(get_run_diffoscope))
        
        // Merge proposals (simplified)
        .route("/merge-proposals", get(get_merge_proposals))
        
        // Runner status
        .route("/runner/status", get(get_runner_status))
        
        // Search and discovery
        .route("/pkgnames", get(get_package_names))
        .route("/search", get(search_packages))
        
        // Campaign endpoints
        .route("/:campaign/merge-proposals", get(get_campaign_merge_proposals))
        .route("/:campaign/ready", get(get_campaign_ready_runs))
        .route("/:campaign/c/:codebase", get(get_campaign_codebase))
        .route("/:campaign/c/:codebase/publish", post(post_codebase_publish))
        
        // Codebase endpoints
        .route("/c/:codebase", get(get_codebase))
        .route("/c/:codebase/merge-proposals", get(get_codebase_merge_proposals))
        .route("/c/:codebase/runs", get(get_codebase_runs))
        
        // Run endpoints (enhanced)
        .route("/run/:run_id", get(get_run_details))
        .route("/run/:run_id", post(post_run_update))
        .route("/run/:run_id/reschedule", post(post_run_reschedule))
        .route("/run/:run_id/schedule-control", post(post_run_schedule_control))
        
        // Apply middleware
        .layer(axum::middleware::from_fn(cors_middleware))
        .layer(axum::middleware::from_fn(metrics_middleware))
        .layer(axum::middleware::from_fn(logging_middleware))
        .layer(axum::middleware::from_fn(content_negotiation_middleware))
}

/// Create the Cupboard (admin) API router
pub fn create_cupboard_api_router() -> Router<Arc<AppState>> {
    Router::new()
        // Admin queue operations - simplified for now
        .route("/status", get(api_status))
        
        // Apply middleware
        .layer(axum::middleware::from_fn(cors_middleware))
        .layer(axum::middleware::from_fn(metrics_middleware))
        .layer(axum::middleware::from_fn(logging_middleware))
        .layer(axum::middleware::from_fn(content_negotiation_middleware))
}

// ============================================================================
// Health and Status Endpoints
// ============================================================================

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses(
        (status = 200, description = "Service health status", body = ApiResponse<serde_json::Value>),
        (status = 500, description = "Health check failed", body = ApiError)
    )
)]
async fn health_check(
    State(app_state): State<Arc<AppState>>,
) -> ApiResult<Json<ApiResponse<serde_json::Value>>> {
    debug!("Health check requested");
    
    // Check database connectivity
    if let Err(e) = app_state.database.health_check().await {
        let error_response = ApiResponse {
            data: None,
            error: Some("database_error".to_string()),
            reason: Some(format!("Database health check failed: {}", e)),
            details: None,
            pagination: None,
        };
        return Ok(Json(error_response));
    }
    
    let status = serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now(),
        "services": {
            "database": "healthy",
            "redis": "unknown" // TODO: Add Redis health check
        }
    });
    
    Ok(Json(ApiResponse::success(status)))
}

/// API status and version information
#[utoipa::path(
    get,
    path = "/status",
    tag = "health",
    responses(
        (status = 200, description = "API status and version", body = ApiResponse<serde_json::Value>)
    )
)]
async fn api_status() -> Json<ApiResponse<serde_json::Value>> {
    let status = serde_json::json!({
        "service": "janitor-site",
        "version": env!("CARGO_PKG_VERSION"),
        "build_time": option_env!("BUILD_TIME").unwrap_or("unknown"),
        "git_revision": option_env!("GIT_REVISION").unwrap_or("unknown"),
        "api_version": "1.0",
        "capabilities": [
            "content_negotiation",
            "authentication",
            "pagination",
            "rate_limiting"
        ]
    });
    
    Json(ApiResponse::success(status))
}

// ============================================================================
// Queue Management
// ============================================================================

/// Get queue status
#[utoipa::path(
    get,
    path = "/queue",
    tag = "queue",
    params(CommonQuery),
    responses(
        (status = 200, description = "Queue status information", body = ApiResponse<QueueStatus>)
    )
)]
async fn get_queue_status(
    State(_app_state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(_query): Query<CommonQuery>,
) -> impl axum::response::IntoResponse {
    debug!("Queue status requested");
    
    // TODO: Implement actual queue status retrieval
    let queue_status = QueueStatus {
        total_candidates: 1000,
        pending_candidates: 50,
        active_runs: 10,
        campaigns: vec![],
    };
    
    negotiate_response(
        ApiResponse::success(queue_status),
        &headers,
        "/api/queue",
    )
}

// ============================================================================
// Run Management
// ============================================================================

/// Get all active runs
#[utoipa::path(
    get,
    path = "/active-runs",
    tag = "runs",
    params(CommonQuery),
    responses(
        (status = 200, description = "List of active runs", body = ApiResponse<Vec<Run>>)
    )
)]
async fn get_active_runs(
    State(_app_state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<CommonQuery>,
) -> impl axum::response::IntoResponse {
    debug!("Active runs requested with query: {:?}", query);
    
    // TODO: Implement actual active runs retrieval
    let runs: Vec<Run> = vec![];
    let pagination = super::types::PaginationInfo::new(
        Some(0),
        query.pagination.get_offset(),
        query.pagination.get_limit(),
        runs.len(),
    );
    
    negotiate_response(
        ApiResponse::success_with_pagination(runs, pagination),
        &headers,
        "/api/active-runs",
    )
}

/// Get specific active run
#[utoipa::path(
    get,
    path = "/active-runs/{run_id}",
    tag = "runs",
    params(
        ("run_id" = String, Path, description = "Run identifier")
    ),
    responses(
        (status = 200, description = "Run information", body = ApiResponse<Run>),
        (status = 404, description = "Run not found", body = ApiResponse<()>)
    )
)]
async fn get_active_run(
    State(_app_state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Active run {} requested", run_id);
    
    // TODO: Implement actual run retrieval
    // For now, return not found
    let error_response = ApiResponse::<()>::error(
        "not_found".to_string(),
        Some(format!("Run {} not found", run_id)),
    );
    
    let response = negotiate_response(error_response, &headers, "/api/active-runs/{id}");
    // Set status to 404 - this would be handled by the error system in real implementation
    response
}

/// Get run logs
#[utoipa::path(
    get,
    path = "/active-runs/{run_id}/log",
    tag = "logs",
    params(
        ("run_id" = String, Path, description = "Run identifier")
    ),
    responses(
        (status = 200, description = "Run logs", body = ApiResponse<serde_json::Value>)
    )
)]
async fn get_run_logs(
    State(_app_state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Run logs for {} requested", run_id);
    
    // TODO: Implement actual log retrieval
    let logs = serde_json::json!({
        "run_id": run_id,
        "logs": [],
        "status": "not_implemented"
    });
    
    negotiate_response(
        ApiResponse::success(logs),
        &headers,
        "/api/active-runs/{id}/log",
    )
}

/// Get specific run log file
#[utoipa::path(
    get,
    path = "/active-runs/{run_id}/log/{filename}",
    tag = "logs",
    params(
        ("run_id" = String, Path, description = "Run identifier"),
        ("filename" = String, Path, description = "Log filename")
    ),
    responses(
        (status = 200, description = "Log file content")
    )
)]
async fn get_run_log_file(
    State(_app_state): State<Arc<AppState>>,
    Path((run_id, filename)): Path<(String, String)>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Run log file {}/{} requested", run_id, filename);
    
    // TODO: Implement actual log file retrieval
    // For log files, we typically want to return plain text or binary content
    let content_type = super::content_negotiation::negotiate_content_type(&headers, &filename);
    
    negotiate_response(
        ApiResponse::success(serde_json::json!({
            "run_id": run_id,
            "filename": filename,
            "content": "Log file content would be here"
        })),
        &headers,
        &format!("/api/active-runs/{}/log/{}", run_id, filename),
    )
}


/// Get VCS diff for run
#[utoipa::path(
    get,
    path = "/run/{run_id}/diff",
    tag = "diffs",
    params(
        ("run_id" = String, Path, description = "Run identifier")
    ),
    responses(
        (status = 200, description = "VCS diff content")
    )
)]
async fn get_run_diff(
    State(_app_state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("VCS diff for run {} requested", run_id);
    
    // For diff endpoints, we want to support text/x-diff content type
    let content_type = super::content_negotiation::negotiate_content_type(&headers, "/run/{id}/diff");
    
    // TODO: Implement actual diff retrieval
    let diff_content = "diff --git a/file.txt b/file.txt\n...";
    
    negotiate_response(
        ApiResponse::success(serde_json::json!({
            "run_id": run_id,
            "diff": diff_content,
            "format": match content_type {
                ContentType::TextDiff => "diff",
                _ => "json"
            }
        })),
        &headers,
        &format!("/api/run/{}/diff", run_id),
    )
}

/// Get debdiff for run
#[utoipa::path(
    get,
    path = "/run/{run_id}/debdiff",
    tag = "diffs",
    params(
        ("run_id" = String, Path, description = "Run identifier")
    ),
    responses(
        (status = 200, description = "Debdiff content")
    )
)]
async fn get_run_debdiff(
    State(_app_state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Debdiff for run {} requested", run_id);
    
    // TODO: Implement actual debdiff retrieval
    let content_type = super::content_negotiation::negotiate_content_type(&headers, "/run/{id}/debdiff");
    
    negotiate_response(
        ApiResponse::success(serde_json::json!({
            "run_id": run_id,
            "debdiff": "debdiff content here",
            "format": match content_type {
                ContentType::TextDiff => "diff",
                _ => "json"
            }
        })),
        &headers,
        &format!("/api/run/{}/debdiff", run_id),
    )
}

/// Get diffoscope output for run
#[utoipa::path(
    get,
    path = "/run/{run_id}/diffoscope",
    tag = "diffs",
    params(
        ("run_id" = String, Path, description = "Run identifier")
    ),
    responses(
        (status = 200, description = "Diffoscope output", body = ApiResponse<serde_json::Value>)
    )
)]
async fn get_run_diffoscope(
    State(_app_state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Diffoscope for run {} requested", run_id);
    
    // TODO: Implement actual diffoscope retrieval
    let result = serde_json::json!({
        "run_id": run_id,
        "diffoscope_html": "<html>Diffoscope output would be here</html>",
        "status": "not_implemented"
    });
    
    negotiate_response(
        ApiResponse::success(result),
        &headers,
        "/api/run/{id}/diffoscope",
    )
}

// ============================================================================
// Merge Proposals
// ============================================================================

/// Get merge proposals
#[utoipa::path(
    get,
    path = "/merge-proposals",
    tag = "merge-proposals",
    params(CommonQuery),
    responses(
        (status = 200, description = "List of merge proposals", body = ApiResponse<Vec<MergeProposal>>)
    )
)]
async fn get_merge_proposals(
    State(_app_state): State<Arc<AppState>>,
    Query(query): Query<CommonQuery>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Merge proposals requested");
    
    // TODO: Implement actual merge proposal retrieval
    let proposals: Vec<MergeProposal> = vec![];
    let pagination = super::types::PaginationInfo::new(
        Some(0),
        query.pagination.get_offset(),
        query.pagination.get_limit(),
        proposals.len(),
    );
    
    negotiate_response(
        ApiResponse::success_with_pagination(proposals, pagination),
        &headers,
        "/api/merge-proposals",
    )
}


// ============================================================================
// Runner Status
// ============================================================================

/// Get runner status
#[utoipa::path(
    get,
    path = "/runner/status",
    tag = "admin",
    responses(
        (status = 200, description = "Runner status information", body = ApiResponse<serde_json::Value>)
    )
)]
async fn get_runner_status(
    State(_app_state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Runner status requested");
    
    // TODO: Implement actual runner status retrieval
    let status = serde_json::json!({
        "status": "healthy",
        "workers": 5,
        "active_runs": 3,
        "queue_length": 10
    });
    
    negotiate_response(
        ApiResponse::success(status),
        &headers,
        "/api/runner/status",
    )
}

// ============================================================================
// Search and Discovery
// ============================================================================

use serde::Deserialize;
use utoipa::IntoParams;

/// Search query parameters
#[derive(Debug, Deserialize, IntoParams)]
pub struct SearchQuery {
    /// Search term for package/codebase names
    pub q: Option<String>,
    /// Limit number of results
    pub limit: Option<u32>,
    /// Campaign filter
    pub campaign: Option<String>,
    /// Result code filter
    pub result_code: Option<String>,
    /// Include only publishable results
    pub publishable_only: Option<bool>,
}

/// Package names endpoint for typeahead
#[utoipa::path(
    get,
    path = "/pkgnames",
    tag = "search",
    params(
        ("q" = Option<String>, Query, description = "Search prefix"),
        ("limit" = Option<u32>, Query, description = "Maximum results to return")
    ),
    responses(
        (status = 200, description = "List of package names", body = Vec<String>)
    )
)]
async fn get_package_names(
    State(app_state): State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Package names search requested: {:?}", query.q);
    
    let limit = query.limit.unwrap_or(20) as i64;
    let search_term = query.q.as_deref();
    
    match app_state.database.search_codebase_names(search_term, Some(limit)).await {
        Ok(names) => {
            negotiate_response(
                ApiResponse::success(names),
                &headers,
                "/api/pkgnames",
            )
        }
        Err(e) => {
            debug!("Failed to search package names: {}", e);
            let error_response = ApiResponse {
                data: None,
                error: Some("search_failed".to_string()),
                reason: Some(format!("Search failed: {}", e)),
                details: None,
                pagination: None,
            };
            negotiate_response(
                error_response,
                &headers,
                "/api/pkgnames",
            )
        }
    }
}

/// Advanced package search endpoint
#[utoipa::path(
    get,
    path = "/search",
    tag = "search",
    params(SearchQuery),
    responses(
        (status = 200, description = "Search results with ranking", body = ApiResponse<serde_json::Value>)
    )
)]
async fn search_packages(
    State(app_state): State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Advanced package search requested: {:?}", query);
    
    let limit = query.limit.unwrap_or(50) as i64;
    let search_term = query.q.as_deref();
    
    match app_state.database.search_packages_advanced(
        search_term,
        query.campaign.as_deref(),
        query.result_code.as_deref(),
        query.publishable_only,
        Some(limit)
    ).await {
        Ok(results) => {
            let response = serde_json::json!({
                "results": results,
                "total": results.len(),
                "query": query.q,
                "filters": {
                    "campaign": query.campaign,
                    "result_code": query.result_code,
                    "publishable_only": query.publishable_only.unwrap_or(false),
                }
            });
            
            negotiate_response(
                ApiResponse::success(response),
                &headers,
                "/api/search",
            )
        }
        Err(e) => {
            debug!("Failed to search packages: {}", e);
            let error_response = ApiResponse {
                data: None,
                error: Some("search_failed".to_string()),
                reason: Some(format!("Search failed: {}", e)),
                details: None,
                pagination: None,
            };
            negotiate_response(
                error_response,
                &headers,
                "/api/search",
            )
        }
    }
}

// ============================================================================
// Campaign Endpoints
// ============================================================================

/// Get merge proposals for a specific campaign
#[utoipa::path(
    get,
    path = "/{campaign}/merge-proposals",
    tag = "campaigns",
    params(
        ("campaign" = String, Path, description = "Campaign name"),
        CommonQuery
    ),
    responses(
        (status = 200, description = "Campaign merge proposals", body = ApiResponse<Vec<MergeProposal>>)
    )
)]
async fn get_campaign_merge_proposals(
    State(_app_state): State<Arc<AppState>>,
    Path(campaign): Path<String>,
    Query(query): Query<CommonQuery>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Campaign {} merge proposals requested", campaign);
    
    // TODO: Implement actual campaign merge proposal retrieval
    let proposals: Vec<MergeProposal> = vec![];
    let pagination = super::types::PaginationInfo::new(
        Some(0),
        query.pagination.get_offset(),
        query.pagination.get_limit(),
        proposals.len(),
    );
    
    negotiate_response(
        ApiResponse::success_with_pagination(proposals, pagination),
        &headers,
        &format!("/api/{}/merge-proposals", campaign),
    )
}

/// Get ready runs for a specific campaign
#[utoipa::path(
    get,
    path = "/{campaign}/ready",
    tag = "campaigns",
    params(
        ("campaign" = String, Path, description = "Campaign name"),
        CommonQuery
    ),
    responses(
        (status = 200, description = "Campaign ready runs", body = ApiResponse<Vec<Run>>)
    )
)]
async fn get_campaign_ready_runs(
    State(_app_state): State<Arc<AppState>>,
    Path(campaign): Path<String>,
    Query(query): Query<CommonQuery>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Campaign {} ready runs requested", campaign);
    
    // TODO: Implement actual campaign ready runs retrieval
    let runs: Vec<Run> = vec![];
    let pagination = super::types::PaginationInfo::new(
        Some(0),
        query.pagination.get_offset(),
        query.pagination.get_limit(),
        runs.len(),
    );
    
    negotiate_response(
        ApiResponse::success_with_pagination(runs, pagination),
        &headers,
        &format!("/api/{}/ready", campaign),
    )
}

/// Get campaign-specific codebase information
#[utoipa::path(
    get,
    path = "/{campaign}/c/{codebase}",
    tag = "campaigns",
    params(
        ("campaign" = String, Path, description = "Campaign name"),
        ("codebase" = String, Path, description = "Codebase identifier")
    ),
    responses(
        (status = 200, description = "Campaign codebase information", body = ApiResponse<serde_json::Value>)
    )
)]
async fn get_campaign_codebase(
    State(_app_state): State<Arc<AppState>>,
    Path((campaign, codebase)): Path<(String, String)>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Campaign {} codebase {} requested", campaign, codebase);
    
    // TODO: Implement actual campaign codebase retrieval
    let result = serde_json::json!({
        "campaign": campaign,
        "codebase": codebase,
        "status": "not_implemented"
    });
    
    negotiate_response(
        ApiResponse::success(result),
        &headers,
        &format!("/api/{}/c/{}", campaign, codebase),
    )
}

/// Publish a codebase within a campaign
#[utoipa::path(
    post,
    path = "/{campaign}/c/{codebase}/publish",
    tag = "campaigns",
    params(
        ("campaign" = String, Path, description = "Campaign name"),
        ("codebase" = String, Path, description = "Codebase identifier")
    ),
    responses(
        (status = 200, description = "Publish operation result", body = ApiResponse<serde_json::Value>),
        (status = 403, description = "Insufficient permissions")
    )
)]
async fn post_codebase_publish(
    State(_app_state): State<Arc<AppState>>,
    Path((campaign, codebase)): Path<(String, String)>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Publish requested for campaign {} codebase {}", campaign, codebase);
    
    // TODO: Implement actual publish operation
    let result = serde_json::json!({
        "campaign": campaign,
        "codebase": codebase,
        "action": "publish",
        "status": "not_implemented"
    });
    
    negotiate_response(
        ApiResponse::success(result),
        &headers,
        &format!("/api/{}/c/{}/publish", campaign, codebase),
    )
}

// ============================================================================
// Codebase Endpoints
// ============================================================================

/// Get general codebase information
#[utoipa::path(
    get,
    path = "/c/{codebase}",
    tag = "codebases",
    params(
        ("codebase" = String, Path, description = "Codebase identifier")
    ),
    responses(
        (status = 200, description = "Codebase information", body = ApiResponse<serde_json::Value>)
    )
)]
async fn get_codebase(
    State(_app_state): State<Arc<AppState>>,
    Path(codebase): Path<String>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Codebase {} requested", codebase);
    
    // TODO: Implement actual codebase retrieval
    let result = serde_json::json!({
        "codebase": codebase,
        "status": "not_implemented"
    });
    
    negotiate_response(
        ApiResponse::success(result),
        &headers,
        &format!("/api/c/{}", codebase),
    )
}

/// Get merge proposals for a specific codebase
#[utoipa::path(
    get,
    path = "/c/{codebase}/merge-proposals",
    tag = "codebases",
    params(
        ("codebase" = String, Path, description = "Codebase identifier"),
        CommonQuery
    ),
    responses(
        (status = 200, description = "Codebase merge proposals", body = ApiResponse<Vec<MergeProposal>>)
    )
)]
async fn get_codebase_merge_proposals(
    State(_app_state): State<Arc<AppState>>,
    Path(codebase): Path<String>,
    Query(query): Query<CommonQuery>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Codebase {} merge proposals requested", codebase);
    
    // TODO: Implement actual codebase merge proposal retrieval
    let proposals: Vec<MergeProposal> = vec![];
    let pagination = super::types::PaginationInfo::new(
        Some(0),
        query.pagination.get_offset(),
        query.pagination.get_limit(),
        proposals.len(),
    );
    
    negotiate_response(
        ApiResponse::success_with_pagination(proposals, pagination),
        &headers,
        &format!("/api/c/{}/merge-proposals", codebase),
    )
}

/// Get runs for a specific codebase
#[utoipa::path(
    get,
    path = "/c/{codebase}/runs",
    tag = "codebases",
    params(
        ("codebase" = String, Path, description = "Codebase identifier"),
        CommonQuery
    ),
    responses(
        (status = 200, description = "Codebase runs", body = ApiResponse<Vec<Run>>)
    )
)]
async fn get_codebase_runs(
    State(_app_state): State<Arc<AppState>>,
    Path(codebase): Path<String>,
    Query(query): Query<CommonQuery>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Codebase {} runs requested", codebase);
    
    // TODO: Implement actual codebase runs retrieval
    let runs: Vec<Run> = vec![];
    let pagination = super::types::PaginationInfo::new(
        Some(0),
        query.pagination.get_offset(),
        query.pagination.get_limit(),
        runs.len(),
    );
    
    negotiate_response(
        ApiResponse::success_with_pagination(runs, pagination),
        &headers,
        &format!("/api/c/{}/runs", codebase),
    )
}

// ============================================================================
// Enhanced Run Endpoints
// ============================================================================

/// Get detailed run information
#[utoipa::path(
    get,
    path = "/run/{run_id}",
    tag = "runs",
    params(
        ("run_id" = String, Path, description = "Run identifier")
    ),
    responses(
        (status = 200, description = "Detailed run information", body = ApiResponse<Run>),
        (status = 404, description = "Run not found")
    )
)]
async fn get_run_details(
    State(_app_state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Run details for {} requested", run_id);
    
    // TODO: Implement actual run details retrieval
    let error_response = ApiResponse::<()>::error(
        "not_found".to_string(),
        Some(format!("Run {} not found", run_id)),
    );
    
    negotiate_response(
        error_response,
        &headers,
        &format!("/api/run/{}", run_id),
    )
}

/// Update run information
#[utoipa::path(
    post,
    path = "/run/{run_id}",
    tag = "runs",
    params(
        ("run_id" = String, Path, description = "Run identifier")
    ),
    responses(
        (status = 200, description = "Run updated successfully", body = ApiResponse<serde_json::Value>),
        (status = 404, description = "Run not found"),
        (status = 403, description = "Insufficient permissions")
    )
)]
async fn post_run_update(
    State(_app_state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
    headers: HeaderMap,
    body: String,
) -> impl axum::response::IntoResponse {
    debug!("Run {} update requested", run_id);
    
    // TODO: Implement actual run update
    let result = serde_json::json!({
        "run_id": run_id,
        "action": "update",
        "status": "not_implemented"
    });
    
    negotiate_response(
        ApiResponse::success(result),
        &headers,
        &format!("/api/run/{}", run_id),
    )
}

/// Reschedule a run
#[utoipa::path(
    post,
    path = "/run/{run_id}/reschedule",
    tag = "runs",
    params(
        ("run_id" = String, Path, description = "Run identifier")
    ),
    responses(
        (status = 200, description = "Run rescheduled successfully", body = ApiResponse<serde_json::Value>),
        (status = 404, description = "Run not found"),
        (status = 403, description = "Insufficient permissions")
    )
)]
async fn post_run_reschedule(
    State(_app_state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Run {} reschedule requested", run_id);
    
    // TODO: Implement actual run reschedule
    let result = serde_json::json!({
        "run_id": run_id,
        "action": "reschedule",
        "status": "not_implemented"
    });
    
    negotiate_response(
        ApiResponse::success(result),
        &headers,
        &format!("/api/run/{}/reschedule", run_id),
    )
}

/// Control run scheduling
#[utoipa::path(
    post,
    path = "/run/{run_id}/schedule-control",
    tag = "runs",
    params(
        ("run_id" = String, Path, description = "Run identifier")
    ),
    responses(
        (status = 200, description = "Run schedule control applied", body = ApiResponse<serde_json::Value>),
        (status = 404, description = "Run not found"),
        (status = 403, description = "Insufficient permissions")
    )
)]
async fn post_run_schedule_control(
    State(_app_state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
    headers: HeaderMap,
    query: Query<std::collections::HashMap<String, String>>,
) -> impl axum::response::IntoResponse {
    debug!("Run {} schedule control requested", run_id);
    
    // TODO: Implement actual run schedule control
    let result = serde_json::json!({
        "run_id": run_id,
        "action": "schedule_control",
        "status": "not_implemented"
    });
    
    negotiate_response(
        ApiResponse::success(result),
        &headers,
        &format!("/api/run/{}/schedule-control", run_id),
    )
}

