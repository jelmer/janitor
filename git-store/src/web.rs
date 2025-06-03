//! Web interface for git-store

use crate::{
    database::DatabaseManager,
    error::Result,
    git_http::{git_backend, git_diff, revision_info},
    repository::RepositoryManager,
    Config,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tera::Tera;
use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};
// use tracing::info; // Will be used later

/// Application state
#[derive(Clone)]
pub struct AppState {
    /// Repository manager
    pub repo_manager: Arc<RepositoryManager>,
    /// Configuration
    pub config: Arc<Config>,
    /// Template engine
    pub tera: Arc<Tera>,
    /// Database manager
    pub db_manager: Arc<DatabaseManager>,
}

/// Create the admin application
pub fn create_admin_app(state: AppState) -> Router {
    Router::new()
        // Admin endpoints
        .route("/health", get(health_check))
        .route("/ready", get(ready_check))
        .route("/", get(list_repositories))
        .route("/:codebase", get(repository_info))
        .route("/:codebase/remote/:name", post(set_remote))
        
        // Git operations
        .route("/:codebase/diff", get(git_diff))
        .route("/:codebase/revision", get(revision_info))
        
        // Git HTTP backend (placeholder)
        .route("/:codebase/git/*path", get(git_backend).post(git_backend))
        
        // Middleware
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        
        // State
        .with_state(state)
}

/// Create the public application
pub fn create_public_app(state: AppState) -> Router {
    Router::new()
        // Public endpoints (read-only)
        .route("/health", get(health_check))
        .route("/", get(list_repositories))
        .route("/:codebase", get(repository_info))
        
        // Git operations (read-only)
        .route("/:codebase/diff", get(git_diff))
        .route("/:codebase/revision", get(revision_info))
        
        // Git HTTP backend (placeholder)
        .route("/:codebase/git/*path", get(git_backend).post(git_backend))
        
        // Middleware
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        
        // State
        .with_state(state)
}

/// Health check endpoint
async fn health_check() -> &'static str {
    "OK"
}

/// Ready check endpoint
async fn ready_check(State(state): State<AppState>) -> Result<&'static str> {
    // Check database connection
    state.db_manager.health_check().await?;
    Ok("OK")
}

/// List repositories
async fn list_repositories(State(state): State<AppState>) -> Result<Response> {
    let repos = state.repo_manager.list_repositories()?;
    
    // Check Accept header for content negotiation
    // For now, just return JSON
    Ok(Json(repos).into_response())
}

/// Get repository info
async fn repository_info(
    State(state): State<AppState>,
    Path(codebase): Path<String>,
) -> Result<Response> {
    let info = state.repo_manager.get_repo_info(&codebase)?;
    Ok(Json(info).into_response())
}

/// Set repository remote
async fn set_remote(
    State(state): State<AppState>,
    Path((codebase, name)): Path<(String, String)>,
    body: String,
) -> Result<StatusCode> {
    state.repo_manager.set_remote(&codebase, &name, &body)?;
    Ok(StatusCode::NO_CONTENT)
}

/// Initialize template engine
pub fn init_templates(templates_path: Option<&std::path::Path>) -> Result<Tera> {
    let tera = if let Some(path) = templates_path {
        let pattern = path.join("**/*.html").display().to_string();
        Tera::new(&pattern)?
    } else {
        // Use embedded templates
        let mut tera = Tera::default();
        
        // Add basic templates
        tera.add_raw_template(
            "base.html",
            r#"<!DOCTYPE html>
<html>
<head>
    <title>{% block title %}Git Store{% endblock %}</title>
    <meta charset="utf-8">
    <style>
        body { font-family: sans-serif; margin: 20px; }
        .repo-list { list-style: none; padding: 0; }
        .repo-item { padding: 10px; border-bottom: 1px solid #ccc; }
    </style>
</head>
<body>
    <h1>Git Store</h1>
    {% block content %}{% endblock %}
</body>
</html>"#,
        )?;
        
        tera.add_raw_template(
            "index.html",
            r#"{% extends "base.html" %}
{% block title %}Git Store - Repositories{% endblock %}
{% block content %}
    <h2>Repositories</h2>
    <ul class="repo-list">
    {% for repo in repositories %}
        <li class="repo-item">
            <a href="/{{ repo }}">{{ repo }}</a>
        </li>
    {% endfor %}
    </ul>
{% endblock %}"#,
        )?;
        
        tera
    };
    
    Ok(tera)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_templates() {
        let tera = init_templates(None).unwrap();
        assert!(tera.get_template_names().any(|name| name == "base.html"));
        assert!(tera.get_template_names().any(|name| name == "index.html"));
    }
}