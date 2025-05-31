use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Json, Response},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tera::Context;

use crate::{
    api::{negotiate_content_type, ContentType},
    app::AppState,
    auth::OptionalUser,
    database::DatabaseError,
    templates::{create_base_context, create_request_context, helpers::BaseContext},
};

#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FilterQuery {
    pub search: Option<String>,
    pub suite: Option<String>,
    pub campaign: Option<String>,
    pub result_code: Option<String>,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct IndexStatistics {
    pub total_packages: i64,
    pub active_runs: i64,
    pub queue_size: i64,
    pub recent_successful_runs: i64,
    pub total_campaigns: usize,
    pub workers_active: i64,
    pub workers_idle: i64,
}

/// Homepage handler - shows statistics and overview
pub async fn index(
    State(state): State<AppState>,
    OptionalUser(user_ctx): OptionalUser,
    headers: header::HeaderMap,
) -> Response {
    let mut context = create_base_context();
    
    // Add user context
    if let Some(user_ctx) = user_ctx {
        context.insert("user", &user_ctx.user());
        context.insert("is_admin", &user_ctx.is_admin());
        context.insert("is_qa_reviewer", &user_ctx.is_qa_reviewer());
    }

    // Fetch statistics from database
    let stats = match fetch_index_statistics(&state).await {
        Ok(stats) => stats,
        Err(e) => {
            tracing::error!("Failed to fetch statistics: {}", e);
            IndexStatistics {
                total_packages: 0,
                active_runs: 0,
                queue_size: 0,
                recent_successful_runs: 0,
                total_campaigns: 0,
                workers_active: 0,
                workers_idle: 0,
            }
        }
    };

    context.insert("stats", &stats);
    
    // Add campaign/suite information
    let campaigns: Vec<String> = state.config.campaigns.keys().cloned().collect();
    context.insert("campaigns", &campaigns);
    context.insert("suites", &campaigns); // Legacy compatibility

    // Content negotiation
    let content_type = negotiate_content_type(&headers, "index");
    
    match content_type {
        ContentType::Json => Json(stats).into_response(),
        _ => {
            match state.templates.render("index.html", &context) {
                Ok(html) => Html(html).into_response(),
                Err(e) => {
                    tracing::error!("Template rendering error: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                }
            }
        }
    }
}

/// About page handler
pub async fn about(
    State(state): State<AppState>,
    OptionalUser(user_ctx): OptionalUser,
) -> Response {
    let mut context = create_base_context();
    
    if let Some(user_ctx) = user_ctx {
        context.insert("user", &user_ctx.user());
        context.insert("is_admin", &user_ctx.is_admin());
        context.insert("is_qa_reviewer", &user_ctx.is_qa_reviewer());
    }

    // Add version information
    context.insert("version", env!("CARGO_PKG_VERSION"));
    context.insert("build_time", option_env!("BUILD_TIME").unwrap_or("unknown"));
    context.insert("git_revision", option_env!("GIT_REVISION").unwrap_or("unknown"));

    match state.templates.render("about.html", &context) {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("Template rendering error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// Credentials page - shows SSH/PGP keys
pub async fn credentials(
    State(state): State<AppState>,
    OptionalUser(user_ctx): OptionalUser,
) -> Response {
    let mut context = create_base_context();
    
    if let Some(user_ctx) = user_ctx {
        context.insert("user", &user_ctx.user());
        context.insert("is_admin", &user_ctx.is_admin());
        context.insert("is_qa_reviewer", &user_ctx.is_qa_reviewer());
    }

    // Fetch credentials from publisher service
    let credentials = match fetch_publisher_credentials(&state).await {
        Ok(creds) => creds,
        Err(e) => {
            tracing::error!("Failed to fetch credentials: {}", e);
            HashMap::new()
        }
    };

    context.insert("ssh_keys", &credentials.get("ssh_keys"));
    context.insert("pgp_keys", &credentials.get("pgp_keys"));
    context.insert("archive_keys", &credentials.get("archive_keys"));

    match state.templates.render("credentials.html", &context) {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("Template rendering error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// Archive keyring endpoints
pub async fn archive_keyring_asc(State(state): State<AppState>) -> Response {
    match fetch_archive_keyring(&state, "asc").await {
        Ok(keyring) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/pgp-keys")
            .header(header::CONTENT_DISPOSITION, "inline; filename=\"archive-keyring.asc\"")
            .body(keyring.into())
            .unwrap(),
        Err(e) => {
            tracing::error!("Failed to fetch archive keyring: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn archive_keyring_gpg(State(state): State<AppState>) -> Response {
    match fetch_archive_keyring(&state, "gpg").await {
        Ok(keyring) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/pgp-keys")
            .header(header::CONTENT_DISPOSITION, "inline; filename=\"archive-keyring.gpg\"")
            .body(keyring.into())
            .unwrap(),
        Err(e) => {
            tracing::error!("Failed to fetch archive keyring: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// Campaign/Suite-specific routes

/// Generic campaign start page
pub async fn campaign_start(
    State(state): State<AppState>,
    Path(campaign): Path<String>,
    OptionalUser(user_ctx): OptionalUser,
) -> Response {
    let mut context = create_base_context();
    
    if let Some(user_ctx) = user_ctx {
        context.insert("user", &user_ctx.user());
        context.insert("is_admin", &user_ctx.is_admin());
        context.insert("is_qa_reviewer", &user_ctx.is_qa_reviewer());
    }

    // Validate campaign exists
    if !state.config.campaigns.contains_key(&campaign) {
        return StatusCode::NOT_FOUND.into_response();
    }

    context.insert("campaign", &campaign);
    context.insert("suite", &campaign); // Legacy compatibility
    
    // Get campaign configuration
    let campaign_config = &state.config.campaigns[&campaign];
    context.insert("campaign_config", &campaign_config);

    // Fetch campaign statistics
    let stats = match fetch_campaign_statistics(&state, &campaign).await {
        Ok(stats) => stats,
        Err(e) => {
            tracing::error!("Failed to fetch campaign statistics: {}", e);
            HashMap::new()
        }
    };
    context.insert("stats", &stats);

    // Try suite-specific template first, fall back to generic
    let template_name = format!("{}/start.html", campaign);
    let html = match state.templates.render(&template_name, &context) {
        Ok(html) => html,
        Err(_) => {
            // Fall back to generic template
            match state.templates.render("generic/start.html", &context) {
                Ok(html) => html,
                Err(e) => {
                    tracing::error!("Template rendering error: {}", e);
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }
            }
        }
    };
    
    Html(html).into_response()
}

/// Codebase candidates listing
pub async fn campaign_candidates(
    State(state): State<AppState>,
    Path(suite): Path<String>,
    Query(pagination): Query<PaginationQuery>,
    Query(filter): Query<FilterQuery>,
    OptionalUser(user_ctx): OptionalUser,
) -> Response {
    let mut context = create_base_context();
    
    if let Some(user_ctx) = user_ctx {
        context.insert("user", &user_ctx.user());
        context.insert("is_admin", &user_ctx.is_admin());
        context.insert("is_qa_reviewer", &user_ctx.is_qa_reviewer());
    }

    if !state.config.campaigns.contains_key(&suite) {
        return StatusCode::NOT_FOUND.into_response();
    }

    context.insert("suite", &suite);
    context.insert("campaign", &suite);

    let page = pagination.page.unwrap_or(1) as i64;
    let per_page = pagination.per_page.unwrap_or(50) as i64;
    let offset = pagination.offset.unwrap_or_else(|| ((page - 1) * per_page) as u32) as i64;

    // Fetch candidates
    match fetch_candidates(&state, &suite, Some(per_page), Some(offset), &filter).await {
        Ok(candidates) => {
            context.insert("candidates", &candidates);
            
            // Get total count for pagination
            if let Ok(total) = count_candidates(&state, &suite, &filter).await {
                let total_pages = (total + per_page - 1) / per_page;
                context.insert("total_count", &total);
                context.insert("total_pages", &total_pages);
            }
        }
        Err(e) => {
            tracing::error!("Failed to fetch candidates: {}", e);
            context.insert("candidates", &Vec::<String>::new());
        }
    }

    context.insert("page", &page);
    context.insert("per_page", &per_page);
    context.insert("filter", &filter);

    // Try suite-specific template first, fall back to generic
    let template_name = format!("{}/candidates.html", suite);
    let html = match state.templates.render(&template_name, &context) {
        Ok(html) => html,
        Err(_) => {
            // Fall back to generic template
            match state.templates.render("generic/candidates.html", &context) {
                Ok(html) => html,
                Err(e) => {
                    tracing::error!("Template rendering error: {}", e);
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }
            }
        }
    };
    
    Html(html).into_response()
}

// Helper functions

async fn fetch_index_statistics(state: &AppState) -> anyhow::Result<IndexStatistics> {
    let mut stats = IndexStatistics {
        total_packages: 0,
        active_runs: 0, 
        queue_size: 0,
        recent_successful_runs: 0,
        total_campaigns: state.config.campaigns.len(),
        workers_active: 0,
        workers_idle: 0,
    };

    // Fetch from database
    if let Ok(db_stats) = state.database.get_stats().await {
        stats.total_packages = *db_stats.get("total_codebases").unwrap_or(&0) as i64;
        stats.active_runs = *db_stats.get("active_runs").unwrap_or(&0) as i64;
        stats.queue_size = *db_stats.get("queue_size").unwrap_or(&0) as i64;
        stats.recent_successful_runs = *db_stats.get("recent_successful_runs").unwrap_or(&0) as i64;
    }

    // Fetch worker stats from runner service
    if let Ok(worker_stats) = fetch_worker_stats(state).await {
        stats.workers_active = worker_stats.get("active").copied().unwrap_or(0);
        stats.workers_idle = worker_stats.get("idle").copied().unwrap_or(0);
    }

    Ok(stats)
}

async fn fetch_publisher_credentials(state: &AppState) -> anyhow::Result<HashMap<String, serde_json::Value>> {
    // TODO: Implement publisher service client
    Ok(HashMap::new())
}

async fn fetch_archive_keyring(state: &AppState, format: &str) -> anyhow::Result<Vec<u8>> {
    // TODO: Implement archive service client
    Ok(Vec::new())
}

async fn fetch_campaign_statistics(state: &AppState, campaign: &str) -> anyhow::Result<HashMap<String, i64>> {
    let mut stats = HashMap::new();
    
    // Fetch campaign-specific stats from database
    if let Ok(count) = state.database.count_candidates(campaign, None).await {
        stats.insert("total_candidates".to_string(), count);
    }
    
    if let Ok(count) = state.database.count_runs_by_result(campaign, "success").await {
        stats.insert("successful_runs".to_string(), count);
    }
    
    if let Ok(count) = state.database.count_pending_publishes(campaign).await {
        stats.insert("pending_publishes".to_string(), count);
    }
    
    Ok(stats)
}

async fn fetch_candidates(
    state: &AppState,
    suite: &str,
    limit: Option<i64>,
    offset: Option<i64>,
    filter: &FilterQuery,
) -> anyhow::Result<Vec<serde_json::Value>> {
    // TODO: Implement full candidate fetching with filtering
    state.database.get_candidates(suite, limit, offset).await.map_err(|e| anyhow::anyhow!(e))
}

async fn count_candidates(
    state: &AppState,
    suite: &str,
    filter: &FilterQuery,
) -> anyhow::Result<i64> {
    // TODO: Implement filtered counting
    state.database.count_candidates(suite, filter.search.as_deref()).await.map_err(|e| anyhow::anyhow!(e))
}

async fn fetch_worker_stats(state: &AppState) -> anyhow::Result<HashMap<String, i64>> {
    // TODO: Implement runner service client to get worker stats
    Ok(HashMap::new())
}