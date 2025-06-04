use axum::{
    response::IntoResponse,
    routing::get,
    Router,
};
use tower_http::services::ServeDir;

use crate::{
    app::AppState,
    handlers::{pkg, simple},
    webhook,
};

mod cupboard;

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
        .route(
            "/:campaign/c/:codebase/:run_id/logs/:log_name",
            get(pkg::view_log),
        )
        .route(
            "/:campaign/c/:codebase/:run_id/logs/:log_name/download",
            get(pkg::download_log),
        )
        // VCS diff viewing
        .route("/:campaign/c/:codebase/:run_id/diff", get(pkg::view_diff))
        // Debian package diff viewing
        .route(
            "/:campaign/c/:codebase/:run_id/debdiff",
            get(pkg::view_debdiff),
        )
        // Result files (logs, artifacts)
        .route("/:suite/pkg/:pkg/:run_id/*filename", get(serve_result_file))
        // Webhooks
        .merge(webhook::create_webhook_routes())
        // Cupboard admin interface
        .merge(cupboard::cupboard_routes())
        .merge(cupboard::cupboard_api_routes())
        // Legacy package routes (redirect to new URLs)
        .route("/pkg", get(legacy_package_list))
        .route("/pkg/:name", get(legacy_package_detail))
}

// Placeholder handlers for routes not yet implemented

async fn vcs_repo_list(
    axum::extract::Path(vcs): axum::extract::Path<String>,
    axum::extract::State(state): axum::extract::State<AppState>,
) -> impl axum::response::IntoResponse {
    use crate::templates::create_base_context;
    use axum::response::Html;

    let mut context = create_base_context();
    context.insert("vcs_type", &vcs);

    // Fetch repositories from database based on VCS type
    match state.database.get_repositories_by_vcs(&vcs).await {
        Ok(repositories) => {
            context.insert("repositories", &repositories);
            context.insert("repository_count", &repositories.len());
        }
        Err(e) => {
            tracing::error!("Failed to fetch {} repositories: {}", vcs, e);
            context.insert("repositories", &Vec::<String>::new());
            context.insert("repository_count", &0);
        }
    }

    match state.templates.render("repo-list.html", &context) {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("Template rendering error: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn serve_result_file(
    axum::extract::Path((suite, pkg, run_id, filename)): axum::extract::Path<(
        String,
        String,
        String,
        String,
    )>,
    axum::extract::State(state): axum::extract::State<AppState>,
) -> impl axum::response::IntoResponse {
    use axum::body::Body;
    use axum::http::{header, StatusCode};
    use axum::response::{IntoResponse, Response};

    // Sanitize the filename to prevent path traversal
    let safe_filename = filename.replace("..", "").replace('/', "_");

    // Try to get the file from the log manager
    match state
        .log_manager
        .get_log(&pkg, &run_id, &safe_filename)
        .await
    {
        Ok(mut content) => {
            // Read the content into a Vec<u8>
            let mut buffer = Vec::new();
            match std::io::Read::read_to_end(&mut *content, &mut buffer) {
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("Failed to read log content: {}", e);
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }
            }
            let content = buffer;
            // Determine content type based on file extension
            let content_type = match safe_filename.split('.').last() {
                Some("log") | Some("txt") => "text/plain; charset=utf-8",
                Some("html") | Some("htm") => "text/html; charset=utf-8",
                Some("json") => "application/json",
                Some("xml") => "application/xml",
                Some("tar") => "application/x-tar",
                Some("gz") => "application/gzip",
                Some("bz2") => "application/x-bzip2",
                Some("xz") => "application/x-xz",
                Some("deb") => "application/x-debian-package",
                Some("dsc") => "text/plain; charset=utf-8",
                Some("changes") => "text/plain; charset=utf-8",
                _ => "application/octet-stream",
            };

            // Set appropriate headers
            let response = Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, content_type)
                .header(
                    header::CONTENT_DISPOSITION,
                    format!("inline; filename=\"{}\"", safe_filename),
                )
                .body(Body::from(content))
                .unwrap();

            response.into_response()
        }
        Err(e) => {
            tracing::error!(
                "Failed to serve file {}/{}/{}: {}",
                pkg,
                run_id,
                safe_filename,
                e
            );

            // Check if it's a not found error or server error
            match e.to_string().contains("not found") || e.to_string().contains("NotFound") {
                true => StatusCode::NOT_FOUND.into_response(),
                false => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
    }
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
    use axum::response::Redirect;

    // Try to determine the most appropriate campaign for this package
    let default_campaign = if let Some(campaigns) = state.config.janitor() {
        // Use the first available campaign as default
        campaigns
            .campaign
            .first()
            .map(|c| c.name())
            .unwrap_or("lintian-fixes")
    } else {
        "lintian-fixes"
    };

    // For now, redirect to the default campaign
    // In a full implementation, this could query the database to find
    // which campaigns this package has been run with
    Redirect::permanent(&format!("/{}/c/{}/", default_campaign, name))
}
