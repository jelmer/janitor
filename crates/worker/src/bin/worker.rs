use axum::{response::Html, response::Json, routing::get, Router};
use clap::Parser;
use janitor_worker::{Assignment, Metadata};
use pyo3::exceptions::PySystemExit;
use pyo3::prelude::*;
use std::net::SocketAddr;
use std::str::FromStr;

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

async fn index() -> Html<&'static str> {
    Html("<h1>Hello, World!</h1>")
}

async fn health() -> String {
    "ok".to_string()
}

async fn assignment() -> Json<Option<Assignment>> {
    // TODO
    Json(None)
}

async fn intermediate_result() -> Json<Option<Metadata>> {
    // TODO
    Json(None)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    args.logging.init();

    // build our application with a route
    let app = Router::new()
        .route("/", get(index))
        .route("/health", get(health))
        .route("/assignment", get(assignment))
        .route("/intermediate-result", get(intermediate_result));

    // run it
    let addr = SocketAddr::new(args.listen_address, args.new_port);
    log::info!("listening on {}", addr);

    tokio::task::spawn_blocking(move || {
        let thread_result = std::thread::spawn(move || {
            match Python::with_gil(|py| {
                let kwargs = pyo3::types::PyDict::new(py);
                kwargs.set_item("base_url", args.base_url.as_str())?;
                kwargs.set_item("output_directory", args.output_directory)?;
                kwargs.set_item("debug", args.logging.debug)?;
                kwargs.set_item("port", args.port)?;
                kwargs.set_item("listen_address", args.listen_address.to_string())?;
                kwargs.set_item("my_url", args.my_url.map(|u| u.to_string()))?;
                kwargs.set_item("external_address", args.external_address)?;
                kwargs.set_item("codebase", args.codebase)?;
                kwargs.set_item("campaign", args.campaign)?;
                kwargs.set_item("prometheus", args.prometheus.map(|p| p.to_string()))?;
                kwargs.set_item("tee", args.tee)?;
                kwargs.set_item("loop", args.r#loop)?;
                kwargs.set_item("credentials", args.credentials)?;
                kwargs.set_item("gcp_logging", args.logging.gcp_logging)?;

                let worker = py.import("janitor.worker")?;
                let main = worker.getattr("main_sync")?;

                match main.call((), Some(kwargs))?.extract::<Option<i32>>() {
                    Ok(o) => Ok(o),
                    Err(e) if e.is_instance_of::<PySystemExit>(py) => {
                        Ok(Some(e.value(py).getattr("code")?.extract::<i32>()?))
                    }
                    Err(e) => Err(e),
                }
            }) {
                Ok(Some(exit_code)) => std::process::exit(exit_code),
                Ok(None) => std::process::exit(0),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    if args.logging.debug {
                        pyo3::Python::with_gil(|py| {
                            if let Some(traceback) = e.traceback(py) {
                                println!("{}", traceback.format().unwrap());
                            }
                        });
                    }
                    std::process::exit(1);
                }
            }
        })
        .join();
        if let Err(e) = thread_result {
            std::process::exit(1);
        }
    });

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}
