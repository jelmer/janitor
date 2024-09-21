use crate::AppState;
use axum::{
    extract::Path, extract::State, response::IntoResponse, routing::delete, routing::get,
    routing::post, Router,
};
use std::sync::Arc;

async fn queue_position(State(state): State<Arc<AppState>>) {
    unimplemented!()
}

async fn schedule_control(State(state): State<Arc<AppState>>) {
    unimplemented!()
}

async fn schedule(State(state): State<Arc<AppState>>) {
    unimplemented!()
}

async fn status(State(state): State<Arc<AppState>>) {
    unimplemented!()
}

async fn log_index(State(state): State<Arc<AppState>>, Path(id): Path<String>) {
    unimplemented!()
}

async fn log(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Path(filename): Path<String>,
) {
    unimplemented!()
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

async fn get_run(State(state): State<Arc<AppState>>, Path(id): Path<String>) {
    unimplemented!()
}

async fn update_run(State(state): State<Arc<AppState>>, Path(id): Path<String>) {
    unimplemented!()
}

async fn get_active_runs(State(state): State<Arc<AppState>>) {
    unimplemented!()
}

async fn get_active_run(State(state): State<Arc<AppState>>, Path(id): Path<String>) {
    unimplemented!()
}

async fn peek_active_run(State(state): State<Arc<AppState>>) {
    unimplemented!()
}

async fn get_queue(State(state): State<Arc<AppState>>) {
    unimplemented!()
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

pub fn public_app(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(public_root))
        .route("/runner/active-runs", post(public_assign))
        .route("/runner/active-runs/:id/finish", post(public_finish))
        .route("/runner/active-runs/:id", get(public_get_active_run))
        .with_state(state)
}

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
        .route("/run/:id", get(get_run))
        .route("/run/:id", post(update_run))
        .route("/active-runs", get(get_active_runs))
        .route("/active-runs/:id", get(get_active_run))
        .route("/active-runs/:id/finish", post(finish_active_run))
        .route("/active-runs/+peek", get(peek_active_run))
        .route("/queue", get(get_queue))
        .route("/health", get(health))
        .route("/ready", get(ready))
        .with_state(state)
}
