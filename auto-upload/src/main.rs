//! Auto-upload service for the Janitor project.
//!
//! This service automatically uploads successful Debian package builds to configured repositories
//! using `debsign` for signing and `dput` for uploading.

use anyhow::Result;
use clap::{Parser, Subcommand};
use janitor_auto_upload::backfill::{BackfillConfig, BackfillProcessor};
use janitor_auto_upload::{run_service, Config};
use std::time::Duration;
use tracing::{error, info};

/// Command-line arguments for the auto-upload service
#[derive(Debug, Parser)]
#[command(
    name = "janitor-auto-upload",
    about = "Automatically upload Debian packages to repositories"
)]
struct Args {
    /// Path to configuration file
    #[arg(long, default_value = "janitor.conf", env = "JANITOR_CONFIG")]
    config: String,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Command to run
    #[command(subcommand)]
    command: Commands,
}

/// Available commands
#[derive(Debug, Subcommand)]
enum Commands {
    /// Run the auto-upload service
    Serve {
        /// Port to listen on for HTTP server
        #[arg(long, default_value = "9933", env = "AUTO_UPLOAD_PORT")]
        port: u16,

        /// Address to listen on for HTTP server
        #[arg(long, default_value = "127.0.0.1", env = "AUTO_UPLOAD_LISTEN_ADDRESS")]
        listen_address: String,

        /// dput host to upload packages to
        #[arg(long, env = "DPUT_HOST")]
        dput_host: String,

        /// GPG key ID to use for signing packages
        #[arg(long, env = "DEBSIGN_KEYID")]
        debsign_keyid: Option<String>,

        /// Only upload source-only changes files
        #[arg(long)]
        source_only: bool,

        /// Build distributions to upload (can be specified multiple times)
        #[arg(long = "distribution", env = "AUTO_UPLOAD_DISTRIBUTIONS")]
        distributions: Vec<String>,
    },
    /// Run backfill operation to upload historical builds
    Backfill {
        /// dput host to upload packages to
        #[arg(long, env = "DPUT_HOST")]
        dput_host: String,

        /// GPG key ID to use for signing packages
        #[arg(long, env = "DEBSIGN_KEYID")]
        debsign_keyid: Option<String>,

        /// Only upload source-only changes files
        #[arg(long)]
        source_only: bool,

        /// Build distributions to upload (can be specified multiple times)
        #[arg(long = "distribution")]
        distributions: Vec<String>,

        /// Source packages to upload (can be specified multiple times)
        #[arg(long = "source")]
        source_packages: Vec<String>,

        /// Maximum number of builds to process
        #[arg(long)]
        max_builds: Option<u64>,

        /// Delay between uploads in seconds
        #[arg(long, default_value = "1")]
        upload_delay: u64,

        /// Number of retries for failed uploads
        #[arg(long, default_value = "3")]
        max_retries: u32,

        /// Batch size for processing
        #[arg(long, default_value = "100")]
        batch_size: u32,

        /// Dry run mode (don't actually upload)
        #[arg(long)]
        dry_run: bool,
    },
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

    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    // Load configuration
    let config = Config::load(&args.config).await?;

    match args.command {
        Commands::Serve {
            port,
            listen_address,
            dput_host,
            debsign_keyid,
            source_only,
            distributions,
        } => {
            info!("Starting Janitor auto-upload service");
            info!("Listen address: {}:{}", listen_address, port);
            info!("dput host: {}", dput_host);
            if let Some(keyid) = &debsign_keyid {
                info!("GPG key ID: {}", keyid);
            }
            if !distributions.is_empty() {
                info!("Distributions: {:?}", distributions);
            }

            // Run the service (without backfill)
            run_service(
                config,
                &listen_address,
                port,
                &dput_host,
                debsign_keyid.as_deref(),
                source_only,
                distributions,
                false, // backfill = false for serve mode
            )
            .await
        }
        Commands::Backfill {
            dput_host,
            debsign_keyid,
            source_only,
            distributions,
            source_packages,
            max_builds,
            upload_delay,
            max_retries,
            batch_size,
            dry_run,
        } => {
            info!("Starting backfill operation");
            info!("dput host: {}", dput_host);
            if let Some(keyid) = &debsign_keyid {
                info!("GPG key ID: {}", keyid);
            }
            if !distributions.is_empty() {
                info!("Distributions: {:?}", distributions);
            }
            if !source_packages.is_empty() {
                info!("Source packages: {:?}", source_packages);
            }
            if let Some(max) = max_builds {
                info!("Max builds: {}", max);
            }
            if dry_run {
                info!("Running in dry-run mode");
            }

            run_backfill_operation(
                config,
                dput_host,
                debsign_keyid,
                source_only,
                distributions,
                source_packages,
                max_builds,
                upload_delay,
                max_retries,
                batch_size,
                dry_run,
            )
            .await
        }
    }
}

/// Run backfill operation
async fn run_backfill_operation(
    config: Config,
    dput_host: String,
    debsign_keyid: Option<String>,
    source_only: bool,
    distributions: Vec<String>,
    source_packages: Vec<String>,
    max_builds: Option<u64>,
    upload_delay: u64,
    max_retries: u32,
    batch_size: u32,
    dry_run: bool,
) -> Result<()> {
    use janitor_auto_upload::upload::UploadConfig;

    // Create upload configuration
    let upload_config =
        UploadConfig::new(dput_host, debsign_keyid, source_only, distributions.clone());

    // Create backfill processor
    let processor = BackfillProcessor::new(
        &config.database_location,
        &config.artifact_location,
        upload_config,
    )
    .await?;

    // Create backfill configuration
    let backfill_config = BackfillConfig {
        distributions,
        source_packages,
        max_builds,
        upload_delay: Duration::from_secs(upload_delay),
        max_retries,
        batch_size,
        dry_run,
    };

    // Run backfill
    match processor.run_backfill(backfill_config).await {
        Ok(summary) => {
            info!("Backfill completed successfully");
            info!("{}", summary);

            if summary.failed_uploads > 0 {
                error!("Some uploads failed during backfill");
                std::process::exit(1);
            }
        }
        Err(e) => {
            error!("Backfill failed: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
