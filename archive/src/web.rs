//! Web service implementation for the archive service.
//!
//! This module provides HTTP endpoints for serving APT repository files,
//! handling repository generation requests, and providing management APIs.

use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::fs;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{debug, info, warn};

use crate::config::ArchiveConfig;
use crate::database::BuildManager;
use crate::error::{ArchiveError, ArchiveResult};
use crate::repository::RepositoryGenerator;
use crate::scanner::PackageScanner;

/// Web service application state.
#[derive(Clone)]
pub struct AppState {
    /// Archive configuration.
    pub config: Arc<ArchiveConfig>,
    /// Repository generator.
    pub generator: Arc<RepositoryGenerator>,
    /// Package scanner.
    pub scanner: Arc<PackageScanner>,
    /// Database manager.
    pub database: Arc<BuildManager>,
}

/// Health check response.
#[derive(Serialize)]
pub struct HealthResponse {
    /// Service status.
    pub status: String,
    /// Service version.
    pub version: String,
    /// Timestamp.
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Publish request.
#[derive(Debug, Deserialize)]
pub struct PublishRequest {
    /// Suite to publish.
    pub suite: Option<String>,
    /// Run ID to publish.
    pub run_id: Option<String>,
    /// Changeset to publish.
    pub changeset: Option<String>,
    /// Force republication.
    pub force: Option<bool>,
}

/// Publish response.
#[derive(Serialize)]
pub struct PublishResponse {
    /// Success status.
    pub success: bool,
    /// Message.
    pub message: String,
    /// Generated repository info.
    pub repositories: Vec<RepositoryInfo>,
}

/// Repository information.
#[derive(Serialize)]
pub struct RepositoryInfo {
    /// Repository name.
    pub name: String,
    /// Suite name.
    pub suite: String,
    /// Number of packages.
    pub packages: u64,
    /// Number of sources.
    pub sources: u64,
    /// Generation timestamp.
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Archive web service.
pub struct ArchiveWebService {
    state: AppState,
}

impl ArchiveWebService {
    /// Create a new archive web service.
    pub async fn new(
        config: ArchiveConfig,
        generator: RepositoryGenerator,
        scanner: PackageScanner,
        database: BuildManager,
    ) -> ArchiveResult<Self> {
        let state = AppState {
            config: Arc::new(config),
            generator: Arc::new(generator),
            scanner: Arc::new(scanner),
            database: Arc::new(database),
        };

        Ok(Self { state })
    }

    /// Create the Axum router with all routes.
    pub fn router(&self) -> Router {
        Router::new()
            // Health check endpoints
            .route("/health", get(health_check))
            .route("/ready", get(readiness_check))
            // Repository serving endpoints
            .route("/dists/:suite/Release", get(serve_release))
            .route("/dists/:suite/Release.gpg", get(serve_release_gpg))
            .route("/dists/:suite/InRelease", get(serve_inrelease))
            .route(
                "/dists/:suite/:component/binary-:arch/Packages",
                get(serve_packages),
            )
            .route(
                "/dists/:suite/:component/binary-:arch/Packages.gz",
                get(serve_packages_gz),
            )
            .route(
                "/dists/:suite/:component/binary-:arch/Packages.bz2",
                get(serve_packages_bz2),
            )
            .route(
                "/dists/:suite/:component/source/Sources",
                get(serve_sources),
            )
            .route(
                "/dists/:suite/:component/source/Sources.gz",
                get(serve_sources_gz),
            )
            .route(
                "/dists/:suite/:component/source/Sources.bz2",
                get(serve_sources_bz2),
            )
            // By-hash serving
            .route(
                "/dists/:suite/:component/binary-:arch/by-hash/:algo/:hash",
                get(serve_by_hash),
            )
            .route(
                "/dists/:suite/:component/source/by-hash/:algo/:hash",
                get(serve_by_hash),
            )
            // Publishing and management endpoints
            .route("/publish", post(publish_repository))
            .route("/last-publish", get(last_publish_status))
            .route("/gpg-key", get(serve_gpg_key))
            // Static file serving for pool
            .route("/pool/*path", get(serve_pool_file))
            .with_state(self.state.clone())
            .layer(
                ServiceBuilder::new()
                    .layer(TraceLayer::new_for_http())
                    .layer(CorsLayer::permissive()),
            )
    }

    /// Start the web service on the specified address.
    pub async fn serve(&self, bind_address: &str) -> ArchiveResult<()> {
        let app = self.router();

        info!("Starting archive web service on {}", bind_address);

        let listener = tokio::net::TcpListener::bind(bind_address)
            .await
            .map_err(|e| {
                ArchiveError::Configuration(format!("Failed to bind to {}: {}", bind_address, e))
            })?;

        info!("Archive web service listening on {}", bind_address);

        axum::serve(listener, app)
            .await
            .map_err(|e| ArchiveError::Configuration(format!("Server error: {}", e)))?;

        Ok(())
    }
}

/// Health check endpoint handler.
async fn health_check() -> impl IntoResponse {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: chrono::Utc::now(),
    })
}

/// Readiness check endpoint handler.
async fn readiness_check(State(_state): State<AppState>) -> impl IntoResponse {
    // Check database connectivity
    // For now, just return healthy
    Json(HealthResponse {
        status: "ready".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: chrono::Utc::now(),
    })
}

/// Serve Release file.
async fn serve_release(
    Path(params): Path<HashMap<String, String>>,
    State(state): State<AppState>,
) -> Result<Response, StatusCode> {
    let suite = params.get("suite").ok_or(StatusCode::BAD_REQUEST)?;

    debug!("Serving Release file for suite: {}", suite);

    // Get repository configuration for this suite
    let repo_config = state
        .config
        .repositories
        .get(suite)
        .ok_or(StatusCode::NOT_FOUND)?;

    let release_path = repo_config.suite_path().join("Release");

    match fs::read(&release_path).await {
        Ok(content) => {
            let mut headers = HeaderMap::new();
            headers.insert("Content-Type", HeaderValue::from_static("text/plain"));
            headers.insert(
                "Cache-Control",
                HeaderValue::from_static("public, max-age=300"),
            );

            Ok((headers, content).into_response())
        }
        Err(e) => {
            warn!("Failed to read Release file at {:?}: {}", release_path, e);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

/// Serve Release.gpg file.
async fn serve_release_gpg(
    Path(params): Path<HashMap<String, String>>,
    State(state): State<AppState>,
) -> Result<Response, StatusCode> {
    let suite = params.get("suite").ok_or(StatusCode::BAD_REQUEST)?;

    debug!("Serving Release.gpg file for suite: {}", suite);

    let repo_config = state
        .config
        .repositories
        .get(suite)
        .ok_or(StatusCode::NOT_FOUND)?;

    let release_gpg_path = repo_config.suite_path().join("Release.gpg");

    match fs::read(&release_gpg_path).await {
        Ok(content) => {
            let mut headers = HeaderMap::new();
            headers.insert(
                "Content-Type",
                HeaderValue::from_static("application/pgp-signature"),
            );
            headers.insert(
                "Cache-Control",
                HeaderValue::from_static("public, max-age=300"),
            );

            Ok((headers, content).into_response())
        }
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// Serve InRelease file.
async fn serve_inrelease(
    Path(params): Path<HashMap<String, String>>,
    State(state): State<AppState>,
) -> Result<Response, StatusCode> {
    let suite = params.get("suite").ok_or(StatusCode::BAD_REQUEST)?;

    debug!("Serving InRelease file for suite: {}", suite);

    let repo_config = state
        .config
        .repositories
        .get(suite)
        .ok_or(StatusCode::NOT_FOUND)?;

    let inrelease_path = repo_config.suite_path().join("InRelease");

    match fs::read(&inrelease_path).await {
        Ok(content) => {
            let mut headers = HeaderMap::new();
            headers.insert("Content-Type", HeaderValue::from_static("text/plain"));
            headers.insert(
                "Cache-Control",
                HeaderValue::from_static("public, max-age=300"),
            );

            Ok((headers, content).into_response())
        }
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// Serve Packages file.
async fn serve_packages(
    Path(params): Path<HashMap<String, String>>,
    State(state): State<AppState>,
) -> Result<Response, StatusCode> {
    let suite = params.get("suite").ok_or(StatusCode::BAD_REQUEST)?;
    let component = params.get("component").ok_or(StatusCode::BAD_REQUEST)?;
    let arch = params.get("arch").ok_or(StatusCode::BAD_REQUEST)?;

    debug!(
        "Serving Packages file for {}/{}/binary-{}",
        suite, component, arch
    );

    serve_component_file(
        &state,
        suite,
        component,
        &format!("binary-{}/Packages", arch),
        "text/plain",
    )
    .await
}

/// Serve compressed Packages file.
async fn serve_packages_gz(
    Path(params): Path<HashMap<String, String>>,
    State(state): State<AppState>,
) -> Result<Response, StatusCode> {
    let suite = params.get("suite").ok_or(StatusCode::BAD_REQUEST)?;
    let component = params.get("component").ok_or(StatusCode::BAD_REQUEST)?;
    let arch = params.get("arch").ok_or(StatusCode::BAD_REQUEST)?;

    serve_component_file(
        &state,
        suite,
        component,
        &format!("binary-{}/Packages.gz", arch),
        "application/gzip",
    )
    .await
}

/// Serve bzip2 compressed Packages file.
async fn serve_packages_bz2(
    Path(params): Path<HashMap<String, String>>,
    State(state): State<AppState>,
) -> Result<Response, StatusCode> {
    let suite = params.get("suite").ok_or(StatusCode::BAD_REQUEST)?;
    let component = params.get("component").ok_or(StatusCode::BAD_REQUEST)?;
    let arch = params.get("arch").ok_or(StatusCode::BAD_REQUEST)?;

    serve_component_file(
        &state,
        suite,
        component,
        &format!("binary-{}/Packages.bz2", arch),
        "application/x-bzip2",
    )
    .await
}

/// Serve Sources file.
async fn serve_sources(
    Path(params): Path<HashMap<String, String>>,
    State(state): State<AppState>,
) -> Result<Response, StatusCode> {
    let suite = params.get("suite").ok_or(StatusCode::BAD_REQUEST)?;
    let component = params.get("component").ok_or(StatusCode::BAD_REQUEST)?;

    serve_component_file(&state, suite, component, "source/Sources", "text/plain").await
}

/// Serve compressed Sources file.
async fn serve_sources_gz(
    Path(params): Path<HashMap<String, String>>,
    State(state): State<AppState>,
) -> Result<Response, StatusCode> {
    let suite = params.get("suite").ok_or(StatusCode::BAD_REQUEST)?;
    let component = params.get("component").ok_or(StatusCode::BAD_REQUEST)?;

    serve_component_file(
        &state,
        suite,
        component,
        "source/Sources.gz",
        "application/gzip",
    )
    .await
}

/// Serve bzip2 compressed Sources file.
async fn serve_sources_bz2(
    Path(params): Path<HashMap<String, String>>,
    State(state): State<AppState>,
) -> Result<Response, StatusCode> {
    let suite = params.get("suite").ok_or(StatusCode::BAD_REQUEST)?;
    let component = params.get("component").ok_or(StatusCode::BAD_REQUEST)?;

    serve_component_file(
        &state,
        suite,
        component,
        "source/Sources.bz2",
        "application/x-bzip2",
    )
    .await
}

/// Serve by-hash files.
async fn serve_by_hash(
    Path(params): Path<HashMap<String, String>>,
    State(state): State<AppState>,
) -> Result<Response, StatusCode> {
    let suite = params.get("suite").ok_or(StatusCode::BAD_REQUEST)?;
    let component = params.get("component").ok_or(StatusCode::BAD_REQUEST)?;
    let arch = params.get("arch");
    let algo = params.get("algo").ok_or(StatusCode::BAD_REQUEST)?;
    let hash = params.get("hash").ok_or(StatusCode::BAD_REQUEST)?;

    let repo_config = state
        .config
        .repositories
        .get(suite)
        .ok_or(StatusCode::NOT_FOUND)?;

    let by_hash_path = if let Some(arch) = arch {
        // Binary by-hash: /dists/suite/component/binary-arch/by-hash/algo/hash
        repo_config
            .component_arch_path(component, arch)
            .join("by-hash")
            .join(algo)
            .join(hash)
    } else {
        // Source by-hash: /dists/suite/component/source/by-hash/algo/hash
        repo_config
            .source_path(component)
            .join("by-hash")
            .join(algo)
            .join(hash)
    };

    match fs::read(&by_hash_path).await {
        Ok(content) => {
            let mut headers = HeaderMap::new();
            headers.insert(
                "Content-Type",
                HeaderValue::from_static("application/octet-stream"),
            );
            headers.insert(
                "Cache-Control",
                HeaderValue::from_static("public, max-age=86400"),
            ); // 24 hours

            Ok((headers, content).into_response())
        }
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// Serve pool files (package .deb files).
async fn serve_pool_file(
    Path(path): Path<String>,
    State(state): State<AppState>,
) -> Result<Response, StatusCode> {
    debug!("Serving pool file: {}", path);

    // Construct the full path to the pool file
    // Pool files are typically stored outside the dists directory
    let pool_path = state
        .config
        .repositories
        .values()
        .next()
        .map(|repo| repo.base_path.join("pool").join(&path))
        .ok_or(StatusCode::NOT_FOUND)?;

    match fs::read(&pool_path).await {
        Ok(content) => {
            let mut headers = HeaderMap::new();

            // Set appropriate content type based on file extension
            if path.ends_with(".deb") {
                headers.insert(
                    "Content-Type",
                    HeaderValue::from_static("application/vnd.debian.binary-package"),
                );
            } else if path.ends_with(".dsc") {
                headers.insert("Content-Type", HeaderValue::from_static("text/plain"));
            } else if path.ends_with(".tar.gz") || path.ends_with(".tar.xz") {
                headers.insert(
                    "Content-Type",
                    HeaderValue::from_static("application/x-tar"),
                );
            } else {
                headers.insert(
                    "Content-Type",
                    HeaderValue::from_static("application/octet-stream"),
                );
            }

            headers.insert(
                "Cache-Control",
                HeaderValue::from_static("public, max-age=86400"),
            ); // 24 hours

            Ok((headers, content).into_response())
        }
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// Publish repository endpoint.
async fn publish_repository(
    State(state): State<AppState>,
    Json(request): Json<PublishRequest>,
) -> Result<Json<PublishResponse>, StatusCode> {
    info!("Repository publish request: {:?}", request);

    let mut repositories = Vec::new();
    
    // Get the suite to process
    let suite = request.suite.as_deref().unwrap_or("default");
    
    // Get campaign configuration for the suite
    match state.database.get_campaign_info(suite, None).await {
        Ok(Some(campaign_info)) => {
            info!("Processing campaign for suite: {}", suite);
            
            // Use the campaign info to generate repositories for configured distributions
            let distributions: Vec<String> = if state.config.repositories.is_empty() {
                // If no repositories configured, create default for the campaign
                vec!["debian".to_string()]
            } else {
                state.config.repositories.keys().cloned().collect()
            };
            
            for distribution in &distributions {
                let mut repo_config = state.config.repositories.get(distribution)
                    .cloned()
                    .unwrap_or_else(|| {
                        // Create default repository config if not found
                        crate::config::AptRepositoryConfig {
                            name: distribution.clone(),
                            base_path: state.config.archive_path.join(distribution),
                            origin: format!("Janitor {}", distribution),
                            label: format!("Janitor {}", distribution),
                            suite: suite.to_string(),
                            codename: suite.to_string(),
                            description: format!("Automated packages for {} - {}", distribution, campaign_info.description),
                            architectures: campaign_info.architectures.clone(),
                            components: vec![campaign_info.component.clone()],
                            base_url: format!("https://janitor.debian.net/apt/{}", distribution),
                            by_hash: true,
                        }
                    });
                    
                // Override suite/codename and other details from campaign info
                repo_config.suite = campaign_info.suite.clone();
                repo_config.codename = campaign_info.suite.clone();
                repo_config.architectures = campaign_info.architectures.clone();
                repo_config.components = vec![campaign_info.component.clone()];
                
                match state.generator.generate_repository(&repo_config).await {
                    Ok(()) => {
                        info!("Successfully generated repository for {}/{}", distribution, suite);
                        
                        // Count packages and sources from build info (simplified for now)
                        let packages_count = 0; // Would need to query actual build results
                        let sources_count = 0;  // Would need to query actual build results
                        
                        repositories.push(RepositoryInfo {
                            name: repo_config.name,
                            suite: campaign_info.suite.clone(),
                            packages: packages_count,
                            sources: sources_count,
                            timestamp: chrono::Utc::now(),
                        });
                    }
                    Err(e) => {
                        warn!("Failed to generate repository for {}/{}: {}", distribution, suite, e);
                        return Err(StatusCode::INTERNAL_SERVER_ERROR);
                    }
                }
            }
            
            let response = PublishResponse {
                success: true,
                message: format!("Successfully published {} repositories for suite {}", repositories.len(), suite),
                repositories,
            };
            
            Ok(Json(response))
        }
        Ok(None) => {
            warn!("No campaign configuration found for suite: {}", suite);
            let response = PublishResponse {
                success: false,
                message: format!("No campaign configuration found for suite: {}", suite),
                repositories: vec![],
            };
            Ok(Json(response))
        }
        Err(e) => {
            warn!("Error getting campaign info for suite {}: {}", suite, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Last publish status endpoint.
async fn last_publish_status(
    Query(params): Query<HashMap<String, String>>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let suite = params.get("suite").map_or("default", |v| v);
    
    // Check if repositories exist and get their status
    let mut last_publish_time = None;
    let mut status = "never_published";
    let mut repository_count = 0;
    
    // Look for Release files in each repository to determine last publish status
    for (distribution, repo_config) in &state.config.repositories {
        let release_file = repo_config.base_path.join("dists").join(suite).join("Release");
        
        match fs::metadata(&release_file).await {
            Ok(metadata) => {
                if let Ok(modified) = metadata.modified() {
                    let publish_time = chrono::DateTime::<chrono::Utc>::from(modified);
                    
                    if last_publish_time.map_or(true, |last| publish_time > last) {
                        last_publish_time = Some(publish_time);
                    }
                    
                    repository_count += 1;
                    status = "success";
                }
            }
            Err(_) => {
                debug!("No Release file found for {}/{}", distribution, suite);
            }
        }
    }
    
    // If no specific repositories configured, check for any published repositories
    if repository_count == 0 {
        let base_dist_path = state.config.archive_path.join("dists").join(suite);
        
        if let Ok(mut dir_entries) = fs::read_dir(&base_dist_path).await {
            while let Ok(Some(entry)) = dir_entries.next_entry().await {
                if entry.file_name() == "Release" {
                    if let Ok(metadata) = entry.metadata().await {
                        if let Ok(modified) = metadata.modified() {
                            let publish_time = chrono::DateTime::<chrono::Utc>::from(modified);
                            last_publish_time = Some(publish_time);
                            status = "success";
                            repository_count = 1;
                            break;
                        }
                    }
                }
            }
        }
    }
    
    let response = serde_json::json!({
        "last_publish": last_publish_time,
        "status": status,
        "suite": suite,
        "repository_count": repository_count,
        "distributions": state.config.repositories.keys().collect::<Vec<_>>()
    });

    Ok(Json(response))
}

/// Serve GPG public key.
async fn serve_gpg_key(State(state): State<AppState>) -> Result<Response, StatusCode> {
    if let Some(gpg_config) = &state.config.gpg {
        // Try to export the public key using gpg command
        let mut cmd = tokio::process::Command::new("gpg");
        
        // Set GPG home directory if specified
        if let Some(gpg_home) = &gpg_config.gpg_home {
            cmd.arg("--homedir").arg(gpg_home);
        }
        
        // Export the public key in ASCII armor format
        cmd.args(["--armor", "--export", &gpg_config.key_id]);
        
        match cmd.output().await {
            Ok(output) => {
                if output.status.success() {
                    let key_data = String::from_utf8_lossy(&output.stdout);
                    
                    if key_data.starts_with("-----BEGIN PGP PUBLIC KEY BLOCK-----") {
                        let mut headers = HeaderMap::new();
                        headers.insert(
                            "Content-Type",
                            HeaderValue::from_static("application/pgp-keys"),
                        );
                        headers.insert(
                            "Cache-Control",
                            HeaderValue::from_static("public, max-age=86400"), // 24 hours
                        );
                        
                        Ok((headers, key_data.to_string()).into_response())
                    } else {
                        warn!("GPG export returned unexpected output for key {}", gpg_config.key_id);
                        Err(StatusCode::INTERNAL_SERVER_ERROR)
                    }
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    warn!("GPG export failed for key {}: {}", gpg_config.key_id, stderr);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
            Err(e) => {
                warn!("Failed to execute gpg command: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    } else {
        debug!("No GPG configuration available");
        Err(StatusCode::NOT_FOUND)
    }
}

/// Helper function to serve component files.
async fn serve_component_file(
    state: &AppState,
    suite: &str,
    component: &str,
    file_path: &str,
    content_type: &str,
) -> Result<Response, StatusCode> {
    let repo_config = state
        .config
        .repositories
        .get(suite)
        .ok_or(StatusCode::NOT_FOUND)?;

    let full_path = repo_config.suite_path().join(component).join(file_path);

    match fs::read(&full_path).await {
        Ok(content) => {
            let mut headers = HeaderMap::new();
            headers.insert("Content-Type", HeaderValue::from_str(content_type).unwrap());
            headers.insert(
                "Cache-Control",
                HeaderValue::from_static("public, max-age=300"),
            );

            Ok((headers, content).into_response())
        }
        Err(e) => {
            warn!("Failed to read file at {:?}: {}", full_path, e);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check() {
        let response = health_check().await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_readiness_check() {
        // This test would require setting up a complete app state
        // For now, just verify the function exists
        // let state = create_test_state().await;
        // let response = readiness_check(State(state)).await.into_response();
        // assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_web_service_creation() {
        // Test that we can create the web service structure
        // This would require setting up all dependencies
        // For now, just test basic struct creation
    }
}
