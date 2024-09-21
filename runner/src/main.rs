use clap::Parser;
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

    let state = Arc::new(RwLock::new(AppState {}));

    let app = janitor_runner::web::app(state.clone());

    // run it
    let addr = SocketAddr::new(args.listen_address, args.new_port);
    log::info!("listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}
