use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Json, Response},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::{
    api::{negotiate_content_type, ContentType},
    app::AppState,
    auth::UserContext,
};

use super::{create_admin_context, log_admin_action, AdminUser, Permission};

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

    // Fetch queue items and statistics using new database methods
    match state
        .database
        .get_queue_items_with_stats(
            filters.suite.as_deref(),
            filters.status.as_deref(),
            filters.priority.as_deref(),
            filters.limit,
            filters.offset,
        )
        .await
    {
        Ok((items, stats)) => {
            context.insert("queue_items", &items);
            context.insert("queue_stats", &stats);
            context.insert("filters", &filters);
        }
        Err(e) => {
            tracing::error!("Failed to fetch queue data: {}", e);
            context.insert(
                "error_message",
                &format!("Failed to load queue data: {}", e),
            );
        }
    }

    // Add available suites for filtering
    let suites: Vec<String> = state.config.campaigns.keys().cloned().collect();
    context.insert("available_suites", &suites);

    let content_type = negotiate_content_type(&headers, "queue_dashboard");

    match content_type {
        ContentType::Json => Json(context.into_json()).into_response(),
        _ => match state.templates.render("cupboard/queue.html", &context) {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("Template rendering error: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        },
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
        ContentType::Json => Json(context.into_json()).into_response(),
        _ => match state.templates.render("cupboard/queue-item.html", &context) {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("Template rendering error: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        },
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
    )
    .await;

    // Execute bulk operation
    match execute_bulk_queue_operation(&state, &operation).await {
        Ok(result) => {
            tracing::info!(
                "Bulk queue operation '{}' completed by {}: {}/{} successful",
                operation.operation,
                admin_user
                    .user
                    .name
                    .as_deref()
                    .unwrap_or(&admin_user.user.email),
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
            context.insert(
                "error_message",
                &format!("Failed to load worker data: {}", e),
            );
        }
    }

    let content_type = negotiate_content_type(&headers, "worker_management");

    match content_type {
        ContentType::Json => Json(context.into_json()).into_response(),
        _ => match state.templates.render("cupboard/workers.html", &context) {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("Template rendering error: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        },
    }
}

// Helper functions

// This function is no longer needed - we use the database methods directly

async fn fetch_queue_item_details(state: &AppState, item_id: &str) -> anyhow::Result<QueueItem> {
    // Query the candidate table for queue item details
    let row = sqlx::query(
        "SELECT 
            c.id, c.codebase, c.suite, c.command, c.priority, c.success_chance,
            c.created_at, c.estimated_duration, r.worker, 
            CASE 
                WHEN r.id IS NOT NULL AND r.result_code IS NOT NULL THEN 'completed'
                WHEN r.id IS NOT NULL THEN 'running'
                ELSE 'pending'
            END as status
        FROM candidate c
        LEFT JOIN run r ON c.id = r.id
        WHERE c.id = $1"
    )
    .bind(item_id)
    .fetch_one(state.database.pool())
    .await?;

    Ok(QueueItem {
        id: row.try_get("id")?,
        codebase: row.try_get("codebase")?,
        suite: row.try_get("suite")?,
        command: row.try_get("command").ok(),
        priority: row.try_get("priority")?,
        success_chance: row.try_get("success_chance").ok(),
        created_at: row.try_get("created_at")?,
        estimated_duration: row.try_get("estimated_duration").ok(),
        worker: row.try_get("worker").ok(),
        status: row.try_get("status")?,
    })
}

async fn fetch_queue_statistics(
    state: &AppState,
    filters: &QueueFilters,
) -> anyhow::Result<QueueStatistics> {
    // Get basic queue statistics without complex filtering for now
    // TODO: Add filtering support when needed
    let stats_row = sqlx::query(
        "SELECT 
            COUNT(*) as total_items,
            COUNT(*) FILTER (WHERE r.id IS NULL) as pending_items,
            COUNT(*) FILTER (WHERE r.id IS NOT NULL AND r.result_code IS NULL) as in_progress_items,
            COUNT(*) FILTER (WHERE r.result_code = 'failed') as failed_items,
            EXTRACT(EPOCH FROM AVG(
                CASE WHEN r.start_time IS NOT NULL AND c.created_at IS NOT NULL 
                THEN r.start_time - c.created_at 
                END
            )) as average_wait_time
        FROM candidate c
        LEFT JOIN run r ON c.id = r.id"
    )
    .fetch_one(state.database.pool())
    .await?;

    // Get worker utilization from recent activity
    let worker_row = sqlx::query(
        "SELECT 
            COUNT(DISTINCT worker) as active_workers,
            COUNT(*) as active_runs
        FROM run 
        WHERE start_time > NOW() - INTERVAL '1 hour' 
        AND result_code IS NULL"
    )
    .fetch_one(state.database.pool())
    .await?;
    
    let active_workers: i64 = worker_row.try_get("active_workers").unwrap_or(0);
    let active_runs: i64 = worker_row.try_get("active_runs").unwrap_or(0);
    
    // Assume each worker can handle 2 concurrent runs for utilization calculation
    let max_capacity = active_workers * 2;
    let worker_utilization = if max_capacity > 0 {
        active_runs as f64 / max_capacity as f64
    } else {
        0.0
    };

    // Estimate completion time based on pending items and current throughput
    let pending_items: i64 = stats_row.try_get("pending_items")?;
    let average_wait_time: Option<f64> = stats_row.try_get("average_wait_time").ok();
    
    let estimated_completion = if pending_items > 0 && active_workers > 0 {
        let avg_wait_seconds = average_wait_time.unwrap_or(1800.0); // Default 30 minutes
        let estimated_seconds = (pending_items as f64 / active_workers as f64) * avg_wait_seconds;
        Some(Utc::now() + chrono::Duration::seconds(estimated_seconds as i64))
    } else {
        None
    };

    Ok(QueueStatistics {
        total_items: stats_row.try_get("total_items")?,
        pending_items: stats_row.try_get("pending_items")?,
        in_progress_items: stats_row.try_get("in_progress_items")?,
        failed_items: stats_row.try_get("failed_items")?,
        average_wait_time: average_wait_time.unwrap_or(0.0) as i64,
        estimated_completion,
        worker_utilization,
    })
}

async fn execute_bulk_queue_operation(
    state: &AppState,
    operation: &BulkQueueOperation,
) -> anyhow::Result<QueueOperationResult> {
    let admin_user_name = "admin"; // This should be passed from context
    let now = Utc::now();

    match operation.operation.as_str() {
        "reschedule" => {
            let affected_rows = state
                .database
                .bulk_reschedule_queue_items(&operation.item_ids, admin_user_name)
                .await?;

            Ok(QueueOperationResult {
                operation: operation.operation.clone(),
                total_items: operation.item_ids.len(),
                successful: affected_rows as usize,
                failed: operation
                    .item_ids
                    .len()
                    .saturating_sub(affected_rows as usize),
                errors: vec![],
                completed_at: now,
            })
        }
        "cancel" => {
            let affected_rows = state
                .database
                .bulk_cancel_queue_items(&operation.item_ids, admin_user_name)
                .await?;

            Ok(QueueOperationResult {
                operation: operation.operation.clone(),
                total_items: operation.item_ids.len(),
                successful: affected_rows as usize,
                failed: operation
                    .item_ids
                    .len()
                    .saturating_sub(affected_rows as usize),
                errors: vec![],
                completed_at: now,
            })
        }
        "priority_boost" => {
            // Extract priority adjustment from parameters
            let priority_adjustment = operation
                .parameters
                .as_ref()
                .and_then(|p| p.get("adjustment"))
                .and_then(|a| a.as_i64())
                .unwrap_or(10) as i32;

            let affected_rows = state
                .database
                .bulk_adjust_priority(&operation.item_ids, priority_adjustment, admin_user_name)
                .await?;

            Ok(QueueOperationResult {
                operation: operation.operation.clone(),
                total_items: operation.item_ids.len(),
                successful: affected_rows as usize,
                failed: operation
                    .item_ids
                    .len()
                    .saturating_sub(affected_rows as usize),
                errors: vec![],
                completed_at: now,
            })
        }
        "priority_decrease" => {
            // Extract priority adjustment from parameters (negative)
            let priority_adjustment = operation
                .parameters
                .as_ref()
                .and_then(|p| p.get("adjustment"))
                .and_then(|a| a.as_i64())
                .map(|a| -a)
                .unwrap_or(-10) as i32;

            let affected_rows = state
                .database
                .bulk_adjust_priority(&operation.item_ids, priority_adjustment, admin_user_name)
                .await?;

            Ok(QueueOperationResult {
                operation: operation.operation.clone(),
                total_items: operation.item_ids.len(),
                successful: affected_rows as usize,
                failed: operation
                    .item_ids
                    .len()
                    .saturating_sub(affected_rows as usize),
                errors: vec![],
                completed_at: now,
            })
        }
        "assign_worker" => {
            // TODO: Implement worker assignment through runner service
            // This would require communication with the runner service
            Ok(QueueOperationResult {
                operation: operation.operation.clone(),
                total_items: operation.item_ids.len(),
                successful: 0,
                failed: operation.item_ids.len(),
                errors: vec!["Worker assignment not yet implemented".to_string()],
                completed_at: now,
            })
        }
        _ => Err(anyhow::anyhow!(
            "Unknown queue operation: {}",
            operation.operation
        )),
    }
}

async fn fetch_worker_information(state: &AppState) -> anyhow::Result<serde_json::Value> {
    // Use the new database method to get real worker information
    state
        .database
        .get_worker_information()
        .await
        .map_err(|e| anyhow::anyhow!("Database error: {}", e))
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
