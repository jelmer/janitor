use askama_axum::IntoResponse;
use askama_axum::Template;
use axum::{
    extract::Path, extract::State, http::HeaderMap, http::StatusCode, response::Html,
    response::Json, response::Response, routing::get, Router,
};
use clap::Parser;
use janitor::api::worker::{Assignment, Metadata};
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

async fn index(
    State(state): State<Arc<RwLock<AppState>>>,
) -> janitor_worker::web::IndexTemplate<'static> {
    let state = state.read().unwrap();
    let lognames: Option<Vec<String>> =
        if let Some(output_directory) = state.output_directory.as_ref() {
            Some(
                output_directory
                    .read_dir()
                    .unwrap()
                    .filter_map(|entry| {
                        let entry = entry.ok()?;
                        let filename = entry.file_name();
                        let name = filename.to_str()?;
                        if name.ends_with(".log") {
                            Some(name.to_owned())
                        } else {
                            None
                        }
                    })
                    .collect(),
            )
        } else {
            None
        };

    let metadata = state.metadata;

    janitor_worker::web::IndexTemplate {
        assignment: state.assignment.as_ref(),
        lognames,
        metadata,
    }
}

async fn health() -> String {
    "ok".to_string()
}

async fn assignment(State(state): State<Arc<RwLock<AppState>>>) -> Json<Option<Assignment>> {
    Json(state.read().unwrap().assignment.clone())
}

async fn get_logs(State(state): State<Arc<RwLock<AppState>>>, headers: HeaderMap) -> Response {
    let output_directory = &state.read().unwrap().output_directory;
    if output_directory.is_none() {
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body("Log directory not created yet".into())
            .unwrap();
    }
    let output_directory = output_directory.as_ref().unwrap();
    let names: Vec<String> = match std::fs::read_dir(output_directory) {
        Ok(dir) => dir
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.extension()?.to_str()? == "log" {
                    Some(entry.file_name().to_str()?.to_owned())
                } else {
                    None
                }
            })
            .collect(),
        Err(e) => {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(format!("Error reading log directory: {}", e).into())
                .unwrap();
        }
    };

    match headers
        .get(axum::http::header::ACCEPT)
        .map(|x| x.to_str().unwrap())
    {
        Some("application/json") => Json(names).into_response(),
        _ => janitor_worker::web::LogIndexTemplate { names }.into_response(),
    }
}

async fn get_artifacts(State(state): State<Arc<RwLock<AppState>>>, headers: HeaderMap) -> Response {
    let output_directory = &state.read().unwrap().output_directory;
    if output_directory.is_none() {
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body("Artifact directory not created yet".into())
            .unwrap();
    }
    let output_directory = output_directory.as_ref().unwrap();
    let names: Vec<String> = match std::fs::read_dir(output_directory) {
        Ok(dir) => dir
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.extension()?.to_str()? != "log" {
                    Some(entry.file_name().to_str()?.to_owned())
                } else {
                    None
                }
            })
            .collect(),
        Err(e) => {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(format!("Error reading log directory: {}", e).into())
                .unwrap();
        }
    };

    match headers
        .get(axum::http::header::ACCEPT)
        .map(|x| x.to_str().unwrap())
    {
        Some("application/json") => Json(names).into_response(),
        _ => janitor_worker::web::ArtifactIndexTemplate { names }.into_response(),
    }
}

async fn get_log_id(State(state): State<Arc<RwLock<AppState>>>) -> Json<Option<String>> {
    Json(
        state
            .read()
            .unwrap()
            .assignment
            .as_ref()
            .map(|a| a.id.clone()),
    )
}

async fn get_log_file(
    State(state): State<Arc<RwLock<AppState>>>,
    Path(filename): Path<String>,
) -> Response {
    // filenames should only contain characters that are safe to use in URLs
    if filename.contains('/') || filename.contains('\\') {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body("Invalid filename".into())
            .unwrap();
    }

    let p = if let Some(output_directory) = state.read().unwrap().output_directory.as_ref() {
        output_directory.join(filename)
    } else {
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body("Log directory not created yet".into())
            .unwrap();
    };

    let file = match tokio::fs::File::open(&p).await {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body("No such log file".into())
                .unwrap();
        }
        Err(e) => {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(format!("Error opening log file: {}", e).into())
                .unwrap();
        }
    };

    let stream = tokio_util::io::ReaderStream::new(file);

    let body = axum::body::Body::from_stream(stream);

    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        "text/plain".parse().unwrap(),
    );

    (StatusCode::OK, headers, body).into_response()
}

async fn get_artifact_file(
    State(state): State<Arc<RwLock<AppState>>>,
    Path(filename): Path<String>,
) -> Response {
    // filenames should only contain characters that are safe to use in URLs
    if filename.contains('/') || filename.contains('\\') {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body("Invalid filename".into())
            .unwrap();
    }

    let p = if let Some(output_directory) = state.read().unwrap().output_directory.as_ref() {
        output_directory.join(filename)
    } else {
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body("Artifact directory not created yet".into())
            .unwrap();
    };

    let file = match tokio::fs::File::open(&p).await {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body("No such artifact file".into())
                .unwrap();
        }
        Err(e) => {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(format!("Error opening artifact file: {}", e).into())
                .unwrap();
        }
    };

    let stream = tokio_util::io::ReaderStream::new(file);

    let body = axum::body::Body::from_stream(stream);

    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        "application/octet-stream".parse().unwrap(),
    );

    (StatusCode::OK, headers, body).into_response()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    args.logging.init();

    let state = Arc::new(RwLock::new(AppState {
        assignment: None,
        output_directory: None,
    }));

    let global_config = breezyshim::config::global_stack().unwrap();
    global_config.set("branch.fetch_tags", &true).unwrap();

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

    // build our application with a route
    let app = Router::new()
        .route("/", get(index))
        .route("/health", get(health))
        .route("/assignment", get(assignment))
        .route("/logs", get(get_logs))
        .route("/logs/:filename", get(get_log_file))
        .route("/log-id", get(get_log_id))
        .route("/artifacts", get(get_artifacts))
        .route("/artifacts/:filename", get(get_artifact_file))
        .with_state(state.clone().clone());

    // run it
    let addr = SocketAddr::new(args.listen_address, args.new_port);
    log::info!("listening on {}", addr);

    // Run worker loop in background
    let state = state.clone();
    let worker = tokio::spawn(async move {
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
