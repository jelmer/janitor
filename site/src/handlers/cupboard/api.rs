use axum::{
    extract::{Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Json, Response},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{app::AppState, auth::UserContext};

use super::{log_admin_action, AdminUser, Permission};

/// Admin API endpoint for system status
pub async fn admin_system_status(
    State(state): State<AppState>,
    user_ctx: UserContext,
    headers: header::HeaderMap,
) -> Response {
    let admin_user = match AdminUser::from_user_context(&user_ctx) {
        Some(admin) => admin,
        None => return StatusCode::FORBIDDEN.into_response(),
    };

    if !admin_user.has_permission(&Permission::ViewSystemMetrics) {
        return StatusCode::FORBIDDEN.into_response();
    }

    // Fetch comprehensive system status
    match fetch_comprehensive_system_status(&state).await {
        Ok(status) => Json(status).into_response(),
        Err(e) => {
            tracing::error!("Failed to fetch system status: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// Admin API endpoint for system configuration
pub async fn admin_system_config(State(state): State<AppState>, user_ctx: UserContext) -> Response {
    let admin_user = match AdminUser::from_user_context(&user_ctx) {
        Some(admin) => admin,
        None => return StatusCode::FORBIDDEN.into_response(),
    };

    if !admin_user.has_permission(&Permission::ViewSystemMetrics) {
        return StatusCode::FORBIDDEN.into_response();
    }

    // Return system configuration (sanitized for security)
    let config = serde_json::json!({
        "campaigns": state.config.campaigns.keys().collect::<Vec<_>>(),
        "database_url": "redacted",
        "external_url": state.config.external_url(),
        "features": {
            "authentication": true,
            "real_time": true, // realtime is always available
            "webhooks": true,
        },
        "version": env!("CARGO_PKG_VERSION"),
    });

    Json(config).into_response()
}

/// Admin API endpoint for system metrics
pub async fn admin_system_metrics(
    State(state): State<AppState>,
    user_ctx: UserContext,
) -> Response {
    let admin_user = match AdminUser::from_user_context(&user_ctx) {
        Some(admin) => admin,
        None => return StatusCode::FORBIDDEN.into_response(),
    };

    if !admin_user.has_permission(&Permission::ViewSystemMetrics) {
        return StatusCode::FORBIDDEN.into_response();
    }

    // Fetch system metrics
    match fetch_system_metrics(&state).await {
        Ok(metrics) => Json(metrics).into_response(),
        Err(e) => {
            tracing::error!("Failed to fetch system metrics: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BulkOperationRequest {
    pub operation: String,
    pub targets: Vec<String>,
    pub parameters: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct BulkOperationResult {
    pub operation: String,
    pub total_targets: usize,
    pub successful: usize,
    pub failed: usize,
    pub errors: Vec<String>,
    pub completed_at: DateTime<Utc>,
}

/// Admin API endpoint for bulk operations
pub async fn admin_bulk_operation(
    State(state): State<AppState>,
    user_ctx: UserContext,
    headers: header::HeaderMap,
    Json(request): Json<BulkOperationRequest>,
) -> Response {
    let admin_user = match AdminUser::from_user_context(&user_ctx) {
        Some(admin) => admin,
        None => return StatusCode::FORBIDDEN.into_response(),
    };

    // Check permissions based on operation type
    let required_permission = match request.operation.as_str() {
        "reschedule" | "cancel" | "requeue" => Permission::BulkQueueOperations,
        "approve_reviews" | "reject_reviews" => Permission::BulkReviewActions,
        "emergency_stop" | "rate_limit" => Permission::EmergencyPublishControls,
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };

    if !admin_user.has_permission(&required_permission) {
        return StatusCode::FORBIDDEN.into_response();
    }

    // Extract IP and User-Agent for audit logging
    let ip_address = headers
        .get("x-forwarded-for")
        .or(headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

    // Log the bulk operation attempt
    log_admin_action(
        &state,
        &admin_user,
        &format!("bulk_operation_{}", request.operation),
        None,
        serde_json::to_value(&request).unwrap_or_default(),
        ip_address,
        user_agent,
    )
    .await;

    // Execute bulk operation
    match execute_bulk_operation(&state, &request).await {
        Ok(result) => {
            tracing::info!(
                "Bulk operation '{}' completed by {}: {}/{} successful",
                request.operation,
                admin_user
                    .user
                    .name
                    .as_deref()
                    .unwrap_or(&admin_user.user.email),
                result.successful,
                result.total_targets
            );
            Json(result).into_response()
        }
        Err(e) => {
            tracing::error!("Bulk operation failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UserQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub search: Option<String>,
}

/// Admin API endpoint to list users
pub async fn admin_list_users(
    State(state): State<AppState>,
    user_ctx: UserContext,
    Query(query): Query<UserQuery>,
) -> Response {
    let admin_user = match AdminUser::from_user_context(&user_ctx) {
        Some(admin) => admin,
        None => return StatusCode::FORBIDDEN.into_response(),
    };

    if !admin_user.has_permission(&Permission::ManageUsers) {
        return StatusCode::FORBIDDEN.into_response();
    }

    // TODO: Implement user listing from database
    // For now, return placeholder data
    let users = serde_json::json!({
        "users": [],
        "total": 0,
        "limit": query.limit.unwrap_or(50),
        "offset": query.offset.unwrap_or(0),
    });

    Json(users).into_response()
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
    pub roles: Vec<String>,
}

/// Admin API endpoint to create users
pub async fn admin_create_user(
    State(state): State<AppState>,
    user_ctx: UserContext,
    headers: header::HeaderMap,
    Json(request): Json<CreateUserRequest>,
) -> Response {
    let admin_user = match AdminUser::from_user_context(&user_ctx) {
        Some(admin) => admin,
        None => return StatusCode::FORBIDDEN.into_response(),
    };

    if !admin_user.has_permission(&Permission::ManageUsers) {
        return StatusCode::FORBIDDEN.into_response();
    }

    // Extract IP and User-Agent for audit logging
    let ip_address = headers
        .get("x-forwarded-for")
        .or(headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

    // Log user creation attempt
    log_admin_action(
        &state,
        &admin_user,
        "create_user",
        Some(&request.name),
        serde_json::to_value(&request).unwrap_or_default(),
        ip_address,
        user_agent,
    )
    .await;

    // TODO: Implement user creation
    // For now, return success response
    let response = serde_json::json!({
        "success": true,
        "message": "User creation not yet implemented",
        "user": {
            "name": request.name,
            "email": request.email,
            "roles": request.roles,
            "created_at": Utc::now(),
        }
    });

    Json(response).into_response()
}

// Helper functions

async fn fetch_comprehensive_system_status(state: &AppState) -> anyhow::Result<serde_json::Value> {
    let mut status = HashMap::new();

    // Database health check
    match state.database.health_check().await {
        Ok(_) => status.insert("database", "healthy"),
        Err(_) => status.insert("database", "unhealthy"),
    };

    // TODO: Add checks for other services:
    // - Runner service health
    // - Publisher service health
    // - Redis connectivity
    // - VCS store health
    // - Log manager health

    // Basic system info
    status.insert("version", env!("CARGO_PKG_VERSION"));
    status.insert("build_time", option_env!("BUILD_TIME").unwrap_or("unknown"));
    status.insert(
        "git_revision",
        option_env!("GIT_REVISION").unwrap_or("unknown"),
    );

    Ok(serde_json::json!({
        "status": status,
        "timestamp": Utc::now(),
        "uptime": "unknown", // TODO: Calculate uptime
        "services": {
            "site": "healthy",
            "database": status.get("database"),
            "runner": "unknown",
            "publisher": "unknown",
            "redis": "unknown",
            "vcs_stores": "unknown",
        }
    }))
}

async fn fetch_system_metrics(state: &AppState) -> anyhow::Result<serde_json::Value> {
    // Fetch database statistics
    let db_stats = state.database.get_stats().await.unwrap_or_default();

    // TODO: Add metrics from other sources:
    // - Prometheus metrics
    // - System resource usage
    // - Performance counters
    // - Error rates

    Ok(serde_json::json!({
        "database": {
            "total_codebases": db_stats.get("total_codebases").unwrap_or(&0),
            "active_runs": db_stats.get("active_runs").unwrap_or(&0),
            "queue_size": db_stats.get("queue_size").unwrap_or(&0),
        },
        "system": {
            "memory_usage": "unknown",
            "cpu_usage": "unknown",
            "disk_usage": "unknown",
        },
        "performance": {
            "requests_per_second": "unknown",
            "average_response_time": "unknown",
            "error_rate": "unknown",
        },
        "timestamp": Utc::now(),
    }))
}

async fn execute_bulk_operation(
    state: &AppState,
    request: &BulkOperationRequest,
) -> anyhow::Result<BulkOperationResult> {
    let total_targets = request.targets.len();
    let mut successful = 0;
    let mut errors = Vec::new();

    match request.operation.as_str() {
        "reschedule" => {
            // TODO: Implement bulk reschedule operation
            // This would integrate with the runner service to reschedule failed runs
            for target in &request.targets {
                // Placeholder implementation
                match target.starts_with("run-") {
                    true => successful += 1,
                    false => errors.push(format!("Invalid run ID: {}", target)),
                }
            }
        }
        "cancel" => {
            // TODO: Implement bulk cancel operation
            for target in &request.targets {
                // Placeholder implementation
                match target.starts_with("run-") {
                    true => successful += 1,
                    false => errors.push(format!("Invalid run ID: {}", target)),
                }
            }
        }
        "approve_reviews" | "reject_reviews" => {
            // TODO: Implement bulk review operations
            for target in &request.targets {
                // Placeholder implementation
                match target.starts_with("review-") {
                    true => successful += 1,
                    false => errors.push(format!("Invalid review ID: {}", target)),
                }
            }
        }
        "emergency_stop" => {
            // TODO: Implement emergency stop operation
            // This would coordinate with publisher service to halt publishing
            successful = total_targets; // Placeholder
        }
        _ => {
            return Err(anyhow::anyhow!(
                "Unknown bulk operation: {}",
                request.operation
            ));
        }
    }

    Ok(BulkOperationResult {
        operation: request.operation.clone(),
        total_targets,
        successful,
        failed: total_targets - successful,
        errors,
        completed_at: Utc::now(),
    })
}
