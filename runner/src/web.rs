use crate::AppState;
use axum::{
    extract::Path, extract::State, http::StatusCode, response::IntoResponse, routing::delete,
    routing::get, routing::post, Json, Router,
};
use serde_json::json;
use std::sync::Arc;

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

async fn update_run(State(state): State<Arc<AppState>>, Path(id): Path<String>) {
    unimplemented!()
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

async fn finish_active_run(State(state): State<Arc<AppState>>, Path(id): Path<String>) {
    unimplemented!()
}

async fn public_root() -> impl IntoResponse {
    ""
}

async fn public_assign(State(state): State<Arc<AppState>>) {
    unimplemented!()
}

async fn public_finish(State(state): State<Arc<AppState>>, Path(id): Path<String>) {
    unimplemented!()
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
