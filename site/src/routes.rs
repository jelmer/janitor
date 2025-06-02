use axum::{
    routing::{get, post},
    Router,
};
use tower_http::services::ServeDir;

use crate::{
    app::AppState,
    handlers::{simple, pkg},
    webhook,
};

pub fn app_routes() -> Router<AppState> {
    Router::new()
        // Static file serving
        .nest_service("/_static", ServeDir::new("static"))
        .nest_service("/static", ServeDir::new("static"))
        
        // Homepage and static pages
        .route("/", get(simple::index))
        .route("/about", get(simple::about))
        .route("/credentials", get(simple::credentials))
        
        // Archive keyrings
        .route("/archive-keyring.asc", get(simple::archive_keyring_asc))
        .route("/archive-keyring.gpg", get(simple::archive_keyring_gpg))
        
        // VCS repository lists
        .route("/git/", get(vcs_repo_list))
        .route("/bzr/", get(vcs_repo_list))
        
        // Campaign/Suite routes
        .route("/:campaign/", get(simple::campaign_start))
        .route("/:suite/candidates", get(simple::campaign_candidates))
        .route("/:suite/ready", get(pkg::ready_list))
        .route("/:campaign/done", get(pkg::done_list))
        .route("/:suite/merge-proposals", get(pkg::merge_proposals))
        
        // Codebase and run routes
        .route("/:campaign/c/:codebase/", get(pkg::codebase_detail))
        .route("/:campaign/c/:codebase/:run_id", get(pkg::run_detail))
        
        // Log viewing and downloading (Phase 3.5.2 Package Views implementation)
        .route("/:campaign/c/:codebase/:run_id/logs/:log_name", get(pkg::view_log))
        .route("/:campaign/c/:codebase/:run_id/logs/:log_name/download", get(pkg::download_log))
        
        // VCS diff viewing
        .route("/:campaign/c/:codebase/:run_id/diff", get(pkg::view_diff))
        
        // Debian package diff viewing
        .route("/:campaign/c/:codebase/:run_id/debdiff", get(pkg::view_debdiff))
        
        // Result files (logs, artifacts)
        .route("/:suite/pkg/:pkg/:run_id/*filename", get(serve_result_file))
        
        // Webhooks
        .merge(webhook::create_webhook_routes())
        
        // Legacy package routes (redirect to new URLs)
        .route("/pkg", get(legacy_package_list))
        .route("/pkg/:name", get(legacy_package_detail))
}

// Placeholder handlers for routes not yet implemented

async fn vcs_repo_list(
    axum::extract::Path(vcs): axum::extract::Path<String>,
    axum::extract::State(state): axum::extract::State<AppState>,
) -> impl axum::response::IntoResponse {
    // TODO: Implement VCS repository listing
    axum::http::StatusCode::NOT_IMPLEMENTED
}

async fn serve_result_file(
    axum::extract::Path((suite, pkg, run_id, filename)): axum::extract::Path<(String, String, String, String)>,
    axum::extract::State(state): axum::extract::State<AppState>,
) -> impl axum::response::IntoResponse {
    // TODO: Implement result file serving
    axum::http::StatusCode::NOT_IMPLEMENTED
}


async fn legacy_package_list(
    axum::extract::State(state): axum::extract::State<AppState>,
    query: axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl axum::response::IntoResponse {
    // Redirect to new URL structure
    axum::response::Redirect::permanent("/")
}

async fn legacy_package_detail(
    axum::extract::Path(name): axum::extract::Path<String>,
    axum::extract::State(state): axum::extract::State<AppState>,
) -> impl axum::response::IntoResponse {
    // Redirect to appropriate campaign page
    // TODO: Determine correct campaign from package name
    axum::response::Redirect::permanent(&format!("/lintian-fixes/c/{}/", name))
}