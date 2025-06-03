//! Auto-upload service for the Janitor project.
//!
//! This service automatically uploads successful Debian package builds to configured repositories
//! using `debsign` for signing and `dput` for uploading.

use anyhow::Result;
use clap::Parser;
use janitor_auto_upload::{run_service, Config};
use tracing::info;

/// Command-line arguments for the auto-upload service
#[derive(Debug, Parser)]
#[command(
    name = "janitor-auto-upload",
    about = "Automatically upload Debian packages to repositories"
)]
struct Args {
    /// Port to listen on for HTTP server
    #[arg(long, default_value = "9933", env = "AUTO_UPLOAD_PORT")]
    port: u16,

    /// Address to listen on for HTTP server
    #[arg(long, default_value = "127.0.0.1", env = "AUTO_UPLOAD_LISTEN_ADDRESS")]
    listen_address: String,

    /// Path to configuration file
    #[arg(long, default_value = "janitor.conf", env = "JANITOR_CONFIG")]
    config: String,

    /// dput host to upload packages to
    #[arg(long, env = "DPUT_HOST")]
    dput_host: String,

    /// GPG key ID to use for signing packages
    #[arg(long, env = "DEBSIGN_KEYID")]
    debsign_keyid: Option<String>,

    /// Enable backfill mode to upload previously built packages
    #[arg(long)]
    backfill: bool,

    /// Only upload source-only changes files
    #[arg(long)]
    source_only: bool,

    /// Build distributions to upload (can be specified multiple times)
    #[arg(long = "distribution", env = "AUTO_UPLOAD_DISTRIBUTIONS")]
    distributions: Vec<String>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    let env_filter = if args.verbose {
        "janitor_auto_upload=debug,info"
    } else {
        "janitor_auto_upload=info,warn"
    };
    
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .init();

    info!("Starting Janitor auto-upload service");
    info!("Listen address: {}:{}", args.listen_address, args.port);
    info!("dput host: {}", args.dput_host);
    if let Some(keyid) = &args.debsign_keyid {
        info!("GPG key ID: {}", keyid);
    }
    if !args.distributions.is_empty() {
        info!("Distributions: {:?}", args.distributions);
    }

    // Load configuration
    let config = Config::load(&args.config).await?;

    // Run the service
    run_service(
        config,
        &args.listen_address,
        args.port,
        &args.dput_host,
        args.debsign_keyid.as_deref(),
        args.source_only,
        args.distributions,
        args.backfill,
    )
    .await
}