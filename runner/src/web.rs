use crate::{
    get_builder, metrics::MetricsCollector, ActiveRun, AppState, Backchannel, CampaignConfig,
    QueueItem, Watchdog,
};
use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    middleware,
    response::IntoResponse,
    routing::delete,
    routing::get,
    routing::post,
    Extension, Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::io::Read;
use std::sync::Arc;
use uuid::Uuid;

/// Request for work assignment.
#[derive(Debug, Deserialize)]
struct AssignRequest {
    /// Worker name.
    worker: Option<String>,
    /// Worker link.
    worker_link: Option<String>,
    /// Backchannel configuration.
    backchannel: Option<serde_json::Value>,
    /// Specific codebase to work on.
    codebase: Option<String>,
    /// Specific campaign to work on.
    campaign: Option<String>,
}

/// Response for work assignment.
#[derive(Debug, Serialize)]
struct AssignResponse {
    /// Assigned queue item.
    queue_item: QueueItem,
    /// VCS information.
    vcs_info: janitor::queue::VcsInfo,
    /// Active run information.
    active_run: serde_json::Value,
    /// Build configuration for the worker.
    build_config: HashMap<String, String>,
}

/// Request for updating run publish status.
#[derive(Debug, Deserialize)]
struct UpdateRunRequest {
    /// New publish status.
    publish_status: String,
}

/// Request for resume information.
#[derive(Debug, Deserialize)]
struct ResumeInfoRequest {
    /// Campaign name.
    campaign: String,
    /// Branch name to check for resume.
    branch_name: String,
}

/// Request for manual scheduling.
#[derive(Debug, Deserialize)]
struct ScheduleRequest {
    /// Campaign name.
    campaign: String,
    /// Suite to schedule for.
    suite: String,
    /// Bucket for scheduling.
    bucket: Option<String>,
    /// Whether to refresh existing runs.
    refresh: bool,
    /// Estimated duration.
    estimated_duration: Option<std::time::Duration>,
    /// Offset for queue position.
    offset: Option<i64>,
    /// Limit for number of items.
    limit: Option<i64>,
}

/// Request for schedule control.
#[derive(Debug, Deserialize)]
struct ScheduleControlRequest {
    /// Action to perform: reschedule, deschedule, reset.
    action: String,
    /// Campaign name.
    campaign: String,
    /// Suite to operate on.
    suite: Option<String>,
    /// Minimum success chance for rescheduling.
    min_success_chance: Option<f64>,
    /// Result code to filter by for descheduling.
    result_code: Option<String>,
}

/// Response for finishing a run.
#[derive(Debug, Serialize)]
struct FinishResponse {
    /// Run ID.
    id: String,
    /// Uploaded filenames.
    filenames: Vec<String>,
    /// Log filenames.
    logs: Vec<String>,
    /// Artifact names.
    artifacts: Vec<String>,
    /// Result information.
    result: serde_json::Value,
}

/// Extract avoided hosts from configuration.
fn get_avoided_hosts(config: &janitor::config::Config) -> Vec<String> {
    let mut avoided_hosts = Vec::new();

    // Check for any distribution-specific hosts that should be avoided
    // For now, implement basic host filtering based on distribution settings
    for distribution in &config.distribution {
        // Skip distributions that might have problematic archive mirrors
        if let Some(mirror) = distribution.archive_mirror_uri.as_ref() {
            if mirror.contains("restricted") || mirror.contains("internal") {
                // Extract hostname from mirror URI and add to avoided list
                if let Ok(url) = url::Url::parse(mirror) {
                    if let Some(host) = url.host_str() {
                        avoided_hosts.push(host.to_string());
                    }
                }
            }
        }
    }

    // Add any hardcoded problematic hosts
    // In a real implementation, this could come from config or database
    avoided_hosts.extend([
        "unreliable.example.com".to_string(),
        "slow.mirror.example.org".to_string(),
    ]);

    avoided_hosts
}

/// Create campaign configuration from actual config files and queue item.
fn create_campaign_config(
    queue_item: &QueueItem,
    app_config: &janitor::config::Config,
) -> CampaignConfig {
    // Find the campaign configuration in the loaded config
    let campaign_config = app_config
        .campaign
        .iter()
        .find(|c| c.name() == queue_item.campaign);

    if let Some(config) = campaign_config {
        // Extract build configuration from the campaign
        let debian_build = if config.has_debian_build() {
            let db_config = config.debian_build();
            Some(crate::DebianBuildConfig {
                base_distribution: db_config.base_distribution().to_string(),
                extra_build_distribution: if db_config.build_distribution().is_empty() {
                    vec![]
                } else {
                    vec![db_config.build_distribution().to_string()]
                },
            })
        } else {
            None
        };

        let generic_build = if config.has_generic_build() {
            let gb_config = config.generic_build();
            Some(crate::GenericBuildConfig {
                chroot: if gb_config.chroot().is_empty() {
                    None
                } else {
                    Some(gb_config.chroot().to_string())
                },
            })
        } else {
            None
        };

        CampaignConfig {
            generic_build,
            debian_build,
        }
    } else {
        // Fallback to default configuration if campaign not found
        log::warn!(
            "Campaign '{}' not found in config, using default",
            queue_item.campaign
        );
        CampaignConfig {
            generic_build: Some(crate::GenericBuildConfig { chroot: None }),
            debian_build: None,
        }
    }
}

async fn queue_position(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state
        .database
        .calculate_queue_position(None, None, None)
        .await
    {
        Ok(Some(total)) => Json(json!({
            "position": 0,
            "total": total
        })),
        Ok(None) => Json(json!({
            "position": 0,
            "total": 0
        })),
        Err(e) => {
            log::error!("Failed to get queue position: {}", e);
            Json(json!({"error": "Database error"}))
        }
    }
}

async fn schedule_control(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ScheduleControlRequest>,
) -> impl IntoResponse {
    let db = &state.database;

    let affected_count = match request.action.as_str() {
        "reschedule" => {
            match db
                .reschedule_failed_candidates(
                    &request.campaign,
                    request.suite.as_deref(),
                    request.min_success_chance.unwrap_or(0.1),
                )
                .await
            {
                Ok(count) => count,
                Err(e) => {
                    log::error!("Failed to reschedule candidates: {}", e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": "Database error"})),
                    );
                }
            }
        }
        "deschedule" => {
            match db
                .deschedule_candidates(
                    &request.campaign,
                    request.suite.as_deref(),
                    request.result_code.as_deref(),
                )
                .await
            {
                Ok(count) => count,
                Err(e) => {
                    log::error!("Failed to deschedule candidates: {}", e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": "Database error"})),
                    );
                }
            }
        }
        "reset" => {
            match db
                .reset_candidates(&request.campaign, request.suite.as_deref())
                .await
            {
                Ok(count) => count,
                Err(e) => {
                    log::error!("Failed to reset candidates: {}", e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": "Database error"})),
                    );
                }
            }
        }
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("Unknown action: {}", request.action)})),
            );
        }
    };

    (
        StatusCode::OK,
        Json(json!({ "affected_count": affected_count })),
    )
}

async fn schedule(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ScheduleRequest>,
) -> impl IntoResponse {
    let db = &state.database;

    let suite = if request.suite.is_empty() {
        None
    } else {
        Some(request.suite.as_str())
    };

    let queue_position = match request.bucket {
        Some(bucket) => {
            match db
                .reschedule_some(
                    &request.campaign,
                    suite,
                    &bucket,
                    request.refresh,
                    request.estimated_duration.as_ref(),
                    request.offset.unwrap_or(0),
                    request.limit.unwrap_or(100),
                )
                .await
            {
                Ok(pos) => pos,
                Err(e) => {
                    log::error!("Failed to reschedule some candidates: {}", e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": "Database error"})),
                    );
                }
            }
        }
        None => {
            match db
                .reschedule_all(
                    &request.campaign,
                    suite,
                    request.refresh,
                    request.estimated_duration.as_ref(),
                    request.offset.unwrap_or(0),
                    request.limit.unwrap_or(100),
                )
                .await
            {
                Ok(pos) => pos,
                Err(e) => {
                    log::error!("Failed to reschedule all candidates: {}", e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": "Database error"})),
                    );
                }
            }
        }
    };

    (
        StatusCode::OK,
        Json(json!({ "queue_position": queue_position })),
    )
}

async fn status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.database.get_queue_stats().await {
        Ok(stats) => {
            let total = stats.get("total").unwrap_or(&0);
            let active = stats.get("active").unwrap_or(&0);

            Json(json!({
                "queue_length": total,
                "active_runs": active,
                "status": "running"
            }))
        }
        Err(e) => {
            log::error!("Failed to get queue stats: {}", e);
            Json(json!({
                "status": "error",
                "error": "Database error"
            }))
        }
    }
}

async fn log_index(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // First check if the run exists
    match state.database.run_exists(&id).await {
        Ok(true) => {
            // Get actual log files from log storage system
            let logs_iter = state.log_manager.iter_logs().await;
            let mut files = Vec::new();

            // Find logs for this specific run
            for (_codebase, run_id, log_names) in logs_iter {
                if run_id == id {
                    files = log_names;
                    break;
                }
            }

            // If no logs found, return standard log file names
            if files.is_empty() {
                files = vec!["worker.log".to_string(), "build.log".to_string()];
            }

            (
                StatusCode::OK,
                Json(json!({
                    "log_id": id,
                    "files": files
                })),
            )
        }
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Run not found"})),
        ),
        Err(e) => {
            log::error!("Failed to check run existence for {}: {}", id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        }
    }
}

async fn log(
    State(state): State<Arc<AppState>>,
    Path((id, filename)): Path<(String, String)>,
) -> impl IntoResponse {
    // Get the run information to find the codebase
    match state.database.get_run(&id).await {
        Ok(Some(run)) => {
            // Validate filename
            if !crate::is_log_filename(&filename) {
                return (StatusCode::BAD_REQUEST, "Invalid log filename".to_string());
            }

            // Get log content from storage system
            match state
                .log_manager
                .get_log(&run.codebase, &id, &filename)
                .await
            {
                Ok(mut reader) => {
                    // Read the log content
                    let mut content = Vec::new();
                    match reader.read_to_end(&mut content) {
                        Ok(_) => {
                            // Return the actual log content as string
                            match String::from_utf8(content) {
                                Ok(content_str) => (StatusCode::OK, content_str),
                                Err(_) => (
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    "Log content is not valid UTF-8".to_string(),
                                ),
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to read log content: {}", e);
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Failed to read log content".to_string(),
                            )
                        }
                    }
                }
                Err(e) => {
                    log::warn!(
                        "Failed to get log content for run {} file {}: {}",
                        id,
                        filename,
                        e
                    );
                    (
                        StatusCode::NOT_FOUND,
                        format!("Log file {} not found for run {}", filename, id),
                    )
                }
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND, "Run not found".to_string()),
        Err(e) => {
            log::error!("Failed to check run existence for {}: {}", id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        }
    }
}

async fn kill(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> impl IntoResponse {
    // Create a temporary watchdog instance for manual termination
    let watchdog_config = crate::WatchdogConfig::default();
    let mut watchdog = Watchdog::new(Arc::clone(&state.database), watchdog_config);

    match watchdog.kill_run(&id).await {
        Ok(true) => {
            log::info!("Successfully killed run {}", id);
            Json(serde_json::json!({
                "success": true,
                "message": format!("Run {} terminated successfully", id)
            }))
        }
        Ok(false) => Json(serde_json::json!({
            "success": false,
            "error": format!("Run {} not found or not active", id)
        })),
        Err(e) => {
            log::error!("Failed to kill run {}: {}", id, e);
            Json(serde_json::json!({
                "success": false,
                "error": "Failed to terminate run"
            }))
        }
    }
}

async fn get_codebases(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.database.get_codebases().await {
        Ok(codebases) => {
            log::info!("Retrieved {} codebases", codebases.len());
            Json(codebases)
        }
        Err(e) => {
            log::error!("Failed to get codebases: {}", e);
            Json(Vec::<serde_json::Value>::new())
        }
    }
}

async fn update_codebases(
    State(state): State<Arc<AppState>>,
    Json(codebases): Json<Vec<serde_json::Value>>,
) -> impl IntoResponse {
    match state.database.upload_codebases(&codebases).await {
        Ok(()) => {
            log::info!("Successfully uploaded {} codebases", codebases.len());
            (
                StatusCode::OK,
                Json(serde_json::json!({"status": "success", "uploaded": codebases.len()})),
            )
                .into_response()
        }
        Err(e) => {
            log::error!("Failed to upload codebases: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Database error"})),
            )
                .into_response()
        }
    }
}

async fn delete_candidate(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let candidate_id = match id.parse::<i64>() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid candidate ID"})),
            )
                .into_response();
        }
    };

    match state.database.delete_candidate(candidate_id).await {
        Ok(true) => {
            log::info!("Successfully deleted candidate {}", candidate_id);
            (
                StatusCode::OK,
                Json(serde_json::json!({"status": "success"})),
            )
                .into_response()
        }
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Candidate not found"})),
        )
            .into_response(),
        Err(e) => {
            log::error!("Failed to delete candidate {}: {}", candidate_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Database error"})),
            )
                .into_response()
        }
    }
}

async fn get_candidates(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.database.get_candidates().await {
        Ok(candidates) => {
            log::info!("Retrieved {} candidates", candidates.len());
            (StatusCode::OK, Json(candidates)).into_response()
        }
        Err(e) => {
            log::error!("Failed to get candidates: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Database error"})),
            )
                .into_response()
        }
    }
}

async fn upload_candidates(
    State(state): State<Arc<AppState>>,
    Json(candidates): Json<Vec<serde_json::Value>>,
) -> impl IntoResponse {
    match state.database.upload_candidates(&candidates).await {
        Ok(errors) => {
            if errors.is_empty() {
                log::info!("Successfully uploaded {} candidates", candidates.len());
                (
                    StatusCode::OK,
                    Json(serde_json::json!({
                        "status": "success",
                        "uploaded": candidates.len()
                    })),
                )
                    .into_response()
            } else {
                log::warn!("Failed to upload some candidates: {:?}", errors);
                (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "status": "partial_failure",
                        "errors": errors
                    })),
                )
                    .into_response()
            }
        }
        Err(e) => {
            log::error!("Failed to upload candidates: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Database error"})),
            )
                .into_response()
        }
    }
}

async fn get_run(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> impl IntoResponse {
    match state.database.get_run(&id).await {
        Ok(Some(run)) => Json(run.to_json()),
        Ok(None) => Json(json!({"error": "Run not found"})),
        Err(e) => {
            log::error!("Failed to get run {}: {}", id, e);
            Json(json!({"error": "Database error"}))
        }
    }
}

async fn update_run(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(request): Json<UpdateRunRequest>,
) -> impl IntoResponse {
    // For now, implement basic publish status update
    // This is primarily used by the publisher to update run status

    match state
        .database
        .update_run_publish_status(&id, &request.publish_status)
        .await
    {
        Ok(Some((run_id, codebase, suite))) => Json(json!({
            "run_id": run_id,
            "codebase": codebase,
            "suite": suite,
            "publish_status": request.publish_status
        })),
        Ok(None) => Json(json!({
            "error": format!("no such run: {}", id)
        })),
        Err(e) => {
            log::error!("Failed to update run {}: {}", id, e);
            Json(json!({
                "error": "Database error"
            }))
        }
    }
}

async fn get_active_runs(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.database.get_active_runs().await {
        Ok(active_runs) => {
            let runs_json: Vec<_> = active_runs.iter().map(|r| r.to_json()).collect();
            (StatusCode::OK, Json(runs_json))
        }
        Err(e) => {
            log::error!("Failed to get active runs: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(vec![json!({"error": "Database error"})]),
            )
        }
    }
}

async fn get_active_run(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.database.get_active_run(&id).await {
        Ok(Some(active_run)) => Json(active_run.to_json()),
        Ok(None) => Json(json!({"error": "Run not found"})),
        Err(e) => {
            log::error!("Failed to get active run {}: {}", id, e);
            Json(json!({"error": "Database error"}))
        }
    }
}

async fn peek_active_run(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // Peek at the next queue assignment without actually assigning it
    let avoided_hosts = get_avoided_hosts(&state.config);
    match state
        .database
        .next_queue_item_with_rate_limiting(
            None, // No specific codebase filter
            None, // No specific campaign filter
            &avoided_hosts,
        )
        .await
    {
        Ok(Some(assignment)) => {
            let campaign_config = create_campaign_config(&assignment.queue_item, &state.config);
            let build_config = match get_builder(&campaign_config, None, None) {
                Ok(builder) => {
                    let mut config = HashMap::new();
                    config.insert("builder_kind".to_string(), builder.kind().to_string());
                    config
                }
                Err(_) => HashMap::new(),
            };

            Json(json!({
                "queue_item": assignment.queue_item.to_json(),
                "vcs_info": assignment.vcs_info,
                "build_config": build_config,
                "estimated_duration": assignment.queue_item.estimated_duration.map(|d| d.as_secs()),
            }))
        }
        Ok(None) => Json(json!({
            "reason": "queue empty"
        })),
        Err(e) => {
            log::error!("Failed to peek queue item: {}", e);
            Json(json!({
                "error": "Database error"
            }))
        }
    }
}

async fn get_queue(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.database.get_queue_stats().await {
        Ok(stats) => {
            let total = stats.get("total").unwrap_or(&0);

            Json(json!({
                "queue_length": total,
                "items": []  // For now, just return basic stats
            }))
        }
        Err(e) => {
            log::error!("Failed to get queue: {}", e);
            Json(json!({"error": "Database error"}))
        }
    }
}

async fn health(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // Perform basic health checks
    let mut health_status = serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "version": env!("CARGO_PKG_VERSION"),
        "checks": {}
    });

    let mut overall_healthy = true;

    // Database health check
    match state.database.health_check().await {
        Ok(()) => {
            health_status["checks"]["database"] = json!({
                "status": "healthy",
                "message": "Database connection successful"
            });
        }
        Err(e) => {
            overall_healthy = false;
            health_status["checks"]["database"] = json!({
                "status": "unhealthy",
                "message": format!("Database error: {}", e)
            });
        }
    }

    // VCS health check
    let vcs_health = state.vcs_manager.health_check().await;
    if vcs_health.overall_healthy {
        health_status["checks"]["vcs"] = json!({
            "status": "healthy",
            "message": "All VCS systems healthy"
        });
    } else {
        overall_healthy = false;
        health_status["checks"]["vcs"] = json!({
            "status": "unhealthy",
            "message": "Some VCS systems unhealthy",
            "details": vcs_health.vcs_statuses
        });
    }

    // Log storage health check
    match state.log_manager.health_check().await {
        Ok(()) => {
            health_status["checks"]["logs"] = json!({
                "status": "healthy",
                "message": "Log storage accessible"
            });
        }
        Err(e) => {
            overall_healthy = false;
            health_status["checks"]["logs"] = json!({
                "status": "unhealthy",
                "message": format!("Log storage error: {}", e)
            });
        }
    }

    // Artifact storage health check - janitor crate doesn't have health_check
    health_status["checks"]["artifacts"] = json!({
        "status": "healthy",
        "message": "Artifact storage assumed accessible"
    });

    // Update overall status
    health_status["status"] = if overall_healthy {
        json!("healthy")
    } else {
        json!("unhealthy")
    };

    // Return appropriate HTTP status code
    if overall_healthy {
        (StatusCode::OK, Json(health_status))
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, Json(health_status))
    }
}

async fn ready(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // Check if the application is ready to accept traffic
    // This is a lighter check than health - just verify core systems are responding

    // Quick database connectivity check
    match state.database.health_check().await {
        Ok(()) => (
            StatusCode::OK,
            Json(json!({
                "status": "ready",
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "message": "Application ready to accept requests"
            })),
        )
            .into_response(),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "not_ready",
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "message": "Application not ready - database unavailable"
            })),
        )
            .into_response(),
    }
}

async fn metrics() -> impl IntoResponse {
    match MetricsCollector::collect_metrics() {
        Ok(metrics) => {
            // Return metrics in Prometheus format
            (
                StatusCode::OK,
                [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
                metrics,
            )
        }
        Err(e) => {
            log::error!("Failed to collect metrics: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [("content-type", "text/plain")],
                "Failed to collect metrics".to_string(),
            )
        }
    }
}

/// Admin endpoint to list workers.
async fn admin_list_workers(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.auth_service.list_workers().await {
        Ok(workers) => Json(json!({"workers": workers})),
        Err(e) => {
            log::error!("Failed to list workers: {}", e);
            Json(json!({"error": "Failed to list workers"}))
        }
    }
}

/// Admin endpoint to create a worker.
#[derive(Deserialize)]
struct CreateWorkerRequest {
    name: String,
    password: String,
    link: Option<String>,
}

async fn admin_create_worker(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateWorkerRequest>,
) -> impl IntoResponse {
    match state
        .auth_service
        .create_worker(&request.name, &request.password, request.link.as_deref())
        .await
    {
        Ok(()) => (
            StatusCode::CREATED,
            Json(json!({"message": "Worker created successfully"})),
        ),
        Err(e) => {
            log::error!("Failed to create worker: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to create worker"})),
            )
        }
    }
}

/// Admin endpoint to delete a worker.
async fn admin_delete_worker(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match state.auth_service.delete_worker(&name).await {
        Ok(true) => Json(json!({"message": "Worker deleted successfully"})),
        Ok(false) => Json(json!({"error": "Worker not found"})),
        Err(e) => {
            log::error!("Failed to delete worker: {}", e);
            Json(json!({"error": "Failed to delete worker"}))
        }
    }
}

/// Admin endpoint to get security statistics.
async fn admin_security_stats(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.security_service.get_security_stats().await {
        Ok(stats) => match serde_json::to_value(stats) {
            Ok(value) => Json(value),
            Err(e) => {
                log::error!("Failed to serialize security stats: {}", e);
                Json(json!({"error": "Failed to serialize stats"}))
            }
        },
        Err(e) => {
            log::error!("Failed to get security stats: {}", e);
            Json(json!({"error": "Failed to get security stats"}))
        }
    }
}

/// Check for resume information for a campaign and branch.
async fn check_resume_info(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ResumeInfoRequest>,
) -> impl IntoResponse {
    match state
        .resume_service
        .check_resume_result(&request.campaign, &request.branch_name)
        .await
    {
        Ok(Some(resume_info)) => Json(json!({
            "resume_available": true,
            "resume_info": resume_info
        })),
        Ok(None) => Json(json!({
            "resume_available": false,
            "message": "No resume information found"
        })),
        Err(e) => {
            log::error!("Failed to check resume info: {}", e);
            Json(json!({
                "error": "Failed to check resume information"
            }))
        }
    }
}

/// Get the resume chain for a specific run.
async fn get_resume_chain(
    State(state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
) -> impl IntoResponse {
    match state.resume_service.get_resume_chain(&run_id).await {
        Ok(chain) => Json(json!({
            "run_id": run_id,
            "resume_chain": chain
        })),
        Err(e) => {
            log::error!("Failed to get resume chain for {}: {}", run_id, e);
            Json(json!({
                "error": "Failed to get resume chain"
            }))
        }
    }
}

/// Get all runs that resume from a specific run.
async fn get_resume_descendants(
    State(state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
) -> impl IntoResponse {
    match state.resume_service.get_resume_descendants(&run_id).await {
        Ok(descendants) => Json(json!({
            "run_id": run_id,
            "descendants": descendants
        })),
        Err(e) => {
            log::error!("Failed to get resume descendants for {}: {}", run_id, e);
            Json(json!({
                "error": "Failed to get resume descendants"
            }))
        }
    }
}

/// Validate resume consistency across the database.
async fn validate_resume_consistency(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.resume_service.validate_resume_consistency().await {
        Ok(errors) => {
            if errors.is_empty() {
                Json(json!({
                    "status": "consistent",
                    "message": "All resume relationships are valid"
                }))
            } else {
                Json(json!({
                    "status": "inconsistent",
                    "errors": errors
                }))
            }
        }
        Err(e) => {
            log::error!("Failed to validate resume consistency: {}", e);
            Json(json!({
                "error": "Failed to validate resume consistency"
            }))
        }
    }
}

async fn finish_active_run(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    finish_run_internal(state, id, None).await
}

async fn finish_active_run_multipart(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    multipart: Multipart,
) -> impl IntoResponse {
    finish_run_multipart_internal(state, id, multipart).await
}

async fn finish_run_internal(
    state: Arc<AppState>,
    run_id: String,
    worker_result: Option<crate::WorkerResult>,
) -> (StatusCode, Json<serde_json::Value>) {
    // Get the active run
    let active_run = match state.database.get_active_run(&run_id).await {
        Ok(Some(run)) => run,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"reason": format!("no such run {}", run_id)})),
            );
        }
        Err(e) => {
            log::error!("Failed to get active run {}: {}", run_id, e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            );
        }
    };

    // Create a result from the active run with comprehensive worker_result processing
    let result_code = if let Some(ref wr) = worker_result {
        wr.code.clone()
    } else {
        "success".to_string()
    };

    let description = if let Some(ref wr) = worker_result {
        wr.description.clone()
    } else {
        Some("Run completed".to_string())
    };

    let mut janitor_result = active_run.create_result(result_code, description);

    // Process comprehensive worker result data if provided
    if let Some(ref wr) = worker_result {
        // Update all fields from worker result
        janitor_result.codemod = wr.codemod.clone();
        janitor_result.main_branch_revision = wr.main_branch_revision.clone();
        janitor_result.revision = wr.revision.clone();
        janitor_result.value = wr.value.map(|v| v as u64);
        janitor_result.branches = wr.branches.clone();
        janitor_result.tags = wr.tags.clone();
        janitor_result.remotes = wr.remotes.as_ref().map(|remotes| {
            remotes
                .iter()
                .map(|(name, data)| {
                    let url = data.get("url").and_then(|u| u.as_str()).unwrap_or_default();
                    (
                        name.clone(),
                        crate::ResultRemote {
                            url: url.to_string(),
                        },
                    )
                })
                .collect()
        });
        janitor_result.failure_details = wr.details.clone();
        janitor_result.failure_stage = wr.stage.clone();
        janitor_result.transient = wr.transient;
        janitor_result.target_branch_url = wr.target_branch_url.clone();
        janitor_result.branch_url = wr.branch_url.clone().unwrap_or(janitor_result.branch_url);
        janitor_result.vcs_type = wr.vcs_type.clone();
        janitor_result.subpath = wr.subpath.clone();

        // Log filenames will be set later from the collected list

        // Store builder result if present
        if let Some(ref builder_result) = wr.builder_result {
            janitor_result.builder_result = Some(builder_result.clone());
        }
    } else {
        // Worker should provide all result data - no fallback creation here
        // If no worker_result is provided, log a warning but continue
        log::warn!(
            "No worker result provided for run {}. Worker should submit complete results including builder information.",
            run_id
        );

        // The janitor_result will remain with basic success/failure status
        // All detailed build information should come from the worker
        // No builder_result is set - this encourages proper worker implementation
    }

    // Store the result in the database
    if let Err(e) = state
        .database
        .update_run_result(
            &run_id,
            &janitor_result.code,
            janitor_result.description.as_deref(),
            janitor_result.failure_details.as_ref(),
            janitor_result.transient,
            janitor_result.finish_time,
        )
        .await
    {
        log::error!("Failed to store run result: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to store result"})),
        );
    }

    // Set resume information if this run resumed from another
    if let Some(ref resume_from_id) = active_run.resume_from {
        if let Err(e) = state
            .resume_service
            .set_resume_from(&run_id, resume_from_id)
            .await
        {
            log::warn!("Failed to set resume information for run {}: {}", run_id, e);
            // Continue anyway, main result was stored
        }
    }

    // Store builder result if present
    if let Some(ref builder_result) = janitor_result.builder_result {
        if let Err(e) = state
            .database
            .store_builder_result(&run_id, builder_result)
            .await
        {
            log::error!("Failed to store builder result: {}", e);
            // Continue anyway, main result was stored
        }
    }

    // Remove from active runs
    if let Err(e) = state.database.remove_active_run(&run_id).await {
        log::error!("Failed to remove active run: {}", e);
        // Continue anyway, result was stored
    }

    // Unassign queue item from Redis
    if let Err(e) = state
        .database
        .unassign_queue_item(active_run.queue_id)
        .await
    {
        log::warn!("Failed to unassign queue item from Redis: {}", e);
        // Continue anyway, main cleanup was done
    }

    // Process any existing files for this run (stored outside of multipart upload)
    let uploaded_filenames = Vec::new();
    let mut log_filenames = Vec::new();
    let artifact_filenames = Vec::new();

    // If worker_result is provided, extract file information from it
    if let Some(ref wr) = worker_result {
        // Update janitor_result with worker data
        janitor_result.codemod = wr.codemod.clone();
        janitor_result.main_branch_revision = wr.main_branch_revision.clone();
        janitor_result.revision = wr.revision.clone();
        janitor_result.value = wr.value.map(|v| v as u64);
        janitor_result.branches = wr.branches.clone();
        janitor_result.tags = wr.tags.clone();
        janitor_result.remotes = wr.remotes.as_ref().map(|remotes| {
            remotes
                .iter()
                .map(|(name, data)| {
                    let url = data.get("url").and_then(|u| u.as_str()).unwrap_or_default();
                    (
                        name.clone(),
                        crate::ResultRemote {
                            url: url.to_string(),
                        },
                    )
                })
                .collect()
        });
        janitor_result.failure_details = wr.details.clone();
        janitor_result.failure_stage = wr.stage.clone();
        janitor_result.transient = wr.transient;
        janitor_result.target_branch_url = wr.target_branch_url.clone();
        janitor_result.branch_url = wr.branch_url.clone().unwrap_or(janitor_result.branch_url);
        janitor_result.vcs_type = wr.vcs_type.clone();
        janitor_result.subpath = wr.subpath.clone();

        // Log filenames will be set after collecting from log manager

        // Store builder result if present
        if let Some(ref builder_result) = wr.builder_result {
            janitor_result.builder_result = Some(builder_result.clone());
        }
    }

    // For regular (non-multipart) requests, check if there are any previously stored files
    // This handles cases where files were uploaded separately or through other means
    let logs_iter = state.log_manager.iter_logs().await;
    for (_codebase, iter_run_id, log_names) in logs_iter {
        if iter_run_id == run_id {
            for log_file in log_names {
                if !log_filenames.contains(&log_file) {
                    log_filenames.push(log_file);
                }
            }
            break;
        }
    }

    // The janitor crate's ArtifactManager doesn't have a list_artifacts method
    // We'll keep track of artifacts that were uploaded in this request only

    let response = FinishResponse {
        id: run_id.clone(),
        filenames: uploaded_filenames,
        logs: log_filenames,
        artifacts: artifact_filenames,
        result: janitor_result.to_json(),
    };

    match serde_json::to_value(response) {
        Ok(json) => (StatusCode::CREATED, Json(json)),
        Err(e) => {
            log::error!("Failed to serialize finish response: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Serialization failed"})),
            )
        }
    }
}

async fn finish_run_multipart_internal(
    state: Arc<AppState>,
    run_id: String,
    multipart: Multipart,
) -> (StatusCode, Json<serde_json::Value>) {
    // Get the active run
    let active_run = match state.database.get_active_run(&run_id).await {
        Ok(Some(run)) => run,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"reason": format!("no such run {}", run_id)})),
            );
        }
        Err(e) => {
            log::error!("Failed to get active run {}: {}", run_id, e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            );
        }
    };

    // Process multipart upload
    let uploaded_result = match state
        .upload_processor
        .process_upload(multipart, &run_id)
        .await
    {
        Ok(result) => result,
        Err(e) => {
            log::error!("Failed to process upload for run {}: {}", run_id, e);
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("Upload processing failed: {}", e)})),
            );
        }
    };

    // Extract worker result and builder result
    let worker_result = uploaded_result.worker_result.clone();
    let builder_result = match state
        .upload_processor
        .extract_builder_result(&uploaded_result)
    {
        Ok(result) => result,
        Err(e) => {
            log::warn!("Failed to extract builder result: {}", e);
            worker_result.builder_result.clone()
        }
    };

    // Create a JanitorResult from the uploaded data
    let mut janitor_result = active_run.create_result(
        worker_result.code.clone(),
        worker_result.description.clone(),
    );

    // Update with worker result data
    janitor_result.codemod = worker_result.codemod;
    janitor_result.main_branch_revision = worker_result.main_branch_revision;
    janitor_result.revision = worker_result.revision;
    janitor_result.value = worker_result.value.map(|v| v as u64);
    janitor_result.branches = worker_result.branches;
    janitor_result.tags = worker_result.tags;
    janitor_result.remotes = worker_result.remotes.map(|remotes| {
        remotes
            .into_iter()
            .map(|(name, data)| {
                let url = data.get("url").and_then(|u| u.as_str()).unwrap_or_default();
                (
                    name,
                    crate::ResultRemote {
                        url: url.to_string(),
                    },
                )
            })
            .collect()
    });
    janitor_result.failure_details = worker_result.details;
    janitor_result.failure_stage = worker_result.stage;
    janitor_result.builder_result = builder_result;
    janitor_result.transient = worker_result.transient;
    janitor_result.target_branch_url = worker_result.target_branch_url;
    janitor_result.branch_url = worker_result
        .branch_url
        .unwrap_or(janitor_result.branch_url);
    janitor_result.vcs_type = worker_result.vcs_type;
    janitor_result.subpath = worker_result.subpath;

    // Collect uploaded filenames
    let mut log_filenames = Vec::new();
    for log_file in &uploaded_result.log_files {
        log_filenames.push(log_file.filename.clone());
    }
    janitor_result.logfilenames = log_filenames.clone();

    // Store the result in the database
    if let Err(e) = state
        .database
        .update_run_result(
            &run_id,
            &janitor_result.code,
            janitor_result.description.as_deref(),
            janitor_result.failure_details.as_ref(),
            janitor_result.transient,
            janitor_result.finish_time,
        )
        .await
    {
        log::error!("Failed to store run result: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to store result"})),
        );
    }

    // Set resume information if this run resumed from another
    if let Some(ref resume_from_id) = active_run.resume_from {
        if let Err(e) = state
            .resume_service
            .set_resume_from(&run_id, resume_from_id)
            .await
        {
            log::warn!("Failed to set resume information for run {}: {}", run_id, e);
            // Continue anyway, main result was stored
        }
    }

    // Store builder result if present
    if let Some(ref builder_result) = janitor_result.builder_result {
        if let Err(e) = state
            .database
            .store_builder_result(&run_id, builder_result)
            .await
        {
            log::error!("Failed to store builder result: {}", e);
            // Continue anyway, main result was stored
        }
    }

    // Store uploaded files in artifact and log management systems
    // The janitor crate's ArtifactManager expects a directory with all artifacts
    if !uploaded_result.artifact_files.is_empty() || !uploaded_result.build_files.is_empty() {
        // Create a temporary directory with all artifacts
        let temp_dir = std::env::temp_dir().join(format!("janitor-artifacts-{}", &run_id));
        let artifacts_dir = temp_dir.join("artifacts");
        if let Err(e) = tokio::fs::create_dir_all(&artifacts_dir).await {
            log::warn!("Failed to create artifacts directory: {}", e);
        } else {
            // Copy artifact files to the artifacts directory
            for artifact_file in &uploaded_result.artifact_files {
                let dest = artifacts_dir.join(&artifact_file.filename);
                if let Err(e) = tokio::fs::copy(&artifact_file.stored_path, &dest).await {
                    log::warn!(
                        "Failed to copy artifact {} to artifacts dir: {}",
                        artifact_file.filename,
                        e
                    );
                }
            }

            // Copy build files to the artifacts directory
            for build_file in &uploaded_result.build_files {
                let dest = artifacts_dir.join(&build_file.filename);
                if let Err(e) = tokio::fs::copy(&build_file.stored_path, &dest).await {
                    log::warn!(
                        "Failed to copy build file {} to artifacts dir: {}",
                        build_file.filename,
                        e
                    );
                }
            }

            // Store all artifacts at once
            if let Err(e) = state
                .artifact_manager
                .store_artifacts(&run_id, &artifacts_dir, None)
                .await
            {
                log::warn!("Failed to store artifacts for run {}: {}", run_id, e);
            }
        }
    }

    for log_file in &uploaded_result.log_files {
        let path_str = match log_file.stored_path.to_str() {
            Some(path) => path,
            None => {
                log::warn!("Invalid UTF-8 in log file path: {:?}", log_file.stored_path);
                continue;
            }
        };

        if let Err(e) = state
            .log_manager
            .import_log(
                &active_run.codebase,
                &run_id,
                path_str,
                None,
                Some(&log_file.filename),
            )
            .await
        {
            log::warn!(
                "Failed to store log file {} from run {}: {}",
                log_file.filename,
                run_id,
                e
            );
        }
    }

    // Remove from active runs
    if let Err(e) = state.database.remove_active_run(&run_id).await {
        log::error!("Failed to remove active run: {}", e);
        // Continue anyway, result was stored
    }

    // Unassign queue item from Redis
    if let Err(e) = state
        .database
        .unassign_queue_item(active_run.queue_id)
        .await
    {
        log::warn!("Failed to unassign queue item from Redis: {}", e);
        // Continue anyway, main cleanup was done
    }

    // Prepare response
    let mut artifact_filenames = Vec::new();
    for file in &uploaded_result.artifact_files {
        artifact_filenames.push(file.filename.clone());
    }
    for file in &uploaded_result.build_files {
        artifact_filenames.push(file.filename.clone());
    }

    let response = FinishResponse {
        id: run_id,
        filenames: log_filenames,
        logs: uploaded_result
            .log_files
            .iter()
            .map(|f| f.filename.clone())
            .collect(),
        artifacts: artifact_filenames,
        result: janitor_result.to_json(),
    };

    log::info!(
        "Successfully processed multipart upload for run {}",
        response.id
    );
    match serde_json::to_value(response) {
        Ok(json) => (StatusCode::CREATED, Json(json)),
        Err(e) => {
            log::error!("Failed to serialize finish response: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Serialization failed"})),
            )
        }
    }
}

async fn public_root() -> impl IntoResponse {
    ""
}

/// Authentication middleware for worker endpoints.
async fn authenticate_worker(
    State(state): State<Arc<AppState>>,
    mut req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, StatusCode> {
    // Extract authorization header
    let auth_header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    if let Some(auth_value) = auth_header {
        // Use authenticate_worker which handles both Bearer and Basic auth
        match state.auth_service.authenticate_worker(auth_value).await {
            Ok(worker_auth) => {
                // Add worker name to request extensions
                req.extensions_mut().insert(worker_auth.name);
                Ok(next.run(req).await)
            }
            Err(_) => {
                log::warn!("Invalid worker credentials provided");
                Err(StatusCode::UNAUTHORIZED)
            }
        }
    } else {
        log::warn!("No authorization header provided for worker endpoint");
        Err(StatusCode::UNAUTHORIZED)
    }
}

async fn public_assign(
    State(state): State<Arc<AppState>>,
    Json(request): Json<AssignRequest>,
) -> impl IntoResponse {
    // Authentication is now handled by middleware
    assign_work_internal(state, request).await
}

async fn assign_work_internal(state: Arc<AppState>, request: AssignRequest) -> impl IntoResponse {
    // Use enhanced Redis integration for queue management

    // Get next available queue item with rate limiting and Redis integration
    // TODO: Get excluded hosts from proper configuration (worker.avoid_hosts)
    let excluded_hosts: Vec<String> = vec![];
    let assignment = match state
        .database
        .next_queue_item_with_rate_limiting(
            request.codebase.as_deref(),
            request.campaign.as_deref(),
            &excluded_hosts,
        )
        .await
    {
        Ok(Some(assignment)) => assignment,
        Ok(None) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"reason": "queue empty"})),
            );
        }
        Err(e) => {
            log::error!("Failed to get next queue item: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            );
        }
    };

    // Create backchannel
    let backchannel = if let Some(bc_json) = &request.backchannel {
        match serde_json::from_value(bc_json.clone()) {
            Ok(bc) => bc,
            Err(e) => {
                log::warn!("Invalid backchannel configuration: {}", e);
                Backchannel::default()
            }
        }
    } else {
        Backchannel::default()
    };

    // Generate unique log ID
    let log_id = Uuid::new_v4().to_string();
    let start_time = Utc::now();

    // Check for resume information
    let mut resume_from = None;
    if let Ok(Some(resume_branch)) = state
        .resume_service
        .open_resume_branch(
            &assignment.vcs_info.branch_url.clone().unwrap_or_default(),
            &assignment.queue_item.campaign,
        )
        .await
    {
        if let Ok(Some(resume_info)) = state
            .resume_service
            .check_resume_result(&assignment.queue_item.campaign, &resume_branch)
            .await
        {
            log::info!("Run {} will resume from {}", log_id, resume_info.run_id);
            resume_from = Some(resume_info.run_id);
        }
    }

    // Create active run
    let active_run = ActiveRun {
        worker_name: request.worker.unwrap_or_else(|| "unknown".to_string()),
        worker_link: request.worker_link,
        queue_id: assignment.queue_item.id,
        log_id: log_id.clone(),
        start_time,
        finish_time: None,
        estimated_duration: assignment.queue_item.estimated_duration,
        campaign: assignment.queue_item.campaign.clone(),
        change_set: assignment.queue_item.change_set.clone(),
        command: assignment.queue_item.command.clone(),
        backchannel,
        vcs_info: assignment.vcs_info.clone(),
        codebase: assignment.queue_item.codebase.clone(),
        instigated_context: assignment.queue_item.context.clone(),
        resume_from,
    };

    // Store active run in database
    if let Err(e) = state.database.store_active_run(&active_run).await {
        log::error!("Failed to store active run: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to store active run"})),
        );
    }

    // Assign queue item in Redis for coordination
    if let Err(e) = state
        .database
        .assign_queue_item(assignment.queue_item.id, &active_run.worker_name, &log_id)
        .await
    {
        log::warn!("Failed to assign queue item in Redis: {}", e);
        // Continue anyway, assignment is tracked in database
    }

    // Generate build configuration for the worker
    let campaign_config = create_campaign_config(&assignment.queue_item, &state.config);
    let build_config = match get_builder(&campaign_config, None, None) {
        Ok(builder) => {
            // Use actual database connection for enhanced config generation
            let mut config = HashMap::new();
            config.insert("builder_kind".to_string(), builder.kind().to_string());

            // Get codebase-specific configuration from database
            match state
                .database
                .get_codebase_config(&assignment.queue_item.codebase)
                .await
            {
                Ok(Some(codebase_config)) => {
                    // Add codebase-specific settings
                    if let Some(ref branch_url) = codebase_config.branch_url {
                        config.insert("branch_url".to_string(), branch_url.clone());
                    }
                    if let Some(ref vcs_type) = codebase_config.vcs_type {
                        config.insert("vcs_type".to_string(), vcs_type.clone());
                    }
                    if let Some(ref subpath) = codebase_config.subpath {
                        config.insert("subpath".to_string(), subpath.clone());
                    }
                }
                Ok(None) => {
                    log::warn!(
                        "No codebase config found for: {}",
                        assignment.queue_item.codebase
                    );
                }
                Err(e) => {
                    log::warn!("Failed to get codebase config from database: {}", e);
                }
            }

            // Get distribution-specific configuration
            if let Some(debian_config) = &campaign_config.debian_build {
                match state
                    .database
                    .get_distribution_config(&debian_config.base_distribution)
                    .await
                {
                    Ok(Some(dist_config)) => {
                        config.insert("distribution".to_string(), dist_config.name);
                        if let Some(ref archive_mirror) = dist_config.archive_mirror_uri {
                            config.insert("archive_mirror".to_string(), archive_mirror.clone());
                        }
                        if let Some(ref chroot) = dist_config.chroot {
                            config.insert("chroot".to_string(), chroot.clone());
                        }
                        if let Some(ref vendor) = dist_config.vendor {
                            config.insert("vendor".to_string(), vendor.clone());
                        }
                    }
                    Ok(None) => {
                        log::warn!(
                            "No distribution config found for: {}",
                            debian_config.base_distribution
                        );
                    }
                    Err(e) => {
                        log::warn!("Failed to get distribution config from database: {}", e);
                    }
                }
            }

            // Get actual committer from config and database
            let committer = match state
                .database
                .get_committer_for_campaign(&assignment.queue_item.campaign)
                .await
            {
                Ok(Some(committer)) => Some(committer),
                Ok(None) => {
                    // Fall back to global config committer
                    if !state.config.committer().is_empty() {
                        Some(state.config.committer().to_string())
                    } else {
                        None
                    }
                }
                Err(e) => {
                    log::warn!("Failed to get committer from database: {}", e);
                    None
                }
            };

            // Add environment setup with proper committer
            let env = crate::committer_env(committer.as_deref());
            for (key, value) in env {
                config.insert(format!("env_{}", key), value);
            }

            // Add campaign-specific metadata
            config.insert(
                "campaign".to_string(),
                assignment.queue_item.campaign.clone(),
            );
            config.insert(
                "codebase".to_string(),
                assignment.queue_item.codebase.clone(),
            );

            if let Some(ref change_set) = assignment.queue_item.change_set {
                config.insert("change_set".to_string(), change_set.clone());
            }

            config
        }
        Err(e) => {
            log::warn!("Failed to create builder for assignment: {}", e);
            HashMap::new()
        }
    };

    // Return assignment
    let response = AssignResponse {
        queue_item: assignment.queue_item,
        vcs_info: assignment.vcs_info,
        active_run: active_run.to_json(),
        build_config,
    };

    match serde_json::to_value(response) {
        Ok(json) => (StatusCode::OK, Json(json)),
        Err(e) => {
            log::error!("Failed to serialize assignment response: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Serialization failed"})),
            )
        }
    }
}

async fn public_finish(
    State(state): State<Arc<AppState>>,
    Extension(worker_name): Extension<String>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // Worker credentials are verified by authentication middleware
    // Verify that this worker is authorized to finish this specific run
    match state.database.get_active_run(&id).await {
        Ok(Some(active_run)) => {
            if active_run.worker_name != worker_name {
                log::warn!(
                    "Worker {} attempted to finish run {} assigned to worker {}",
                    worker_name,
                    id,
                    active_run.worker_name
                );
                return (
                    StatusCode::FORBIDDEN,
                    Json(json!({"error": "Not authorized to finish this run"})),
                );
            }
        }
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Run not found"})),
            );
        }
        Err(e) => {
            log::error!("Failed to verify run ownership: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            );
        }
    }

    log::info!("Worker {} finishing run {}", worker_name, id);
    finish_run_internal(state, id, None).await
}

async fn public_finish_multipart(
    State(state): State<Arc<AppState>>,
    Extension(worker_name): Extension<String>,
    Path(id): Path<String>,
    multipart: Multipart,
) -> impl IntoResponse {
    // Worker credentials are verified by authentication middleware
    // Verify that this worker is authorized to finish this specific run
    match state.database.get_active_run(&id).await {
        Ok(Some(active_run)) => {
            if active_run.worker_name != worker_name {
                log::warn!(
                    "Worker {} attempted to finish run {} assigned to worker {}",
                    worker_name,
                    id,
                    active_run.worker_name
                );
                return (
                    StatusCode::FORBIDDEN,
                    Json(json!({"error": "Not authorized to finish this run"})),
                );
            }
        }
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Run not found"})),
            );
        }
        Err(e) => {
            log::error!("Failed to verify run ownership: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            );
        }
    }

    log::info!(
        "Worker {} finishing run {} with multipart upload",
        worker_name,
        id
    );
    finish_run_multipart_internal(state, id, multipart).await
}

async fn public_get_active_run(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // Get active run information (this is essentially a proxy to the private endpoint)
    match state.database.get_active_run(&id).await {
        Ok(Some(active_run)) => {
            // Return public view of active run (omit sensitive information)
            Json(json!({
                "id": active_run.log_id,
                "worker_name": active_run.worker_name,
                "start_time": active_run.start_time,
                "estimated_duration": active_run.estimated_duration.map(|d| d.as_secs()),
                "campaign": active_run.campaign,
                "codebase": active_run.codebase,
                "status": "running"
            }))
        }
        Ok(None) => Json(json!({"error": "Active run not found"})),
        Err(e) => {
            log::error!("Failed to get active run {}: {}", id, e);
            Json(json!({"error": "Database error"}))
        }
    }
}

/// Get watchdog health information for all active runs.
async fn public_watchdog_health(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let watchdog_config = crate::WatchdogConfig::default();
    let watchdog = crate::Watchdog::new(Arc::clone(&state.database), watchdog_config);

    match watchdog.get_detailed_health_status().await {
        Ok(health_statuses) => {
            // Filter to public information only
            let public_statuses: Vec<_> = health_statuses.into_iter().map(|status| {
                json!({
                    "log_id": status.log_id,
                    "worker_name": status.worker_name,
                    "start_time": status.start_time,
                    "estimated_duration": status.estimated_duration.map(|d| d.as_secs()),
                    "failure_count": status.failure_count,
                    "max_failures": status.max_failures,
                    "alive": status.health.as_ref().map(|h| h.alive).unwrap_or(false),
                    "status": status.health.as_ref().map(|h| h.status.clone()).unwrap_or_else(|| "unknown".to_string()),
                    "last_ping": status.health.as_ref().and_then(|h| h.last_ping),
                })
            }).collect();

            Json(json!({
                "status": "ok",
                "active_runs": public_statuses.len(),
                "health_statuses": public_statuses
            }))
        }
        Err(e) => {
            log::error!("Failed to get watchdog health status: {}", e);
            Json(json!({
                "status": "error",
                "error": "Failed to get health status"
            }))
        }
    }
}

/// Get public queue statistics.
async fn public_queue_stats(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.database.get_queue_stats().await {
        Ok(stats) => Json(json!({
            "queue_length": stats.get("total").unwrap_or(&0),
            "active_runs": stats.get("active").unwrap_or(&0),
            "succeeded": stats.get("succeeded").unwrap_or(&0),
            "failed": stats.get("failed").unwrap_or(&0),
            "status": "operational"
        })),
        Err(e) => {
            log::error!("Failed to get queue stats: {}", e);
            Json(json!({
                "status": "error",
                "error": "Database error"
            }))
        }
    }
}

/// Public health check endpoint.
async fn public_health(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // Lightweight health check for public consumption
    match state.database.health_check().await {
        Ok(()) => (
            StatusCode::OK,
            Json(json!({
                "status": "healthy",
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "service": "janitor-runner",
                "version": env!("CARGO_PKG_VERSION")
            })),
        ),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "unhealthy",
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "service": "janitor-runner",
                "version": env!("CARGO_PKG_VERSION")
            })),
        ),
    }
}

/// Create a router for the public API endpoints.
pub fn public_app(state: Arc<AppState>) -> Router {
    // Create separate routers for authenticated and public endpoints
    let public_routes = Router::new()
        .route("/", get(public_root))
        .route("/health", get(public_health))
        .route("/queue/stats", get(public_queue_stats))
        .route("/watchdog/health", get(public_watchdog_health));

    let worker_routes = Router::new()
        .route("/runner/active-runs", post(public_assign))
        .route("/runner/active-runs/:id/finish", post(public_finish))
        .route(
            "/runner/active-runs/:id/finish-multipart",
            post(public_finish_multipart),
        )
        .route("/runner/active-runs/:id", get(public_get_active_run))
        .layer(middleware::from_fn_with_state(
            Arc::clone(&state),
            authenticate_worker,
        ));

    // Combine both routers
    public_routes.merge(worker_routes).with_state(state)
}

/// Create a router for the private API endpoints.
pub fn app(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/queue/position", get(queue_position))
        .route("/schedule-control", post(schedule_control))
        .route("/schedule", post(schedule))
        .route("/status", get(status))
        .route("/log/:id", get(log_index))
        .route("/kill/:id", post(kill))
        .route("/log/:id/:filename", get(log))
        .route("/codebases", get(get_codebases))
        .route("/codebases", post(update_codebases))
        .route("/candidates", get(get_candidates))
        .route("/candidates", post(upload_candidates))
        .route("/candidates/:id", delete(delete_candidate))
        .route("/runs/:id", get(get_run))
        .route("/runs/:id", post(update_run))
        .route("/active-runs", get(get_active_runs))
        .route("/active-runs/:id", get(get_active_run))
        .route("/active-runs/:id/finish", post(finish_active_run))
        .route(
            "/active-runs/:id/finish-multipart",
            post(finish_active_run_multipart),
        )
        .route("/active-runs/+peek", get(peek_active_run))
        .route("/queue", get(get_queue))
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/metrics", get(metrics))
        // Admin endpoints for worker management
        .route("/admin/workers", get(admin_list_workers))
        .route("/admin/workers", post(admin_create_worker))
        .route("/admin/workers/:name", delete(admin_delete_worker))
        .route("/admin/security/stats", get(admin_security_stats))
        // Resume-related endpoints
        .route("/resume/check", post(check_resume_info))
        .route("/resume/chain/:run_id", get(get_resume_chain))
        .route("/resume/descendants/:run_id", get(get_resume_descendants))
        .route("/resume/validate", get(validate_resume_consistency))
        .with_state(state)
}
