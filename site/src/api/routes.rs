use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    response::Json,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tracing::debug;

use super::{
    content_negotiation::{negotiate_response, ContentType},
    middleware::{
        content_negotiation_middleware, cors_middleware, logging_middleware, metrics_middleware,
    },
    schemas::{MergeProposal, Run},
    types::{ApiResponse, CommonQuery, QueueStatus},
};
use crate::app::AppState;

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
        .route(
            "/admin/runs/:run_id/reprocess-logs",
            post(admin_reprocess_run_logs),
        )
        .route("/admin/publish/autopublish", post(admin_autopublish))
        .route("/admin/publish/scan", post(admin_publish_scan))
        .route("/admin/workers", get(admin_get_workers))
        .route("/admin/workers/:worker_id", get(admin_get_worker_details))
        // Campaign endpoints
        .route(
            "/:campaign/merge-proposals",
            get(get_campaign_merge_proposals),
        )
        .route("/:campaign/ready", get(get_campaign_ready_runs))
        .route("/:campaign/c/:codebase", get(get_campaign_codebase))
        .route(
            "/:campaign/c/:codebase/publish",
            post(post_codebase_publish),
        )
        // Codebase endpoints
        .route("/c/:codebase", get(get_codebase))
        .route(
            "/c/:codebase/merge-proposals",
            get(get_codebase_merge_proposals),
        )
        .route("/c/:codebase/runs", get(get_codebase_runs))
        // Run endpoints (enhanced)
        .route("/run/:run_id", get(get_run_details))
        .route("/run/:run_id", post(post_run_update))
        .route("/run/:run_id/reschedule", post(post_run_reschedule))
        .route(
            "/run/:run_id/schedule-control",
            post(post_run_schedule_control),
        )
        // Apply middleware
        .layer(axum::middleware::from_fn(cors_middleware))
        .layer(axum::middleware::from_fn(metrics_middleware))
        .layer(axum::middleware::from_fn(logging_middleware))
        .layer(axum::middleware::from_fn(content_negotiation_middleware))
}

/// Create the Cupboard (admin) API router
pub fn create_cupboard_api_router() -> Router<Arc<AppState>> {
    Router::new()
        // Core admin status
        .route("/status", get(api_status))
        // Phase 3.7.1: Admin Dashboard
        .route("/dashboard", get(cupboard_dashboard))
        .route("/dashboard/stats", get(cupboard_dashboard_stats))
        // Worker monitoring and management
        .route("/workers", get(cupboard_workers))
        .route("/workers/:worker_id", get(cupboard_worker_details))
        .route("/workers/:worker_id/pause", post(cupboard_worker_pause))
        .route("/workers/:worker_id/resume", post(cupboard_worker_resume))
        // System status and health
        .route("/system", get(cupboard_system_status))
        .route("/system/health", get(cupboard_system_health))
        .route("/system/metrics", get(cupboard_system_metrics))
        .route("/system/config", get(cupboard_system_config))
        // Metrics and reporting
        .route("/reports", get(cupboard_reports))
        .route("/reports/performance", get(cupboard_performance_report))
        .route("/reports/success-rates", get(cupboard_success_rates))
        .route("/reports/trending", get(cupboard_trending_report))
        // Queue management (Phase 3.7.2)
        .route("/queue", get(cupboard_queue_status))
        .route("/queue/browse", get(cupboard_queue_browse))
        .route("/queue/manage", get(cupboard_queue_manage))
        .route(
            "/queue/bulk-operations",
            post(cupboard_queue_bulk_operations),
        )
        // Job control operations
        .route("/jobs/pause", post(cupboard_jobs_pause))
        .route("/jobs/resume", post(cupboard_jobs_resume))
        .route("/jobs/cancel", post(cupboard_jobs_cancel))
        .route("/jobs/requeue", post(cupboard_jobs_requeue))
        // Run management for admins
        .route("/runs", get(cupboard_runs))
        .route("/runs/failed", get(cupboard_failed_runs))
        .route("/runs/stuck", get(cupboard_stuck_runs))
        .route("/runs/:run_id/details", get(cupboard_run_details))
        .route(
            "/runs/:run_id/force-finish",
            post(cupboard_run_force_finish),
        )
        // Publishing oversight
        .route("/publish", get(cupboard_publish_overview))
        .route("/publish/pending", get(cupboard_publish_pending))
        .route("/publish/rate-limits", get(cupboard_publish_rate_limits))
        .route("/publish/history", get(cupboard_publish_history))
        // Review system management
        .route("/reviews", get(cupboard_reviews))
        .route("/reviews/pending", get(cupboard_reviews_pending))
        .route("/reviews/assign", post(cupboard_reviews_assign))
        .route("/reviews/verdicts", get(cupboard_reviews_verdicts))
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

    // Check Redis connectivity if available
    if let Some(ref redis_client) = app_state.redis {
        match redis_client.get_async_connection().await {
            Ok(mut conn) => {
                use redis::AsyncCommands;
                match conn.ping().await {
                    Ok(_) => {
                        services.insert("redis", "healthy");
                    }
                    Err(_) => {
                        services.insert("redis", "unhealthy");
                    }
                }
            }
            Err(_) => {
                services.insert("redis", "unhealthy");
            }
        }
    } else {
        services.insert("redis", "not_configured");
    }

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
            let queue_status = QueueStatus {
                total_candidates: *stats.get("total_codebases").unwrap_or(&0),
                pending_candidates: *stats.get("queue_size").unwrap_or(&0),
                active_runs: *stats.get("active_runs").unwrap_or(&0),
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
            negotiate_response(error_response, &headers, "/api/queue")
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

    // Set status to 404 - this would be handled by the error system in real implementation
    negotiate_response(error_response, &headers, "/api/active-runs/{id}")
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
    let content_type =
        super::content_negotiation::negotiate_content_type(&headers, "/run/{id}/diff");

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
    let content_type =
        super::content_negotiation::negotiate_content_type(&headers, "/run/{id}/debdiff");

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

    negotiate_response(ApiResponse::success(status), &headers, "/api/runner/status")
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

    match app_state
        .database
        .search_codebase_names(search_term, Some(limit))
        .await
    {
        Ok(names) => negotiate_response(ApiResponse::success(names), &headers, "/api/pkgnames"),
        Err(e) => {
            debug!("Failed to search package names: {}", e);
            let error_response = ApiResponse {
                data: None,
                error: Some("search_failed".to_string()),
                reason: Some(format!("Search failed: {}", e)),
                details: None,
                pagination: None,
            };
            negotiate_response(error_response, &headers, "/api/pkgnames")
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

    match app_state
        .database
        .search_packages_advanced(
            search_term,
            query.campaign.as_deref(),
            query.result_code.as_deref(),
            query.publishable_only,
            Some(limit),
        )
        .await
    {
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

            negotiate_response(ApiResponse::success(response), &headers, "/api/search")
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
            negotiate_response(error_response, &headers, "/api/search")
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

    match app_state
        .database
        .get_codebases(Some(limit), Some(offset), search_term)
        .await
    {
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
                        (Some(_), None) => {
                            if ascending {
                                std::cmp::Ordering::Less
                            } else {
                                std::cmp::Ordering::Greater
                            }
                        }
                        (None, Some(_)) => {
                            if ascending {
                                std::cmp::Ordering::Greater
                            } else {
                                std::cmp::Ordering::Less
                            }
                        }
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

            negotiate_response(error_response, &headers, "/api/codebases")
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

    negotiate_response(ApiResponse::success(result), &headers, "/api/runs")
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

    match app_state
        .database
        .get_codebases(Some(limit), Some(offset), search_term)
        .await
    {
        Ok(codebases) => {
            match format {
                "csv" => {
                    let csv_headers = "name,url,branch,vcs_type\n";
                    let csv_rows: String = codebases
                        .iter()
                        .map(|cb| {
                            format!(
                                "{},{},{},{}",
                                cb.name,
                                cb.url,
                                cb.branch.as_deref().unwrap_or(""),
                                "git" // Default, would need to be fetched from DB
                            )
                        })
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

            negotiate_response(error_response, &headers, "/api/export/codebases")
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

    negotiate_response(ApiResponse::success(result), &headers, "/api/export/runs")
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

            negotiate_response(error_response, &headers, "/api/admin/system/status")
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

            return negotiate_response(error_response, &headers, "/api/admin/system/config");
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

            negotiate_response(error_response, &headers, "/api/admin/system/metrics")
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
    debug!(
        "Publish requested for campaign {} codebase {}",
        campaign, codebase
    );

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
        Ok(codebase_data) => negotiate_response(
            ApiResponse::success(serde_json::to_value(codebase_data).unwrap_or_default()),
            &headers,
            &format!("/api/c/{}", codebase),
        ),
        Err(e) => {
            let (error_code, reason) = match e {
                crate::database::DatabaseError::NotFound(_) => {
                    ("not_found", format!("Codebase '{}' not found", codebase))
                }
                _ => (
                    "database_error",
                    format!("Failed to retrieve codebase: {}", e),
                ),
            };

            debug!("Failed to get codebase {}: {}", codebase, e);
            let error_response = ApiResponse::<serde_json::Value> {
                data: None,
                error: Some(error_code.to_string()),
                reason: Some(reason),
                details: None,
                pagination: None,
            };

            negotiate_response(error_response, &headers, &format!("/api/c/{}", codebase))
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

    match app_state
        .database
        .get_runs_for_codebase(&codebase, Some(limit), Some(offset))
        .await
    {
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

    negotiate_response(error_response, &headers, &format!("/api/run/{}", run_id))
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

// ============================================================================
// Phase 3.7: Cupboard Admin Interface Implementation
// ============================================================================

/// Cupboard query parameters for filtering and control
#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct CupboardQuery {
    /// Limit number of results
    pub limit: Option<u32>,
    /// Offset for pagination
    pub offset: Option<u32>,
    /// Filter by status
    pub status: Option<String>,
    /// Filter by worker ID
    pub worker_id: Option<String>,
    /// Include detailed information
    pub detailed: Option<bool>,
}

/// Worker control parameters
#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct WorkerControlQuery {
    /// Reason for the action
    pub reason: Option<String>,
    /// Force the action even if worker is busy
    pub force: Option<bool>,
}

/// Bulk operation parameters
#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct BulkOperationQuery {
    /// Operation type (pause, resume, cancel, requeue)
    pub operation: String,
    /// Target selection criteria
    pub target: Option<String>,
    /// Batch size for processing
    pub batch_size: Option<u32>,
    /// Requester information
    pub requester: Option<String>,
}

// ============================================================================
// Phase 3.7.1: Admin Dashboard Endpoints
// ============================================================================

/// Get main cupboard dashboard overview
#[utoipa::path(
    get,
    path = "/dashboard",
    tag = "cupboard",
    responses(
        (status = 200, description = "Dashboard overview", body = ApiResponse<serde_json::Value>),
        (status = 403, description = "Admin access required")
    )
)]
async fn cupboard_dashboard(
    State(app_state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Cupboard dashboard requested");

    match app_state.database.get_stats().await {
        Ok(stats) => {
            let dashboard_data = serde_json::json!({
                "summary": {
                    "total_codebases": stats.get("total_codebases").unwrap_or(&0),
                    "active_runs": stats.get("active_runs").unwrap_or(&0),
                    "queue_size": stats.get("queue_size").unwrap_or(&0),
                    "recent_successful_runs": stats.get("recent_successful_runs").unwrap_or(&0),
                },
                "system_status": {
                    "status": "operational",
                    "uptime": "unknown", // TODO: Track uptime
                    "workers": {
                        "total": "unknown", // TODO: Get from runner service
                        "active": "unknown",
                        "idle": "unknown"
                    }
                },
                "recent_activity": {
                    "last_24h": {
                        "runs_completed": "unknown", // TODO: Calculate from database
                        "success_rate": "unknown",
                        "average_duration": "unknown"
                    }
                },
                "alerts": [], // TODO: Implement alert system
                "timestamp": chrono::Utc::now()
            });

            negotiate_response(
                ApiResponse::success(dashboard_data),
                &headers,
                "/cupboard/dashboard",
            )
        }
        Err(e) => {
            debug!("Failed to get dashboard data: {}", e);
            let error_response = ApiResponse::<serde_json::Value> {
                data: None,
                error: Some("dashboard_error".to_string()),
                reason: Some(format!("Failed to retrieve dashboard data: {}", e)),
                details: None,
                pagination: None,
            };

            negotiate_response(error_response, &headers, "/cupboard/dashboard")
        }
    }
}

/// Get detailed dashboard statistics
#[utoipa::path(
    get,
    path = "/dashboard/stats",
    tag = "cupboard",
    responses(
        (status = 200, description = "Detailed dashboard statistics", body = ApiResponse<serde_json::Value>)
    )
)]
async fn cupboard_dashboard_stats(
    State(app_state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Cupboard dashboard stats requested");

    match app_state.database.get_stats().await {
        Ok(stats) => {
            let detailed_stats = serde_json::json!({
                "performance": {
                    "average_run_duration": "unknown", // TODO: Calculate from database
                    "runs_per_hour": "unknown",
                    "success_rate_7d": "unknown",
                    "failure_rate_7d": "unknown"
                },
                "queue_metrics": {
                    "average_wait_time": "unknown",
                    "queue_length_trend": "unknown",
                    "processing_rate": "unknown"
                },
                "resource_usage": {
                    "database_connections": "unknown", // TODO: Get from connection pool
                    "memory_usage": "unknown",
                    "cpu_usage": "unknown"
                },
                "campaign_breakdown": {
                    "active_campaigns": "unknown", // TODO: Query campaigns
                    "top_campaigns": []
                },
                "worker_statistics": {
                    "worker_efficiency": "unknown",
                    "worker_utilization": "unknown",
                    "average_tasks_per_worker": "unknown"
                },
                "database_stats": stats,
                "generated_at": chrono::Utc::now()
            });

            negotiate_response(
                ApiResponse::success(detailed_stats),
                &headers,
                "/cupboard/dashboard/stats",
            )
        }
        Err(e) => {
            debug!("Failed to get detailed dashboard stats: {}", e);
            let error_response = ApiResponse::<serde_json::Value> {
                data: None,
                error: Some("stats_error".to_string()),
                reason: Some(format!("Failed to retrieve dashboard statistics: {}", e)),
                details: None,
                pagination: None,
            };

            negotiate_response(error_response, &headers, "/cupboard/dashboard/stats")
        }
    }
}

// ============================================================================
// Worker Monitoring and Management
// ============================================================================

/// Get all workers status and information
#[utoipa::path(
    get,
    path = "/workers",
    tag = "cupboard",
    params(CupboardQuery),
    responses(
        (status = 200, description = "Worker information", body = ApiResponse<serde_json::Value>)
    )
)]
async fn cupboard_workers(
    State(_app_state): State<Arc<AppState>>,
    Query(query): Query<CupboardQuery>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Cupboard workers list requested: {:?}", query);

    // TODO: Integrate with runner service to get actual worker data
    let workers_data = serde_json::json!({
        "workers": [],
        "summary": {
            "total_workers": 0,
            "active_workers": 0,
            "idle_workers": 0,
            "failed_workers": 0
        },
        "status": "not_implemented",
        "message": "Worker monitoring requires integration with runner service",
        "query_parameters": {
            "limit": query.limit.unwrap_or(50),
            "offset": query.offset.unwrap_or(0),
            "status_filter": query.status,
            "detailed": query.detailed.unwrap_or(false)
        },
        "timestamp": chrono::Utc::now()
    });

    negotiate_response(
        ApiResponse::success(workers_data),
        &headers,
        "/cupboard/workers",
    )
}

/// Get detailed information about a specific worker
#[utoipa::path(
    get,
    path = "/workers/{worker_id}",
    tag = "cupboard",
    params(
        ("worker_id" = String, Path, description = "Worker identifier")
    ),
    responses(
        (status = 200, description = "Worker details", body = ApiResponse<serde_json::Value>),
        (status = 404, description = "Worker not found")
    )
)]
async fn cupboard_worker_details(
    State(_app_state): State<Arc<AppState>>,
    Path(worker_id): Path<String>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Cupboard worker details requested for: {}", worker_id);

    // TODO: Get actual worker details from runner service
    let worker_details = serde_json::json!({
        "worker_id": worker_id,
        "status": "not_implemented",
        "message": "Worker details require integration with runner service",
        "placeholder_data": {
            "worker_name": worker_id,
            "status": "unknown",
            "current_task": null,
            "last_heartbeat": null,
            "capabilities": [],
            "performance_metrics": {
                "tasks_completed": 0,
                "average_task_duration": null,
                "success_rate": null
            }
        },
        "timestamp": chrono::Utc::now()
    });

    negotiate_response(
        ApiResponse::success(worker_details),
        &headers,
        &format!("/cupboard/workers/{}", worker_id),
    )
}

/// Pause a specific worker
#[utoipa::path(
    post,
    path = "/workers/{worker_id}/pause",
    tag = "cupboard",
    params(
        ("worker_id" = String, Path, description = "Worker identifier"),
        WorkerControlQuery
    ),
    responses(
        (status = 200, description = "Worker pause initiated", body = ApiResponse<serde_json::Value>),
        (status = 404, description = "Worker not found")
    )
)]
async fn cupboard_worker_pause(
    State(_app_state): State<Arc<AppState>>,
    Path(worker_id): Path<String>,
    Query(control): Query<WorkerControlQuery>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!(
        "Cupboard worker pause requested for: {} with control: {:?}",
        worker_id, control
    );

    // TODO: Implement actual worker pause via runner service
    let result = serde_json::json!({
        "worker_id": worker_id,
        "action": "pause",
        "status": "not_implemented",
        "message": "Worker control requires integration with runner service",
        "parameters": {
            "reason": control.reason,
            "force": control.force.unwrap_or(false)
        },
        "timestamp": chrono::Utc::now()
    });

    negotiate_response(
        ApiResponse::success(result),
        &headers,
        &format!("/cupboard/workers/{}/pause", worker_id),
    )
}

/// Resume a specific worker
#[utoipa::path(
    post,
    path = "/workers/{worker_id}/resume",
    tag = "cupboard",
    params(
        ("worker_id" = String, Path, description = "Worker identifier"),
        WorkerControlQuery
    ),
    responses(
        (status = 200, description = "Worker resume initiated", body = ApiResponse<serde_json::Value>),
        (status = 404, description = "Worker not found")
    )
)]
async fn cupboard_worker_resume(
    State(_app_state): State<Arc<AppState>>,
    Path(worker_id): Path<String>,
    Query(control): Query<WorkerControlQuery>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!(
        "Cupboard worker resume requested for: {} with control: {:?}",
        worker_id, control
    );

    // TODO: Implement actual worker resume via runner service
    let result = serde_json::json!({
        "worker_id": worker_id,
        "action": "resume",
        "status": "not_implemented",
        "message": "Worker control requires integration with runner service",
        "parameters": {
            "reason": control.reason,
            "force": control.force.unwrap_or(false)
        },
        "timestamp": chrono::Utc::now()
    });

    negotiate_response(
        ApiResponse::success(result),
        &headers,
        &format!("/cupboard/workers/{}/resume", worker_id),
    )
}

// ============================================================================
// System Status and Health
// ============================================================================

/// Get comprehensive system status for cupboard admin
#[utoipa::path(
    get,
    path = "/system",
    tag = "cupboard",
    responses(
        (status = 200, description = "System status", body = ApiResponse<serde_json::Value>)
    )
)]
async fn cupboard_system_status(
    State(app_state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Cupboard system status requested");

    match app_state.database.get_stats().await {
        Ok(stats) => {
            let system_status = serde_json::json!({
                "overall_status": "operational",
                "services": {
                    "database": {
                        "status": "healthy",
                        "stats": stats
                    },
                    "runner": {
                        "status": "unknown", // TODO: Check runner service
                        "endpoint": "unknown"
                    },
                    "publisher": {
                        "status": "unknown", // TODO: Check publisher service
                        "endpoint": "unknown"
                    },
                    "differ": {
                        "status": "unknown", // TODO: Check differ service
                        "endpoint": "unknown"
                    }
                },
                "infrastructure": {
                    "redis": {
                        "status": "unknown", // TODO: Check Redis connection
                        "connection_count": "unknown"
                    },
                    "storage": {
                        "status": "unknown", // TODO: Check storage systems
                        "disk_usage": "unknown"
                    }
                },
                "performance": {
                    "response_time_avg": "unknown",
                    "throughput": "unknown",
                    "error_rate": "unknown"
                },
                "last_updated": chrono::Utc::now()
            });

            negotiate_response(
                ApiResponse::success(system_status),
                &headers,
                "/cupboard/system",
            )
        }
        Err(e) => {
            debug!("Failed to get system status: {}", e);
            let error_response = ApiResponse::<serde_json::Value> {
                data: None,
                error: Some("system_error".to_string()),
                reason: Some(format!("Failed to retrieve system status: {}", e)),
                details: None,
                pagination: None,
            };

            negotiate_response(error_response, &headers, "/cupboard/system")
        }
    }
}

/// Get system health check information
#[utoipa::path(
    get,
    path = "/system/health",
    tag = "cupboard",
    responses(
        (status = 200, description = "System health check", body = ApiResponse<serde_json::Value>)
    )
)]
async fn cupboard_system_health(
    State(app_state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Cupboard system health check requested");

    let mut health_status = serde_json::json!({
        "overall_health": "healthy",
        "checks": {},
        "timestamp": chrono::Utc::now()
    });

    // Database health check
    match app_state.database.health_check().await {
        Ok(_) => {
            health_status["checks"]["database"] = serde_json::json!({
                "status": "healthy",
                "response_time_ms": "unknown" // TODO: Measure response time
            });
        }
        Err(e) => {
            health_status["checks"]["database"] = serde_json::json!({
                "status": "unhealthy",
                "error": format!("Database health check failed: {}", e)
            });
            health_status["overall_health"] = serde_json::Value::String("degraded".to_string());
        }
    }

    // TODO: Add health checks for other services (Redis, external APIs, etc.)
    health_status["checks"]["redis"] = serde_json::json!({
        "status": "unknown",
        "message": "Redis health check not implemented"
    });

    negotiate_response(
        ApiResponse::success(health_status),
        &headers,
        "/cupboard/system/health",
    )
}

/// Get detailed system metrics for monitoring
#[utoipa::path(
    get,
    path = "/system/metrics",
    tag = "cupboard",
    responses(
        (status = 200, description = "System metrics", body = ApiResponse<serde_json::Value>)
    )
)]
async fn cupboard_system_metrics(
    State(app_state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Cupboard system metrics requested");

    match app_state.database.get_stats().await {
        Ok(stats) => {
            let metrics = serde_json::json!({
                "application": {
                    "uptime_seconds": "unknown", // TODO: Track application uptime
                    "requests_total": "unknown", // TODO: Track total requests
                    "requests_per_second": "unknown",
                    "active_connections": "unknown"
                },
                "database": {
                    "connection_pool_size": "unknown", // TODO: Get from sqlx pool
                    "active_connections": "unknown",
                    "idle_connections": "unknown",
                    "query_duration_avg": "unknown",
                    "stats": stats
                },
                "memory": {
                    "heap_used_bytes": "unknown", // TODO: Add memory monitoring
                    "heap_total_bytes": "unknown",
                    "rss_bytes": "unknown"
                },
                "business_metrics": {
                    "total_codebases": stats.get("total_codebases").unwrap_or(&0),
                    "active_runs": stats.get("active_runs").unwrap_or(&0),
                    "queue_size": stats.get("queue_size").unwrap_or(&0),
                    "recent_successful_runs": stats.get("recent_successful_runs").unwrap_or(&0)
                },
                "collection_interval_seconds": 60,
                "timestamp": chrono::Utc::now()
            });

            negotiate_response(
                ApiResponse::success(metrics),
                &headers,
                "/cupboard/system/metrics",
            )
        }
        Err(e) => {
            debug!("Failed to get system metrics: {}", e);
            let error_response = ApiResponse::<serde_json::Value> {
                data: None,
                error: Some("metrics_error".to_string()),
                reason: Some(format!("Failed to retrieve system metrics: {}", e)),
                details: None,
                pagination: None,
            };

            negotiate_response(error_response, &headers, "/cupboard/system/metrics")
        }
    }
}

/// Get system configuration (admin view)
#[utoipa::path(
    get,
    path = "/system/config",
    tag = "cupboard",
    responses(
        (status = 200, description = "System configuration", body = ApiResponse<serde_json::Value>)
    )
)]
async fn cupboard_system_config(
    State(_app_state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Cupboard system config requested");

    // Return non-sensitive configuration information
    let config = serde_json::json!({
        "application": {
            "name": "janitor-site",
            "version": env!("CARGO_PKG_VERSION"),
            "build_time": option_env!("BUILD_TIME").unwrap_or("unknown"),
            "git_revision": option_env!("GIT_REVISION").unwrap_or("unknown")
        },
        "features": {
            "cupboard_enabled": true,
            "admin_api_enabled": true,
            "worker_monitoring": true,
            "system_metrics": true
        },
        "limits": {
            "max_concurrent_requests": "unknown", // TODO: Get from server config
            "max_database_connections": "unknown",
            "request_timeout_seconds": 30
        },
        "endpoints": {
            "runner_service": "unknown", // TODO: Get from config
            "publisher_service": "unknown",
            "differ_service": "unknown"
        },
        "timestamp": chrono::Utc::now()
    });

    negotiate_response(
        ApiResponse::success(config),
        &headers,
        "/cupboard/system/config",
    )
}

// ============================================================================
// Metrics and Reporting
// ============================================================================

/// Get available reports overview
#[utoipa::path(
    get,
    path = "/reports",
    tag = "cupboard",
    responses(
        (status = 200, description = "Available reports", body = ApiResponse<serde_json::Value>)
    )
)]
async fn cupboard_reports(
    State(_app_state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Cupboard reports list requested");

    let reports = serde_json::json!({
        "available_reports": [
            {
                "name": "performance",
                "title": "Performance Report",
                "description": "System and application performance metrics",
                "endpoint": "/cupboard/reports/performance"
            },
            {
                "name": "success-rates",
                "title": "Success Rates Report",
                "description": "Campaign and run success rate analysis",
                "endpoint": "/cupboard/reports/success-rates"
            },
            {
                "name": "trending",
                "title": "Trending Analysis",
                "description": "Trending data for campaigns and success rates",
                "endpoint": "/cupboard/reports/trending"
            }
        ],
        "report_generation": {
            "formats": ["json", "csv", "html"],
            "time_ranges": ["1h", "24h", "7d", "30d", "custom"],
            "scheduling": "not_implemented"
        },
        "timestamp": chrono::Utc::now()
    });

    negotiate_response(ApiResponse::success(reports), &headers, "/cupboard/reports")
}

/// Generate performance report
#[utoipa::path(
    get,
    path = "/reports/performance",
    tag = "cupboard",
    responses(
        (status = 200, description = "Performance report", body = ApiResponse<serde_json::Value>)
    )
)]
async fn cupboard_performance_report(
    State(_app_state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Cupboard performance report requested");

    // TODO: Generate actual performance report from database
    let performance_report = serde_json::json!({
        "report_type": "performance",
        "time_range": "24h",
        "metrics": {
            "response_times": {
                "avg_ms": "unknown",
                "p50_ms": "unknown",
                "p95_ms": "unknown",
                "p99_ms": "unknown"
            },
            "throughput": {
                "requests_per_second": "unknown",
                "runs_per_hour": "unknown"
            },
            "errors": {
                "error_rate_percent": "unknown",
                "total_errors": "unknown"
            }
        },
        "trends": {
            "response_time_trend": "stable",
            "throughput_trend": "unknown",
            "error_trend": "unknown"
        },
        "status": "not_implemented",
        "message": "Performance reporting requires database queries and metrics collection",
        "generated_at": chrono::Utc::now()
    });

    negotiate_response(
        ApiResponse::success(performance_report),
        &headers,
        "/cupboard/reports/performance",
    )
}

/// Generate success rates report
#[utoipa::path(
    get,
    path = "/reports/success-rates",
    tag = "cupboard",
    responses(
        (status = 200, description = "Success rates report", body = ApiResponse<serde_json::Value>)
    )
)]
async fn cupboard_success_rates(
    State(_app_state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Cupboard success rates report requested");

    // TODO: Calculate actual success rates from database
    let success_report = serde_json::json!({
        "report_type": "success_rates",
        "time_range": "7d",
        "overall_success_rate": "unknown",
        "by_campaign": {},
        "by_time_period": {
            "last_24h": "unknown",
            "last_7d": "unknown",
            "last_30d": "unknown"
        },
        "failure_analysis": {
            "top_failure_reasons": [],
            "transient_failures": "unknown",
            "permanent_failures": "unknown"
        },
        "trends": {
            "success_rate_trend": "unknown",
            "volume_trend": "unknown"
        },
        "status": "not_implemented",
        "message": "Success rate reporting requires database queries for run outcomes",
        "generated_at": chrono::Utc::now()
    });

    negotiate_response(
        ApiResponse::success(success_report),
        &headers,
        "/cupboard/reports/success-rates",
    )
}

/// Generate trending analysis report
#[utoipa::path(
    get,
    path = "/reports/trending",
    tag = "cupboard",
    responses(
        (status = 200, description = "Trending analysis report", body = ApiResponse<serde_json::Value>)
    )
)]
async fn cupboard_trending_report(
    State(_app_state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Cupboard trending report requested");

    // TODO: Generate actual trending analysis
    let trending_report = serde_json::json!({
        "report_type": "trending",
        "analysis_period": "30d",
        "campaign_trends": {
            "most_active_campaigns": [],
            "improving_campaigns": [],
            "declining_campaigns": []
        },
        "package_trends": {
            "most_processed_packages": [],
            "successful_packages": [],
            "problematic_packages": []
        },
        "system_trends": {
            "processing_volume": "unknown",
            "queue_length_trend": "unknown",
            "worker_efficiency_trend": "unknown"
        },
        "predictions": {
            "next_week_volume": "unknown",
            "resource_requirements": "unknown"
        },
        "status": "not_implemented",
        "message": "Trending analysis requires time-series data and statistical calculations",
        "generated_at": chrono::Utc::now()
    });

    negotiate_response(
        ApiResponse::success(trending_report),
        &headers,
        "/cupboard/reports/trending",
    )
}

// ============================================================================
// Phase 3.7.2: Queue Management Endpoints
// ============================================================================

/// Get queue status for cupboard management
#[utoipa::path(
    get,
    path = "/queue",
    tag = "cupboard",
    params(CupboardQuery),
    responses(
        (status = 200, description = "Queue status for admin", body = ApiResponse<serde_json::Value>)
    )
)]
async fn cupboard_queue_status(
    State(app_state): State<Arc<AppState>>,
    Query(query): Query<CupboardQuery>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Cupboard queue status requested: {:?}", query);

    match app_state.database.get_stats().await {
        Ok(stats) => {
            let queue_status = serde_json::json!({
                "queue_overview": {
                    "total_items": stats.get("queue_size").unwrap_or(&0),
                    "processing_items": stats.get("active_runs").unwrap_or(&0),
                    "waiting_items": "unknown", // TODO: Calculate waiting items
                    "failed_items": "unknown"
                },
                "queue_health": {
                    "processing_rate": "unknown", // TODO: Calculate processing rate
                    "average_wait_time": "unknown",
                    "oldest_item_age": "unknown"
                },
                "worker_allocation": {
                    "busy_workers": "unknown", // TODO: Get from runner service
                    "idle_workers": "unknown",
                    "total_workers": "unknown"
                },
                "priority_distribution": {
                    "high_priority": "unknown",
                    "normal_priority": "unknown",
                    "low_priority": "unknown"
                },
                "campaign_breakdown": "unknown", // TODO: Group by campaign
                "management_actions": {
                    "pause_available": true,
                    "resume_available": true,
                    "bulk_operations_available": true
                },
                "query_parameters": {
                    "limit": query.limit.unwrap_or(50),
                    "offset": query.offset.unwrap_or(0),
                    "status_filter": query.status
                },
                "timestamp": chrono::Utc::now()
            });

            negotiate_response(
                ApiResponse::success(queue_status),
                &headers,
                "/cupboard/queue",
            )
        }
        Err(e) => {
            debug!("Failed to get queue status: {}", e);
            let error_response = ApiResponse::<serde_json::Value> {
                data: None,
                error: Some("queue_error".to_string()),
                reason: Some(format!("Failed to retrieve queue status: {}", e)),
                details: None,
                pagination: None,
            };

            negotiate_response(error_response, &headers, "/cupboard/queue")
        }
    }
}

/// Browse queue items with detailed information
#[utoipa::path(
    get,
    path = "/queue/browse",
    tag = "cupboard",
    params(CupboardQuery),
    responses(
        (status = 200, description = "Queue items for browsing", body = ApiResponse<serde_json::Value>)
    )
)]
async fn cupboard_queue_browse(
    State(_app_state): State<Arc<AppState>>,
    Query(query): Query<CupboardQuery>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Cupboard queue browse requested: {:?}", query);

    // TODO: Implement actual queue browsing from database
    let queue_items = serde_json::json!({
        "items": [],
        "pagination": {
            "limit": query.limit.unwrap_or(50),
            "offset": query.offset.unwrap_or(0),
            "total": 0,
            "has_more": false
        },
        "filtering": {
            "status_filter": query.status,
            "available_statuses": ["pending", "running", "failed", "completed"]
        },
        "status": "not_implemented",
        "message": "Queue browsing requires database queries for queue items",
        "timestamp": chrono::Utc::now()
    });

    negotiate_response(
        ApiResponse::success(queue_items),
        &headers,
        "/cupboard/queue/browse",
    )
}

/// Queue management operations interface
#[utoipa::path(
    get,
    path = "/queue/manage",
    tag = "cupboard",
    responses(
        (status = 200, description = "Queue management options", body = ApiResponse<serde_json::Value>)
    )
)]
async fn cupboard_queue_manage(
    State(_app_state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Cupboard queue management interface requested");

    let management_options = serde_json::json!({
        "available_operations": [
            {
                "operation": "pause_queue",
                "description": "Pause queue processing",
                "endpoint": "/cupboard/jobs/pause",
                "method": "POST"
            },
            {
                "operation": "resume_queue",
                "description": "Resume queue processing",
                "endpoint": "/cupboard/jobs/resume",
                "method": "POST"
            },
            {
                "operation": "bulk_cancel",
                "description": "Cancel multiple jobs",
                "endpoint": "/cupboard/queue/bulk-operations",
                "method": "POST"
            },
            {
                "operation": "requeue_failed",
                "description": "Requeue failed items",
                "endpoint": "/cupboard/jobs/requeue",
                "method": "POST"
            }
        ],
        "current_status": {
            "queue_processing": "unknown", // TODO: Get actual status
            "admin_controls": "enabled"
        },
        "bulk_operation_limits": {
            "max_batch_size": 1000,
            "rate_limit": "10 operations per minute"
        },
        "timestamp": chrono::Utc::now()
    });

    negotiate_response(
        ApiResponse::success(management_options),
        &headers,
        "/cupboard/queue/manage",
    )
}

/// Execute bulk operations on queue items
#[utoipa::path(
    post,
    path = "/queue/bulk-operations",
    tag = "cupboard",
    params(BulkOperationQuery),
    responses(
        (status = 200, description = "Bulk operation initiated", body = ApiResponse<serde_json::Value>)
    )
)]
async fn cupboard_queue_bulk_operations(
    State(_app_state): State<Arc<AppState>>,
    Query(operation): Query<BulkOperationQuery>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Cupboard bulk operation requested: {:?}", operation);

    // TODO: Implement actual bulk operations
    let result = serde_json::json!({
        "operation": operation.operation,
        "status": "not_implemented",
        "message": "Bulk operations require integration with runner service",
        "parameters": {
            "target": operation.target,
            "batch_size": operation.batch_size.unwrap_or(100),
            "requester": operation.requester
        },
        "estimated_affected_items": 0,
        "execution_time_estimate": "unknown",
        "initiated_at": chrono::Utc::now()
    });

    negotiate_response(
        ApiResponse::success(result),
        &headers,
        "/cupboard/queue/bulk-operations",
    )
}

// ============================================================================
// Job Control Operations
// ============================================================================

/// Pause job processing
#[utoipa::path(
    post,
    path = "/jobs/pause",
    tag = "cupboard",
    responses(
        (status = 200, description = "Job processing paused", body = ApiResponse<serde_json::Value>)
    )
)]
async fn cupboard_jobs_pause(
    State(_app_state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Cupboard jobs pause requested");

    // TODO: Implement actual job pause via runner service
    let result = serde_json::json!({
        "action": "pause_jobs",
        "status": "not_implemented",
        "message": "Job control requires integration with runner service",
        "timestamp": chrono::Utc::now()
    });

    negotiate_response(
        ApiResponse::success(result),
        &headers,
        "/cupboard/jobs/pause",
    )
}

/// Resume job processing
#[utoipa::path(
    post,
    path = "/jobs/resume",
    tag = "cupboard",
    responses(
        (status = 200, description = "Job processing resumed", body = ApiResponse<serde_json::Value>)
    )
)]
async fn cupboard_jobs_resume(
    State(_app_state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Cupboard jobs resume requested");

    // TODO: Implement actual job resume via runner service
    let result = serde_json::json!({
        "action": "resume_jobs",
        "status": "not_implemented",
        "message": "Job control requires integration with runner service",
        "timestamp": chrono::Utc::now()
    });

    negotiate_response(
        ApiResponse::success(result),
        &headers,
        "/cupboard/jobs/resume",
    )
}

/// Cancel specific jobs
#[utoipa::path(
    post,
    path = "/jobs/cancel",
    tag = "cupboard",
    responses(
        (status = 200, description = "Job cancellation initiated", body = ApiResponse<serde_json::Value>)
    )
)]
async fn cupboard_jobs_cancel(
    State(_app_state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Cupboard jobs cancel requested");

    // TODO: Implement actual job cancellation via runner service
    let result = serde_json::json!({
        "action": "cancel_jobs",
        "status": "not_implemented",
        "message": "Job control requires integration with runner service",
        "timestamp": chrono::Utc::now()
    });

    negotiate_response(
        ApiResponse::success(result),
        &headers,
        "/cupboard/jobs/cancel",
    )
}

/// Requeue failed jobs
#[utoipa::path(
    post,
    path = "/jobs/requeue",
    tag = "cupboard",
    responses(
        (status = 200, description = "Job requeue initiated", body = ApiResponse<serde_json::Value>)
    )
)]
async fn cupboard_jobs_requeue(
    State(_app_state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Cupboard jobs requeue requested");

    // TODO: Implement actual job requeue via runner service
    let result = serde_json::json!({
        "action": "requeue_jobs",
        "status": "not_implemented",
        "message": "Job control requires integration with runner service",
        "timestamp": chrono::Utc::now()
    });

    negotiate_response(
        ApiResponse::success(result),
        &headers,
        "/cupboard/jobs/requeue",
    )
}

// ============================================================================
// Advanced Run Management for Admins
// ============================================================================

/// Get runs with admin-level details
#[utoipa::path(
    get,
    path = "/runs",
    tag = "cupboard",
    params(CupboardQuery),
    responses(
        (status = 200, description = "Admin view of runs", body = ApiResponse<serde_json::Value>)
    )
)]
async fn cupboard_runs(
    State(_app_state): State<Arc<AppState>>,
    Query(query): Query<CupboardQuery>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Cupboard runs requested: {:?}", query);

    // TODO: Implement admin-level runs query with enhanced details
    let runs_data = serde_json::json!({
        "runs": [],
        "summary": {
            "total_runs": 0,
            "active_runs": 0,
            "failed_runs": 0,
            "completed_runs": 0
        },
        "admin_actions": [
            "force_finish",
            "requeue",
            "cancel",
            "view_logs"
        ],
        "status": "not_implemented",
        "message": "Admin runs view requires database queries for run details",
        "query_parameters": {
            "limit": query.limit.unwrap_or(50),
            "offset": query.offset.unwrap_or(0),
            "status_filter": query.status,
            "detailed": query.detailed.unwrap_or(true)
        },
        "timestamp": chrono::Utc::now()
    });

    negotiate_response(ApiResponse::success(runs_data), &headers, "/cupboard/runs")
}

/// Get failed runs for troubleshooting
#[utoipa::path(
    get,
    path = "/runs/failed",
    tag = "cupboard",
    params(CupboardQuery),
    responses(
        (status = 200, description = "Failed runs for analysis", body = ApiResponse<serde_json::Value>)
    )
)]
async fn cupboard_failed_runs(
    State(_app_state): State<Arc<AppState>>,
    Query(query): Query<CupboardQuery>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Cupboard failed runs requested: {:?}", query);

    // TODO: Query database for failed runs with failure reasons
    let failed_runs = serde_json::json!({
        "failed_runs": [],
        "failure_analysis": {
            "total_failed": 0,
            "failure_categories": {},
            "recent_failures": 0,
            "retry_candidates": 0
        },
        "troubleshooting": {
            "common_failure_patterns": [],
            "suggested_actions": []
        },
        "status": "not_implemented",
        "message": "Failed runs analysis requires database queries for failure data",
        "timestamp": chrono::Utc::now()
    });

    negotiate_response(
        ApiResponse::success(failed_runs),
        &headers,
        "/cupboard/runs/failed",
    )
}

/// Get stuck runs that need intervention
#[utoipa::path(
    get,
    path = "/runs/stuck",
    tag = "cupboard",
    params(CupboardQuery),
    responses(
        (status = 200, description = "Stuck runs needing intervention", body = ApiResponse<serde_json::Value>)
    )
)]
async fn cupboard_stuck_runs(
    State(_app_state): State<Arc<AppState>>,
    Query(query): Query<CupboardQuery>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Cupboard stuck runs requested: {:?}", query);

    // TODO: Identify runs that have been running too long or are stuck
    let stuck_runs = serde_json::json!({
        "stuck_runs": [],
        "detection_criteria": {
            "max_runtime_hours": 24,
            "no_progress_hours": 4,
            "heartbeat_timeout_hours": 1
        },
        "intervention_options": [
            "force_finish",
            "restart",
            "cancel_and_requeue"
        ],
        "status": "not_implemented",
        "message": "Stuck run detection requires runtime and heartbeat analysis",
        "timestamp": chrono::Utc::now()
    });

    negotiate_response(
        ApiResponse::success(stuck_runs),
        &headers,
        "/cupboard/runs/stuck",
    )
}

/// Get detailed run information for admin analysis
#[utoipa::path(
    get,
    path = "/runs/{run_id}/details",
    tag = "cupboard",
    params(
        ("run_id" = String, Path, description = "Run identifier")
    ),
    responses(
        (status = 200, description = "Detailed admin view of run", body = ApiResponse<serde_json::Value>),
        (status = 404, description = "Run not found")
    )
)]
async fn cupboard_run_details(
    State(_app_state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Cupboard run details requested for: {}", run_id);

    // TODO: Get comprehensive run details including logs, worker info, etc.
    let run_details = serde_json::json!({
        "run_id": run_id,
        "admin_details": {
            "worker_assigned": "unknown",
            "start_time": "unknown",
            "estimated_completion": "unknown",
            "resource_usage": "unknown"
        },
        "debugging_info": {
            "log_files": [],
            "error_traces": [],
            "performance_metrics": {}
        },
        "admin_actions": {
            "can_force_finish": true,
            "can_restart": true,
            "can_cancel": true
        },
        "status": "not_implemented",
        "message": "Run details require database and log system integration",
        "timestamp": chrono::Utc::now()
    });

    negotiate_response(
        ApiResponse::success(run_details),
        &headers,
        &format!("/cupboard/runs/{}/details", run_id),
    )
}

/// Force finish a run (admin emergency action)
#[utoipa::path(
    post,
    path = "/runs/{run_id}/force-finish",
    tag = "cupboard",
    params(
        ("run_id" = String, Path, description = "Run identifier")
    ),
    responses(
        (status = 200, description = "Run force finish initiated", body = ApiResponse<serde_json::Value>),
        (status = 404, description = "Run not found")
    )
)]
async fn cupboard_run_force_finish(
    State(_app_state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Cupboard run force finish requested for: {}", run_id);

    // TODO: Implement force finish via runner service
    let result = serde_json::json!({
        "run_id": run_id,
        "action": "force_finish",
        "status": "not_implemented",
        "message": "Force finish requires integration with runner service",
        "warning": "This is an emergency action that may result in data loss",
        "timestamp": chrono::Utc::now()
    });

    negotiate_response(
        ApiResponse::success(result),
        &headers,
        &format!("/cupboard/runs/{}/force-finish", run_id),
    )
}

// ============================================================================
// Publishing Oversight
// ============================================================================

/// Get publishing overview for admin monitoring
#[utoipa::path(
    get,
    path = "/publish",
    tag = "cupboard",
    responses(
        (status = 200, description = "Publishing system overview", body = ApiResponse<serde_json::Value>)
    )
)]
async fn cupboard_publish_overview(
    State(_app_state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Cupboard publish overview requested");

    // TODO: Get publishing statistics and status
    let publish_overview = serde_json::json!({
        "overview": {
            "publishing_enabled": true,
            "auto_publish_enabled": "unknown",
            "total_published": "unknown",
            "pending_publish": "unknown"
        },
        "rate_limiting": {
            "current_rate": "unknown",
            "rate_limit": "unknown",
            "next_available": "unknown"
        },
        "recent_activity": {
            "last_24h_published": "unknown",
            "success_rate": "unknown",
            "failed_publishes": "unknown"
        },
        "monitoring": {
            "forge_connectivity": "unknown",
            "merge_proposal_status": "unknown"
        },
        "status": "not_implemented",
        "message": "Publishing oversight requires integration with publisher service",
        "timestamp": chrono::Utc::now()
    });

    negotiate_response(
        ApiResponse::success(publish_overview),
        &headers,
        "/cupboard/publish",
    )
}

/// Get pending publishing items
#[utoipa::path(
    get,
    path = "/publish/pending",
    tag = "cupboard",
    params(CupboardQuery),
    responses(
        (status = 200, description = "Pending publishing items", body = ApiResponse<serde_json::Value>)
    )
)]
async fn cupboard_publish_pending(
    State(_app_state): State<Arc<AppState>>,
    Query(query): Query<CupboardQuery>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Cupboard pending publishes requested: {:?}", query);

    // TODO: Query database for pending publishing items
    let pending_publishes = serde_json::json!({
        "pending_items": [],
        "summary": {
            "total_pending": 0,
            "awaiting_review": 0,
            "rate_limited": 0,
            "ready_to_publish": 0
        },
        "admin_actions": [
            "force_publish",
            "skip_rate_limit",
            "bulk_approve"
        ],
        "status": "not_implemented",
        "message": "Pending publishes require database queries for publishing queue",
        "timestamp": chrono::Utc::now()
    });

    negotiate_response(
        ApiResponse::success(pending_publishes),
        &headers,
        "/cupboard/publish/pending",
    )
}

/// Get rate limiting status and controls
#[utoipa::path(
    get,
    path = "/publish/rate-limits",
    tag = "cupboard",
    responses(
        (status = 200, description = "Publishing rate limits", body = ApiResponse<serde_json::Value>)
    )
)]
async fn cupboard_publish_rate_limits(
    State(_app_state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Cupboard publish rate limits requested");

    // TODO: Get rate limiting configuration and status
    let rate_limits = serde_json::json!({
        "rate_limits": {
            "github": {
                "limit": "unknown",
                "remaining": "unknown",
                "reset_time": "unknown"
            },
            "gitlab": {
                "limit": "unknown",
                "remaining": "unknown",
                "reset_time": "unknown"
            }
        },
        "configuration": {
            "respect_forge_limits": true,
            "custom_rate_limit": "unknown",
            "burst_allowance": "unknown"
        },
        "admin_controls": {
            "can_override_limits": true,
            "can_pause_publishing": true,
            "can_adjust_rates": true
        },
        "status": "not_implemented",
        "message": "Rate limiting requires integration with publisher service",
        "timestamp": chrono::Utc::now()
    });

    negotiate_response(
        ApiResponse::success(rate_limits),
        &headers,
        "/cupboard/publish/rate-limits",
    )
}

/// Get publishing history and analytics
#[utoipa::path(
    get,
    path = "/publish/history",
    tag = "cupboard",
    params(CupboardQuery),
    responses(
        (status = 200, description = "Publishing history", body = ApiResponse<serde_json::Value>)
    )
)]
async fn cupboard_publish_history(
    State(_app_state): State<Arc<AppState>>,
    Query(query): Query<CupboardQuery>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Cupboard publish history requested: {:?}", query);

    // TODO: Query database for publishing history
    let publish_history = serde_json::json!({
        "history": [],
        "analytics": {
            "total_published": 0,
            "success_rate": "unknown",
            "average_time_to_publish": "unknown",
            "most_active_campaigns": []
        },
        "trends": {
            "publishing_volume": "unknown",
            "success_rate_trend": "unknown",
            "forge_performance": {}
        },
        "status": "not_implemented",
        "message": "Publishing history requires database queries for publish records",
        "query_parameters": {
            "limit": query.limit.unwrap_or(50),
            "offset": query.offset.unwrap_or(0)
        },
        "timestamp": chrono::Utc::now()
    });

    negotiate_response(
        ApiResponse::success(publish_history),
        &headers,
        "/cupboard/publish/history",
    )
}

// ============================================================================
// Review System Management
// ============================================================================

/// Get review system overview
#[utoipa::path(
    get,
    path = "/reviews",
    tag = "cupboard",
    responses(
        (status = 200, description = "Review system overview", body = ApiResponse<serde_json::Value>)
    )
)]
async fn cupboard_reviews(
    State(_app_state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Cupboard reviews overview requested");

    // TODO: Get review system statistics
    let reviews_overview = serde_json::json!({
        "system_status": {
            "reviews_enabled": true,
            "auto_review_enabled": "unknown",
            "manual_review_required": "unknown"
        },
        "statistics": {
            "total_reviews": "unknown",
            "pending_reviews": "unknown",
            "approved_reviews": "unknown",
            "rejected_reviews": "unknown"
        },
        "reviewers": {
            "active_reviewers": "unknown",
            "review_load_distribution": {}
        },
        "performance": {
            "average_review_time": "unknown",
            "review_throughput": "unknown"
        },
        "status": "not_implemented",
        "message": "Review system requires database queries for review data",
        "timestamp": chrono::Utc::now()
    });

    negotiate_response(
        ApiResponse::success(reviews_overview),
        &headers,
        "/cupboard/reviews",
    )
}

/// Get pending reviews needing attention
#[utoipa::path(
    get,
    path = "/reviews/pending",
    tag = "cupboard",
    params(CupboardQuery),
    responses(
        (status = 200, description = "Pending reviews", body = ApiResponse<serde_json::Value>)
    )
)]
async fn cupboard_reviews_pending(
    State(_app_state): State<Arc<AppState>>,
    Query(query): Query<CupboardQuery>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Cupboard pending reviews requested: {:?}", query);

    // TODO: Query database for pending reviews
    let pending_reviews = serde_json::json!({
        "pending_reviews": [],
        "summary": {
            "total_pending": 0,
            "overdue_reviews": 0,
            "high_priority": 0,
            "auto_reviewable": 0
        },
        "admin_actions": [
            "bulk_approve",
            "assign_reviewer",
            "escalate_review"
        ],
        "status": "not_implemented",
        "message": "Pending reviews require database queries for review queue",
        "timestamp": chrono::Utc::now()
    });

    negotiate_response(
        ApiResponse::success(pending_reviews),
        &headers,
        "/cupboard/reviews/pending",
    )
}

/// Assign reviews to reviewers
#[utoipa::path(
    post,
    path = "/reviews/assign",
    tag = "cupboard",
    responses(
        (status = 200, description = "Review assignment completed", body = ApiResponse<serde_json::Value>)
    )
)]
async fn cupboard_reviews_assign(
    State(_app_state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Cupboard review assignment requested");

    // TODO: Implement review assignment logic
    let assignment_result = serde_json::json!({
        "action": "assign_reviews",
        "status": "not_implemented",
        "message": "Review assignment requires reviewer management system",
        "timestamp": chrono::Utc::now()
    });

    negotiate_response(
        ApiResponse::success(assignment_result),
        &headers,
        "/cupboard/reviews/assign",
    )
}

/// Get review verdicts and outcomes
#[utoipa::path(
    get,
    path = "/reviews/verdicts",
    tag = "cupboard",
    params(CupboardQuery),
    responses(
        (status = 200, description = "Review verdicts", body = ApiResponse<serde_json::Value>)
    )
)]
async fn cupboard_reviews_verdicts(
    State(_app_state): State<Arc<AppState>>,
    Query(query): Query<CupboardQuery>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Cupboard review verdicts requested: {:?}", query);

    // TODO: Query database for review verdicts and outcomes
    let verdicts = serde_json::json!({
        "verdicts": [],
        "summary": {
            "total_verdicts": 0,
            "approved_count": 0,
            "rejected_count": 0,
            "pending_count": 0
        },
        "analytics": {
            "approval_rate": "unknown",
            "average_review_time": "unknown",
            "reviewer_performance": {}
        },
        "status": "not_implemented",
        "message": "Review verdicts require database queries for verdict data",
        "timestamp": chrono::Utc::now()
    });

    negotiate_response(
        ApiResponse::success(verdicts),
        &headers,
        "/cupboard/reviews/verdicts",
    )
}
