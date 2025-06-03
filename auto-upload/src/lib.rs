//! Auto-upload crate for the Janitor project.
//!
//! This crate provides functionality for automatically uploading Debian packages.

#![deny(missing_docs)]

pub mod config;
pub mod error;
pub mod utils;
pub mod web;

use anyhow::Result;
use prometheus::{register_counter, Counter};
use std::sync::Arc;
use tracing::{error, info};

pub use config::Config;
pub use error::{Result as UploadResult, UploadError};

lazy_static::lazy_static! {
    /// Counter for failed package signings
    pub static ref DEBSIGN_FAILED_COUNT: Counter = register_counter!(
        "debsign_failed_total",
        "Number of packages for which signing failed"
    ).unwrap();
    
    /// Counter for failed package uploads
    pub static ref UPLOAD_FAILED_COUNT: Counter = register_counter!(
        "upload_failed_total", 
        "Number of packages for which uploading failed"
    ).unwrap();
}

/// Run the auto-upload service
pub async fn run_service(
    config: Config,
    listen_addr: &str,
    port: u16,
    _dput_host: &str,
    _debsign_keyid: Option<&str>,
    _source_only: bool,
    _distributions: Vec<String>,
    _backfill: bool,
) -> Result<()> {
    let config = Arc::new(config);
    
    // Create web application
    let app = web::create_app(config.clone());
    
    // Create tasks
    let mut tasks = vec![];
    
    // Start web server
    let web_task = tokio::spawn({
        let app = app.clone();
        let listen_addr = listen_addr.to_string();
        async move {
            if let Err(e) = web::run_web_server(app, &listen_addr, port).await {
                error!("Web server failed: {}", e);
            }
        }
    });
    tasks.push(web_task);
    
    // TODO: Add Redis listener task
    // TODO: Add backfill task if enabled
    
    info!("Auto-upload service started");
    
    // Wait for tasks
    for task in tasks {
        task.await?;
    }
    
    Ok(())
}
