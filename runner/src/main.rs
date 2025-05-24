use clap::Parser;
use janitor_runner::{database, AppState};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Parser)]
struct Args {
    #[clap(long, default_value = "localhost")]
    listen_address: String,

    #[clap(long, default_value = "9911")]
    port: u16,

    #[clap(long, default_value = "9919")]
    public_port: u16,

    #[clap(long)]
    /// Command to run to check codebase before pushing
    post_check: Option<String>,

    #[clap(long)]
    /// Command to run to check whether to process codebase
    pre_check: Option<String>,

    #[clap(long)]
    /// Use cached branches only.
    use_cached_only: bool,

    #[clap(long, default_value = "janitor.conf")]
    /// Path to configuration.
    config: Option<PathBuf>,

    #[clap(long)]
    /// Backup directory to write files to if artifact or log manager is unreachable.
    backup_directory: Option<PathBuf>,

    #[clap(long)]
    /// Public vcs location (used for URLs handed to worker)
    public_vcs_location: Option<String>,

    #[clap(long)]
    /// Base location for our own APT archive
    public_apt_archive_location: Option<String>,

    #[clap(long)]
    public_dep_server_url: Option<String>,

    #[clap(flatten)]
    logging: janitor::logging::LoggingArgs,

    #[clap(long)]
    /// Print debugging info
    debug: bool,

    #[clap(long, default_value = "60")]
    /// Time before marking a run as having timed out (minutes)
    run_timeout: u64,

    #[clap(long)]
    /// Avoid processing runs on a host (e.g. 'salsa.debian.org')
    avoid_host: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), i32> {
    let args = Args::parse();

    args.logging.init();

    // Load configuration
    let config_path = args.config.unwrap_or_else(|| PathBuf::from("janitor.conf"));
    let config = match janitor::config::read_file(&config_path) {
        Ok(config) => config,
        Err(e) => {
            eprintln!(
                "Failed to read config file {}: {}",
                config_path.display(),
                e
            );
            return Err(1);
        }
    };

    // Create database connection
    let pool = match janitor::state::create_pool(&config).await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Failed to create database pool: {}", e);
            return Err(1);
        }
    };

    let database = Arc::new(database::RunnerDatabase::new(pool));
    let state = Arc::new(AppState { database });

    let app = janitor_runner::web::app(state.clone());

    // Run it
    let addr = format!("{}:{}", args.listen_address, args.port);
    log::info!("listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.map_err(|e| {
        eprintln!("Failed to bind to {}: {}", addr, e);
        1
    })?;

    axum::serve(listener, app.into_make_service())
        .await
        .map_err(|e| {
            eprintln!("Server error: {}", e);
            1
        })?;

    Ok(())
}
