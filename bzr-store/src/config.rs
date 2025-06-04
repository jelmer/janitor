//! Configuration management for BZR Store service

use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Configuration for the BZR Store service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Database connection URL
    pub database_url: String,

    /// Base path for repository storage
    pub repository_path: PathBuf,

    /// Admin interface bind address (full access)
    pub admin_bind: SocketAddr,

    /// Public interface bind address (read-only)
    pub public_bind: SocketAddr,

    /// Optional Python path for PyO3
    pub python_path: Option<String>,

    /// Maximum number of database connections
    pub max_connections: u32,

    /// Request timeout in seconds
    pub request_timeout: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            database_url: "postgresql://localhost/janitor".to_string(),
            repository_path: PathBuf::from("/var/lib/janitor/bzr"),
            admin_bind: "127.0.0.1:9929".parse().unwrap(),
            public_bind: "127.0.0.1:9930".parse().unwrap(),
            python_path: None,
            max_connections: 10,
            request_timeout: 30,
        }
    }
}

impl Config {
    /// Load configuration from environment variables and config file
    pub async fn load() -> Result<Self> {
        let mut config = Self::default();

        // Override with environment variables
        if let Ok(database_url) = env::var("DATABASE_URL") {
            config.database_url = database_url;
        }

        if let Ok(repo_path) = env::var("BZR_REPOSITORY_PATH") {
            config.repository_path = PathBuf::from(repo_path);
        }

        if let Ok(admin_bind) = env::var("BZR_ADMIN_BIND") {
            config.admin_bind = admin_bind
                .parse()
                .context("Invalid BZR_ADMIN_BIND address")?;
        }

        if let Ok(public_bind) = env::var("BZR_PUBLIC_BIND") {
            config.public_bind = public_bind
                .parse()
                .context("Invalid BZR_PUBLIC_BIND address")?;
        }

        if let Ok(python_path) = env::var("PYTHON_PATH") {
            config.python_path = Some(python_path);
        }

        if let Ok(max_conn) = env::var("BZR_MAX_CONNECTIONS") {
            config.max_connections = max_conn.parse().context("Invalid BZR_MAX_CONNECTIONS")?;
        }

        if let Ok(timeout) = env::var("BZR_REQUEST_TIMEOUT") {
            config.request_timeout = timeout.parse().context("Invalid BZR_REQUEST_TIMEOUT")?;
        }

        // Try to load from config file if it exists
        if let Ok(config_path) = env::var("BZR_CONFIG_PATH") {
            if let Ok(config_str) = tokio::fs::read_to_string(&config_path).await {
                let file_config: Config =
                    toml::from_str(&config_str).context("Failed to parse config file")?;
                // Merge file config with environment config (env takes precedence)
                config = file_config;
            }
        }

        // Validate configuration
        config.validate()?;

        Ok(config)
    }

    /// Validate the configuration
    fn validate(&self) -> Result<()> {
        // Ensure repository path is absolute
        if !self.repository_path.is_absolute() {
            return Err(anyhow::anyhow!(
                "Repository path must be absolute: {}",
                self.repository_path.display()
            ));
        }

        // Ensure admin and public ports are different
        if self.admin_bind.port() == self.public_bind.port() {
            return Err(anyhow::anyhow!(
                "Admin and public interfaces cannot use the same port"
            ));
        }

        Ok(())
    }
}
