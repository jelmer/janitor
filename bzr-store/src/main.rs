//! BZR Store service main entry point
//!
//! This service provides HTTP-accessible Bazaar repositories with administrative and public interfaces.
//! It uses PyO3 to integrate with the Python Breezy library for Bazaar protocol support.

use anyhow::Result;
use bzr_store::{config::Config, web::create_applications};
use pyo3::prelude::*;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Load configuration
    let config = Config::load().await.map_err(|e| {
        eprintln!("Failed to load configuration: {}", e);
        e
    })?;

    info!("Starting BZR Store service");
    info!("Admin interface: {}", config.admin_bind);
    info!("Public interface: {}", config.public_bind);
    info!("Repository path: {}", config.repository_path.display());

    // Initialize Python interpreter for PyO3
    Python::with_gil(|py| {
        // Import required Python modules to verify availability
        match py.import_bound("breezy") {
            Ok(_) => info!("Breezy library available"),
            Err(e) => {
                warn!("Breezy library not available, falling back to subprocess: {}", e);
            }
        }
    });

    // Create the applications
    let (admin_app, public_app) = create_applications(config.clone()).await?;

    // Start both servers concurrently
    let admin_server = {
        let listener = tokio::net::TcpListener::bind(config.admin_bind).await?;
        info!("Admin server listening on {}", config.admin_bind);
        axum::serve(listener, admin_app)
    };

    let public_server = {
        let listener = tokio::net::TcpListener::bind(config.public_bind).await?;
        info!("Public server listening on {}", config.public_bind);
        axum::serve(listener, public_app)
    };

    // Run both servers concurrently
    tokio::try_join!(admin_server, public_server)?;

    Ok(())
}