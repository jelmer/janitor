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
use crate::service::ServiceHealth;

/// Application state shared between handlers
#[derive(Clone)]
pub struct AppState {
    /// Service configuration
    pub config: Arc<Config>,
    /// Service health status
    pub health_status: Option<Arc<ServiceHealth>>,
}

/// Create the web application
pub fn create_app(config: Arc<Config>) -> Router {
    create_app_with_health(config, None)
}

/// Create the web application with health status
pub fn create_app_with_health(
    config: Arc<Config>,
    health_status: Option<Arc<ServiceHealth>>,
) -> Router {
    let state = AppState {
        config,
        health_status,
    };

    Router::new()
        .route("/health", get(health_handler))
        .route("/metrics", get(metrics_handler))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CompressionLayer::new()),
        )
        .with_state(state)
}

/// Health check handler
async fn health_handler(axum::extract::State(state): axum::extract::State<AppState>) -> Response {
    use std::sync::atomic::Ordering;

    let (status, status_code) = if let Some(ref health_status) = state.health_status {
        let healthy = health_status.is_healthy();
        let status = if healthy { "healthy" } else { "unhealthy" };
        let code = if healthy {
            StatusCode::OK
        } else {
            StatusCode::SERVICE_UNAVAILABLE
        };
        (status, code)
    } else {
        ("healthy", StatusCode::OK)
    };

    let mut response = json!({
        "status": status,
        "service": "janitor-auto-upload",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    if let Some(ref health_status) = state.health_status {
        if let Some(components) = response.as_object_mut() {
            components.insert(
                "components".to_string(),
                json!({
                    "web": health_status.web_healthy.load(Ordering::SeqCst),
                    "redis": health_status.redis_healthy.load(Ordering::SeqCst),
                }),
            );
            components.insert(
                "uptime_seconds".to_string(),
                json!(health_status.uptime().as_secs()),
            );
            components.insert(
                "messages_processed".to_string(),
                json!(health_status.messages_processed.load(Ordering::SeqCst)),
            );
        }
    }

    (status_code, Json(response)).into_response()
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
pub async fn run_web_server(app: Router, listen_addr: &str, port: u16) -> anyhow::Result<()> {
    let addr = format!("{}:{}", listen_addr, port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    info!("Web server listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
