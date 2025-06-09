use anyhow::Result;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{Json, Response},
    routing::{get, post},
    Router,
};
use axum_extra::extract::CookieJar;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{info, warn};

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
use auth::{
    middleware::{auth_middleware_layer, AuthState},
    oidc::OidcClient,
    routes::{api_auth_routes, auth_routes},
    session::SessionManager,
};
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

    if let Some(janitor_config) = config.janitor() {
        info!(
            "Loaded janitor configuration with {} campaigns",
            janitor_config.campaign.len()
        );
    }

    // Initialize application state
    let app_state = AppState::new(config).await?;

    // Build the application router
    let listen_addr = app_state.config.site().listen_address;
    let app = create_app(app_state).await?;

    // Start the server
    let listener = TcpListener::bind(&listen_addr).await?;
    info!("Server listening on {}", listen_addr);

    axum::serve(listener, app).await?;

    Ok(())
}

async fn create_app(state: AppState) -> Result<Router> {
    // Set up authentication state
    let auth_state = setup_auth(&state).await?;
    let auth_state_arc = Arc::new(auth_state);
    
    // Add auth state to app state
    let state_with_auth = state.with_auth_state(auth_state_arc.clone());

    // Create main app routes with AppState (including auth)
    let app_routes = Router::new()
        // Health check endpoint
        .route("/health", get(health_check))
        // API routes
        .nest("/api", api_routes())
        // Main site routes
        .nest("/", site_routes())
        // Static assets
        .nest("/static", static_routes());

    // Create auth routes with wrapper handlers that extract auth state from app state
    let auth_router = Router::new()
        .route("/login", get(login_handler_wrapper))
        .route("/auth/callback", get(callback_handler_wrapper))
        .route("/logout", post(logout_handler_wrapper))
        .route("/status", get(status_handler_wrapper))
        .route("/protected", get(protected_handler_wrapper))
        .route("/admin", get(admin_handler_wrapper))
        .route("/qa", get(qa_handler_wrapper))
        .route("/api/auth/status", get(status_handler_wrapper))
        .route(
            "/api/auth/user-info",
            get(protected_handler_wrapper).route_layer(axum::middleware::from_fn(auth::middleware::require_login)),
        )
        .route(
            "/api/auth/admin-info",
            get(admin_handler_wrapper).route_layer(axum::middleware::from_fn(auth::middleware::require_admin)),
        );

    // Create the application router with middleware and state
    Ok(Router::new()
        .merge(app_routes)
        .merge(auth_router)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .layer(axum::middleware::from_fn_with_state(
            state_with_auth.clone(),
            |State(app_state): State<AppState>, jar: CookieJar, req: Request, next: Next| async move {
                let auth_state = app_state.auth_state.clone().expect("Auth state not initialized");
                auth::middleware::session_middleware(State(auth_state), jar, req, next).await
            },
        ))
        .with_state(state_with_auth))
}

/// Set up authentication state with OIDC client if configured
async fn setup_auth(app_state: &AppState) -> Result<AuthState> {
    let config = app_state.config.site();
    
    // Initialize session manager
    let session_manager = SessionManager::new(app_state.database.pool().clone());
    
    // Ensure session tables exist
    if let Err(e) = session_manager.ensure_table_exists().await {
        warn!("Failed to create session tables: {}", e);
    }
    
    // Create base auth state
    let mut auth_state = AuthState::new(session_manager, config);
    
    // Try to initialize OIDC client if configured
    if config.oidc_client_id.is_some() && config.oidc_client_secret.is_some() {
        match OidcClient::new(config).await {
            Ok(oidc_client) => {
                info!("OIDC client initialized successfully");
                auth_state.oidc_client = Some(Arc::new(oidc_client));
            }
            Err(e) => {
                warn!("Failed to initialize OIDC client: {}. Authentication will be limited.", e);
            }
        }
    } else {
        info!("OIDC not configured - authentication will be limited");
    }
    
    Ok(auth_state)
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
    let mut status = json!({
        "status": "ok",
        "uptime": state.start_time.elapsed().as_secs(),
        "timestamp": chrono::Utc::now(),
        "version": env!("CARGO_PKG_VERSION")
    });

    // Check database connectivity
    let db_status = match state.database.health_check().await {
        Ok(_) => "healthy",
        Err(_) => "unhealthy",
    };
    status["database"] = json!(db_status);

    // Check Redis connectivity if available
    let redis_status = if let Some(ref redis_client) = state.redis {
        match redis_client.get_multiplexed_async_connection().await {
            Ok(mut conn) => {
                use redis::AsyncCommands;
                match redis::cmd("PING").query_async::<String>(&mut conn).await {
                    Ok(_) => "healthy",
                    Err(_) => "unhealthy",
                }
            }
            Err(_) => "unhealthy",
        }
    } else {
        "not_configured"
    };
    status["redis"] = json!(redis_status);

    // Overall status based on critical services
    if db_status == "unhealthy" {
        status["status"] = json!("degraded");
    }

    Json(status)
}

// Wrapper handlers that extract auth state from app state
async fn login_handler_wrapper(
    State(app_state): State<AppState>,
    query: axum::extract::Query<auth::handlers::LoginQuery>,
    user: auth::middleware::OptionalUser,
    jar: CookieJar,
) -> Result<Response, StatusCode> {
    let auth_state = app_state.auth_state.clone().expect("Auth state not initialized");
    auth::handlers::login_handler(State(auth_state), query, user, jar).await
}

async fn callback_handler_wrapper(
    State(app_state): State<AppState>,
    query: axum::extract::Query<auth::handlers::CallbackQuery>,
    jar: CookieJar,
) -> Result<Response, StatusCode> {
    let auth_state = app_state.auth_state.clone().expect("Auth state not initialized");
    auth::handlers::callback_handler(State(auth_state), query, jar).await
}

async fn logout_handler_wrapper(
    State(app_state): State<AppState>,
    jar: CookieJar,
) -> Result<Response, StatusCode> {
    let auth_state = app_state.auth_state.clone().expect("Auth state not initialized");
    auth::handlers::logout_handler(State(auth_state), jar).await
}

async fn status_handler_wrapper(
    State(_app_state): State<AppState>,
    user: auth::middleware::OptionalUser,
) -> Result<Json<auth::handlers::LoginStatus>, StatusCode> {
    auth::handlers::status_handler(user).await
}

async fn protected_handler_wrapper(
    State(_app_state): State<AppState>,
    user: auth::middleware::UserContext,
) -> Result<Json<serde_json::Value>, StatusCode> {
    auth::handlers::protected_handler(user).await
}

async fn admin_handler_wrapper(
    State(_app_state): State<AppState>,
    user: auth::middleware::UserContext,
) -> Result<Json<serde_json::Value>, StatusCode> {
    auth::handlers::admin_handler(user).await
}

async fn qa_handler_wrapper(
    State(_app_state): State<AppState>,
    user: auth::middleware::UserContext,
) -> Result<Json<serde_json::Value>, StatusCode> {
    auth::handlers::qa_handler(user).await
}
