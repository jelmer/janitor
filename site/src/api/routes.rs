use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::{get, post, put, delete},
    Router,
};
use std::sync::Arc;
use tracing::{debug, info};

use crate::{
    app::AppState,
    auth::{require_admin, require_login, require_qa_reviewer, UserContext, OptionalUser},
};
use super::{
    content_negotiation::{negotiate_response, ContentType, NegotiatedResponse},
    middleware::{content_negotiation_middleware, logging_middleware, metrics_middleware, cors_middleware},
    types::{
        ApiResponse, ApiResult, CommonQuery, PaginationParams,
        RunInfo, QueueStatus, MergeProposalInfo, PublishRequest, RescheduleRequest, MassRescheduleRequest,
    },
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
async fn health_check(
    State(app_state): State<Arc<AppState>>,
) -> ApiResult<Json<ApiResponse<serde_json::Value>>> {
    debug!("Health check requested");
    
    // Check database connectivity
    if let Err(e) = app_state.database.health_check().await {
        return Ok(Json(ApiResponse::error(
            "database_error".to_string(),
            Some(format!("Database health check failed: {}", e)),
        )));
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
async fn get_active_runs(
    State(_app_state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<CommonQuery>,
) -> impl axum::response::IntoResponse {
    debug!("Active runs requested with query: {:?}", query);
    
    // TODO: Implement actual active runs retrieval
    let runs: Vec<RunInfo> = vec![];
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
    
    let mut response = negotiate_response(error_response, &headers, "/api/active-runs/{id}");
    // Set status to 404 - this would be handled by the error system in real implementation
    response
}

/// Get run logs
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
async fn get_run_log_file(
    State(_app_state): State<Arc<AppState>>,
    Path((run_id, filename)): Path<(String, String)>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Run log file {}/{} requested", run_id, filename);
    
    // TODO: Implement actual log file retrieval
    // For log files, we typically want to return plain text or binary content
    let content_type = super::content_negotiation::negotiate_content_type(&headers, &filename);
    
    match content_type {
        ContentType::TextPlain | ContentType::Html => {
            NegotiatedResponse::new("Log file content would be here".to_string(), ContentType::TextPlain)
        }
        _ => {
            NegotiatedResponse::new(
                ApiResponse::success("Log file content".to_string()),
                ContentType::Json,
            )
        }
    }
}


/// Get VCS diff for run
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
    
    match content_type {
        ContentType::TextDiff => {
            NegotiatedResponse::new(diff_content.to_string(), ContentType::TextDiff)
        }
        _ => {
            NegotiatedResponse::new(
                ApiResponse::success(serde_json::json!({
                    "run_id": run_id,
                    "diff": diff_content
                })),
                content_type,
            )
        }
    }
}

/// Get debdiff for run
async fn get_run_debdiff(
    State(_app_state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Debdiff for run {} requested", run_id);
    
    // TODO: Implement actual debdiff retrieval
    let content_type = super::content_negotiation::negotiate_content_type(&headers, "/run/{id}/debdiff");
    
    match content_type {
        ContentType::TextDiff => {
            NegotiatedResponse::new("debdiff content here".to_string(), ContentType::TextDiff)
        }
        _ => {
            NegotiatedResponse::new(
                ApiResponse::success(serde_json::json!({
                    "run_id": run_id,
                    "debdiff": "debdiff content here"
                })),
                content_type,
            )
        }
    }
}

/// Get diffoscope output for run
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
async fn get_merge_proposals(
    State(_app_state): State<Arc<AppState>>,
    Query(query): Query<CommonQuery>,
    headers: HeaderMap,
) -> impl axum::response::IntoResponse {
    debug!("Merge proposals requested");
    
    // TODO: Implement actual merge proposal retrieval
    let proposals: Vec<MergeProposalInfo> = vec![];
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

