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
};

use super::{create_admin_context, log_admin_action, AdminUser, Permission};

#[derive(Debug, Deserialize, Serialize)]
pub struct PublishFilters {
    pub status: Option<String>,
    pub suite: Option<String>,
    pub result_code: Option<String>,
    pub mode: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PublishItem {
    pub id: String,
    pub codebase: String,
    pub suite: String,
    pub branch_name: Option<String>,
    pub mode: String,
    pub result_code: Option<String>,
    pub description: Option<String>,
    pub merge_proposal_url: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub vcs_browse: Option<String>,
    pub rate_limited: bool,
    pub retry_after: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct PublishStatistics {
    pub total_published: i64,
    pub successful_publishes: i64,
    pub failed_publishes: i64,
    pub rate_limited_publishes: i64,
    pub avg_publish_time: f64,
    pub active_publishers: i64,
    pub pending_queue_size: i64,
    pub rate_limit_status: HashMap<String, RateLimitInfo>,
}

#[derive(Debug, Serialize)]
pub struct RateLimitInfo {
    pub bucket: String,
    pub current_requests: i64,
    pub max_requests: i64,
    pub window_seconds: i64,
    pub reset_time: Option<DateTime<Utc>>,
    pub is_limited: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EmergencyStopParams {
    pub action: String, // "stop_all", "stop_suite", "stop_codebase", "resume_all"
    pub suite: Option<String>,
    pub codebase: Option<String>,
    pub reason: String,
    pub duration: Option<i64>, // Duration in minutes, None for indefinite
}

#[derive(Debug, Serialize)]
pub struct EmergencyStopResult {
    pub action: String,
    pub affected_items: i64,
    pub reason: String,
    pub stopped_at: DateTime<Utc>,
    pub resume_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RateLimitAdjustment {
    pub bucket: String,
    pub max_requests: Option<i64>,
    pub window_seconds: Option<i64>,
    pub action: String, // "increase", "decrease", "reset", "disable", "enable"
}

#[derive(Debug, Serialize)]
pub struct ReadyRun {
    pub id: String,
    pub codebase: String,
    pub suite: String,
    pub command: Option<String>,
    pub result_code: String,
    pub publish_status: Option<String>,
    pub value: Option<i32>,
    pub finish_time: Option<DateTime<Utc>>,
    pub can_publish: bool,
}

/// Publishing dashboard - main publishing oversight interface
pub async fn publish_dashboard(
    State(state): State<AppState>,
    user_ctx: UserContext,
    Query(filters): Query<PublishFilters>,
    headers: header::HeaderMap,
) -> Response {
    let admin_user = match AdminUser::from_user_context(&user_ctx) {
        Some(admin) => admin,
        None => return StatusCode::FORBIDDEN.into_response(),
    };

    if !admin_user.has_permission(&Permission::ViewPublishQueue) {
        return StatusCode::FORBIDDEN.into_response();
    }

    let mut context = create_admin_context(&admin_user);

    // Fetch publishing data and statistics
    match fetch_publish_dashboard_data(&state, &filters).await {
        Ok((publish_items, stats)) => {
            context.insert("publish_items", &publish_items);
            context.insert("publish_stats", &stats);
            context.insert("filters", &filters);
        }
        Err(e) => {
            tracing::error!("Failed to fetch publish data: {}", e);
            context.insert(
                "error_message",
                &format!("Failed to load publish data: {}", e),
            );
        }
    }

    // Add available suites for filtering
    let campaigns: Vec<String> = state.config.campaigns.keys().cloned().collect();
    context.insert("available_campaigns", &campaigns);

    let content_type = negotiate_content_type(&headers, "publish_dashboard");

    match content_type {
        ContentType::Json => Json(context.into_json()).into_response(),
        _ => {
            match state
                .templates
                .render("cupboard/publish-dashboard.html", &context)
            {
                Ok(html) => Html(html).into_response(),
                Err(e) => {
                    tracing::error!("Template rendering error: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                }
            }
        }
    }
}

/// Publishing history view
pub async fn publish_history(
    State(state): State<AppState>,
    user_ctx: UserContext,
    Query(filters): Query<PublishFilters>,
    headers: header::HeaderMap,
) -> Response {
    let admin_user = match AdminUser::from_user_context(&user_ctx) {
        Some(admin) => admin,
        None => return StatusCode::FORBIDDEN.into_response(),
    };

    if !admin_user.has_permission(&Permission::ViewPublishQueue) {
        return StatusCode::FORBIDDEN.into_response();
    }

    let mut context = create_admin_context(&admin_user);

    // Fetch publish history
    match fetch_publish_history(&state, &filters).await {
        Ok(history) => {
            context.insert("publish_history", &history);
            context.insert("filters", &filters);
        }
        Err(e) => {
            tracing::error!("Failed to fetch publish history: {}", e);
            context.insert(
                "error_message",
                &format!("Failed to load publish history: {}", e),
            );
        }
    }

    let content_type = negotiate_content_type(&headers, "publish_history");

    match content_type {
        ContentType::Json => Json(context.into_json()).into_response(),
        _ => {
            match state
                .templates
                .render("cupboard/publish-history.html", &context)
            {
                Ok(html) => Html(html).into_response(),
                Err(e) => {
                    tracing::error!("Template rendering error: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                }
            }
        }
    }
}

/// Individual publish details
pub async fn publish_details(
    State(state): State<AppState>,
    user_ctx: UserContext,
    Path(publish_id): Path<String>,
    headers: header::HeaderMap,
) -> Response {
    let admin_user = match AdminUser::from_user_context(&user_ctx) {
        Some(admin) => admin,
        None => return StatusCode::FORBIDDEN.into_response(),
    };

    if !admin_user.has_permission(&Permission::ViewPublishQueue) {
        return StatusCode::FORBIDDEN.into_response();
    }

    let mut context = create_admin_context(&admin_user);
    context.insert("publish_id", &publish_id);

    // Fetch publish details
    match fetch_publish_details(&state, &publish_id).await {
        Ok(publish) => {
            context.insert("publish", &publish);
        }
        Err(e) => {
            tracing::error!("Failed to fetch publish details: {}", e);
            return StatusCode::NOT_FOUND.into_response();
        }
    }

    let content_type = negotiate_content_type(&headers, "publish_details");

    match content_type {
        ContentType::Json => Json(context.into_json()).into_response(),
        _ => match state.templates.render("cupboard/publish.html", &context) {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("Template rendering error: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        },
    }
}

/// Ready runs for publishing
pub async fn ready_runs(
    State(state): State<AppState>,
    user_ctx: UserContext,
    Query(filters): Query<PublishFilters>,
    headers: header::HeaderMap,
) -> Response {
    let admin_user = match AdminUser::from_user_context(&user_ctx) {
        Some(admin) => admin,
        None => return StatusCode::FORBIDDEN.into_response(),
    };

    if !admin_user.has_permission(&Permission::ViewPublishQueue) {
        return StatusCode::FORBIDDEN.into_response();
    }

    let mut context = create_admin_context(&admin_user);

    // Fetch ready runs
    match fetch_ready_runs(&state, &filters).await {
        Ok(ready_runs) => {
            context.insert("ready_runs", &ready_runs);
            context.insert("filters", &filters);
        }
        Err(e) => {
            tracing::error!("Failed to fetch ready runs: {}", e);
            context.insert(
                "error_message",
                &format!("Failed to load ready runs: {}", e),
            );
        }
    }

    let content_type = negotiate_content_type(&headers, "ready_runs");

    match content_type {
        ContentType::Json => Json(context.into_json()).into_response(),
        _ => match state.templates.render("cupboard/ready-list.html", &context) {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("Template rendering error: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        },
    }
}

/// Emergency publish controls
pub async fn emergency_publish_stop(
    State(state): State<AppState>,
    user_ctx: UserContext,
    headers: header::HeaderMap,
    Json(params): Json<EmergencyStopParams>,
) -> Response {
    let admin_user = match AdminUser::from_user_context(&user_ctx) {
        Some(admin) => admin,
        None => return StatusCode::FORBIDDEN.into_response(),
    };

    if !admin_user.has_permission(&Permission::EmergencyPublishControls) {
        return StatusCode::FORBIDDEN.into_response();
    }

    // Extract IP and User-Agent for audit logging
    let ip_address = extract_ip_address(&headers);
    let user_agent = extract_user_agent(&headers);

    // Log the emergency action
    log_admin_action(
        &state,
        &admin_user,
        &format!("emergency_publish_{}", params.action),
        None,
        serde_json::to_value(&params).unwrap_or_default(),
        &ip_address,
        &user_agent,
    )
    .await;

    // Execute emergency action
    match execute_emergency_publish_action(&state, &params).await {
        Ok(result) => {
            tracing::warn!(
                "Emergency publish action '{}' executed by {}: {} items affected. Reason: {}",
                params.action,
                admin_user
                    .user
                    .name
                    .as_deref()
                    .unwrap_or(&admin_user.user.email),
                result.affected_items,
                params.reason
            );
            Json(result).into_response()
        }
        Err(e) => {
            tracing::error!("Emergency publish action failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// Rate limit management
pub async fn adjust_rate_limits(
    State(state): State<AppState>,
    user_ctx: UserContext,
    headers: header::HeaderMap,
    Json(adjustment): Json<RateLimitAdjustment>,
) -> Response {
    let admin_user = match AdminUser::from_user_context(&user_ctx) {
        Some(admin) => admin,
        None => return StatusCode::FORBIDDEN.into_response(),
    };

    if !admin_user.has_permission(&Permission::ModifyPublishSettings) {
        return StatusCode::FORBIDDEN.into_response();
    }

    // Extract IP and User-Agent for audit logging
    let ip_address = extract_ip_address(&headers);
    let user_agent = extract_user_agent(&headers);

    // Log the rate limit adjustment
    log_admin_action(
        &state,
        &admin_user,
        "adjust_rate_limits",
        Some(&adjustment.bucket),
        serde_json::to_value(&adjustment).unwrap_or_default(),
        &ip_address,
        &user_agent,
    )
    .await;

    // Apply rate limit changes
    match apply_rate_limit_adjustment(&state, &adjustment).await {
        Ok(new_limits) => {
            tracing::info!(
                "Rate limits adjusted for bucket {} by {}: action={}",
                adjustment.bucket,
                admin_user
                    .user
                    .name
                    .as_deref()
                    .unwrap_or(&admin_user.user.email),
                adjustment.action
            );
            Json(new_limits).into_response()
        }
        Err(e) => {
            tracing::error!("Rate limit adjustment failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// Publishing statistics endpoint
pub async fn publish_statistics(
    State(state): State<AppState>,
    user_ctx: UserContext,
    Query(filters): Query<PublishFilters>,
) -> Response {
    let admin_user = match AdminUser::from_user_context(&user_ctx) {
        Some(admin) => admin,
        None => return StatusCode::FORBIDDEN.into_response(),
    };

    if !admin_user.has_permission(&Permission::ViewPublishQueue) {
        return StatusCode::FORBIDDEN.into_response();
    }

    match fetch_publish_statistics(&state, &filters).await {
        Ok(stats) => Json(stats).into_response(),
        Err(e) => {
            tracing::error!("Failed to fetch publish statistics: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

// Helper functions

async fn fetch_publish_dashboard_data(
    state: &AppState,
    filters: &PublishFilters,
) -> anyhow::Result<(Vec<PublishItem>, PublishStatistics)> {
    // TODO: Implement comprehensive publish dashboard data fetching
    // This would query the publish table with filtering and statistics

    let items = vec![PublishItem {
        id: "pub-1".to_string(),
        codebase: "example-package".to_string(),
        suite: "lintian-fixes".to_string(),
        branch_name: Some("debian/lintian-fixes".to_string()),
        mode: "propose".to_string(),
        result_code: Some("success".to_string()),
        description: Some("Successfully created merge proposal".to_string()),
        merge_proposal_url: Some(
            "https://salsa.debian.org/jelmer/example-package/-/merge_requests/1".to_string(),
        ),
        timestamp: Utc::now() - chrono::Duration::hours(1),
        vcs_browse: Some("https://salsa.debian.org/jelmer/example-package".to_string()),
        rate_limited: false,
        retry_after: None,
    }];

    let stats = PublishStatistics {
        total_published: 150,
        successful_publishes: 142,
        failed_publishes: 8,
        rate_limited_publishes: 3,
        avg_publish_time: 45.5, // seconds
        active_publishers: 2,
        pending_queue_size: 12,
        rate_limit_status: {
            let mut limits = HashMap::new();
            limits.insert(
                "salsa.debian.org".to_string(),
                RateLimitInfo {
                    bucket: "salsa.debian.org".to_string(),
                    current_requests: 8,
                    max_requests: 10,
                    window_seconds: 3600,
                    reset_time: Some(Utc::now() + chrono::Duration::minutes(45)),
                    is_limited: false,
                },
            );
            limits
        },
    };

    Ok((items, stats))
}

async fn fetch_publish_history(
    state: &AppState,
    filters: &PublishFilters,
) -> anyhow::Result<Vec<PublishItem>> {
    // TODO: Implement publish history fetching
    // This would query the publish table with date filtering and pagination

    Ok(vec![PublishItem {
        id: "pub-hist-1".to_string(),
        codebase: "historical-package".to_string(),
        suite: "lintian-fixes".to_string(),
        branch_name: Some("debian/lintian-fixes".to_string()),
        mode: "push".to_string(),
        result_code: Some("success".to_string()),
        description: Some("Successfully pushed changes".to_string()),
        merge_proposal_url: None,
        timestamp: Utc::now() - chrono::Duration::days(1),
        vcs_browse: Some("https://salsa.debian.org/jelmer/historical-package".to_string()),
        rate_limited: false,
        retry_after: None,
    }])
}

async fn fetch_publish_details(state: &AppState, publish_id: &str) -> anyhow::Result<PublishItem> {
    // TODO: Implement detailed publish fetching
    // This would query the publish table for specific publish details

    Ok(PublishItem {
        id: publish_id.to_string(),
        codebase: "example-package".to_string(),
        suite: "lintian-fixes".to_string(),
        branch_name: Some("debian/lintian-fixes".to_string()),
        mode: "propose".to_string(),
        result_code: Some("success".to_string()),
        description: Some("Successfully created merge proposal".to_string()),
        merge_proposal_url: Some(
            "https://salsa.debian.org/jelmer/example-package/-/merge_requests/1".to_string(),
        ),
        timestamp: Utc::now() - chrono::Duration::hours(1),
        vcs_browse: Some("https://salsa.debian.org/jelmer/example-package".to_string()),
        rate_limited: false,
        retry_after: None,
    })
}

async fn fetch_ready_runs(
    state: &AppState,
    filters: &PublishFilters,
) -> anyhow::Result<Vec<ReadyRun>> {
    // TODO: Implement ready runs fetching
    // This would query the publish_ready table

    Ok(vec![ReadyRun {
        id: "run-ready-1".to_string(),
        codebase: "ready-package".to_string(),
        suite: "lintian-fixes".to_string(),
        command: Some("fix-lintian-issues".to_string()),
        result_code: "success".to_string(),
        publish_status: Some("ready".to_string()),
        value: Some(90),
        finish_time: Some(Utc::now() - chrono::Duration::minutes(30)),
        can_publish: true,
    }])
}

async fn execute_emergency_publish_action(
    state: &AppState,
    params: &EmergencyStopParams,
) -> anyhow::Result<EmergencyStopResult> {
    // TODO: Implement emergency publish actions
    // This would communicate with the publisher service to stop/resume publishing

    let affected_items = match params.action.as_str() {
        "stop_all" => 25,     // All pending publishes
        "stop_suite" => 8,    // Suite-specific publishes
        "stop_codebase" => 3, // Codebase-specific publishes
        "resume_all" => 25,   // Resume all stopped publishes
        _ => 0,
    };

    let resume_at = params
        .duration
        .map(|duration| Utc::now() + chrono::Duration::minutes(duration));

    Ok(EmergencyStopResult {
        action: params.action.clone(),
        affected_items,
        reason: params.reason.clone(),
        stopped_at: Utc::now(),
        resume_at,
    })
}

async fn apply_rate_limit_adjustment(
    state: &AppState,
    adjustment: &RateLimitAdjustment,
) -> anyhow::Result<RateLimitInfo> {
    // TODO: Implement rate limit adjustment
    // This would communicate with the publisher service to adjust rate limits

    let new_max_requests = match adjustment.action.as_str() {
        "increase" => adjustment.max_requests.unwrap_or(20),
        "decrease" => adjustment.max_requests.unwrap_or(5),
        "reset" => 10,     // Default limit
        "disable" => 1000, // Very high limit
        "enable" => 10,    // Standard limit
        _ => 10,
    };

    Ok(RateLimitInfo {
        bucket: adjustment.bucket.clone(),
        current_requests: 0, // Reset after adjustment
        max_requests: new_max_requests,
        window_seconds: adjustment.window_seconds.unwrap_or(3600),
        reset_time: Some(Utc::now() + chrono::Duration::hours(1)),
        is_limited: false,
    })
}

async fn fetch_publish_statistics(
    state: &AppState,
    filters: &PublishFilters,
) -> anyhow::Result<PublishStatistics> {
    // TODO: Implement comprehensive publish statistics
    // This would aggregate data from the publish table

    Ok(PublishStatistics {
        total_published: 1250,
        successful_publishes: 1180,
        failed_publishes: 70,
        rate_limited_publishes: 25,
        avg_publish_time: 42.3, // seconds
        active_publishers: 3,
        pending_queue_size: 18,
        rate_limit_status: {
            let mut limits = HashMap::new();
            limits.insert(
                "salsa.debian.org".to_string(),
                RateLimitInfo {
                    bucket: "salsa.debian.org".to_string(),
                    current_requests: 5,
                    max_requests: 10,
                    window_seconds: 3600,
                    reset_time: Some(Utc::now() + chrono::Duration::minutes(30)),
                    is_limited: false,
                },
            );
            limits.insert(
                "github.com".to_string(),
                RateLimitInfo {
                    bucket: "github.com".to_string(),
                    current_requests: 45,
                    max_requests: 50,
                    window_seconds: 3600,
                    reset_time: Some(Utc::now() + chrono::Duration::minutes(15)),
                    is_limited: true,
                },
            );
            limits
        },
    })
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
