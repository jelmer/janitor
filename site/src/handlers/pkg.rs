use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Json, Response},
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tera::Context;

use crate::{
    api::content_negotiation::{negotiate_content_type, ContentType},
    app::AppState,
    auth::OptionalUser,
    database::DatabaseError,
    templates::create_base_context,
};

#[derive(Debug, Deserialize)]
pub struct RunQuery {
    pub show_diff: Option<bool>,
    pub show_debdiff: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct CodebaseInfo {
    pub name: String,
    pub vcs_url: Option<String>,
    pub vcs_type: Option<String>,
    pub branch: Option<String>,
    pub maintainer: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RunInfo {
    pub id: String,
    pub codebase: String,
    pub suite: String,
    pub command: Option<String>,
    pub result_code: Option<String>,
    pub description: Option<String>,
    pub start_time: DateTime<Utc>,
    pub finish_time: DateTime<Utc>,
    pub worker: Option<String>,
    pub build_version: Option<String>,
    pub result_branches: Vec<serde_json::Value>,
    pub result_tags: Vec<serde_json::Value>,
    pub publish_status: Option<String>,
}

/// Generic codebase detail page
pub async fn codebase_detail(
    State(state): State<AppState>,
    Path((campaign, codebase)): Path<(String, String)>,
    Query(query): Query<RunQuery>,
    OptionalUser(user_ctx): OptionalUser,
    headers: header::HeaderMap,
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
    context.insert("suite", &campaign);
    context.insert("codebase", &codebase);

    // Generate full codebase context (matches Python generate_codebase_context)
    match generate_codebase_context(&state, &campaign, &codebase, &query, &user).await {
        Ok(codebase_context) => {
            // Merge the codebase context into the main context
            for (key, value) in codebase_context {
                context.insert(&key, &value);
            }
        }
        Err(e) => {
            tracing::error!("Failed to generate codebase context: {}", e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }

    // Content negotiation
    let content_type = negotiate_content_type(&headers, "codebase");
    
    match content_type {
        ContentType::Json => {
            // Return JSON representation
            let json_data = context.into_json();
            Json(json_data).into_response()
        }
        _ => {
            // Try suite-specific template first, fall back to generic
            let template_name = format!("{}/codebase.html", campaign);
            let html = match state.templates.render(&template_name, &context) {
                Ok(html) => html,
                Err(_) => {
                    // Fall back to generic template
                    match state.templates.render("generic/codebase.html", &context) {
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
    }
}

/// Generic run detail page
pub async fn run_detail(
    State(state): State<AppState>,
    Path((campaign, codebase, run_id)): Path<(String, String, String)>,
    Query(query): Query<RunQuery>,
    OptionalUser(user_ctx): OptionalUser,
    headers: header::HeaderMap,
) -> Response {
    let mut context = create_base_context();
    
    if let Some(user) = user.as_ref() {
        context.insert("user", &user);
        context.insert("is_admin", &user.is_admin());
        context.insert("is_qa_reviewer", &user.is_qa_reviewer());
    }

    // Validate campaign exists
    if !state.config.campaigns.contains_key(&campaign) {
        return StatusCode::NOT_FOUND.into_response();
    }

    context.insert("campaign", &campaign);
    context.insert("suite", &campaign);
    context.insert("codebase", &codebase);
    context.insert("run_id", &run_id);

    // Generate full run context (matches Python generate_run_file)
    match generate_run_file(&state, &campaign, &codebase, &run_id, &query, &user).await {
        Ok(run_context) => {
            // Merge the run context into the main context
            for (key, value) in run_context {
                context.insert(&key, &value);
            }
        }
        Err(e) => {
            tracing::error!("Failed to generate run context: {}", e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }

    // Content negotiation
    let content_type = negotiate_content_type(&headers, "run");
    
    match content_type {
        ContentType::Json => {
            // Return JSON representation
            let json_data = context.into_json();
            Json(json_data).into_response()
        }
        _ => {
            // Try suite-specific template first, fall back to generic
            let template_name = format!("{}/run.html", campaign);
            let html = match state.templates.render(&template_name, &context) {
                Ok(html) => html,
                Err(_) => {
                    // Fall back to generic template
                    match state.templates.render("generic/run.html", &context) {
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
    }
}

/// Ready list - runs ready for publishing
pub async fn ready_list(
    State(state): State<AppState>,
    Path(suite): Path<String>,
    Query(filter): Query<FilterQuery>,
    Query(pagination): Query<PaginationQuery>,
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

    // Fetch ready runs
    match generate_ready_list(&state, &suite, &filter, &pagination).await {
        Ok(ready_runs) => {
            context.insert("runs", &ready_runs);
        }
        Err(e) => {
            tracing::error!("Failed to fetch ready list: {}", e);
            context.insert("runs", &Vec::<String>::new());
        }
    }

    match state.templates.render("ready-list.html", &context) {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("Template rendering error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// Done list - completed/absorbed runs
pub async fn done_list(
    State(state): State<AppState>,
    Path(campaign): Path<String>,
    Query(filter): Query<FilterQuery>,
    Query(pagination): Query<PaginationQuery>,
    OptionalUser(user_ctx): OptionalUser,
) -> Response {
    let mut context = create_base_context();
    
    if let Some(user_ctx) = user_ctx {
        context.insert("user", &user_ctx.user());
        context.insert("is_admin", &user_ctx.is_admin());
        context.insert("is_qa_reviewer", &user_ctx.is_qa_reviewer());
    }

    if !state.config.campaigns.contains_key(&campaign) {
        return StatusCode::NOT_FOUND.into_response();
    }

    context.insert("campaign", &campaign);

    // Fetch done runs
    match generate_done_list(&state, &campaign, &filter, &pagination).await {
        Ok(done_runs) => {
            context.insert("runs", &done_runs);
        }
        Err(e) => {
            tracing::error!("Failed to fetch done list: {}", e);
            context.insert("runs", &Vec::<String>::new());
        }
    }

    // Try campaign-specific template first, fall back to generic
    let template_name = format!("{}/done.html", campaign);
    let html = match state.templates.render(&template_name, &context) {
        Ok(html) => html,
        Err(_) => {
            // Fall back to generic template
            match state.templates.render("generic/done.html", &context) {
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

/// Merge proposals listing
pub async fn merge_proposals(
    State(state): State<AppState>,
    Path(suite): Path<String>,
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

    // Fetch merge proposals by status
    match fetch_merge_proposals(&state, &suite).await {
        Ok(proposals) => {
            context.insert("open_proposals", &proposals.get("open").unwrap_or(&vec![]));
            context.insert("merged_proposals", &proposals.get("merged").unwrap_or(&vec![]));
            context.insert("closed_proposals", &proposals.get("closed").unwrap_or(&vec![]));
            context.insert("abandoned_proposals", &proposals.get("abandoned").unwrap_or(&vec![]));
            context.insert("rejected_proposals", &proposals.get("rejected").unwrap_or(&vec![]));
        }
        Err(e) => {
            tracing::error!("Failed to fetch merge proposals: {}", e);
        }
    }

    match state.templates.render("merge-proposals.html", &context) {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("Template rendering error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

// Helper types from simple.rs
use super::simple::{FilterQuery, PaginationQuery};

// Complex helper functions that match Python implementations

async fn generate_codebase_context(
    state: &AppState,
    campaign: &str,
    codebase: &str,
    query: &RunQuery,
    user: &Option<crate::auth::User>,
) -> anyhow::Result<HashMap<String, serde_json::Value>> {
    let mut context = HashMap::new();

    // Fetch candidate info
    let candidate = state.database.get_candidate(campaign, codebase).await?;
    context.insert("candidate".to_string(), serde_json::to_value(&candidate)?);

    // Fetch VCS info
    if let Ok(vcs_info) = state.database.get_vcs_info(codebase).await {
        context.insert("vcs_url".to_string(), serde_json::to_value(&vcs_info.url)?);
        context.insert("vcs_type".to_string(), serde_json::to_value(&vcs_info.vcs_type)?);
        context.insert("branch_url".to_string(), serde_json::to_value(&vcs_info.branch_url)?);
    }

    // Fetch last unabsorbed run
    match state.database.get_last_unabsorbed_run(campaign, codebase).await {
        Ok(run) => {
            context.insert("run".to_string(), serde_json::to_value(&run)?);
            context.insert("run_id".to_string(), serde_json::to_value(&run.id)?);
            context.insert("result_code".to_string(), serde_json::to_value(&run.result_code)?);
            
            // Check if we should show diff
            if query.show_diff.unwrap_or(false) && run.result_code == Some("success".to_string()) {
                if let Ok(diff) = fetch_diff(&state, &run.id).await {
                    context.insert("diff".to_string(), serde_json::to_value(&diff)?);
                }
            }
            
            // Check if we should show debdiff
            if query.show_debdiff.unwrap_or(false) {
                if let Ok(debdiff) = fetch_debdiff(&state, &run.id).await {
                    context.insert("debdiff".to_string(), serde_json::to_value(&debdiff)?);
                }
            }
        }
        Err(_) => {
            // No unabsorbed run found
            context.insert("run".to_string(), serde_json::Value::Null);
        }
    }

    // Fetch previous runs
    if let Ok(previous_runs) = state.database.get_previous_runs(codebase, campaign, Some(10)).await {
        context.insert("previous_runs".to_string(), serde_json::to_value(&previous_runs)?);
    }

    // Fetch merge proposals
    if let Ok(merge_proposals) = state.database.get_merge_proposals_for_codebase(campaign, codebase).await {
        context.insert("merge_proposals".to_string(), serde_json::to_value(&merge_proposals)?);
    }

    // Check queue position
    if let Ok(queue_position) = state.database.get_queue_position(campaign, codebase).await {
        context.insert("queue_position".to_string(), serde_json::to_value(&queue_position)?);
        
        // Estimate wait time
        if queue_position > 0 {
            if let Ok(avg_time) = state.database.get_average_run_time(campaign).await {
                let wait_seconds = queue_position as i64 * avg_time;
                let wait_duration = Duration::seconds(wait_seconds);
                context.insert("queue_wait_time".to_string(), serde_json::to_value(&wait_duration)?);
            }
        }
    }

    // Add publish policy
    if let Ok(publish_policy) = state.database.get_publish_policy(campaign, codebase).await {
        context.insert("publish_policy".to_string(), serde_json::to_value(&publish_policy)?);
    }

    // Add changelog policy
    if let Ok(changelog_policy) = state.database.get_changelog_policy(campaign, codebase).await {
        context.insert("changelog_policy".to_string(), serde_json::to_value(&changelog_policy)?);
    }

    Ok(context)
}

async fn generate_run_file(
    state: &AppState,
    campaign: &str,
    codebase: &str,
    run_id: &str,
    query: &RunQuery,
    user: &Option<crate::auth::User>,
) -> anyhow::Result<HashMap<String, serde_json::Value>> {
    let mut context = HashMap::new();

    // This is the most complex function - it aggregates all data for a run detail page
    
    // Fetch the run
    let run = state.database.get_run(run_id).await?;
    context.insert("run".to_string(), serde_json::to_value(&run)?);
    
    // Verify run belongs to this campaign/codebase
    if run.suite != campaign || run.codebase != codebase {
        return Err(anyhow::anyhow!("Run does not match campaign/codebase"));
    }

    // Basic run info
    context.insert("result_code".to_string(), serde_json::to_value(&run.result_code)?);
    context.insert("description".to_string(), serde_json::to_value(&run.description)?);
    context.insert("command".to_string(), serde_json::to_value(&run.command)?);
    context.insert("worker_name".to_string(), serde_json::to_value(&run.worker)?);
    
    // Calculate success probability
    if let Ok(stats) = state.database.get_run_statistics(campaign, codebase).await {
        let success_probability = stats.successful as f64 / stats.total as f64;
        context.insert("success_probability".to_string(), serde_json::to_value(&success_probability)?);
        context.insert("total_previous_runs".to_string(), serde_json::to_value(&stats.total)?);
    }

    // Fetch logs
    let mut logs = HashMap::new();
    for log_name in ["worker", "codemod", "build", "dist"] {
        if let Ok(log_info) = fetch_log_info(&state, run_id, log_name).await {
            logs.insert(log_name.to_string(), log_info);
        }
    }
    context.insert("logs".to_string(), serde_json::to_value(&logs)?);
    
    // Determine primary log to display
    let primary_log = determine_primary_log(&run.result_code, &run.failure_stage);
    context.insert("primary_log".to_string(), serde_json::to_value(&primary_log)?);

    // Analyze logs for failure information
    if let Some(failure_stage) = &run.failure_stage {
        if let Ok(failure_info) = analyze_log_failure(&state, run_id, failure_stage).await {
            context.insert("failure_info".to_string(), serde_json::to_value(&failure_info)?);
        }
    }

    // Fetch unchanged/control run
    if let Ok(unchanged_run) = state.database.get_unchanged_run(campaign, codebase, Some(&run.start_time)).await {
        context.insert("unchanged_run".to_string(), serde_json::to_value(&unchanged_run)?);
    }

    // Queue position
    if let Ok(queue_position) = state.database.get_queue_position(campaign, codebase).await {
        context.insert("queue_position".to_string(), serde_json::to_value(&queue_position)?);
    }

    // VCS info
    if let Ok(vcs_info) = state.database.get_vcs_info(codebase).await {
        context.insert("vcs_url".to_string(), serde_json::to_value(&vcs_info.url)?);
        context.insert("vcs_type".to_string(), serde_json::to_value(&vcs_info.vcs_type)?);
        context.insert("branch_url".to_string(), serde_json::to_value(&vcs_info.branch_url)?);
    }

    // Diff/debdiff
    if query.show_diff.unwrap_or(true) && run.result_code == Some("success".to_string()) {
        if let Ok(diff) = fetch_diff(&state, run_id).await {
            context.insert("diff".to_string(), serde_json::to_value(&diff)?);
        }
    }
    
    if query.show_debdiff.unwrap_or(true) {
        if let Ok(debdiff) = fetch_debdiff(&state, run_id).await {
            context.insert("debdiff".to_string(), serde_json::to_value(&debdiff)?);
        }
    }

    // Publish history
    if let Some(revision) = &run.main_branch_revision {
        if let Ok(publish_history) = get_publish_history(&state, revision).await {
            context.insert("publish_history".to_string(), serde_json::to_value(&publish_history)?);
        }
    }

    // Binary packages
    if let Ok(binary_packages) = state.database.get_binary_packages(run_id).await {
        context.insert("binary_packages".to_string(), serde_json::to_value(&binary_packages)?);
    }

    // Lintian result
    if let Ok(lintian_result) = fetch_lintian_result(&state, run_id).await {
        context.insert("lintian_result".to_string(), serde_json::to_value(&lintian_result)?);
    }

    // Reviews
    if let Ok(reviews) = state.database.get_reviews(run_id).await {
        context.insert("reviews".to_string(), serde_json::to_value(&reviews)?);
    }

    Ok(context)
}

async fn generate_ready_list(
    state: &AppState,
    suite: &str,
    filter: &FilterQuery,
    pagination: &PaginationQuery,
) -> anyhow::Result<Vec<serde_json::Value>> {
    let limit = pagination.per_page.unwrap_or(50) as i64;
    let offset = pagination.offset.unwrap_or(0) as i64;
    
    state.database.get_ready_runs(
        suite,
        filter.search.as_deref(),
        filter.result_code.as_deref(),
        Some(limit),
        Some(offset),
    ).await
}

async fn generate_done_list(
    state: &AppState,
    campaign: &str,
    filter: &FilterQuery,
    pagination: &PaginationQuery,
) -> anyhow::Result<Vec<serde_json::Value>> {
    let limit = pagination.per_page.unwrap_or(50) as i64;
    let offset = pagination.offset.unwrap_or(0) as i64;
    
    // Parse date filters
    let from_date = filter.from_date.as_ref()
        .and_then(|d| DateTime::parse_from_rfc3339(d).ok())
        .map(|d| d.with_timezone(&Utc));
    
    let to_date = filter.to_date.as_ref()
        .and_then(|d| DateTime::parse_from_rfc3339(d).ok())
        .map(|d| d.with_timezone(&Utc));
    
    state.database.get_absorbed_runs(
        campaign,
        from_date.as_ref(),
        to_date.as_ref(),
        Some(limit),
        Some(offset),
    ).await
}

async fn fetch_merge_proposals(
    state: &AppState,
    suite: &str,
) -> anyhow::Result<HashMap<String, Vec<serde_json::Value>>> {
    let mut proposals = HashMap::new();
    
    // Fetch proposals by status
    for status in ["open", "merged", "closed", "abandoned", "rejected"] {
        if let Ok(props) = state.database.get_merge_proposals_by_status(suite, status).await {
            proposals.insert(status.to_string(), props);
        }
    }
    
    Ok(proposals)
}

// Service client functions

async fn fetch_diff(state: &AppState, run_id: &str) -> anyhow::Result<String> {
    // TODO: Implement differ service client
    Ok(String::new())
}

async fn fetch_debdiff(state: &AppState, run_id: &str) -> anyhow::Result<String> {
    // TODO: Implement differ service client  
    Ok(String::new())
}

async fn fetch_log_info(state: &AppState, run_id: &str, log_name: &str) -> anyhow::Result<serde_json::Value> {
    // TODO: Implement log file manager
    Ok(serde_json::json!({
        "name": log_name,
        "url": format!("/logs/{}/{}", run_id, log_name),
        "size": 0,
        "exists": false
    }))
}

async fn analyze_log_failure(state: &AppState, run_id: &str, failure_stage: &str) -> anyhow::Result<serde_json::Value> {
    // TODO: Implement log analysis
    Ok(serde_json::json!({
        "stage": failure_stage,
        "lines": []
    }))
}

async fn get_publish_history(state: &AppState, revision: &str) -> anyhow::Result<Vec<serde_json::Value>> {
    // TODO: Implement publish history fetching
    Ok(vec![])
}

async fn fetch_lintian_result(state: &AppState, run_id: &str) -> anyhow::Result<serde_json::Value> {
    // TODO: Implement lintian result fetching
    Ok(serde_json::Value::Null)
}

fn determine_primary_log(result_code: &Option<String>, failure_stage: &Option<String>) -> String {
    match (result_code.as_deref(), failure_stage.as_deref()) {
        (Some("success"), _) => "worker".to_string(),
        (_, Some(stage)) if stage.starts_with("build") => "build".to_string(),
        (_, Some(stage)) if stage.starts_with("dist") => "dist".to_string(),
        (_, Some(stage)) if stage.starts_with("codemod") => "codemod".to_string(),
        _ => "worker".to_string(),
    }
}