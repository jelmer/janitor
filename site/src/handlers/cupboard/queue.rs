use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Json, Response},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{
    api::{negotiate_content_type, ContentType},
    app::AppState,
    auth::UserContext,
    database::DatabaseError,
    templates::create_base_context,
};

use super::{AdminUser, Permission, create_admin_context, log_admin_action};

#[derive(Debug, Deserialize, Serialize)]
pub struct QueueFilters {
    pub suite: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub search: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct QueueItem {
    pub id: String,
    pub codebase: String,
    pub suite: String,
    pub command: Option<String>,
    pub priority: i32,
    pub success_chance: Option<f64>,
    pub created_at: DateTime<Utc>,
    pub estimated_duration: Option<i64>,
    pub worker: Option<String>,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct QueueStatistics {
    pub total_items: i64,
    pub pending_items: i64,
    pub in_progress_items: i64,
    pub failed_items: i64,
    pub average_wait_time: i64,
    pub estimated_completion: Option<DateTime<Utc>>,
    pub worker_utilization: f64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BulkQueueOperation {
    pub operation: String, // "reschedule", "cancel", "priority_boost", "assign_worker"
    pub item_ids: Vec<String>,
    pub parameters: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct QueueOperationResult {
    pub operation: String,
    pub total_items: usize,
    pub successful: usize,
    pub failed: usize,
    pub errors: Vec<String>,
    pub completed_at: DateTime<Utc>,
}

/// Queue dashboard - main queue management interface
pub async fn queue_dashboard(
    State(state): State<AppState>,
    user_ctx: UserContext,
    Query(filters): Query<QueueFilters>,
    headers: header::HeaderMap,
) -> Response {
    let admin_user = match AdminUser::from_user_context(&user_ctx) {
        Some(admin) => admin,
        None => return StatusCode::FORBIDDEN.into_response(),
    };
    
    if !admin_user.has_permission(&Permission::ViewQueue) {
        return StatusCode::FORBIDDEN.into_response();
    }
    
    let mut context = create_admin_context(&admin_user);
    
    // Fetch queue items and statistics
    match fetch_queue_data(&state, &filters).await {
        Ok((items, stats)) => {
            context.insert("queue_items", &items);
            context.insert("queue_stats", &stats);
            context.insert("filters", &filters);
        }
        Err(e) => {
            tracing::error!("Failed to fetch queue data: {}", e);
            context.insert("error_message", &format!("Failed to load queue data: {}", e));
        }
    }
    
    // Add available suites for filtering
    let suites: Vec<String> = state.config.campaigns.keys().cloned().collect();
    context.insert("available_suites", &suites);
    
    let content_type = negotiate_content_type(&headers, "queue_dashboard");
    
    match content_type {
        ContentType::Json => {
            Json(context.into_json()).into_response()
        }
        _ => {
            match state.templates.render("cupboard/queue.html", &context) {
                Ok(html) => Html(html).into_response(),
                Err(e) => {
                    tracing::error!("Template rendering error: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                }
            }
        }
    }
}

/// Queue item details
pub async fn queue_item_details(
    State(state): State<AppState>,
    user_ctx: UserContext,
    Path(item_id): Path<String>,
    headers: header::HeaderMap,
) -> Response {
    let admin_user = match AdminUser::from_user_context(&user_ctx) {
        Some(admin) => admin,
        None => return StatusCode::FORBIDDEN.into_response(),
    };
    
    if !admin_user.has_permission(&Permission::ViewQueue) {
        return StatusCode::FORBIDDEN.into_response();
    }
    
    let mut context = create_admin_context(&admin_user);
    context.insert("item_id", &item_id);
    
    // Fetch queue item details
    match fetch_queue_item_details(&state, &item_id).await {
        Ok(item) => {
            context.insert("queue_item", &item);
        }
        Err(e) => {
            tracing::error!("Failed to fetch queue item details: {}", e);
            return StatusCode::NOT_FOUND.into_response();
        }
    }
    
    let content_type = negotiate_content_type(&headers, "queue_item");
    
    match content_type {
        ContentType::Json => {
            Json(context.into_json()).into_response()
        }
        _ => {
            match state.templates.render("cupboard/queue-item.html", &context) {
                Ok(html) => Html(html).into_response(),
                Err(e) => {
                    tracing::error!("Template rendering error: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                }
            }
        }
    }
}

/// Bulk queue operations
pub async fn bulk_queue_operation(
    State(state): State<AppState>,
    user_ctx: UserContext,
    headers: header::HeaderMap,
    Json(operation): Json<BulkQueueOperation>,
) -> Response {
    let admin_user = match AdminUser::from_user_context(&user_ctx) {
        Some(admin) => admin,
        None => return StatusCode::FORBIDDEN.into_response(),
    };
    
    if !admin_user.has_permission(&Permission::BulkQueueOperations) {
        return StatusCode::FORBIDDEN.into_response();
    }
    
    // Extract IP and User-Agent for audit logging
    let ip_address = extract_ip_address(&headers);
    let user_agent = extract_user_agent(&headers);
    
    // Log the bulk operation attempt
    log_admin_action(
        &state,
        &admin_user,
        &format!("bulk_queue_{}", operation.operation),
        None,
        serde_json::to_value(&operation).unwrap_or_default(),
        &ip_address,
        &user_agent,
    ).await;
    
    // Execute bulk operation
    match execute_bulk_queue_operation(&state, &operation).await {
        Ok(result) => {
            tracing::info!(
                "Bulk queue operation '{}' completed by {}: {}/{} successful",
                operation.operation,
                admin_user.user.name.as_deref().unwrap_or(&admin_user.user.email),
                result.successful,
                result.total_items
            );
            Json(result).into_response()
        }
        Err(e) => {
            tracing::error!("Bulk queue operation failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// Queue statistics endpoint
pub async fn queue_statistics(
    State(state): State<AppState>,
    user_ctx: UserContext,
    Query(filters): Query<QueueFilters>,
) -> Response {
    let admin_user = match AdminUser::from_user_context(&user_ctx) {
        Some(admin) => admin,
        None => return StatusCode::FORBIDDEN.into_response(),
    };
    
    if !admin_user.has_permission(&Permission::ViewQueue) {
        return StatusCode::FORBIDDEN.into_response();
    }
    
    match fetch_queue_statistics(&state, &filters).await {
        Ok(stats) => Json(stats).into_response(),
        Err(e) => {
            tracing::error!("Failed to fetch queue statistics: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// Worker management interface
pub async fn worker_management(
    State(state): State<AppState>,
    user_ctx: UserContext,
    headers: header::HeaderMap,
) -> Response {
    let admin_user = match AdminUser::from_user_context(&user_ctx) {
        Some(admin) => admin,
        None => return StatusCode::FORBIDDEN.into_response(),
    };
    
    if !admin_user.has_permission(&Permission::ViewQueue) {
        return StatusCode::FORBIDDEN.into_response();
    }
    
    let mut context = create_admin_context(&admin_user);
    
    // Fetch worker information
    match fetch_worker_information(&state).await {
        Ok(workers) => {
            context.insert("workers", &workers);
        }
        Err(e) => {
            tracing::error!("Failed to fetch worker information: {}", e);
            context.insert("error_message", &format!("Failed to load worker data: {}", e));
        }
    }
    
    let content_type = negotiate_content_type(&headers, "worker_management");
    
    match content_type {
        ContentType::Json => {
            Json(context.into_json()).into_response()
        }
        _ => {
            match state.templates.render("cupboard/workers.html", &context) {
                Ok(html) => Html(html).into_response(),
                Err(e) => {
                    tracing::error!("Template rendering error: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                }
            }
        }
    }
}

// Helper functions

async fn fetch_queue_data(
    state: &AppState,
    filters: &QueueFilters,
) -> anyhow::Result<(Vec<QueueItem>, QueueStatistics)> {
    // TODO: Implement queue data fetching from database and runner service
    // This would query the queue table and integrate with runner service for real-time status
    
    // Placeholder implementation
    let items = vec![
        QueueItem {
            id: "queue-1".to_string(),
            codebase: "example-package".to_string(),
            suite: "lintian-fixes".to_string(),
            command: Some("fix-lintian-issues".to_string()),
            priority: 100,
            success_chance: Some(0.85),
            created_at: Utc::now() - chrono::Duration::hours(2),
            estimated_duration: Some(300), // 5 minutes
            worker: None,
            status: "pending".to_string(),
        },
    ];
    
    let stats = QueueStatistics {
        total_items: 1,
        pending_items: 1,
        in_progress_items: 0,
        failed_items: 0,
        average_wait_time: 300, // 5 minutes
        estimated_completion: Some(Utc::now() + chrono::Duration::minutes(5)),
        worker_utilization: 0.0,
    };
    
    Ok((items, stats))
}

async fn fetch_queue_item_details(
    state: &AppState,
    item_id: &str,
) -> anyhow::Result<QueueItem> {
    // TODO: Implement queue item detail fetching
    // This would query the database for detailed queue item information
    
    // Placeholder implementation
    Ok(QueueItem {
        id: item_id.to_string(),
        codebase: "example-package".to_string(),
        suite: "lintian-fixes".to_string(),
        command: Some("fix-lintian-issues".to_string()),
        priority: 100,
        success_chance: Some(0.85),
        created_at: Utc::now() - chrono::Duration::hours(2),
        estimated_duration: Some(300),
        worker: None,
        status: "pending".to_string(),
    })
}

async fn fetch_queue_statistics(
    state: &AppState,
    filters: &QueueFilters,
) -> anyhow::Result<QueueStatistics> {
    // TODO: Implement comprehensive queue statistics
    // This would aggregate data from the queue and provide real-time metrics
    
    // Use database stats as baseline
    let db_stats = state.database.get_stats().await.unwrap_or_default();
    
    Ok(QueueStatistics {
        total_items: db_stats.get("queue_size").copied().unwrap_or(0),
        pending_items: db_stats.get("queue_size").copied().unwrap_or(0),
        in_progress_items: db_stats.get("active_runs").copied().unwrap_or(0),
        failed_items: 0, // TODO: Calculate failed items
        average_wait_time: 300, // TODO: Calculate from historical data
        estimated_completion: Some(Utc::now() + chrono::Duration::minutes(30)),
        worker_utilization: 0.0, // TODO: Calculate from worker data
    })
}

async fn execute_bulk_queue_operation(
    state: &AppState,
    operation: &BulkQueueOperation,
) -> anyhow::Result<QueueOperationResult> {
    let total_items = operation.item_ids.len();
    let mut successful = 0;
    let mut errors = Vec::new();
    
    match operation.operation.as_str() {
        "reschedule" => {
            // TODO: Implement bulk reschedule through runner service
            for item_id in &operation.item_ids {
                // Placeholder implementation
                if item_id.starts_with("queue-") {
                    successful += 1;
                } else {
                    errors.push(format!("Invalid queue item ID: {}", item_id));
                }
            }
        }
        "cancel" => {
            // TODO: Implement bulk cancel through runner service
            for item_id in &operation.item_ids {
                if item_id.starts_with("queue-") {
                    successful += 1;
                } else {
                    errors.push(format!("Invalid queue item ID: {}", item_id));
                }
            }
        }
        "priority_boost" => {
            // TODO: Implement priority adjustment in database
            for item_id in &operation.item_ids {
                if item_id.starts_with("queue-") {
                    successful += 1;
                } else {
                    errors.push(format!("Invalid queue item ID: {}", item_id));
                }
            }
        }
        "assign_worker" => {
            // TODO: Implement worker assignment through runner service
            for item_id in &operation.item_ids {
                if item_id.starts_with("queue-") {
                    successful += 1;
                } else {
                    errors.push(format!("Invalid queue item ID: {}", item_id));
                }
            }
        }
        _ => {
            return Err(anyhow::anyhow!("Unknown queue operation: {}", operation.operation));
        }
    }
    
    Ok(QueueOperationResult {
        operation: operation.operation.clone(),
        total_items,
        successful,
        failed: total_items - successful,
        errors,
        completed_at: Utc::now(),
    })
}

async fn fetch_worker_information(state: &AppState) -> anyhow::Result<serde_json::Value> {
    // TODO: Implement worker information fetching from runner service
    // This would query the runner service for current worker status
    
    Ok(serde_json::json!({
        "workers": [],
        "total_workers": 0,
        "active_workers": 0,
        "idle_workers": 0,
        "offline_workers": 0,
        "last_updated": Utc::now(),
    }))
}

fn extract_ip_address(headers: &header::HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .or(headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string()
}

fn extract_user_agent(headers: &header::HeaderMap) -> String {
    headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string()
}