//! Auto-upload crate for the Janitor project.
//!
//! This crate provides functionality for automatically uploading Debian packages.

#![deny(missing_docs)]

pub mod artifacts;
pub mod backfill;
pub mod config;
pub mod database;
pub mod error;
pub mod message_handler;
pub mod process;
pub mod redis_client;
pub mod service;
pub mod upload;
pub mod utils;
pub mod web;

use anyhow::Result;
use prometheus::{register_counter, Counter};

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
    dput_host: &str,
    debsign_keyid: Option<&str>,
    source_only: bool,
    distributions: Vec<String>,
    backfill: bool,
) -> Result<()> {
    use service::ServiceOrchestrator;
    
    // Create and run service orchestrator
    let orchestrator = ServiceOrchestrator::new(
        config,
        listen_addr.to_string(),
        port,
        dput_host,
        debsign_keyid,
        source_only,
        distributions,
        backfill,
    );
    
    orchestrator.run().await.map_err(Into::into)
}

