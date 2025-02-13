use clap::Parser;
use janitor_worker::AppState;
use serde::Deserialize;
use std::fs::File;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};

#[derive(Parser, Debug)]
#[command(author, version)]
struct Args {
    /// Base URL
    #[clap(long, env = "JANITOR_BASE_URL")]
    base_url: url::Url,

    /// Output directory
    #[clap(long, default_value = ".")]
    output_directory: std::path::PathBuf,

    /// Path to credentials file (JSON).
    #[clap(long, env = "JANITOR_CREDENTIALS")]
    credentials: Option<std::path::PathBuf>,

    #[command(flatten)]
    logging: janitor::logging::LoggingArgs,

    /// Prometheus push gateway to export to
    #[clap(long)]
    prometheus: Option<url::Url>,

    /// Port to use for diagnostics web server
    port: Option<u16>,

    /// Port to use for diagnostics web server (rust)
    #[clap(long, default_value_t = 9820)]
    new_port: u16,

    /// Request run for specified codebase
    #[clap(long)]
    codebase: Option<String>,

    /// Request run for specified campaign
    #[clap(long)]
    campaign: Option<String>,

    /// Address to listen on
    #[clap(long, default_value = "0.0.0.0")]
    listen_address: std::net::IpAddr,

    /// IP / hostname this instance can be reached on by runner
    #[clap(long)]
    external_address: Option<String>,

    #[clap(long)]
    site_port: u16,

    /// URL this instance can be reached on by runner
    #[clap(long)]
    my_url: Option<url::Url>,

    /// Keep building until the queue is empty
    #[clap(long)]
    r#loop: bool,

    /// Copy work output to standard out, in addition to worker.log
    #[clap(long)]
    tee: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    args.logging.init();

    let state = Arc::new(RwLock::new(AppState {
        assignment: None,
        output_directory: None,
        metadata: None,
    }));

    let global_config = breezyshim::config::global_stack().unwrap();
    global_config.set("branch.fetch_tags", true).unwrap();

    let base_url = args.base_url;

    let auth = if let Some(credentials) = args.credentials {
        #[derive(Deserialize)]
        struct JsonCredentials {
            login: String,
            password: String,
        }
        let creds: JsonCredentials =
            serde_json::from_reader(File::open(credentials).unwrap()).unwrap();
        janitor_worker::client::Credentials::Basic {
            username: creds.login,
            password: Some(creds.password),
        }
    } else if let Ok(worker_name) = std::env::var("WORKER_NAME") {
        janitor_worker::client::Credentials::Basic {
            username: worker_name,
            password: std::env::var("WORKER_PASSWORD").ok(),
        }
    } else {
        janitor_worker::client::Credentials::from_url(&base_url)
    };

    let jenkins_build_url: Option<url::Url> =
        std::env::var("BUILD_URL").ok().map(|x| x.parse().unwrap());

    let node_name = std::env::var("NODE_NAME")
        .unwrap_or_else(|_| gethostname::gethostname().to_str().unwrap().to_owned());

    let my_url = if let Some(my_url) = args.my_url.as_ref() {
        Some(my_url.clone())
    } else if let Some(external_address) = args.external_address {
        Some(
            format!("http://{}:{}", external_address, args.site_port)
                .parse()
                .unwrap(),
        )
    } else if let Ok(my_ip) = std::env::var("MY_IP") {
        Some(
            format!("http://{}:{}", my_ip, args.site_port)
                .parse()
                .unwrap(),
        )
    } else if janitor_worker::is_gce_instance().await {
        if let Some(external_ip) = janitor_worker::gce_external_ip().await.unwrap() {
            Some(
                format!("http://{}:{}", external_ip, args.site_port)
                    .parse()
                    .unwrap(),
            )
        } else {
            // TODO(jelmer): Find out kubernetes IP?
            None
        }
    } else {
        None
    };

    if let Some(my_url) = my_url.as_ref() {
        log::info!("Diagnostics available at {}", my_url);
    }

    let app = janitor_worker::web::app(state.clone());

    // run it
    let addr = SocketAddr::new(args.listen_address, args.new_port);
    log::info!("listening on {}", addr);

    // Run worker loop in background
    let state = state.clone();
    tokio::spawn(async move {
        let client =
            janitor_worker::client::Client::new(base_url, auth, janitor_worker::DEFAULT_USER_AGENT);
        loop {
            let exit_code = match janitor_worker::process_single_item(
                &client,
                my_url.as_ref(),
                &node_name,
                jenkins_build_url.as_ref(),
                args.prometheus.as_ref(),
                args.codebase.as_deref(),
                args.campaign.as_deref(),
                args.tee,
                Some(&args.output_directory),
                state.clone(),
            )
            .await
            {
                Err(janitor_worker::SingleItemError::AssignmentFailure(e)) => {
                    log::error!("failed to get assignment: {}", e);
                    1
                }
                Err(janitor_worker::SingleItemError::ResultUploadFailure(e)) => {
                    log::error!("failed to upload result: {}", e);
                    1
                }
                Err(janitor_worker::SingleItemError::EmptyQueue) => {
                    log::info!("queue is empty");
                    0
                }
                Ok(_) => 0,
            };

            if !args.r#loop {
                std::process::exit(exit_code);
            }
        }
    });

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service()).await?;
    Ok(())
}
