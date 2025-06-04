//! Archive service main entry point.
//!
//! This is the main entry point for the Janitor archive service, which provides
//! APT repository generation and serving functionality.

use clap::{Arg, Command};
use std::path::PathBuf;
use tracing::{error, info, warn};

use janitor_archive::{
    config::{AptRepositoryConfig, ArchiveConfig, GpgConfig},
    database::ArchiveDatabase,
    error::ArchiveResult,
    repository::{RepositoryGenerationConfig, RepositoryGenerator},
    scanner::PackageScanner,
    web::ArchiveWebService,
};

#[tokio::main]
async fn main() -> ArchiveResult<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let matches = Command::new("janitor-archive")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Janitor Archive Service - APT repository generation and serving")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Configuration file path")
                .default_value("archive.json"),
        )
        .arg(
            Arg::new("bind")
                .short('b')
                .long("bind")
                .value_name("ADDRESS")
                .help("Bind address for web server")
                .default_value("0.0.0.0:8080"),
        )
        .arg(
            Arg::new("database-url")
                .long("database-url")
                .value_name("URL")
                .help("Database connection URL")
                .env("DATABASE_URL"),
        )
        .subcommand(
            Command::new("generate").about("Generate repositories").arg(
                Arg::new("suite")
                    .short('s')
                    .long("suite")
                    .value_name("SUITE")
                    .help("Suite to generate (optional, generates all if not specified)"),
            ),
        )
        .subcommand(
            Command::new("serve").about("Start web server").arg(
                Arg::new("bind")
                    .short('b')
                    .long("bind")
                    .value_name("ADDRESS")
                    .help("Bind address")
                    .default_value("0.0.0.0:8080"),
            ),
        )
        .subcommand(Command::new("cleanup").about("Clean up old repository files"))
        .get_matches();

    let config_path = PathBuf::from(matches.get_one::<String>("config").unwrap());
    let bind_address = matches.get_one::<String>("bind").unwrap();

    // Load configuration
    let config = if config_path.exists() {
        info!("Loading configuration from: {:?}", config_path);
        ArchiveConfig::from_file(&config_path)?
    } else {
        warn!("Configuration file not found, using defaults");
        create_default_config(&config_path)?
    };

    // Override database URL if provided via argument
    let mut config = config;
    if let Some(database_url) = matches.get_one::<String>("database-url") {
        config.database.url = database_url.clone();
    }

    info!("Archive service starting with configuration loaded");

    match matches.subcommand() {
        Some(("generate", sub_matches)) => {
            let suite = sub_matches.get_one::<String>("suite");
            generate_repositories(&config, suite).await?;
        }
        Some(("serve", sub_matches)) => {
            let bind_addr = sub_matches
                .get_one::<String>("bind")
                .unwrap_or(bind_address);
            start_web_server(&config, bind_addr).await?;
        }
        Some(("cleanup", _)) => {
            cleanup_repositories(&config).await?;
        }
        _ => {
            // Default: start web server
            start_web_server(&config, bind_address).await?;
        }
    }

    Ok(())
}

/// Generate repositories.
async fn generate_repositories(
    config: &ArchiveConfig,
    suite: Option<&String>,
) -> ArchiveResult<()> {
    info!("Starting repository generation...");

    // Initialize services for generation
    let db_pool = sqlx::PgPool::connect(&config.database.url)
        .await
        .map_err(|e| janitor_archive::error::ArchiveError::Database(e))?;
    let database = std::sync::Arc::new(ArchiveDatabase::new(db_pool));
    let scanner = std::sync::Arc::new(PackageScanner::new().await?);
    let repo_config = RepositoryGenerationConfig::default();
    let generator = RepositoryGenerator::new(scanner, database, repo_config);

    if let Some(suite_name) = suite {
        // Generate specific suite
        if let Some(repo_config) = config.repositories.get(suite_name) {
            info!("Generating repository for suite: {}", suite_name);
            generator.generate_repository(repo_config).await?;
        } else {
            error!("Suite not found in configuration: {}", suite_name);
            return Err(janitor_archive::error::ArchiveError::InvalidConfiguration(
                format!("Unknown suite: {}", suite_name),
            ));
        }
    } else {
        // Generate all repositories
        info!("Generating all repositories...");
        generator
            .generate_repositories(&config.repositories)
            .await?;
    }

    info!("Repository generation completed");
    Ok(())
}

/// Start web server.
async fn start_web_server(config: &ArchiveConfig, bind_address: &str) -> ArchiveResult<()> {
    info!("Starting web server on: {}", bind_address);

    // Initialize services for web server
    let db_pool = sqlx::PgPool::connect(&config.database.url)
        .await
        .map_err(|e| janitor_archive::error::ArchiveError::Database(e))?;
    let database = ArchiveDatabase::new(db_pool);
    let scanner = PackageScanner::new().await?;
    let repo_config = RepositoryGenerationConfig::default();
    let generator = RepositoryGenerator::new(
        std::sync::Arc::new(scanner),
        std::sync::Arc::new(database),
        repo_config,
    );

    // Initialize web service
    let web_service = ArchiveWebService::new(
        config.clone(),
        generator,
        PackageScanner::new().await?,
        ArchiveDatabase::new(
            sqlx::PgPool::connect(&config.database.url)
                .await
                .map_err(|e| janitor_archive::error::ArchiveError::Database(e))?,
        ),
    )
    .await?;

    web_service.serve(bind_address).await
}

/// Clean up old repository files.
async fn cleanup_repositories(config: &ArchiveConfig) -> ArchiveResult<()> {
    info!("Starting repository cleanup...");

    // Initialize services for cleanup
    let db_pool = sqlx::PgPool::connect(&config.database.url)
        .await
        .map_err(|e| janitor_archive::error::ArchiveError::Database(e))?;
    let database = std::sync::Arc::new(ArchiveDatabase::new(db_pool));
    let scanner = std::sync::Arc::new(PackageScanner::new().await?);
    let repo_config = RepositoryGenerationConfig::default();
    let generator = RepositoryGenerator::new(scanner, database, repo_config);

    for (name, repo_config) in &config.repositories {
        info!("Cleaning up repository: {}", name);
        generator.cleanup_repository(repo_config).await?;
    }

    info!("Repository cleanup completed");
    Ok(())
}

/// Create a default configuration file.
fn create_default_config(config_path: &PathBuf) -> ArchiveResult<ArchiveConfig> {
    info!("Creating default configuration at: {:?}", config_path);

    let mut config = ArchiveConfig::default();

    // Add example repository
    let example_repo = AptRepositoryConfig::new(
        "lintian-fixes".to_string(),
        "lintian-fixes".to_string(),
        vec!["amd64".to_string(), "arm64".to_string()],
        PathBuf::from("/var/lib/janitor/archive/lintian-fixes"),
    );
    config.add_repository("lintian-fixes".to_string(), example_repo);

    // Add GPG configuration if available
    if let Ok(gpg_key_id) = std::env::var("GPG_KEY_ID") {
        config.gpg = Some(GpgConfig::new(gpg_key_id));
    }

    // Save configuration file
    config.to_file(config_path)?;

    info!(
        "Default configuration created. Please edit {:?} to customize settings.",
        config_path
    );

    Ok(config)
}
