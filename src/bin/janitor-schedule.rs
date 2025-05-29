use clap::Parser;
use std::time::Instant;

#[derive(Parser)]
struct Args {
    #[clap(long)]
    /// Create branches but don't push or propose anything.
    dry_run: bool,

    #[clap(long)]
    /// Prometheus push gateway to export to.
    prometheus: Option<String>,

    #[clap(long, default_value = "janitor.conf")]
    /// Path to configuration.
    config: std::path::PathBuf,

    #[clap(long)]
    /// Restrict to a specific campaign.
    campaign: Option<String>,

    /// Codebase to process.
    codebases: Vec<String>,

    #[clap(long)]
    /// Bucket to use.
    bucket: Option<String>,

    #[clap(long)]
    /// Requester to use.
    requester: Option<String>,

    #[clap(long)]
    /// Refresh the queue.
    refresh: bool,

    #[clap(long)]
    /// Enable GCP logging integration.
    gcp_logging: bool,

    #[clap(flatten)]
    logging: janitor::logging::LoggingArgs,
}

#[tokio::main]
async fn main() -> Result<(), i32> {
    let args = Args::parse();
    let start_time = Instant::now();

    args.logging.init();

    // GCP logging setup if requested
    if args.gcp_logging {
        log::info!("GCP logging integration enabled");
        // Note: Full GCP logging integration would require additional dependencies
        // For now, this is a placeholder for future implementation
    }

    log::info!("Starting janitor scheduler");
    log::info!("Reading configuration from: {:?}", args.config);

    let config = janitor::config::read_file(&args.config).map_err(|e| {
        log::error!("Failed to read configuration: {}", e);
        1
    })?;

    log::info!("Connecting to database");
    let db = janitor::state::create_pool(&config).await.map_err(|e| {
        log::error!("Failed to connect to database: {}", e);
        1
    })?;

    // Log configuration summary
    if let Some(ref campaign) = args.campaign {
        log::info!("Filtering by campaign: {}", campaign);
    }
    if !args.codebases.is_empty() {
        log::info!("Processing {} specific codebases", args.codebases.len());
    }
    if args.dry_run {
        log::warn!("DRY RUN MODE: No actual scheduling will occur");
    }

    log::info!("Finding candidates with publish policy");
    let todo = janitor::schedule::iter_schedule_requests_from_candidates(
        &db,
        if args.codebases.is_empty() {
            None
        } else {
            Some(args.codebases.iter().map(|x| x.as_str()).collect())
        },
        args.campaign.as_deref(),
    )
    .await
    .map_err(|e| {
        log::error!("Failed to find candidates: {}", e);
        1
    })?
    .collect::<Vec<_>>();

    if todo.is_empty() {
        log::info!("No candidates found for scheduling");
        return Ok(());
    }

    log::info!("Determined schedule for {} candidates", todo.len());

    // Log some statistics about what we're scheduling
    let campaigns: std::collections::HashSet<_> = todo.iter().map(|t| &t.campaign).collect();
    let codebases: std::collections::HashSet<_> = todo.iter().map(|t| &t.codebase).collect();
    log::info!(
        "Scheduling across {} campaigns and {} codebases",
        campaigns.len(),
        codebases.len()
    );

    if args.dry_run {
        log::info!("DRY RUN: Would schedule {} items", todo.len());
        for item in &todo {
            log::debug!(
                "Would schedule: {} on {} (campaign: {}, value: {})",
                item.command,
                item.codebase,
                item.campaign,
                item.value
            );
        }
    } else {
        log::info!("Adding {} items to queue", todo.len());

        janitor::schedule::bulk_add_to_queue(
            &db,
            &todo,
            args.dry_run,
            0.0, // default_offset
            args.bucket.as_deref(),
            args.requester.as_deref(),
            args.refresh,
        )
        .await
        .map_err(|e| {
            log::error!("Failed to add items to queue: {}", e);
            1
        })?;

        log::info!("Successfully scheduled {} items", todo.len());
    }

    let duration = start_time.elapsed();
    log::info!("Scheduling completed in {:.2}s", duration.as_secs_f64());

    // Prometheus metrics export if configured
    if let Some(prometheus_gateway) = args.prometheus {
        log::info!(
            "Exporting metrics to Prometheus gateway: {}",
            prometheus_gateway
        );

        // Create a simple success metric
        let success_gauge = prometheus::Gauge::new(
            "janitor_schedule_last_success",
            "Timestamp of last successful scheduling run",
        )
        .map_err(|e| {
            log::error!("Failed to create Prometheus gauge: {}", e);
            1
        })?;

        success_gauge.set(chrono::Utc::now().timestamp() as f64);

        let metric_families = prometheus::gather();
        let labels = prometheus::labels! {"instance".to_string() => "main".to_string()};
        if let Err(e) = prometheus::push_metrics(
            "janitor-scheduler",
            labels,
            &prometheus_gateway,
            metric_families,
            None,
        ) {
            log::warn!("Failed to push metrics to Prometheus: {}", e);
            // Don't fail the entire run for metrics issues
        } else {
            log::info!("Metrics pushed to Prometheus successfully");
        }
    }

    log::info!("Scheduler finished successfully");
    Ok(())
}
