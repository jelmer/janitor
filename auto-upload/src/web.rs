//! Web server implementation for the auto-upload service

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use prometheus::{Encoder, TextEncoder};
use serde_json::json;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{compression::CompressionLayer, trace::TraceLayer};
use tracing::info;

use crate::config::Config;

/// Application state shared between handlers
#[derive(Clone)]
pub struct AppState {
    /// Service configuration
    pub config: Arc<Config>,
}

/// Create the web application
pub fn create_app(config: Arc<Config>) -> Router {
    let state = AppState { config };
    
    Router::new()
        .route("/health", get(health_handler))
        .route("/metrics", get(metrics_handler))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CompressionLayer::new())
        )
        .with_state(state)
}

/// Health check handler
async fn health_handler() -> Response {
    Json(json!({
        "status": "healthy",
        "service": "janitor-auto-upload",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
    .into_response()
}

/// Metrics handler for Prometheus
async fn metrics_handler() -> Response {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    
    match encoder.encode(&metric_families, &mut buffer) {
        Ok(_) => (
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, encoder.format_type())],
            buffer,
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to encode metrics: {}", e),
        )
            .into_response(),
    }
}

/// Run the web server
pub async fn run_web_server(
    app: Router,
    listen_addr: &str,
    port: u16,
) -> anyhow::Result<()> {
    let addr = format!("{}:{}", listen_addr, port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    
    info!("Web server listening on {}", addr);
    
    axum::serve(listener, app).await?;
    
    Ok(())
}