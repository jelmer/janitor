//! Git Store service main entry point

use anyhow::Result;
use janitor_git_store::{config::Config, database::DatabaseManager, repository::RepositoryManager, web};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "git_store=debug,janitor_git_store=debug,info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Git Store service");

    // Load configuration
    let config = match std::env::args().nth(1) {
        Some(config_path) => {
            info!("Loading configuration from: {}", config_path);
            Config::from_file(&config_path)?
        }
        None => {
            info!("Loading configuration from environment");
            Config::from_env()?
        }
    };

    let config = Arc::new(config);
    info!("Configuration loaded successfully");

    // Create repository manager
    let repo_manager = Arc::new(RepositoryManager::new(config.local_path.clone()));
    info!("Repository manager initialized at: {:?}", config.local_path);

    // Create database pool
    let db_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await?;
    info!("Database connection established");

    // Run migrations if needed
    // sqlx::migrate!("./migrations").run(&db_pool).await?;

    // Create database manager
    let db_manager = Arc::new(DatabaseManager::new(
        db_pool,
        config.worker_table.clone(),
        config.codebase_table.clone(),
    ));
    info!("Database manager initialized");

    // Initialize templates
    let tera = Arc::new(web::init_templates(config.templates_path.as_deref())?);
    info!("Templates initialized");

    // Create application state
    let app_state = web::AppState {
        repo_manager: repo_manager.clone(),
        config: config.clone(),
        tera,
        db_manager,
    };

    // Create applications
    let admin_app = web::create_admin_app(app_state.clone());
    let public_app = web::create_public_app(app_state);

    // Start servers
    let admin_addr = format!("{}:{}", config.host, config.admin_port);
    let public_addr = format!("{}:{}", config.host, config.public_port);

    info!("Starting admin server on {}", admin_addr);
    info!("Starting public server on {}", public_addr);

    // Spawn admin server
    let admin_listener = TcpListener::bind(&admin_addr).await?;
    let admin_server = tokio::spawn(async move {
        axum::serve(admin_listener, admin_app)
            .await
            .expect("Admin server failed");
    });

    // Spawn public server
    let public_listener = TcpListener::bind(&public_addr).await?;
    let public_server = tokio::spawn(async move {
        axum::serve(public_listener, public_app)
            .await
            .expect("Public server failed");
    });

    // Wait for servers
    tokio::select! {
        result = admin_server => {
            error!("Admin server stopped: {:?}", result);
        }
        result = public_server => {
            error!("Public server stopped: {:?}", result);
        }
    }

    Ok(())
}