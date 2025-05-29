use anyhow::Result;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, Json},
    routing::{get, post},
    Router,
};
use serde_json::{json, Value};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    trace::TraceLayer,
};
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod app;
mod config;
mod database;
mod handlers;
mod middleware;
mod templates;

use app::AppState;
use config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "janitor_site=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config = Config::from_env()?;
    info!("Starting Janitor Site server on {}", config.listen_address);

    // Initialize application state
    let app_state = AppState::new(config).await?;

    // Build the application router
    let listen_addr = app_state.config.listen_address;
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
                .layer(CorsLayer::permissive())
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
    Router::new()
        .nest_service("/", tower_http::services::ServeDir::new("static"))
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