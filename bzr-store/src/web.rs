//! Web server implementation for BZR Store service

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    middleware,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tera::{Context, Tera};
use tower::ServiceBuilder;
use tower_http::{compression::CompressionLayer, trace::TraceLayer};
use tracing::{info, warn};

use crate::config::Config;
use crate::database::DatabaseManager;
use crate::error::{BzrError, Result};
use crate::repository::{RepositoryManager, RepositoryPath, SubprocessRepositoryManager};
use crate::smart_protocol::{smart_protocol_handler, serve_bzr_file_handler};

/// Application state shared between handlers
#[derive(Clone)]
pub struct AppState {
    /// Application configuration
    pub config: Config,
    /// Database connection manager for authentication and validation
    pub database: DatabaseManager,
    /// Repository management interface for Bazaar operations
    pub repository_manager: Arc<dyn RepositoryManager>,
    /// Template engine for rendering HTML responses
    pub templates: Tera,
}

/// Query parameters for diff endpoint
#[derive(Debug, Deserialize)]
pub struct DiffQuery {
    /// Old revision identifier to diff from
    pub old: String,
    /// New revision identifier to diff to
    pub new: String,
}

/// Query parameters for revision info endpoint
#[derive(Debug, Deserialize)]
pub struct RevisionInfoQuery {
    /// Starting revision identifier for the range
    pub old: String,
    /// Ending revision identifier for the range
    pub new: String,
}

/// Response for repository listing
#[derive(Debug, Serialize)]
pub struct RepositoryListResponse {
    /// List of repository paths in the format campaign/codebase/role
    pub repositories: Vec<String>,
    /// Total number of repositories found
    pub count: usize,
}

/// Create both admin and public applications
pub async fn create_applications(config: Config) -> Result<(Router, Router)> {
    // Initialize database
    let database = DatabaseManager::new(&config).await?;
    
    // Initialize repository manager
    let repository_manager: Arc<dyn RepositoryManager> = Arc::new(
        SubprocessRepositoryManager::new(config.repository_path.clone(), database.clone())
    );
    
    // Initialize templates
    let mut templates = Tera::new("templates/**/*").unwrap_or_else(|_| {
        warn!("Template directory not found, using empty template engine");
        Tera::default()
    });
    
    // Add basic templates programmatically if directory doesn't exist
    if templates.get_template_names().count() == 0 {
        templates.add_raw_template("health.html", r#"
<!DOCTYPE html>
<html>
<head><title>BZR Store Health</title></head>
<body>
    <h1>BZR Store Service</h1>
    <p>Status: {{ status }}</p>
    <p>Timestamp: {{ timestamp }}</p>
</body>
</html>
"#).expect("Failed to add health template");

        templates.add_raw_template("repositories.html", r#"
<!DOCTYPE html>
<html>
<head><title>BZR Repositories</title></head>
<body>
    <h1>Bazaar Repositories</h1>
    <ul>
    {% for repo in repositories %}
        <li>{{ repo.path.campaign }}/{{ repo.path.codebase }}/{{ repo.path.role }}
            {% if repo.exists %} ✓{% else %} ✗{% endif %}</li>
    {% endfor %}
    </ul>
    <p>Total: {{ repositories | length }}</p>
</body>
</html>
"#).expect("Failed to add repositories template");
    }
    
    let app_state = AppState {
        config: config.clone(),
        database,
        repository_manager,
        templates,
    };
    
    // Create admin application (full access)
    let admin_app = create_admin_app(app_state.clone()).await;
    
    // Create public application (read-only)
    let public_app = create_public_app(app_state).await;
    
    Ok((admin_app, public_app))
}

/// Create the admin application with full access
async fn create_admin_app(state: AppState) -> Router {
    Router::new()
        // Health endpoints
        .route("/health", get(health_handler))
        .route("/ready", get(ready_handler))
        
        // Repository management
        .route("/repositories", get(list_repositories_handler))
        .route("/repositories/:campaign/:codebase/:role", post(create_repository_handler))
        .route("/:campaign/:codebase/:role/info", get(repository_info_handler))
        
        // Repository operations
        .route("/:campaign/:codebase/:role/diff", get(diff_handler))
        .route("/:campaign/:codebase/:role/revision-info", get(revision_info_handler))
        
        // Remote configuration
        .route("/:campaign/:codebase/:role/remotes", post(configure_remote_handler))
        .route("/:campaign/:codebase/:role/remotes", get(list_remotes_handler))
        
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CompressionLayer::new())
                .layer(middleware::from_fn_with_state(state.clone(), auth_middleware))
        )
        .with_state(state)
}

/// Create the public application with read-only access
async fn create_public_app(state: AppState) -> Router {
    Router::new()
        // Health endpoints
        .route("/health", get(health_handler))
        .route("/ready", get(ready_handler))
        
        // Read-only repository operations
        .route("/:campaign/:codebase/:role/diff", get(diff_handler))
        .route("/:campaign/:codebase/:role/revision-info", get(revision_info_handler))
        
        // Repository browsing
        .route("/repositories", get(list_repositories_handler))
        .route("/:campaign/:codebase/:role/info", get(repository_info_handler))
        
        // Bazaar smart protocol endpoint
        .route("/:campaign/:codebase/:role/.bzr/smart", post(smart_protocol_handler))
        
        // Repository file access (.bzr directory)
        .route("/:campaign/:codebase/:role/*file_path", get(serve_bzr_file_handler))
        
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CompressionLayer::new())
        )
        .with_state(state)
}

/// Authentication middleware for admin interface
async fn auth_middleware(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<Response> {
    // Extract Authorization header
    if let Some(auth_header) = headers.get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Basic ") {
                if let Ok(decoded) = base64::decode(&auth_str[6..]) {
                    if let Ok(auth_string) = String::from_utf8(decoded) {
                        if let Some((username, password)) = auth_string.split_once(':') {
                            if state.database.authenticate_worker(username, password).await? {
                                info!("Authenticated worker: {}", username);
                                return Ok(next.run(request).await);
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Authentication failed
    Err(BzrError::AuthenticationFailed)
}

/// Health check handler
async fn health_handler(State(state): State<AppState>) -> Result<Response> {
    // Check database health
    state.database.health_check().await?;
    
    let mut context = Context::new();
    context.insert("status", "healthy");
    context.insert("timestamp", &chrono::Utc::now().to_rfc3339());
    
    let html = state.templates.render("health.html", &context)
        .unwrap_or_else(|_| "OK".to_string());
    
    Ok(Html(html).into_response())
}

/// Readiness check handler
async fn ready_handler(State(state): State<AppState>) -> Result<Json<serde_json::Value>> {
    // Check if service is ready to serve traffic
    state.database.health_check().await?;
    
    Ok(Json(json!({
        "status": "ready",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "database": "connected"
    })))
}

/// List repositories handler
async fn list_repositories_handler(State(state): State<AppState>) -> Result<Response> {
    let repositories = state.repository_manager.list_repositories().await?;
    
    // Check if request accepts HTML
    let mut context = Context::new();
    context.insert("repositories", &repositories);
    
    let html = state.templates.render("repositories.html", &context)
        .unwrap_or_else(|e| format!("Template error: {}", e));
    
    Ok(Html(html).into_response())
}

/// Create repository handler
async fn create_repository_handler(
    State(state): State<AppState>,
    Path((campaign, codebase, role)): Path<(String, String, String)>,
) -> Result<Json<serde_json::Value>> {
    let repo_path = RepositoryPath::new(campaign, codebase, role);
    let path = state.repository_manager.ensure_repository(&repo_path).await?;
    
    Ok(Json(json!({
        "status": "created",
        "path": repo_path.relative_path(),
        "full_path": path.to_string_lossy()
    })))
}

/// Repository info handler
async fn repository_info_handler(
    State(state): State<AppState>,
    Path((campaign, codebase, role)): Path<(String, String, String)>,
) -> Result<Json<serde_json::Value>> {
    let repo_path = RepositoryPath::new(campaign, codebase, role);
    let info = state.repository_manager.get_repository_info(&repo_path).await?;
    
    Ok(Json(serde_json::to_value(info)?))
}

/// Diff handler
async fn diff_handler(
    State(state): State<AppState>,
    Path((campaign, codebase, role)): Path<(String, String, String)>,
    Query(query): Query<DiffQuery>,
) -> Result<Response> {
    let repo_path = RepositoryPath::new(campaign, codebase, role);
    let diff = state.repository_manager.get_diff(&repo_path, &query.old, &query.new).await?;
    
    Ok((
        StatusCode::OK,
        [("content-type", "text/plain; charset=utf-8")],
        diff,
    ).into_response())
}

/// Revision info handler
async fn revision_info_handler(
    State(state): State<AppState>,
    Path((campaign, codebase, role)): Path<(String, String, String)>,
    Query(query): Query<RevisionInfoQuery>,
) -> Result<Json<serde_json::Value>> {
    let repo_path = RepositoryPath::new(campaign, codebase, role);
    let revisions = state.repository_manager.get_revision_info(&repo_path, &query.old, &query.new).await?;
    
    Ok(Json(json!({
        "revisions": revisions
    })))
}

/// Configure remote handler
async fn configure_remote_handler(
    State(state): State<AppState>,
    Path((campaign, codebase, role)): Path<(String, String, String)>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>> {
    let repo_path = RepositoryPath::new(campaign, codebase, role);
    
    let remote_url = payload.get("remote_url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| BzrError::invalid_request("remote_url is required"))?;
    
    state.repository_manager.configure_remote(&repo_path, remote_url).await?;
    
    Ok(Json(json!({
        "status": "configured",
        "remote_url": remote_url
    })))
}

/// List remotes handler (placeholder)
async fn list_remotes_handler(
    Path((campaign, codebase, role)): Path<(String, String, String)>,
) -> Result<Json<serde_json::Value>> {
    // TODO: Implement actual remote listing
    Ok(Json(json!({
        "repository": format!("{}/{}/{}", campaign, codebase, role),
        "remotes": []
    })))
}

// Add base64 decode for auth middleware
mod base64 {
    pub fn decode(input: &str) -> Result<Vec<u8>, &'static str> {
        use std::collections::HashMap;
        
        let chars: HashMap<char, u8> = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
            .chars()
            .enumerate()
            .map(|(i, c)| (c, i as u8))
            .collect();
        
        let mut result = Vec::new();
        let input = input.trim_end_matches('=');
        let mut temp = 0u32;
        let mut bits = 0;
        
        for c in input.chars() {
            let val = chars.get(&c).ok_or("Invalid character")?;
            temp = (temp << 6) | (*val as u32);
            bits += 6;
            
            if bits >= 8 {
                result.push((temp >> (bits - 8)) as u8);
                bits -= 8;
                temp &= (1 << bits) - 1;
            }
        }
        
        Ok(result)
    }
}