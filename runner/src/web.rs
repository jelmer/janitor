use crate::{ActiveRun, AppState, Backchannel, QueueItem, CampaignConfig, get_builder};
use axum::{
    extract::Path, extract::State, http::StatusCode, response::IntoResponse, routing::delete,
    routing::get, routing::post, Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
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

/// Create a basic campaign configuration from a queue item.
/// TODO: In a full implementation, this would load from actual config files
fn create_campaign_config(queue_item: &QueueItem) -> CampaignConfig {
    // For now, create a basic configuration
    // In reality, this would be loaded from campaign configuration files
    CampaignConfig {
        generic_build: Some(crate::GenericBuildConfig {
            chroot: None,
        }),
        debian_build: None, // Could be determined by campaign type
    }
}

async fn queue_position(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // For now, return a basic response indicating position functionality
    // TODO: Implement actual queue position calculation
    match state.database.get_queue_stats().await {
        Ok(stats) => Json(json!({
            "position": 0,
            "total": stats.get("total").unwrap_or(&0)
        })),
        Err(e) => {
            log::error!("Failed to get queue position: {}", e);
            Json(json!({"error": "Database error"}))
        }
    }
}

async fn schedule_control(State(state): State<Arc<AppState>>) {
    unimplemented!()
}

async fn schedule(State(state): State<Arc<AppState>>) {
    unimplemented!()
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
            // For now, return a basic log file listing
            // TODO: Integrate with actual log storage system
            (
                StatusCode::OK,
                Json(json!({
                    "log_id": id,
                    "files": ["worker.log", "build.log"]
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
    // Check if the run exists
    match state.database.run_exists(&id).await {
        Ok(true) => {
            // For now, return placeholder content
            // TODO: Integrate with actual log storage system
            if crate::is_log_filename(&filename) {
                (
                    StatusCode::OK,
                    format!("Log content for run {} file {}", id, filename),
                )
            } else {
                (StatusCode::BAD_REQUEST, "Invalid log filename".to_string())
            }
        }
        Ok(false) => (StatusCode::NOT_FOUND, "Run not found".to_string()),
        Err(e) => {
            log::error!("Failed to check run existence for {}: {}", id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        }
    }
}

async fn kill(State(state): State<Arc<AppState>>, Path(id): Path<String>) {
    unimplemented!()
}

async fn get_codebases(State(state): State<Arc<AppState>>) {
    unimplemented!()
}

async fn update_codebases(State(state): State<Arc<AppState>>) {
    unimplemented!()
}

async fn delete_candidate(State(state): State<Arc<AppState>>, Path(id): Path<String>) {
    unimplemented!()
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
    
    match state.database.update_run_publish_status(&id, &request.publish_status).await {
        Ok(Some((run_id, codebase, suite))) => {
            Json(json!({
                "run_id": run_id,
                "codebase": codebase,
                "suite": suite,
                "publish_status": request.publish_status
            }))
        }
        Ok(None) => {
            Json(json!({
                "error": format!("no such run: {}", id)
            }))
        }
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

async fn peek_active_run(State(state): State<Arc<AppState>>) {
    unimplemented!()
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

async fn health() -> impl IntoResponse {
    "OK"
}

async fn ready() -> impl IntoResponse {
    "OK"
}

async fn finish_active_run(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    finish_run_internal(state, id, None).await
}

async fn finish_run_internal(
    state: Arc<AppState>,
    run_id: String,
    worker_result: Option<crate::WorkerResult>,
) -> impl IntoResponse {
    // Get the active run
    let active_run = match state.database.get_active_run(&run_id).await {
        Ok(Some(run)) => run,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"reason": format!("no such run {}", run_id)}))
            );
        }
        Err(e) => {
            log::error!("Failed to get active run {}: {}", run_id, e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"}))
            );
        }
    };

    // Create a result from the active run
    // For now, create a simple success result
    // TODO: Process worker_result when provided via multipart upload
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

    // Process builder result if we have worker_result with builder data
    if let Some(ref wr) = worker_result {
        if let Some(ref builder_result) = wr.builder_result {
            janitor_result.builder_result = Some(builder_result.clone());
        }
    } else {
        // For now, create a basic builder result based on the campaign
        // TODO: This should be done by the actual worker, not here
        let campaign_config = create_campaign_config(&QueueItem {
            id: active_run.queue_id,
            context: active_run.instigated_context.clone(),
            command: active_run.command.clone(),
            estimated_duration: active_run.estimated_duration,
            campaign: active_run.campaign.clone(),
            refresh: false, // TODO: Get from queue item
            requester: None, // TODO: Get from queue item
            change_set: active_run.change_set.clone(),
            codebase: active_run.codebase.clone(),
        });

        // Get appropriate builder
        if let Ok(builder) = get_builder(&campaign_config, None, None) {
            // For basic completion, just create a simple result
            // In a real implementation, the worker would provide the output directory
            if builder.kind() == "debian" {
                janitor_result.builder_result = Some(crate::BuilderResult::Debian {
                    source: None,
                    build_version: None,
                    build_distribution: None,
                    changes_filenames: None,
                    lintian_result: None,
                    binary_packages: None,
                });
            } else {
                janitor_result.builder_result = Some(crate::BuilderResult::Generic);
            }
        }
    }

    // Store the result in the database
    if let Err(e) = state.database.update_run_result(
        &run_id,
        &janitor_result.code,
        janitor_result.description.as_deref(),
        janitor_result.failure_details.as_ref(),
        janitor_result.transient,
        janitor_result.finish_time,
    ).await {
        log::error!("Failed to store run result: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to store result"}))
        );
    }

    // Store builder result if present
    if let Some(ref builder_result) = janitor_result.builder_result {
        if let Err(e) = state.database.store_builder_result(&run_id, builder_result).await {
            log::error!("Failed to store builder result: {}", e);
            // Continue anyway, main result was stored
        }
    }

    // Remove from active runs
    if let Err(e) = state.database.remove_active_run(&run_id).await {
        log::error!("Failed to remove active run: {}", e);
        // Continue anyway, result was stored
    }

    // For now, return basic response
    // TODO: Handle file uploads, log processing, artifact management
    let response = FinishResponse {
        id: run_id,
        filenames: vec![], // TODO: Process uploaded files
        logs: vec![], // TODO: Process log files
        artifacts: vec![], // TODO: Process artifacts
        result: janitor_result.to_json(),
    };

    (StatusCode::CREATED, Json(serde_json::to_value(response).unwrap()))
}

async fn public_root() -> impl IntoResponse {
    ""
}

async fn public_assign(
    State(state): State<Arc<AppState>>,
    Json(request): Json<AssignRequest>,
) -> impl IntoResponse {
    assign_work_internal(state, request).await
}

async fn assign_work_internal(
    state: Arc<AppState>,
    request: AssignRequest,
) -> impl IntoResponse {
    // For now, implement a simplified version without Redis or rate limiting
    // TODO: Add Redis integration for tracking assigned items and rate limiting
    
    // Get next available queue item
    let assignment = match state.database.next_queue_item(
        request.codebase.as_deref(),
        request.campaign.as_deref(),
        &[], // TODO: Add excluded hosts from rate limiting
        &[], // TODO: Get assigned queue items from Redis
    ).await {
        Ok(Some(assignment)) => assignment,
        Ok(None) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"reason": "queue empty"}))
            );
        }
        Err(e) => {
            log::error!("Failed to get next queue item: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"}))
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

    // Create active run
    let active_run = ActiveRun {
        worker_name: request.worker.unwrap_or_else(|| "unknown".to_string()),
        worker_link: request.worker_link,
        queue_id: assignment.queue_item.id,
        log_id: log_id.clone(),
        start_time,
        estimated_duration: assignment.queue_item.estimated_duration,
        campaign: assignment.queue_item.campaign.clone(),
        change_set: assignment.queue_item.change_set.clone(),
        command: assignment.queue_item.command.clone(),
        backchannel,
        vcs_info: assignment.vcs_info.clone(),
        codebase: assignment.queue_item.codebase.clone(),
        instigated_context: assignment.queue_item.context.clone(),
        resume_from: None,
    };

    // Store active run in database
    if let Err(e) = state.database.store_active_run(&active_run).await {
        log::error!("Failed to store active run: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to store active run"}))
        );
    }

    // Generate build configuration for the worker
    let campaign_config = create_campaign_config(&assignment.queue_item);
    let build_config = match get_builder(&campaign_config, None, None) {
        Ok(builder) => {
            // TODO: Use actual database connection for config generation
            // For now, create basic config without database
            let mut config = HashMap::new();
            config.insert("builder_kind".to_string(), builder.kind().to_string());
            
            // Add basic environment setup
            let env = crate::committer_env(None); // TODO: Get actual committer
            for (key, value) in env {
                config.insert(format!("env_{}", key), value);
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

    (StatusCode::OK, Json(serde_json::to_value(response).unwrap()))
}

async fn public_finish(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // TODO: Add worker credentials check
    // TODO: Handle multipart uploads for files and worker result
    finish_run_internal(state, id, None).await
}

async fn public_get_active_run(State(state): State<Arc<AppState>>, Path(id): Path<String>) {
    unimplemented!()
}

/// Create a router for the public API endpoints.
pub fn public_app(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(public_root))
        .route("/runner/active-runs", post(public_assign))
        .route("/runner/active-runs/:id/finish", post(public_finish))
        .route("/runner/active-runs/:id", get(public_get_active_run))
        .with_state(state)
}

/// Create a router for the private API endpoints.
pub fn app(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/queue/position", get(queue_position))
        .route("/schedule-control", post(schedule_control))
        .route("/schedule", post(schedule))
        .route("/status", get(status))
        .route("/log/:id", get(log_index))
        .route("/kill:id", post(kill))
        .route("/log/:id/:filename", get(log))
        .route("/codebases", get(get_codebases))
        .route("/codebases", post(update_codebases))
        .route("/candidates/:id", delete(delete_candidate))
        .route("/runs/:id", get(get_run))
        .route("/runs/:id", post(update_run))
        .route("/active-runs", get(get_active_runs))
        .route("/active-runs/:id", get(get_active_run))
        .route("/active-runs/:id/finish", post(finish_active_run))
        .route("/active-runs/+peek", get(peek_active_run))
        .route("/queue", get(get_queue))
        .route("/health", get(health))
        .route("/ready", get(ready))
        .with_state(state)
}
