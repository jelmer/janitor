use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Json, Response},
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::HashMap;
use tera::Context;
use tracing::instrument;
use url::Url;

use crate::{
    api::{negotiate_content_type, ContentType},
    app::AppState,
    auth::OptionalUser,
    database::{DatabaseError, RunDetails, VcsInfo},
    templates::create_base_context,
};

// Constants matching Python implementation
const FAIL_BUILD_LOG_LEN: usize = 15;
const BUILD_LOG_FILENAME: &str = "build.log";
const DIST_LOG_FILENAME: &str = "dist.log";
const WORKER_LOG_FILENAME: &str = "worker.log";
const CODEMOD_LOG_FILENAME: &str = "codemod.log";

#[derive(Debug, Deserialize)]
pub struct RunQuery {
    pub show_diff: Option<bool>,
    pub show_debdiff: Option<bool>,
    pub show_diffoscope: Option<bool>,
    pub line_start: Option<usize>,
    pub line_end: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct LogQuery {
    pub offset: Option<usize>,
    pub limit: Option<usize>,
    pub filter: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LogInfo {
    pub name: String,
    pub exists: bool,
    pub size: Option<i64>,
    pub url: String,
    pub line_count: Option<usize>,
    pub include_lines: Option<(usize, usize)>,
    pub highlight_lines: Option<Vec<usize>>,
}

#[derive(Debug, Serialize)]
pub struct DiffInfo {
    pub diff_type: String,
    pub content: String,
    pub content_type: String,
    pub base_revision: Option<String>,
    pub new_revision: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PublishHistory {
    pub mode: String,
    pub merge_proposal_url: Option<String>,
    pub description: Option<String>,
    pub result_code: Option<String>,
    pub requester: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct FailureAnalysis {
    pub primary_log: String,
    pub failure_stage: Option<String>,
    pub line_count: usize,
    pub include_lines: Option<(usize, usize)>,
    pub highlight_lines: Vec<usize>,
    pub error_summary: Option<String>,
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

    if let Some(user_ctx) = user_ctx.as_ref() {
        context.insert("user", user_ctx.user());
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
    match generate_codebase_context(&state, &campaign, &codebase, &query, &user_ctx).await {
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

    if let Some(user_ctx) = user_ctx.as_ref() {
        context.insert("user", user_ctx.user());
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
    context.insert("run_id", &run_id);

    // Generate full run context (matches Python generate_run_file)
    match generate_run_file(&state, &campaign, &codebase, &run_id, &query, &user_ctx).await {
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

/// Log file viewer - displays individual log files with syntax highlighting
pub async fn view_log(
    State(state): State<AppState>,
    Path((campaign, codebase, run_id, log_name)): Path<(String, String, String, String)>,
    Query(query): Query<LogQuery>,
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
    context.insert("run_id", &run_id);
    context.insert("log_name", &log_name);

    // Fetch log content and metadata
    match get_log_content(&state, &run_id, &log_name, &query).await {
        Ok(log_info) => {
            context.insert("log_info", &log_info);

            // If log exists, fetch actual content
            if log_info.exists {
                match download_log_file(&state, &run_id, &log_name).await {
                    Ok(content) => {
                        let log_text = String::from_utf8_lossy(&content);
                        context.insert("log_content", &log_text);

                        // Add line filtering if requested
                        if let Some(start) = query.offset {
                            context.insert("line_start", &start);
                        }
                        if let Some(limit) = query.limit {
                            context.insert("line_end", &(query.offset.unwrap_or(0) + limit));
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to download log content: {}", e);
                        context.insert(
                            "error_message",
                            &format!("Failed to load log content: {}", e),
                        );
                    }
                }
            }

            // Content negotiation for raw vs formatted
            let content_type = negotiate_content_type(&headers, "log");

            match content_type {
                ContentType::Json => Json(serde_json::to_value(&log_info).unwrap()).into_response(),
                _ => {
                    // HTML view with syntax highlighting
                    match state.templates.render("log-viewer.html", &context) {
                        Ok(html) => Html(html).into_response(),
                        Err(e) => {
                            tracing::error!("Template rendering error: {}", e);
                            StatusCode::INTERNAL_SERVER_ERROR.into_response()
                        }
                    }
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to get log content: {}", e);
            context.insert("error_message", &format!("Log not found: {}", e));
            match state.templates.render("log-viewer.html", &context) {
                Ok(html) => Html(html).into_response(),
                Err(_) => StatusCode::NOT_FOUND.into_response(),
            }
        }
    }
}

/// Download raw log file
pub async fn download_log(
    State(state): State<AppState>,
    Path((campaign, codebase, run_id, log_name)): Path<(String, String, String, String)>,
    OptionalUser(user_ctx): OptionalUser,
) -> Response {
    // Validate campaign exists
    if !state.config.campaigns.contains_key(&campaign) {
        return StatusCode::NOT_FOUND.into_response();
    }

    // Check permissions for sensitive logs
    let requires_admin = matches!(log_name.as_str(), "worker.log" | "debug.log");
    if requires_admin {
        if let Some(user_ctx) = user_ctx {
            if !user_ctx.is_admin() {
                return StatusCode::FORBIDDEN.into_response();
            }
        } else {
            return StatusCode::UNAUTHORIZED.into_response();
        }
    }

    match download_log_file(&state, &run_id, &log_name).await {
        Ok(log_data) => {
            let headers = [
                (header::CONTENT_TYPE, "text/plain; charset=utf-8"),
                (
                    header::CONTENT_DISPOSITION,
                    &format!("attachment; filename=\"{}-{}\"", run_id, log_name),
                ),
            ];
            (headers, log_data).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to download log: {}", e);
            StatusCode::NOT_FOUND.into_response()
        }
    }
}

/// Diff viewer - displays VCS diffs between revisions
pub async fn view_diff(
    State(state): State<AppState>,
    Path((campaign, codebase, run_id)): Path<(String, String, String)>,
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
    context.insert("run_id", &run_id);

    // Generate diff content
    match generate_diff_content(&state, &run_id, &query).await {
        Ok(diff_info) => {
            context.insert("diff_info", &diff_info);

            let content_type = negotiate_content_type(&headers, "diff");

            match content_type {
                ContentType::Json => {
                    Json(serde_json::to_value(&diff_info).unwrap()).into_response()
                }
                _ => match state.templates.render("diff-viewer.html", &context) {
                    Ok(html) => Html(html).into_response(),
                    Err(e) => {
                        tracing::error!("Template rendering error: {}", e);
                        StatusCode::INTERNAL_SERVER_ERROR.into_response()
                    }
                },
            }
        }
        Err(e) => {
            tracing::error!("Failed to generate diff: {}", e);
            StatusCode::NOT_FOUND.into_response()
        }
    }
}

/// Debdiff viewer - displays Debian package differences
pub async fn view_debdiff(
    State(state): State<AppState>,
    Path((campaign, codebase, run_id)): Path<(String, String, String)>,
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
    context.insert("run_id", &run_id);

    // Generate debdiff content using differ service
    match generate_debdiff_content(&state, &run_id, &query).await {
        Ok(diff_info) => {
            context.insert("debdiff_info", &diff_info);

            let content_type = negotiate_content_type(&headers, "debdiff");

            match content_type {
                ContentType::Json => {
                    Json(serde_json::to_value(&diff_info).unwrap()).into_response()
                }
                _ => match state.templates.render("debdiff-viewer.html", &context) {
                    Ok(html) => Html(html).into_response(),
                    Err(e) => {
                        tracing::error!("Template rendering error: {}", e);
                        StatusCode::INTERNAL_SERVER_ERROR.into_response()
                    }
                },
            }
        }
        Err(e) => {
            tracing::error!("Failed to generate debdiff: {}", e);
            StatusCode::NOT_FOUND.into_response()
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
            context.insert(
                "merged_proposals",
                &proposals.get("merged").unwrap_or(&vec![]),
            );
            context.insert(
                "closed_proposals",
                &proposals.get("closed").unwrap_or(&vec![]),
            );
            context.insert(
                "abandoned_proposals",
                &proposals.get("abandoned").unwrap_or(&vec![]),
            );
            context.insert(
                "rejected_proposals",
                &proposals.get("rejected").unwrap_or(&vec![]),
            );
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

#[instrument(skip(state, user_ctx))]
async fn generate_codebase_context(
    state: &AppState,
    campaign: &str,
    codebase: &str,
    query: &RunQuery,
    user_ctx: &Option<crate::auth::UserContext>,
) -> anyhow::Result<HashMap<String, serde_json::Value>> {
    let mut context = HashMap::new();

    // Fetch candidate info
    let candidate = state.database.get_candidate(campaign, codebase).await?;
    context.insert("candidate".to_string(), serde_json::to_value(&candidate)?);

    // Fetch VCS info
    if let Ok(vcs_info) = state.database.get_vcs_info(codebase).await {
        context.insert("vcs_url".to_string(), serde_json::to_value(&vcs_info.url)?);
        context.insert(
            "vcs_type".to_string(),
            serde_json::to_value(&vcs_info.vcs_type)?,
        );
        context.insert(
            "branch_url".to_string(),
            serde_json::to_value(&vcs_info.branch_url)?,
        );
    }

    // Fetch last unabsorbed run
    match state
        .database
        .get_last_unabsorbed_run(campaign, codebase)
        .await
    {
        Ok(run) => {
            context.insert("run".to_string(), serde_json::to_value(&run)?);
            context.insert("run_id".to_string(), serde_json::to_value(&run.id)?);
            context.insert(
                "result_code".to_string(),
                serde_json::to_value(&run.result_code)?,
            );

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
    if let Ok(previous_runs) = state
        .database
        .get_previous_runs(codebase, campaign, Some(10))
        .await
    {
        context.insert(
            "previous_runs".to_string(),
            serde_json::to_value(&previous_runs)?,
        );
    }

    // Fetch merge proposals
    if let Ok(merge_proposals) = state
        .database
        .get_merge_proposals_for_codebase(campaign, codebase)
        .await
    {
        context.insert(
            "merge_proposals".to_string(),
            serde_json::to_value(&merge_proposals)?,
        );
    }

    // Check queue position
    if let Ok(queue_position) = state.database.get_queue_position(campaign, codebase).await {
        context.insert(
            "queue_position".to_string(),
            serde_json::to_value(&queue_position)?,
        );

        // Estimate wait time
        if queue_position > 0 {
            if let Ok(avg_time) = state.database.get_average_run_time(campaign).await {
                let wait_seconds = queue_position as i64 * avg_time;
                let wait_duration = Duration::seconds(wait_seconds);
                context.insert(
                    "queue_wait_time".to_string(),
                    serde_json::to_value(&wait_duration)?,
                );
            }
        }
    }

    // Add publish policy
    if let Ok(publish_policy) = state.database.get_publish_policy(campaign, codebase).await {
        context.insert(
            "publish_policy".to_string(),
            serde_json::to_value(&publish_policy)?,
        );
    }

    // Add changelog policy
    if let Ok(changelog_policy) = state
        .database
        .get_changelog_policy(campaign, codebase)
        .await
    {
        context.insert(
            "changelog_policy".to_string(),
            serde_json::to_value(&changelog_policy)?,
        );
    }

    Ok(context)
}

#[instrument(skip(state, user_ctx))]
async fn generate_run_file(
    state: &AppState,
    campaign: &str,
    codebase: &str,
    run_id: &str,
    query: &RunQuery,
    user_ctx: &Option<crate::auth::UserContext>,
) -> anyhow::Result<HashMap<String, serde_json::Value>> {
    let mut context = HashMap::new();

    // This is the most complex function - aggregates all data for a run detail page
    // and matches the Python generate_run_file implementation exactly

    // Fetch the run
    let run = state.database.get_run(run_id).await?;
    context.insert("run".to_string(), serde_json::to_value(&run)?);
    context.insert("run_id".to_string(), serde_json::to_value(run_id)?);

    // Verify run belongs to this campaign/codebase
    if run.suite != campaign || run.codebase != codebase {
        return Err(anyhow::anyhow!("Run does not match campaign/codebase"));
    }

    // Add basic run metadata
    context.insert("suite".to_string(), serde_json::to_value(&run.suite)?);
    context.insert("codebase".to_string(), serde_json::to_value(&run.codebase)?);
    context.insert(
        "resume_from".to_string(),
        serde_json::to_value(&run.failure_stage)?,
    );

    // Add user context
    if let Some(user_ctx) = user_ctx {
        context.insert(
            "is_admin".to_string(),
            serde_json::to_value(user_ctx.is_admin())?,
        );
    } else {
        context.insert("is_admin".to_string(), serde_json::to_value(false)?);
    }

    // Calculate success probability and stats (matches Python logic)
    if let Ok(stats) = state.database.get_run_statistics(campaign, codebase).await {
        let success_probability = if stats.total > 0 {
            stats.successful as f64 / stats.total as f64
        } else {
            0.0
        };
        context.insert(
            "success_probability".to_string(),
            serde_json::to_value(&success_probability)?,
        );
        context.insert(
            "total_previous_runs".to_string(),
            serde_json::to_value(&stats.total)?,
        );
    }

    // Analyze logs and determine primary log (matches Python logic)
    let primary_log = determine_primary_log(&run.result_code, &run.failure_stage);
    context.insert(
        "primary_log".to_string(),
        serde_json::to_value(&primary_log)?,
    );

    // Add log information for each log type
    let log_names = [
        BUILD_LOG_FILENAME,
        DIST_LOG_FILENAME,
        WORKER_LOG_FILENAME,
        CODEMOD_LOG_FILENAME,
    ];
    for &log_name in &log_names {
        if let Ok(log_info) = get_log_content(
            state,
            run_id,
            log_name,
            &LogQuery {
                offset: None,
                limit: None,
                filter: None,
            },
        )
        .await
        {
            if log_info.exists {
                let key = format!("{}_log_name", log_name.replace(".log", ""));
                context.insert(key, serde_json::to_value(log_name)?);

                // Add failure analysis for primary logs
                if log_name == BUILD_LOG_FILENAME || log_name == DIST_LOG_FILENAME {
                    if let Some(line_count) = log_info.line_count {
                        let key_count = format!("{}_log_line_count", log_name.replace(".log", ""));
                        context.insert(key_count, serde_json::to_value(line_count)?);
                    }
                    if let Some(include_lines) = log_info.include_lines {
                        let key_include =
                            format!("{}_log_include_lines", log_name.replace(".log", ""));
                        context.insert(key_include, serde_json::to_value(include_lines)?);
                    }
                    if let Some(ref highlight_lines) = log_info.highlight_lines {
                        let key_highlight =
                            format!("{}_log_highlight_lines", log_name.replace(".log", ""));
                        context.insert(key_highlight, serde_json::to_value(highlight_lines)?);
                    }
                }
            }
        }
    }

    // Add unchanged run for comparison (matches Python logic)
    if let Ok(unchanged_run) = state
        .database
        .get_unchanged_run(campaign, codebase, Some(&run.start_time))
        .await
    {
        context.insert(
            "unchanged_run".to_string(),
            serde_json::to_value(&unchanged_run)?,
        );
    }

    // VCS diff and debdiff generation (matches Python show_diff and show_debdiff functions)
    if query.show_diff.unwrap_or(true) && run.result_code == Some("success".to_string()) {
        if let Ok(diff) = fetch_diff(state, run_id).await {
            context.insert("diff".to_string(), serde_json::to_value(&diff)?);
        }
    }

    if query.show_debdiff.unwrap_or(true) {
        if let Ok(debdiff) = fetch_debdiff(state, run_id).await {
            context.insert("debdiff".to_string(), serde_json::to_value(&debdiff)?);
        }
    }

    // Publish history (matches Python get_publish_history)
    if let Some(revision) = &run.main_branch_revision {
        if let Ok(publish_history) = get_publish_history(state, revision).await {
            context.insert(
                "publish_history".to_string(),
                serde_json::to_value(&publish_history)?,
            );
        }
    }

    // Reviews
    if let Ok(reviews) = state.database.get_reviews(run_id).await {
        context.insert("reviews".to_string(), serde_json::to_value(&reviews)?);
    }

    // Queue position and wait time
    if let Ok(queue_position) = state.database.get_queue_position(campaign, codebase).await {
        context.insert(
            "queue_position".to_string(),
            serde_json::to_value(&queue_position)?,
        );

        if queue_position > 0 {
            if let Ok(avg_time) = state.database.get_average_run_time(campaign).await {
                let wait_seconds = queue_position as i64 * avg_time;
                context.insert(
                    "queue_wait_time".to_string(),
                    serde_json::to_value(&wait_seconds)?,
                );
            }
        }
    }

    // Binary packages and lintian results
    if let Ok(binary_packages) = state.database.get_binary_packages(run_id).await {
        context.insert(
            "binary_packages".to_string(),
            serde_json::to_value(&binary_packages)?,
        );
    }

    if let Ok(lintian_result) = fetch_lintian_result(state, run_id).await {
        context.insert(
            "lintian_result".to_string(),
            serde_json::to_value(&lintian_result)?,
        );
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

    state
        .database
        .get_ready_runs(
            suite,
            filter.search.as_deref(),
            filter.result_code.as_deref(),
            Some(limit),
            Some(offset),
        )
        .await
        .map_err(|e| anyhow::anyhow!(e))
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
    let from_date = filter
        .from_date
        .as_ref()
        .and_then(|d| DateTime::parse_from_rfc3339(d).ok())
        .map(|d| d.with_timezone(&Utc));

    let to_date = filter
        .to_date
        .as_ref()
        .and_then(|d| DateTime::parse_from_rfc3339(d).ok())
        .map(|d| d.with_timezone(&Utc));

    state
        .database
        .get_absorbed_runs(
            campaign,
            from_date.as_ref(),
            to_date.as_ref(),
            Some(limit),
            Some(offset),
        )
        .await
        .map_err(|e| anyhow::anyhow!(e))
}

async fn fetch_merge_proposals(
    state: &AppState,
    suite: &str,
) -> anyhow::Result<HashMap<String, Vec<serde_json::Value>>> {
    let mut proposals = HashMap::new();

    // Optimize: Single query instead of N+1 pattern (one query per status)
    // Fetch all proposals for all statuses in one database call
    match state
        .database
        .get_merge_proposals_by_statuses(
            suite, 
            &["open", "merged", "closed", "abandoned", "rejected"]
        )
        .await 
    {
        Ok(grouped_proposals) => {
            proposals = grouped_proposals;
        }
        Err(_) => {
            // Fallback to individual queries if batch method not available
            for status in ["open", "merged", "closed", "abandoned", "rejected"] {
                if let Ok(props) = state
                    .database
                    .get_merge_proposals_by_status(suite, status)
                    .await
                {
                    proposals.insert(status.to_string(), props);
                }
            }
        }
    }

    // Ensure all statuses have entries (empty vectors for statuses with no proposals)
    for status in ["open", "merged", "closed", "abandoned", "rejected"] {
        proposals.entry(status.to_string()).or_insert_with(Vec::new);
    }

    Ok(proposals)
}

// Service client functions for external integrations

/// Fetch VCS diff content using configured VCS managers
#[instrument(skip(state))]
async fn fetch_diff(state: &AppState, run_id: &str) -> anyhow::Result<String> {
    // Get run info to determine VCS type
    let run = state.database.get_run(run_id).await?;

    // Extract revision info from result_branches
    let (base_revision, new_revision) = extract_diff_revisions(&run.result_branches)?;

    // Call appropriate VCS manager based on VCS type
    match run.vcs_type.as_deref() {
        Some("git") => {
            let git_url = format!("{}/git", state.config.external_url().unwrap_or(""));
            fetch_git_diff(&git_url, &run.codebase, &base_revision, &new_revision).await
        }
        Some("bzr") => {
            let bzr_url = format!("{}/bzr", state.config.external_url().unwrap_or(""));
            fetch_bzr_diff(&bzr_url, &run.codebase, &base_revision, &new_revision).await
        }
        _ => Err(anyhow::anyhow!("Unsupported VCS type")),
    }
}

/// Fetch debdiff using differ service
#[instrument(skip(state))]
async fn fetch_debdiff(state: &AppState, run_id: &str) -> anyhow::Result<String> {
    let differ_url = state
        .config
        .differ_url()
        .ok_or_else(|| anyhow::anyhow!("Differ service not configured"))?;

    // Get unchanged run for comparison
    let run = state.database.get_run(run_id).await?;
    let unchanged_run = state
        .database
        .get_unchanged_run(&run.suite, &run.codebase, Some(&run.start_time))
        .await?;

    let url = format!("{}/debdiff/{}/{}", differ_url, unchanged_run.id, run_id);

    let client = &state.http_client;
    let response = client
        .get(&url)
        .query(&[
            ("filter_boring", "yes"),
            ("jquery_url", "/_static/jquery.js"),
        ])
        .header("Accept", "text/html")
        .send()
        .await?;

    if response.status().is_success() {
        Ok(response.text().await?)
    } else if response.status() == 404 {
        Err(anyhow::anyhow!("Debdiff not available"))
    } else {
        Err(anyhow::anyhow!(
            "Failed to fetch debdiff: HTTP {}",
            response.status()
        ))
    }
}

/// Get log file information and content
#[instrument(skip(state))]
async fn get_log_content(
    state: &AppState,
    run_id: &str,
    log_name: &str,
    query: &LogQuery,
) -> anyhow::Result<LogInfo> {
    let log_manager = &state.log_manager;

    // Check if log exists (using empty codebase for now - TODO: pass actual codebase)
    let exists = log_manager.has_log("", run_id, log_name).await?;
    if !exists {
        return Ok(LogInfo {
            name: log_name.to_string(),
            exists: false,
            size: None,
            url: format!("/logs/{}/{}", run_id, log_name),
            line_count: None,
            include_lines: None,
            highlight_lines: None,
        });
    }

    // Get log metadata - we need to get the log to determine size
    // For now, set a placeholder size
    let size = 0i64; // TODO: Implement proper size retrieval if needed

    // Analyze log for failure information if it's a primary log
    let (line_count, include_lines, highlight_lines) =
        if matches!(log_name, BUILD_LOG_FILENAME | DIST_LOG_FILENAME) {
            analyze_log_failure_lines(state, run_id, log_name)
                .await
                .unwrap_or((0, None, vec![]))
        } else {
            (0, None, vec![])
        };

    Ok(LogInfo {
        name: log_name.to_string(),
        exists: true,
        size: Some(size),
        url: format!("/logs/{}/{}", run_id, log_name),
        line_count: Some(line_count),
        include_lines,
        highlight_lines: Some(highlight_lines),
    })
}

/// Download raw log file content
#[instrument(skip(state))]
async fn download_log_file(
    state: &AppState,
    run_id: &str,
    log_name: &str,
) -> anyhow::Result<Vec<u8>> {
    use std::io::Read;

    let log_manager = &state.log_manager;
    // Get log reader (using empty codebase for now - TODO: pass actual codebase)
    let mut reader = log_manager
        .get_log("", run_id, log_name)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get log: {}", e))?;

    // Read content into bytes
    let mut content = Vec::new();
    reader
        .read_to_end(&mut content)
        .map_err(|e| anyhow::anyhow!("Failed to read log content: {}", e))?;

    Ok(content)
}

/// Analyze log files for failure information (matches Python find_build_log_failure/find_dist_log_failure)
#[instrument(skip(state))]
async fn analyze_log_failure_lines(
    state: &AppState,
    run_id: &str,
    log_name: &str,
) -> anyhow::Result<(usize, Option<(usize, usize)>, Vec<usize>)> {
    let log_content = download_log_file(state, run_id, log_name).await?;
    let log_text = String::from_utf8_lossy(&log_content);
    let lines: Vec<&str> = log_text.lines().collect();

    let line_count = lines.len();

    // Find failure patterns based on log type
    let (include_lines, highlight_lines) = match log_name {
        BUILD_LOG_FILENAME => find_build_failure_lines(&lines),
        DIST_LOG_FILENAME => find_dist_failure_lines(&lines),
        _ => (None, vec![]),
    };

    Ok((line_count, include_lines, highlight_lines))
}

/// Generate diff content for display
#[instrument(skip(state))]
async fn generate_diff_content(
    state: &AppState,
    run_id: &str,
    query: &RunQuery,
) -> anyhow::Result<DiffInfo> {
    let diff_content = fetch_diff(state, run_id).await?;

    let run = state.database.get_run(run_id).await?;
    let (base_revision, new_revision) = extract_diff_revisions(&run.result_branches)?;

    Ok(DiffInfo {
        diff_type: "vcs".to_string(),
        content: diff_content,
        content_type: "text/plain".to_string(),
        base_revision: Some(base_revision),
        new_revision: Some(new_revision),
    })
}

/// Generate debdiff content for display
#[instrument(skip(state))]
async fn generate_debdiff_content(
    state: &AppState,
    run_id: &str,
    query: &RunQuery,
) -> anyhow::Result<DiffInfo> {
    let debdiff_content = fetch_debdiff(state, run_id).await?;

    Ok(DiffInfo {
        diff_type: "debdiff".to_string(),
        content: debdiff_content,
        content_type: "text/html".to_string(),
        base_revision: None,
        new_revision: None,
    })
}

/// Get publish history for a revision
#[instrument(skip(state))]
async fn get_publish_history(
    state: &AppState,
    revision: &str,
) -> anyhow::Result<Vec<PublishHistory>> {
    // Query publish table for this revision
    let records = sqlx::query(
        "SELECT mode, merge_proposal_url, description, result_code, requester, timestamp 
         FROM publish WHERE revision = $1 ORDER BY timestamp DESC",
    )
    .bind(revision.as_bytes())
    .fetch_all(state.database.pool())
    .await?;

    let mut history = Vec::new();
    for record in records {
        history.push(PublishHistory {
            mode: record.get("mode"),
            merge_proposal_url: record.get("merge_proposal_url"),
            description: record.get("description"),
            result_code: record.get("result_code"),
            requester: record.get("requester"),
            timestamp: record.get("timestamp"),
        });
    }

    Ok(history)
}

/// Fetch lintian analysis results
#[instrument(skip(state))]
async fn fetch_lintian_result(state: &AppState, run_id: &str) -> anyhow::Result<serde_json::Value> {
    // Query debian_build table for lintian results
    let record = sqlx::query("SELECT lintian_result FROM debian_build WHERE run_id = $1")
        .bind(run_id)
        .fetch_optional(state.database.pool())
        .await?;

    match record {
        Some(record) => {
            let lintian_result: Option<serde_json::Value> = record.get("lintian_result");
            Ok(lintian_result.unwrap_or(serde_json::Value::Null))
        }
        None => Ok(serde_json::Value::Null),
    }
}

// VCS-specific diff fetching functions

async fn fetch_git_diff(
    git_url: &str,
    codebase: &str,
    base_revision: &str,
    new_revision: &str,
) -> anyhow::Result<String> {
    let url = format!(
        "{}/{}/diff/{}/{}",
        git_url, codebase, base_revision, new_revision
    );

    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;

    if response.status().is_success() {
        Ok(response.text().await?)
    } else {
        Err(anyhow::anyhow!(
            "Failed to fetch git diff: HTTP {}",
            response.status()
        ))
    }
}

async fn fetch_bzr_diff(
    bzr_url: &str,
    codebase: &str,
    base_revision: &str,
    new_revision: &str,
) -> anyhow::Result<String> {
    let url = format!(
        "{}/{}/diff?old={}&new={}",
        bzr_url, codebase, base_revision, new_revision
    );

    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;

    if response.status().is_success() {
        Ok(response.text().await?)
    } else {
        Err(anyhow::anyhow!(
            "Failed to fetch bzr diff: HTTP {}",
            response.status()
        ))
    }
}

// Utility functions for log analysis and revision extraction

fn extract_diff_revisions(
    result_branches: &[serde_json::Value],
) -> anyhow::Result<(String, String)> {
    // Extract base and new revisions from result_branches
    // This matches the Python logic in state.get_result_branch
    for branch in result_branches {
        if let Some(role) = branch.get("role").and_then(|r| r.as_str()) {
            if role == "main" {
                let base_revid = branch
                    .get("base_revid")
                    .and_then(|r| r.as_str())
                    .unwrap_or("null")
                    .to_string();
                let revid = branch
                    .get("revid")
                    .and_then(|r| r.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing revid"))?
                    .to_string();
                return Ok((base_revid, revid));
            }
        }
    }
    Err(anyhow::anyhow!("No main branch found"))
}

/// Find build failure patterns in log lines (matches Python find_build_log_failure)
fn find_build_failure_lines(lines: &[&str]) -> (Option<(usize, usize)>, Vec<usize>) {
    let mut error_lines = Vec::new();

    // Look for common build failure patterns
    for (i, line) in lines.iter().enumerate() {
        let line_lower = line.to_lowercase();
        if line_lower.contains("error:")
            || line_lower.contains("fatal:")
            || line_lower.contains("failed to build")
            || line_lower.contains("make: *** ")
            || line_lower.contains("dpkg-buildpackage: error")
        {
            error_lines.push(i + 1); // 1-based line numbers
        }
    }

    if !error_lines.is_empty() {
        // Include context around the last error
        let last_error = *error_lines.last().unwrap();
        let start = last_error.saturating_sub(FAIL_BUILD_LOG_LEN);
        let end = (last_error + FAIL_BUILD_LOG_LEN).min(lines.len());
        (Some((start, end)), error_lines)
    } else {
        (None, error_lines)
    }
}

/// Find dist failure patterns in log lines (matches Python find_dist_log_failure)
fn find_dist_failure_lines(lines: &[&str]) -> (Option<(usize, usize)>, Vec<usize>) {
    let mut error_lines = Vec::new();

    // Look for dist-specific failure patterns
    for (i, line) in lines.iter().enumerate() {
        let line_lower = line.to_lowercase();
        if line_lower.contains("error")
            || line_lower.contains("failed")
            || line_lower.contains("exception")
            || line_lower.contains("traceback")
        {
            error_lines.push(i + 1); // 1-based line numbers
        }
    }

    if !error_lines.is_empty() {
        let last_error = *error_lines.last().unwrap();
        let start = last_error.saturating_sub(FAIL_BUILD_LOG_LEN);
        let end = (last_error + FAIL_BUILD_LOG_LEN).min(lines.len());
        (Some((start, end)), error_lines)
    } else {
        (None, error_lines)
    }
}

// Helper function to check if line is within boundaries (matches Python in_line_boundaries)
fn in_line_boundaries(line_num: usize, boundaries: Option<(usize, usize)>) -> bool {
    match boundaries {
        Some((start, end)) => line_num >= start && line_num <= end,
        None => true,
    }
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
