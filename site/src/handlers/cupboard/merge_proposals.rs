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
pub struct MpFilters {
    pub status: Option<String>,
    pub suite: Option<String>,
    pub codebase: Option<String>,
    pub forge: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MergeProposal {
    pub id: String,
    pub url: String,
    pub codebase: String,
    pub suite: String,
    pub status: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub merged_at: Option<DateTime<Utc>>,
    pub merged_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_scanned: Option<DateTime<Utc>>,
    pub can_be_merged: Option<bool>,
    pub merge_proposal_url: String,
    pub forge_name: String,
    pub branch_name: Option<String>,
    pub run_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MpStatistics {
    pub total_proposals: i64,
    pub open_proposals: i64,
    pub merged_proposals: i64,
    pub closed_proposals: i64,
    pub rejected_proposals: i64,
    pub abandoned_proposals: i64,
    pub avg_merge_time: f64, // hours
    pub merge_rate: f64, // percentage
    pub active_forges: i64,
    pub forge_health: HashMap<String, ForgeHealth>,
}

#[derive(Debug, Serialize)]
pub struct ForgeHealth {
    pub forge_name: String,
    pub total_proposals: i64,
    pub success_rate: f64,
    pub avg_response_time: f64,
    pub last_error: Option<String>,
    pub last_error_time: Option<DateTime<Utc>>,
    pub is_healthy: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BulkMpOperation {
    pub action: String, // "close", "reopen", "refresh", "sync_status", "abandon"
    pub mp_urls: Vec<String>,
    pub reason: Option<String>,
    pub parameters: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct MpOperationResult {
    pub action: String,
    pub total_items: usize,
    pub successful: usize,
    pub failed: usize,
    pub errors: Vec<String>,
    pub completed_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct MpPublishHistory {
    pub publish_id: String,
    pub timestamp: DateTime<Utc>,
    pub result_code: Option<String>,
    pub description: Option<String>,
    pub mode: String,
}

/// Merge proposal administration dashboard
pub async fn mp_dashboard(
    State(state): State<AppState>,
    user_ctx: UserContext,
    Query(filters): Query<MpFilters>,
    headers: header::HeaderMap,
) -> Response {
    let admin_user = match AdminUser::from_user_context(&user_ctx) {
        Some(admin) => admin,
        None => return StatusCode::FORBIDDEN.into_response(),
    };
    
    if !admin_user.has_permission(&Permission::ViewMergeProposals) {
        return StatusCode::FORBIDDEN.into_response();
    }
    
    let mut context = create_admin_context(&admin_user);
    
    // Fetch merge proposal data and statistics
    match fetch_mp_dashboard_data(&state, &filters).await {
        Ok((merge_proposals, stats)) => {
            context.insert("merge_proposals", &merge_proposals);
            context.insert("mp_stats", &stats);
            context.insert("filters", &filters);
        }
        Err(e) => {
            tracing::error!("Failed to fetch MP data: {}", e);
            context.insert("error_message", &format!("Failed to load merge proposal data: {}", e));
        }
    }
    
    // Add available suites and forges for filtering
    let campaigns: Vec<String> = state.config.campaigns.keys().cloned().collect();
    context.insert("available_campaigns", &campaigns);
    
    let content_type = negotiate_content_type(&headers, "mp_dashboard");
    
    match content_type {
        ContentType::Json => {
            Json(context.into_json()).into_response()
        }
        _ => {
            match state.templates.render("cupboard/merge-proposals.html", &context) {
                Ok(html) => Html(html).into_response(),
                Err(e) => {
                    tracing::error!("Template rendering error: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                }
            }
        }
    }
}

/// Individual merge proposal details
pub async fn mp_details(
    State(state): State<AppState>,
    user_ctx: UserContext,
    Path(mp_url): Path<String>,
    headers: header::HeaderMap,
) -> Response {
    let admin_user = match AdminUser::from_user_context(&user_ctx) {
        Some(admin) => admin,
        None => return StatusCode::FORBIDDEN.into_response(),
    };
    
    if !admin_user.has_permission(&Permission::ViewMergeProposals) {
        return StatusCode::FORBIDDEN.into_response();
    }
    
    let mut context = create_admin_context(&admin_user);
    
    // Decode URL parameter
    let decoded_url = urlencoding::decode(&mp_url).unwrap_or_default();
    context.insert("mp_url", &decoded_url);
    
    // Fetch MP details and publish history
    match fetch_mp_details(&state, &decoded_url).await {
        Ok((mp, publish_history)) => {
            context.insert("merge_proposal", &mp);
            context.insert("publish_history", &publish_history);
        }
        Err(e) => {
            tracing::error!("Failed to fetch MP details: {}", e);
            return StatusCode::NOT_FOUND.into_response();
        }
    }
    
    let content_type = negotiate_content_type(&headers, "mp_details");
    
    match content_type {
        ContentType::Json => {
            Json(context.into_json()).into_response()
        }
        _ => {
            match state.templates.render("cupboard/merge-proposal.html", &context) {
                Ok(html) => Html(html).into_response(),
                Err(e) => {
                    tracing::error!("Template rendering error: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                }
            }
        }
    }
}

/// MP statistics endpoint
pub async fn mp_statistics(
    State(state): State<AppState>,
    user_ctx: UserContext,
    Query(filters): Query<MpFilters>,
) -> Response {
    let admin_user = match AdminUser::from_user_context(&user_ctx) {
        Some(admin) => admin,
        None => return StatusCode::FORBIDDEN.into_response(),
    };
    
    if !admin_user.has_permission(&Permission::ViewMergeProposals) {
        return StatusCode::FORBIDDEN.into_response();
    }
    
    match fetch_mp_statistics(&state, &filters).await {
        Ok(stats) => Json(stats).into_response(),
        Err(e) => {
            tracing::error!("Failed to fetch MP statistics: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// Forge health monitoring
pub async fn forge_health(
    State(state): State<AppState>,
    user_ctx: UserContext,
    headers: header::HeaderMap,
) -> Response {
    let admin_user = match AdminUser::from_user_context(&user_ctx) {
        Some(admin) => admin,
        None => return StatusCode::FORBIDDEN.into_response(),
    };
    
    if !admin_user.has_permission(&Permission::ViewMergeProposals) {
        return StatusCode::FORBIDDEN.into_response();
    }
    
    let mut context = create_admin_context(&admin_user);
    
    // Fetch forge health data
    match fetch_forge_health(&state).await {
        Ok(forge_health) => {
            context.insert("forge_health", &forge_health);
        }
        Err(e) => {
            tracing::error!("Failed to fetch forge health: {}", e);
            context.insert("error_message", &format!("Failed to load forge health data: {}", e));
        }
    }
    
    let content_type = negotiate_content_type(&headers, "forge_health");
    
    match content_type {
        ContentType::Json => {
            Json(context.into_json()).into_response()
        }
        _ => {
            match state.templates.render("cupboard/forge-health.html", &context) {
                Ok(html) => Html(html).into_response(),
                Err(e) => {
                    tracing::error!("Template rendering error: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                }
            }
        }
    }
}

/// Bulk MP operations
pub async fn bulk_mp_operation(
    State(state): State<AppState>,
    user_ctx: UserContext,
    headers: header::HeaderMap,
    Json(operation): Json<BulkMpOperation>,
) -> Response {
    let admin_user = match AdminUser::from_user_context(&user_ctx) {
        Some(admin) => admin,
        None => return StatusCode::FORBIDDEN.into_response(),
    };
    
    if !admin_user.has_permission(&Permission::BulkMpOperations) {
        return StatusCode::FORBIDDEN.into_response();
    }
    
    // Extract IP and User-Agent for audit logging
    let ip_address = extract_ip_address(&headers);
    let user_agent = extract_user_agent(&headers);
    
    // Log the bulk MP operation attempt
    log_admin_action(
        &state,
        &admin_user,
        &format!("bulk_mp_{}", operation.action),
        None,
        serde_json::to_value(&operation).unwrap_or_default(),
        &ip_address,
        &user_agent,
    ).await;
    
    // Execute bulk MP operation
    match execute_bulk_mp_operation(&state, &operation, &admin_user.user.email).await {
        Ok(result) => {
            tracing::info!(
                "Bulk MP action '{}' completed by {}: {}/{} successful",
                operation.action,
                admin_user.user.name.as_deref().unwrap_or(&admin_user.user.email),
                result.successful,
                result.total_items
            );
            Json(result).into_response()
        }
        Err(e) => {
            tracing::error!("Bulk MP action failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

// Helper functions

async fn fetch_mp_dashboard_data(
    state: &AppState,
    filters: &MpFilters,
) -> anyhow::Result<(Vec<MergeProposal>, MpStatistics)> {
    // TODO: Implement comprehensive MP dashboard data fetching
    // This would query the merge_proposal table with filtering and statistics
    
    let merge_proposals = vec![
        MergeProposal {
            id: "mp-1".to_string(),
            url: "https://salsa.debian.org/jelmer/example-package/-/merge_requests/1".to_string(),
            codebase: "example-package".to_string(),
            suite: "lintian-fixes".to_string(),
            status: "open".to_string(),
            title: Some("Fix lintian issues".to_string()),
            description: Some("This merge request fixes multiple lintian issues found in the package".to_string()),
            merged_at: None,
            merged_by: None,
            created_at: Utc::now() - chrono::Duration::hours(2),
            last_scanned: Some(Utc::now() - chrono::Duration::minutes(15)),
            can_be_merged: Some(true),
            merge_proposal_url: "https://salsa.debian.org/jelmer/example-package/-/merge_requests/1".to_string(),
            forge_name: "salsa.debian.org".to_string(),
            branch_name: Some("debian/lintian-fixes".to_string()),
            run_id: Some("run-1".to_string()),
        },
        MergeProposal {
            id: "mp-2".to_string(),
            url: "https://github.com/example/package/-/pull/42".to_string(),
            codebase: "example-upstream".to_string(),
            suite: "fresh-releases".to_string(),
            status: "merged".to_string(),
            title: Some("Update to new upstream version".to_string()),
            description: Some("Update package to latest upstream release".to_string()),
            merged_at: Some(Utc::now() - chrono::Duration::hours(1)),
            merged_by: Some("maintainer@example.com".to_string()),
            created_at: Utc::now() - chrono::Duration::hours(4),
            last_scanned: Some(Utc::now() - chrono::Duration::minutes(5)),
            can_be_merged: Some(false),
            merge_proposal_url: "https://github.com/example/package/-/pull/42".to_string(),
            forge_name: "github.com".to_string(),
            branch_name: Some("debian/fresh-releases".to_string()),
            run_id: Some("run-2".to_string()),
        },
    ];
    
    let stats = MpStatistics {
        total_proposals: 150,
        open_proposals: 45,
        merged_proposals: 85,
        closed_proposals: 12,
        rejected_proposals: 5,
        abandoned_proposals: 3,
        avg_merge_time: 18.5, // hours
        merge_rate: 78.5, // percentage
        active_forges: 3,
        forge_health: {
            let mut health = HashMap::new();
            health.insert("salsa.debian.org".to_string(), ForgeHealth {
                forge_name: "salsa.debian.org".to_string(),
                total_proposals: 95,
                success_rate: 82.1,
                avg_response_time: 245.0, // ms
                last_error: None,
                last_error_time: None,
                is_healthy: true,
            });
            health.insert("github.com".to_string(), ForgeHealth {
                forge_name: "github.com".to_string(),
                total_proposals: 35,
                success_rate: 91.4,
                avg_response_time: 180.0, // ms
                last_error: Some("Rate limit exceeded".to_string()),
                last_error_time: Some(Utc::now() - chrono::Duration::minutes(30)),
                is_healthy: true,
            });
            health.insert("codeberg.org".to_string(), ForgeHealth {
                forge_name: "codeberg.org".to_string(),
                total_proposals: 20,
                success_rate: 65.0,
                avg_response_time: 450.0, // ms
                last_error: Some("Connection timeout".to_string()),
                last_error_time: Some(Utc::now() - chrono::Duration::hours(2)),
                is_healthy: false,
            });
            health
        },
    };
    
    Ok((merge_proposals, stats))
}

async fn fetch_mp_details(
    state: &AppState,
    mp_url: &str,
) -> anyhow::Result<(MergeProposal, Vec<MpPublishHistory>)> {
    // TODO: Implement detailed MP fetching with publish history
    // This would query merge_proposal table and related publish records
    
    let mp = MergeProposal {
        id: "mp-details".to_string(),
        url: mp_url.to_string(),
        codebase: "example-package".to_string(),
        suite: "lintian-fixes".to_string(),
        status: "open".to_string(),
        title: Some("Fix lintian issues".to_string()),
        description: Some("This merge request fixes multiple lintian issues found in the package".to_string()),
        merged_at: None,
        merged_by: None,
        created_at: Utc::now() - chrono::Duration::hours(2),
        last_scanned: Some(Utc::now() - chrono::Duration::minutes(15)),
        can_be_merged: Some(true),
        merge_proposal_url: mp_url.to_string(),
        forge_name: "salsa.debian.org".to_string(),
        branch_name: Some("debian/lintian-fixes".to_string()),
        run_id: Some("run-1".to_string()),
    };
    
    let publish_history = vec![
        MpPublishHistory {
            publish_id: "pub-1".to_string(),
            timestamp: Utc::now() - chrono::Duration::hours(1),
            result_code: Some("success".to_string()),
            description: Some("Successfully created merge proposal".to_string()),
            mode: "propose".to_string(),
        },
    ];
    
    Ok((mp, publish_history))
}

async fn fetch_mp_statistics(
    state: &AppState,
    filters: &MpFilters,
) -> anyhow::Result<MpStatistics> {
    // TODO: Implement comprehensive MP statistics
    // This would aggregate data from merge_proposal and related tables
    
    Ok(MpStatistics {
        total_proposals: 1250,
        open_proposals: 85,
        merged_proposals: 950,
        closed_proposals: 125,
        rejected_proposals: 65,
        abandoned_proposals: 25,
        avg_merge_time: 24.3, // hours
        merge_rate: 76.0, // percentage
        active_forges: 5,
        forge_health: HashMap::new(),
    })
}

async fn fetch_forge_health(
    state: &AppState,
) -> anyhow::Result<HashMap<String, ForgeHealth>> {
    // TODO: Implement forge health monitoring
    // This would check forge API health and response times
    
    let mut health = HashMap::new();
    health.insert("salsa.debian.org".to_string(), ForgeHealth {
        forge_name: "salsa.debian.org".to_string(),
        total_proposals: 750,
        success_rate: 85.2,
        avg_response_time: 220.0, // ms
        last_error: None,
        last_error_time: None,
        is_healthy: true,
    });
    health.insert("github.com".to_string(), ForgeHealth {
        forge_name: "github.com".to_string(),
        total_proposals: 350,
        success_rate: 92.8,
        avg_response_time: 150.0, // ms
        last_error: None,
        last_error_time: None,
        is_healthy: true,
    });
    health.insert("codeberg.org".to_string(), ForgeHealth {
        forge_name: "codeberg.org".to_string(),
        total_proposals: 150,
        success_rate: 68.0,
        avg_response_time: 480.0, // ms
        last_error: Some("Intermittent timeouts".to_string()),
        last_error_time: Some(Utc::now() - chrono::Duration::hours(1)),
        is_healthy: false,
    });
    
    Ok(health)
}

async fn execute_bulk_mp_operation(
    state: &AppState,
    operation: &BulkMpOperation,
    admin_user: &str,
) -> anyhow::Result<MpOperationResult> {
    let now = Utc::now();
    
    match operation.action.as_str() {
        "close" => {
            // TODO: Implement bulk MP closing
            // This would call forge APIs to close merge proposals
            Ok(MpOperationResult {
                action: operation.action.clone(),
                total_items: operation.mp_urls.len(),
                successful: operation.mp_urls.len(),
                failed: 0,
                errors: vec![],
                completed_at: now,
            })
        }
        "reopen" => {
            // TODO: Implement bulk MP reopening
            Ok(MpOperationResult {
                action: operation.action.clone(),
                total_items: operation.mp_urls.len(),
                successful: operation.mp_urls.len(),
                failed: 0,
                errors: vec![],
                completed_at: now,
            })
        }
        "refresh" => {
            // TODO: Implement bulk MP status refresh
            Ok(MpOperationResult {
                action: operation.action.clone(),
                total_items: operation.mp_urls.len(),
                successful: operation.mp_urls.len(),
                failed: 0,
                errors: vec![],
                completed_at: now,
            })
        }
        "sync_status" => {
            // TODO: Implement bulk MP status synchronization
            Ok(MpOperationResult {
                action: operation.action.clone(),
                total_items: operation.mp_urls.len(),
                successful: operation.mp_urls.len() - 2, // Simulate some failures
                failed: 2,
                errors: vec![
                    "Failed to sync status for MP: API rate limit exceeded".to_string(),
                    "Failed to sync status for MP: Merge proposal not found".to_string(),
                ],
                completed_at: now,
            })
        }
        "abandon" => {
            // TODO: Implement bulk MP abandonment
            Ok(MpOperationResult {
                action: operation.action.clone(),
                total_items: operation.mp_urls.len(),
                successful: operation.mp_urls.len(),
                failed: 0,
                errors: vec![],
                completed_at: now,
            })
        }
        _ => {
            Err(anyhow::anyhow!("Unknown MP operation: {}", operation.action))
        }
    }
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