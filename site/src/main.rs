use anyhow::Result;
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;

mod analyze;
mod api;
mod app;
mod assets;
mod auth;
mod config;
mod database;
mod handlers;
mod logging;
mod middleware;
mod realtime;
mod routes;
mod templates;
mod webhook;

use app::AppState;
use config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration first
    let config = Config::from_env()?;

    // Initialize logging with configuration
    logging::init_logging(config.site())?;

    info!(
        "Starting Janitor Site server on {} (debug: {})",
        config.site().listen_address,
        config.site().debug
    );

    if let Some(ref janitor_config) = config.janitor() {
        info!(
            "Loaded janitor configuration with {} campaigns",
            janitor_config.campaign.len()
        );
    }

    // Initialize application state
    let app_state = AppState::new(config).await?;

    // Build the application router
    let listen_addr = app_state.config.site().listen_address;
    let app = create_app(app_state);

    // Start the server
    let listener = TcpListener::bind(&listen_addr).await?;
    info!("Server listening on {}", listen_addr);

    axum::serve(listener, app).await?;

    Ok(())
}

fn create_app(state: AppState) -> Router {
    Router::new()
        // Health check endpoint
        .route("/health", get(health_check))
        // API routes
        .nest("/api", api_routes())
        // Main site routes
        .nest("/", site_routes())
        // Static assets
        .nest("/static", static_routes())
        // Add middleware
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive()),
        )
        .with_state(state)
}

fn api_routes() -> Router<AppState> {
    Router::new()
        .route("/v1/health", get(api_health))
        .route("/v1/status", get(api_status))
}

fn site_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::index))
        .route("/about", get(handlers::about))
        .route("/pkg", get(handlers::package_list))
        .route("/pkg/:name", get(handlers::package_detail))
}

fn static_routes() -> Router<AppState> {
    Router::new().nest_service("/", tower_http::services::ServeDir::new("static"))
}

async fn health_check() -> StatusCode {
    StatusCode::OK
}

async fn api_health() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "service": "janitor-site",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

async fn api_status(State(state): State<AppState>) -> Json<Value> {
    // TODO: Add actual status checks (database, redis, etc.)
    Json(json!({
        "status": "ok",
        "database": "connected",
        "redis": "connected",
        "uptime": state.start_time.elapsed().as_secs()
    }))
}
