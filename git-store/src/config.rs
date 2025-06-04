//! Configuration for the git-store service

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration for the git-store service
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    /// Path to local git repositories
    pub local_path: PathBuf,

    /// Public URL for the git store
    pub public_url: String,

    /// Database URL
    pub database_url: String,

    /// Port to listen on for admin interface
    pub admin_port: u16,

    /// Port to listen on for public interface
    pub public_port: u16,

    /// Host to bind to
    #[serde(default = "default_host")]
    pub host: String,

    /// Path to templates directory
    pub templates_path: Option<PathBuf>,

    /// Git backend type (cgi or dulwich)
    #[serde(default = "default_backend")]
    pub git_backend: GitBackend,

    /// Enable tracing
    #[serde(default)]
    pub enable_tracing: bool,

    /// Prometheus push gateway URL
    pub prometheus_push_gateway: Option<String>,

    /// Worker authentication database table
    #[serde(default = "default_worker_table")]
    pub worker_table: String,

    /// Codebase validation database table
    #[serde(default = "default_codebase_table")]
    pub codebase_table: String,

    /// Title for the web interface
    #[serde(default = "default_title")]
    pub title: String,

    /// Maximum diff size in bytes
    #[serde(default = "default_max_diff_size")]
    pub max_diff_size: usize,

    /// Timeout for git operations in seconds
    #[serde(default = "default_git_timeout")]
    pub git_timeout: u64,
}

/// Git backend type
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum GitBackend {
    /// Use git http-backend CGI
    Cgi,
    /// Use pure Rust implementation
    Native,
}

impl Config {
    /// Load configuration from file
    pub fn from_file(path: &str) -> Result<Self, config::ConfigError> {
        let settings = config::Config::builder()
            .add_source(config::File::with_name(path))
            .add_source(config::Environment::with_prefix("GIT_STORE").separator("_"))
            .build()?;

        settings.try_deserialize()
    }

    /// Load configuration from environment only
    pub fn from_env() -> Result<Self, config::ConfigError> {
        let settings = config::Config::builder()
            .add_source(config::Environment::with_prefix("GIT_STORE").separator("_"))
            .set_default("local_path", "/srv/git")?
            .set_default("public_url", "http://localhost:9422")?
            .set_default("database_url", "postgresql://localhost/janitor")?
            .set_default("admin_port", 9421)?
            .set_default("public_port", 9422)?
            .build()?;

        settings.try_deserialize()
    }
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_backend() -> GitBackend {
    GitBackend::Cgi
}

fn default_worker_table() -> String {
    "worker".to_string()
}

fn default_codebase_table() -> String {
    "codebase".to_string()
}

fn default_title() -> String {
    "Janitor Git Store".to_string()
}

fn default_max_diff_size() -> usize {
    10 * 1024 * 1024 // 10MB
}

fn default_git_timeout() -> u64 {
    30 // seconds
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config {
            local_path: PathBuf::from("/srv/git"),
            public_url: "http://localhost:9422".to_string(),
            database_url: "postgresql://localhost/janitor".to_string(),
            admin_port: 9421,
            public_port: 9422,
            host: default_host(),
            templates_path: None,
            git_backend: default_backend(),
            enable_tracing: false,
            prometheus_push_gateway: None,
            worker_table: default_worker_table(),
            codebase_table: default_codebase_table(),
            title: default_title(),
            max_diff_size: default_max_diff_size(),
            git_timeout: default_git_timeout(),
        };

        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.git_backend, GitBackend::Cgi);
        assert_eq!(config.worker_table, "worker");
        assert_eq!(config.codebase_table, "codebase");
        assert_eq!(config.title, "Janitor Git Store");
        assert_eq!(config.max_diff_size, 10 * 1024 * 1024);
        assert_eq!(config.git_timeout, 30);
    }
}
