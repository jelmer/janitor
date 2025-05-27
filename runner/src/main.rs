use clap::Parser;
use janitor_runner::application::Application;
use std::path::PathBuf;

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

    // Build application from config file or use defaults
    let config_path = args.config.unwrap_or_else(|| PathBuf::from("janitor.conf"));

    let mut app_builder = if config_path.exists() {
        Application::builder_from_file(&config_path).map_err(|e| {
            eprintln!(
                "Failed to load config from {}: {}",
                config_path.display(),
                e
            );
            1
        })?
    } else {
        log::info!(
            "Config file {} not found, using defaults",
            config_path.display()
        );
        Application::builder()
    };

    // Override config with command line arguments
    app_builder = app_builder
        .with_listen_address(args.listen_address)
        .with_port(args.port)
        .with_debug(args.debug);

    // Build and initialize the application
    let app = app_builder.build().await.map_err(|e| {
        eprintln!("Failed to initialize application: {}", e);
        1
    })?;

    // Run the application with graceful shutdown
    app.run_with_graceful_shutdown(|state| async move {
        let router = janitor_runner::web::app(state.clone()).layer(axum::middleware::from_fn(
            janitor_runner::tracing::http_tracing_middleware,
        ));

        let addr = format!("{}:{}", args.listen_address, args.port);
        log::info!("Listening on {}", addr);

        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, router.into_make_service()).await?;

        Ok(())
    })
    .await
    .map_err(|e| {
        eprintln!("Application error: {}", e);
        1
    })?;

    Ok(())
}
