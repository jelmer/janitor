use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::{get, post, put, delete},
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

