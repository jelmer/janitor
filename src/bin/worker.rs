use clap::Parser;
use pyo3::prelude::*;

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

    /// Request run for specified codebase
    #[clap(long)]
    codebase: Option<String>,

    /// Request run for specified campaign
    #[clap(long)]
    campaign: Option<String>,

    /// Address to listen on
    #[clap(long)]
    listen_address: Option<std::net::IpAddr>,

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    args.logging.init();

    let r = Python::with_gil(|py| {
        let kwargs = pyo3::types::PyDict::new(py);
        kwargs.set_item("base_url", args.base_url.as_str())?;
        kwargs.set_item("output_directory", args.output_directory)?;
        kwargs.set_item("debug", args.logging.debug)?;
        kwargs.set_item("port", args.port)?;
        kwargs.set_item("listen_address", args.listen_address)?;
        kwargs.set_item("my_url", args.my_url.map(|u| u.to_string()))?;
        kwargs.set_item("external_address", args.external_address)?;
        kwargs.set_item("codebase", args.codebase)?;
        kwargs.set_item("campaign", args.campaign)?;
        kwargs.set_item("prometheus", args.prometheus.map(|p| p.to_string()))?;
        kwargs.set_item("tee", args.tee)?;
        kwargs.set_item("loop", args.r#loop)?;
        kwargs.set_item("credentials", args.credentials)?;

        let worker = py.import("janitor.worker")?;
        let main = worker.getattr("main_sync")?;
        main.call((), Some(kwargs))?.extract::<Option<i32>>()
    });

    match r {
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
}
