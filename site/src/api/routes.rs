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
        .route("/codebases", get(get_codebases_filtered))
        .route("/runs", get(get_runs_filtered))
        .route("/export/codebases", get(export_codebases))
        .route("/export/runs", get(export_runs))
        
        // Administrative APIs (Phase 3.6.3)
        .route("/admin/system/status", get(admin_system_status))
        .route("/admin/system/config", get(admin_system_config))
        .route("/admin/system/metrics", get(admin_system_metrics))
        .route("/admin/runs/:run_id/kill", post(admin_kill_run))
        .route("/admin/runs/mass-reschedule", post(admin_mass_reschedule))
        .route("/admin/runs/:run_id/reprocess-logs", post(admin_reprocess_run_logs))
        .route("/admin/publish/autopublish", post(admin_autopublish))
        .route("/admin/publish/scan", post(admin_publish_scan))
        .route("/admin/workers", get(admin_get_workers))
        .route("/admin/workers/:worker_id", get(admin_get_worker_details))
        
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
) -> Json<ApiResponse<serde_json::Value>> {
    debug!("Health check requested");
    
    let mut services = std::collections::HashMap::new();
    
    // Check database connectivity
    match app_state.database.health_check().await {
        Ok(_) => {
            services.insert("database", "healthy");
        }
        Err(e) => {
            services.insert("database", "unhealthy");
            let error_response = ApiResponse {
                data: None,
                error: Some("database_error".to_string()),
                reason: Some(format!("Database health check failed: {}", e)),
                details: Some(serde_json::json!({"services": services})),
                pagination: None,
            };
            return Json(error_response);
        }
    }
    
    // TODO: Add Redis health check when available
    services.insert("redis", "unknown");
    
    let status = serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now(),
        "services": services
    });
    
    Json(ApiResponse::success(status))
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
    State(app_state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<CommonQuery>,
) -> impl axum::response::IntoResponse {
    debug!("Queue status requested with query: {:?}", query);
    
    match app_state.database.get_stats().await {
        Ok(stats) => {
            let mut queue_status = QueueStatus {
                total_candidates: stats.get("total_codebases").unwrap_or(&0).clone(),
                pending_candidates: stats.get("queue_size").unwrap_or(&0).clone(),
                active_runs: stats.get("active_runs").unwrap_or(&0).clone(),
                campaigns: vec![], // TODO: Implement campaign listing
            };
            
            // Enhanced response with additional statistics if requested
            let enhanced_response = serde_json::json!({
                "queue": queue_status,
                "additional_stats": {
                    "recent_successful_runs": stats.get("recent_successful_runs").unwrap_or(&0),
                    "timestamp": chrono::Utc::now(),
                },
                "query_options": {
                    "limit_supported": true,
                    "filtering_supported": true,
                    "available_filters": ["campaign", "result_code", "vcs_type"]
                }
            });
            
            negotiate_response(
                ApiResponse::success(enhanced_response),
                &headers,
                "/api/queue",
            )
        }
        Err(e) => {
            debug!("Failed to get queue status: {}", e);
            let error_response = ApiResponse {
                data: None,
                error: Some("database_error".to_string()),
                reason: Some(format!("Failed to retrieve queue status: {}", e)),
                details: None,
                pagination: None,
            };
            negotiate_response(
                error_response,
                &headers,
                "/api/queue",
            )
        }
    }
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

use serde::{Deserialize, Serialize};
use utoipa::IntoParams;

/// Search query parameters
#[derive(Debug, Serialize, Deserialize, IntoParams)]
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

/// Enhanced filtering parameters for complex queries
#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct FilterQuery {
    /// Search term for package/codebase names
    pub q: Option<String>,
    /// Limit number of results
    pub limit: Option<u32>,
    /// Offset for pagination
    pub offset: Option<u32>,
    /// Campaign filter
    pub campaign: Option<String>,
    /// Suite filter (alias for campaign)
    pub suite: Option<String>,
    /// Result code filter
    pub result_code: Option<String>,
    /// Multiple result codes (comma-separated)
    pub result_codes: Option<String>,
    /// VCS type filter (git, bzr, etc.)
    pub vcs_type: Option<String>,
    /// Include only publishable results
    pub publishable_only: Option<bool>,
    /// Include only successful runs
    pub success_only: Option<bool>,
    /// Time range filter - start time
    pub start_time: Option<String>,
    /// Time range filter - end time  
    pub end_time: Option<String>,
    /// Minimum success chance
    pub min_success_chance: Option<f64>,
    /// Maximum success chance
    pub max_success_chance: Option<f64>,
    /// Sort field (name, last_run, success_chance, etc.)
    pub sort: Option<String>,
    /// Sort order (asc, desc)
    pub order: Option<String>,
    /// Include inactive codebases
    pub include_inactive: Option<bool>,
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
// Phase 3.6.2: Enhanced Query & Filtering APIs
// ============================================================================

/// Enhanced codebases listing with filtering and sorting
#[utoipa::path(
    get,
    path = "/codebases",
    tag = "query",
    params(FilterQuery),
    responses(
        (status = 200, description = "Filtered codebases list", body = ApiResponse<Vec<serde_json::Value>>)
    )
)]
async fn get_codebases_filtered(
    State(app_state): State<Arc<AppState>>,
    Query(filter): Query<FilterQuery>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Filtered codebases requested with filter: {:?}", filter);
    
    // Parse pagination from filter query
    let limit = filter.limit.unwrap_or(50) as i64;
    let offset = filter.offset.unwrap_or(0) as i64;
    
    // Use campaign or suite (campaign alias)
    let campaign_filter = filter.campaign.or(filter.suite);
    
    // Build search term
    let search_term = filter.q.as_deref();
    
    match app_state.database.get_codebases(Some(limit), Some(offset), search_term).await {
        Ok(codebases) => {
            // Apply additional filtering that's not handled in the database query
            let mut filtered_codebases: Vec<serde_json::Value> = codebases
                .into_iter()
                .map(|cb| serde_json::to_value(cb).unwrap_or_default())
                .collect();
            
            // Apply VCS type filter if specified
            if let Some(vcs_filter) = &filter.vcs_type {
                filtered_codebases.retain(|cb| {
                    cb.get("vcs_type")
                        .and_then(|v| v.as_str())
                        .map(|vcs| vcs.to_lowercase() == vcs_filter.to_lowercase())
                        .unwrap_or(false)
                });
            }
            
            // Apply sorting if specified
            if let Some(sort_field) = &filter.sort {
                let ascending = filter.order.as_deref() != Some("desc");
                
                filtered_codebases.sort_by(|a, b| {
                    let a_val = a.get(sort_field);
                    let b_val = b.get(sort_field);
                    
                    match (a_val, b_val) {
                        (Some(a_str), Some(b_str)) => {
                            if ascending {
                                a_str.to_string().cmp(&b_str.to_string())
                            } else {
                                b_str.to_string().cmp(&a_str.to_string())
                            }
                        }
                        (Some(_), None) => if ascending { std::cmp::Ordering::Less } else { std::cmp::Ordering::Greater },
                        (None, Some(_)) => if ascending { std::cmp::Ordering::Greater } else { std::cmp::Ordering::Less },
                        (None, None) => std::cmp::Ordering::Equal,
                    }
                });
            }
            
            let pagination = super::types::PaginationInfo::new(
                None, // total_count would need additional query
                offset,
                limit,
                filtered_codebases.len(),
            );
            
            let response_data = serde_json::json!({
                "codebases": filtered_codebases,
                "filters_applied": {
                    "search": search_term,
                    "campaign": campaign_filter,
                    "vcs_type": filter.vcs_type,
                    "sort": filter.sort,
                    "order": filter.order,
                },
                "total_results": filtered_codebases.len()
            });
            
            negotiate_response(
                ApiResponse::success_with_pagination(response_data, pagination),
                &headers,
                "/api/codebases",
            )
        }
        Err(e) => {
            debug!("Failed to get filtered codebases: {}", e);
            let error_response = ApiResponse::<serde_json::Value> {
                data: None,
                error: Some("database_error".to_string()),
                reason: Some(format!("Failed to retrieve codebases: {}", e)),
                details: None,
                pagination: None,
            };
            
            negotiate_response(
                error_response,
                &headers,
                "/api/codebases",
            )
        }
    }
}

/// Enhanced runs listing with filtering and sorting
#[utoipa::path(
    get,
    path = "/runs",
    tag = "query",
    params(FilterQuery),
    responses(
        (status = 200, description = "Filtered runs list", body = ApiResponse<Vec<serde_json::Value>>)
    )
)]
async fn get_runs_filtered(
    State(app_state): State<Arc<AppState>>,
    Query(filter): Query<FilterQuery>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Filtered runs requested with filter: {:?}", filter);
    
    // For runs, we need to query across multiple codebases or use a different approach
    // This is a simplified implementation - in a real system, you'd want a dedicated runs table query
    
    let result = serde_json::json!({
        "runs": [],
        "message": "Enhanced runs filtering not yet implemented - needs dedicated database query",
        "filters_requested": {
            "search": filter.q,
            "campaign": filter.campaign.or(filter.suite),
            "result_code": filter.result_code,
            "result_codes": filter.result_codes,
            "vcs_type": filter.vcs_type,
            "publishable_only": filter.publishable_only,
            "success_only": filter.success_only,
            "time_range": {
                "start": filter.start_time,
                "end": filter.end_time
            },
            "success_chance_range": {
                "min": filter.min_success_chance,
                "max": filter.max_success_chance
            },
            "sort": filter.sort,
            "order": filter.order,
            "include_inactive": filter.include_inactive
        }
    });
    
    negotiate_response(
        ApiResponse::success(result),
        &headers,
        "/api/runs",
    )
}

/// Export query parameters
#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct ExportQuery {
    /// Export format (json, csv, xml)
    pub format: Option<String>,
    /// Include all filters from FilterQuery
    #[serde(flatten)]
    pub filter: FilterQuery,
}

/// Admin mass reschedule parameters
#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct MassRescheduleQuery {
    /// Result code to reschedule
    pub result_code: Option<String>,
    /// Include transient failures
    pub include_transient: Option<bool>,
    /// Campaign/suite filter
    pub campaign: Option<String>,
    /// Maximum number of runs to reschedule
    pub limit: Option<u32>,
    /// Requester information
    pub requester: Option<String>,
}

/// Admin configuration query parameters
#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct AdminConfigQuery {
    /// Include sensitive configuration values
    pub include_sensitive: Option<bool>,
    /// Configuration section filter
    pub section: Option<String>,
}

/// Export codebases data in various formats
#[utoipa::path(
    get,
    path = "/export/codebases",
    tag = "export",
    params(ExportQuery),
    responses(
        (status = 200, description = "Exported codebases data")
    )
)]
async fn export_codebases(
    State(app_state): State<Arc<AppState>>,
    Query(export_query): Query<ExportQuery>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Codebases export requested: {:?}", export_query);
    
    let format = export_query.format.as_deref().unwrap_or("json");
    
    // Get filtered codebases using the same logic as get_codebases_filtered
    let limit = export_query.filter.limit.unwrap_or(1000) as i64; // Higher default for export
    let offset = export_query.filter.offset.unwrap_or(0) as i64;
    let search_term = export_query.filter.q.as_deref();
    
    match app_state.database.get_codebases(Some(limit), Some(offset), search_term).await {
        Ok(codebases) => {
            match format {
                "csv" => {
                    let csv_headers = "name,url,branch,vcs_type\n";
                    let csv_rows: String = codebases
                        .iter()
                        .map(|cb| format!("{},{},{},{}", 
                            cb.name, 
                            cb.url, 
                            cb.branch.as_deref().unwrap_or(""), 
                            "git" // Default, would need to be fetched from DB
                        ))
                        .collect::<Vec<_>>()
                        .join("\n");
                    
                    let csv_content = format!("{}{}", csv_headers, csv_rows);
                    
                    negotiate_response(
                        ApiResponse::success(serde_json::json!({
                            "format": "csv",
                            "content": csv_content,
                            "content_type": "text/csv",
                            "filename": format!("codebases_export_{}.csv", chrono::Utc::now().format("%Y%m%d_%H%M%S"))
                        })),
                        &headers,
                        "/api/export/codebases",
                    )
                }
                "xml" => {
                    let xml_content = format!(
                        r#"<?xml version="1.0" encoding="UTF-8"?>
<codebases exported="{}" count="{}">
{}
</codebases>"#,
                        chrono::Utc::now().to_rfc3339(),
                        codebases.len(),
                        codebases
                            .iter()
                            .map(|cb| format!(
                                r#"  <codebase name="{}" url="{}" branch="{}" />"#,
                                cb.name,
                                cb.url,
                                cb.branch.as_deref().unwrap_or("")
                            ))
                            .collect::<Vec<_>>()
                            .join("\n")
                    );
                    
                    negotiate_response(
                        ApiResponse::success(serde_json::json!({
                            "format": "xml",
                            "content": xml_content,
                            "content_type": "application/xml",
                            "filename": format!("codebases_export_{}.xml", chrono::Utc::now().format("%Y%m%d_%H%M%S"))
                        })),
                        &headers,
                        "/api/export/codebases",
                    )
                }
                _ => {
                    // Default to JSON
                    negotiate_response(
                        ApiResponse::success(serde_json::json!({
                            "format": "json",
                            "codebases": codebases,
                            "export_metadata": {
                                "exported_at": chrono::Utc::now(),
                                "total_count": codebases.len(),
                                "filters_applied": {
                                    "search": search_term,
                                    "limit": limit,
                                    "offset": offset
                                }
                            }
                        })),
                        &headers,
                        "/api/export/codebases",
                    )
                }
            }
        }
        Err(e) => {
            debug!("Failed to export codebases: {}", e);
            let error_response = ApiResponse::<serde_json::Value> {
                data: None,
                error: Some("export_failed".to_string()),
                reason: Some(format!("Failed to export codebases: {}", e)),
                details: None,
                pagination: None,
            };
            
            negotiate_response(
                error_response,
                &headers,
                "/api/export/codebases",
            )
        }
    }
}

/// Export runs data in various formats
#[utoipa::path(
    get,
    path = "/export/runs",
    tag = "export",
    params(ExportQuery),
    responses(
        (status = 200, description = "Exported runs data")
    )
)]
async fn export_runs(
    State(_app_state): State<Arc<AppState>>,
    Query(export_query): Query<ExportQuery>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Runs export requested: {:?}", export_query);
    
    let format = export_query.format.as_deref().unwrap_or("json");
    
    // Placeholder implementation - would need proper runs table query
    let result = serde_json::json!({
        "format": format,
        "message": "Runs export not yet implemented - needs dedicated database queries",
        "export_metadata": {
            "requested_at": chrono::Utc::now(),
            "filters_requested": export_query.filter
        }
    });
    
    negotiate_response(
        ApiResponse::success(result),
        &headers,
        "/api/export/runs",
    )
}

// ============================================================================
// Phase 3.6.3: Administrative APIs
// ============================================================================

/// Get comprehensive system status for administrators
#[utoipa::path(
    get,
    path = "/admin/system/status",
    tag = "admin",
    responses(
        (status = 200, description = "System status information", body = ApiResponse<serde_json::Value>),
        (status = 403, description = "Insufficient permissions")
    )
)]
async fn admin_system_status(
    State(app_state): State<Arc<AppState>>,
    headers: HeaderMap,
    // TODO: Add admin authentication middleware
) -> impl axum::response::IntoResponse {
    debug!("Admin system status requested");
    
    // Get comprehensive system information
    match app_state.database.get_stats().await {
        Ok(stats) => {
            let system_status = serde_json::json!({
                "system": {
                    "status": "operational",
                    "uptime": "unknown", // TODO: Track uptime
                    "version": env!("CARGO_PKG_VERSION"),
                    "build_time": option_env!("BUILD_TIME").unwrap_or("unknown"),
                    "git_revision": option_env!("GIT_REVISION").unwrap_or("unknown"),
                },
                "database": {
                    "status": "healthy",
                    "total_codebases": stats.get("total_codebases").unwrap_or(&0),
                    "active_runs": stats.get("active_runs").unwrap_or(&0),
                    "queue_size": stats.get("queue_size").unwrap_or(&0),
                    "recent_successful_runs": stats.get("recent_successful_runs").unwrap_or(&0),
                },
                "services": {
                    "runner": "unknown", // TODO: Check runner service
                    "publisher": "unknown", // TODO: Check publisher service
                    "worker_pool": "unknown", // TODO: Check worker pool
                },
                "resources": {
                    "memory_usage": "unknown", // TODO: Add memory monitoring
                    "cpu_usage": "unknown", // TODO: Add CPU monitoring
                    "disk_usage": "unknown", // TODO: Add disk monitoring
                },
                "timestamp": chrono::Utc::now(),
            });
            
            negotiate_response(
                ApiResponse::success(system_status),
                &headers,
                "/api/admin/system/status",
            )
        }
        Err(e) => {
            debug!("Failed to get admin system status: {}", e);
            let error_response = ApiResponse::<serde_json::Value> {
                data: None,
                error: Some("system_error".to_string()),
                reason: Some(format!("Failed to retrieve system status: {}", e)),
                details: None,
                pagination: None,
            };
            
            negotiate_response(
                error_response,
                &headers,
                "/api/admin/system/status",
            )
        }
    }
}

/// Get system configuration for administrators
#[utoipa::path(
    get,
    path = "/admin/system/config",
    tag = "admin",
    params(AdminConfigQuery),
    responses(
        (status = 200, description = "System configuration", body = ApiResponse<serde_json::Value>),
        (status = 403, description = "Insufficient permissions")
    )
)]
async fn admin_system_config(
    State(_app_state): State<Arc<AppState>>,
    Query(query): Query<AdminConfigQuery>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Admin system config requested: {:?}", query);
    
    let include_sensitive = query.include_sensitive.unwrap_or(false);
    
    // Get system configuration (filtered for security)
    let mut config = serde_json::json!({
        "application": {
            "name": "janitor-site",
            "version": env!("CARGO_PKG_VERSION"),
            "environment": "production", // TODO: Get from actual config
        },
        "features": {
            "authentication_enabled": true,
            "rate_limiting_enabled": true,
            "export_enabled": true,
            "admin_api_enabled": true,
        },
        "limits": {
            "max_page_size": 1000,
            "max_export_size": 10000,
            "request_timeout_seconds": 30,
        }
    });
    
    if include_sensitive {
        // Add sensitive configuration (would need proper admin auth)
        config["sensitive"] = serde_json::json!({
            "database_url": "***REDACTED***",
            "redis_url": "***REDACTED***",
            "secret_keys": "***REDACTED***",
            "note": "Sensitive values redacted for security"
        });
    }
    
    if let Some(section) = query.section {
        if let Some(section_data) = config.get(&section) {
            config = section_data.clone();
        } else {
            let error_response = ApiResponse::<serde_json::Value> {
                data: None,
                error: Some("section_not_found".to_string()),
                reason: Some(format!("Configuration section '{}' not found", section)),
                details: None,
                pagination: None,
            };
            
            return negotiate_response(
                error_response,
                &headers,
                "/api/admin/system/config",
            );
        }
    }
    
    negotiate_response(
        ApiResponse::success(config),
        &headers,
        "/api/admin/system/config",
    )
}

/// Get system metrics for monitoring
#[utoipa::path(
    get,
    path = "/admin/system/metrics",
    tag = "admin",
    responses(
        (status = 200, description = "System metrics", body = ApiResponse<serde_json::Value>),
        (status = 403, description = "Insufficient permissions")
    )
)]
async fn admin_system_metrics(
    State(app_state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Admin system metrics requested");
    
    match app_state.database.get_stats().await {
        Ok(stats) => {
            let metrics = serde_json::json!({
                "performance": {
                    "avg_response_time_ms": "unknown", // TODO: Add performance tracking
                    "requests_per_second": "unknown",
                    "error_rate_percent": "unknown",
                },
                "database": {
                    "connection_pool_size": "unknown", // TODO: Get from pool
                    "active_connections": "unknown",
                    "query_avg_time_ms": "unknown",
                    "total_queries": "unknown",
                },
                "business_metrics": {
                    "total_codebases": stats.get("total_codebases").unwrap_or(&0),
                    "active_runs": stats.get("active_runs").unwrap_or(&0),
                    "queue_size": stats.get("queue_size").unwrap_or(&0),
                    "recent_successful_runs": stats.get("recent_successful_runs").unwrap_or(&0),
                    "success_rate_24h": "unknown", // TODO: Calculate success rate
                },
                "system_resources": {
                    "memory_used_mb": "unknown", // TODO: Add system monitoring
                    "memory_total_mb": "unknown",
                    "cpu_percent": "unknown",
                    "disk_used_percent": "unknown",
                },
                "timestamp": chrono::Utc::now(),
                "collection_interval_seconds": 60,
            });
            
            negotiate_response(
                ApiResponse::success(metrics),
                &headers,
                "/api/admin/system/metrics",
            )
        }
        Err(e) => {
            debug!("Failed to get admin metrics: {}", e);
            let error_response = ApiResponse::<serde_json::Value> {
                data: None,
                error: Some("metrics_error".to_string()),
                reason: Some(format!("Failed to retrieve system metrics: {}", e)),
                details: None,
                pagination: None,
            };
            
            negotiate_response(
                error_response,
                &headers,
                "/api/admin/system/metrics",
            )
        }
    }
}

/// Kill a specific run (admin only)
#[utoipa::path(
    post,
    path = "/admin/runs/{run_id}/kill",
    tag = "admin",
    params(
        ("run_id" = String, Path, description = "Run ID to kill")
    ),
    responses(
        (status = 200, description = "Run kill initiated", body = ApiResponse<serde_json::Value>),
        (status = 404, description = "Run not found"),
        (status = 403, description = "Insufficient permissions")
    )
)]
async fn admin_kill_run(
    State(_app_state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Admin kill run requested for run: {}", run_id);
    
    // TODO: Implement actual run killing via runner service
    let result = serde_json::json!({
        "run_id": run_id,
        "action": "kill",
        "status": "not_implemented",
        "message": "Run killing requires integration with runner service",
        "timestamp": chrono::Utc::now()
    });
    
    negotiate_response(
        ApiResponse::success(result),
        &headers,
        &format!("/api/admin/runs/{}/kill", run_id),
    )
}

/// Mass reschedule runs based on criteria (admin only)
#[utoipa::path(
    post,
    path = "/admin/runs/mass-reschedule",
    tag = "admin",
    params(MassRescheduleQuery),
    responses(
        (status = 200, description = "Mass reschedule initiated", body = ApiResponse<serde_json::Value>),
        (status = 403, description = "Insufficient permissions")
    )
)]
async fn admin_mass_reschedule(
    State(_app_state): State<Arc<AppState>>,
    Query(query): Query<MassRescheduleQuery>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Admin mass reschedule requested: {:?}", query);
    
    // TODO: Implement actual mass rescheduling
    let result = serde_json::json!({
        "action": "mass_reschedule",
        "criteria": {
            "result_code": query.result_code,
            "include_transient": query.include_transient.unwrap_or(false),
            "campaign": query.campaign,
            "limit": query.limit.unwrap_or(100),
        },
        "status": "not_implemented",
        "message": "Mass rescheduling requires integration with runner service",
        "estimated_affected_runs": 0,
        "timestamp": chrono::Utc::now()
    });
    
    negotiate_response(
        ApiResponse::success(result),
        &headers,
        "/api/admin/runs/mass-reschedule",
    )
}

/// Reprocess logs for a specific run (admin only)
#[utoipa::path(
    post,
    path = "/admin/runs/{run_id}/reprocess-logs",
    tag = "admin",
    params(
        ("run_id" = String, Path, description = "Run ID to reprocess logs")
    ),
    responses(
        (status = 200, description = "Log reprocessing initiated", body = ApiResponse<serde_json::Value>),
        (status = 404, description = "Run not found"),
        (status = 403, description = "Insufficient permissions")
    )
)]
async fn admin_reprocess_run_logs(
    State(_app_state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Admin reprocess logs requested for run: {}", run_id);
    
    // TODO: Implement actual log reprocessing
    let result = serde_json::json!({
        "run_id": run_id,
        "action": "reprocess_logs",
        "status": "not_implemented",
        "message": "Log reprocessing requires integration with log management service",
        "timestamp": chrono::Utc::now()
    });
    
    negotiate_response(
        ApiResponse::success(result),
        &headers,
        &format!("/api/admin/runs/{}/reprocess-logs", run_id),
    )
}

/// Trigger autopublish scan (admin only)
#[utoipa::path(
    post,
    path = "/admin/publish/autopublish",
    tag = "admin",
    responses(
        (status = 200, description = "Autopublish scan initiated", body = ApiResponse<serde_json::Value>),
        (status = 403, description = "Insufficient permissions")
    )
)]
async fn admin_autopublish(
    State(_app_state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Admin autopublish scan requested");
    
    // TODO: Implement actual autopublish trigger
    let result = serde_json::json!({
        "action": "autopublish_scan",
        "status": "not_implemented",
        "message": "Autopublish scanning requires integration with publisher service",
        "timestamp": chrono::Utc::now()
    });
    
    negotiate_response(
        ApiResponse::success(result),
        &headers,
        "/api/admin/publish/autopublish",
    )
}

/// Trigger publish scan (admin only)
#[utoipa::path(
    post,
    path = "/admin/publish/scan",
    tag = "admin",
    responses(
        (status = 200, description = "Publish scan initiated", body = ApiResponse<serde_json::Value>),
        (status = 403, description = "Insufficient permissions")
    )
)]
async fn admin_publish_scan(
    State(_app_state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Admin publish scan requested");
    
    // TODO: Implement actual publish scan
    let result = serde_json::json!({
        "action": "publish_scan",
        "status": "not_implemented", 
        "message": "Publish scanning requires integration with publisher service",
        "timestamp": chrono::Utc::now()
    });
    
    negotiate_response(
        ApiResponse::success(result),
        &headers,
        "/api/admin/publish/scan",
    )
}

/// Get worker information (admin only)
#[utoipa::path(
    get,
    path = "/admin/workers",
    tag = "admin",
    responses(
        (status = 200, description = "Worker information", body = ApiResponse<serde_json::Value>),
        (status = 403, description = "Insufficient permissions")
    )
)]
async fn admin_get_workers(
    State(_app_state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Admin workers list requested");
    
    // TODO: Implement actual worker status retrieval
    let workers = serde_json::json!({
        "workers": [],
        "total_workers": 0,
        "active_workers": 0,
        "idle_workers": 0,
        "status": "not_implemented",
        "message": "Worker monitoring requires integration with runner service",
        "timestamp": chrono::Utc::now()
    });
    
    negotiate_response(
        ApiResponse::success(workers),
        &headers,
        "/api/admin/workers",
    )
}

/// Get detailed worker information (admin only)
#[utoipa::path(
    get,
    path = "/admin/workers/{worker_id}",
    tag = "admin",
    params(
        ("worker_id" = String, Path, description = "Worker ID")
    ),
    responses(
        (status = 200, description = "Worker details", body = ApiResponse<serde_json::Value>),
        (status = 404, description = "Worker not found"),
        (status = 403, description = "Insufficient permissions")
    )
)]
async fn admin_get_worker_details(
    State(_app_state): State<Arc<AppState>>,
    Path(worker_id): Path<String>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Admin worker details requested for worker: {}", worker_id);
    
    // TODO: Implement actual worker details retrieval
    let worker_details = serde_json::json!({
        "worker_id": worker_id,
        "status": "not_implemented",
        "message": "Worker details require integration with runner service",
        "timestamp": chrono::Utc::now()
    });
    
    negotiate_response(
        ApiResponse::success(worker_details),
        &headers,
        &format!("/api/admin/workers/{}", worker_id),
    )
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
    State(app_state): State<Arc<AppState>>,
    Path(codebase): Path<String>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Codebase {} requested", codebase);
    
    match app_state.database.get_codebase(&codebase).await {
        Ok(codebase_data) => {
            negotiate_response(
                ApiResponse::success(serde_json::to_value(codebase_data).unwrap_or_default()),
                &headers,
                &format!("/api/c/{}", codebase),
            )
        }
        Err(e) => {
            let (error_code, reason) = match e {
                crate::database::DatabaseError::NotFound(_) => {
                    ("not_found", format!("Codebase '{}' not found", codebase))
                }
                _ => {
                    ("database_error", format!("Failed to retrieve codebase: {}", e))
                }
            };
            
            debug!("Failed to get codebase {}: {}", codebase, e);
            let error_response = ApiResponse::<serde_json::Value> {
                data: None,
                error: Some(error_code.to_string()),
                reason: Some(reason),
                details: None,
                pagination: None,
            };
            
            negotiate_response(
                error_response,
                &headers,
                &format!("/api/c/{}", codebase),
            )
        }
    }
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
    State(app_state): State<Arc<AppState>>,
    Path(codebase): Path<String>,
    Query(query): Query<CommonQuery>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Codebase {} runs requested", codebase);
    
    let limit = query.pagination.get_limit();
    let offset = query.pagination.get_offset();
    
    match app_state.database.get_runs_for_codebase(&codebase, Some(limit), Some(offset)).await {
        Ok(runs) => {
            // For proper pagination, we'd need to count total runs
            // For now, we'll use the returned runs length as an approximation
            let pagination = super::types::PaginationInfo::new(
                None, // total_count not implemented yet
                offset,
                limit,
                runs.len(),
            );
            
            negotiate_response(
                ApiResponse::success_with_pagination(runs, pagination),
                &headers,
                &format!("/api/c/{}/runs", codebase),
            )
        }
        Err(e) => {
            debug!("Failed to get runs for codebase {}: {}", codebase, e);
            let error_response = ApiResponse {
                data: None,
                error: Some("database_error".to_string()),
                reason: Some(format!("Failed to retrieve runs for codebase: {}", e)),
                details: None,
                pagination: None,
            };
            
            negotiate_response(
                error_response,
                &headers,
                &format!("/api/c/{}/runs", codebase),
            )
        }
    }
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

