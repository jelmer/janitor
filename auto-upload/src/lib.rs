//! Auto-upload crate for the Janitor project.
//!
//! This crate provides functionality for automatically uploading Debian packages.

#![deny(missing_docs)]

pub mod artifacts;
pub mod config;
pub mod error;
pub mod message_handler;
pub mod process;
pub mod redis_client;
pub mod upload;
pub mod utils;
pub mod web;

use anyhow::Result;
use prometheus::{register_counter, Counter};
use std::sync::Arc;
use tracing::{error, info, warn};

pub use config::Config;
pub use error::{Result as UploadResult, UploadError};
use message_handler::MessageHandler;
use redis_client::RedisConnectionManager;
use upload::UploadConfig;

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
    dput_host: &str,
    debsign_keyid: Option<&str>,
    source_only: bool,
    distributions: Vec<String>,
    backfill: bool,
) -> Result<()> {
    let config = Arc::new(config);
    
    // Create upload configuration
    let upload_config = UploadConfig::new(
        dput_host.to_string(),
        debsign_keyid.map(|s| s.to_string()),
        source_only,
        distributions,
    );
    
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
    
    // Start Redis listener task
    let redis_task = tokio::spawn({
        let config = config.clone();
        let upload_config = upload_config.clone();
        async move {
            if let Err(e) = run_redis_listener(config, upload_config).await {
                error!("Redis listener failed: {}", e);
            }
        }
    });
    tasks.push(redis_task);
    
    // TODO: Add backfill task if enabled
    if backfill {
        info!("Backfill mode not yet implemented");
    }
    
    info!("Auto-upload service started");
    
    // Wait for tasks
    let results = futures::future::join_all(tasks).await;
    
    // Check if any task failed
    for result in results {
        if let Err(e) = result {
            error!("Task failed: {}", e);
        }
    }
    
    Ok(())
}

/// Run the Redis listener with automatic reconnection
async fn run_redis_listener(config: Arc<Config>, upload_config: UploadConfig) -> Result<()> {
    let mut connection_manager = RedisConnectionManager::new(config.redis_location.clone());
    
    loop {
        info!("Starting Redis listener");
        
        // Create message handler
        let message_handler = match MessageHandler::new(&config.artifact_location, upload_config.clone()).await {
            Ok(handler) => Arc::new(handler),
            Err(e) => {
                error!("Failed to create message handler: {}", e);
                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                continue;
            }
        };
        
        // Get Redis client
        let redis_client = match connection_manager.get_client().await {
            Ok(client) => client,
            Err(e) => {
                error!("Failed to get Redis client: {}", e);
                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                continue;
            }
        };
        
        // Subscribe to messages
        let result = redis_client.subscribe_to_results({
            let handler = message_handler.clone();
            move |message| {
                let handler = handler.clone();
                Box::pin(async move {
                    handler.handle_message(message).await
                })
            }
        }).await;
        
        // Handle disconnection
        match result {
            Ok(_) => {
                warn!("Redis subscription ended normally");
            }
            Err(e) => {
                error!("Redis subscription failed: {}", e);
                if let Err(reconnect_err) = connection_manager.handle_connection_error(&e).await {
                    error!("Failed to handle Redis reconnection: {}", reconnect_err);
                }
            }
        }
        
        // Wait before reconnecting
        info!("Waiting 5 seconds before reconnecting to Redis...");
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}
