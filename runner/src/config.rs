//! Comprehensive configuration management for the runner.

use crate::{
    artifacts::ArtifactConfig,
    error_tracking::ErrorTrackingConfig,
    logs::LogConfig,
    performance::PerformanceConfig,
    tracing::TracingConfig,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Complete runner configuration combining all subsystem configurations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnerConfig {
    /// Database configuration.
    pub database: DatabaseConfig,
    /// Redis configuration.
    pub redis: Option<RedisConfig>,
    /// VCS management configuration.
    pub vcs: VcsConfig,
    /// Log management configuration.
    pub logs: LogConfig,
    /// Artifact storage configuration.
    pub artifacts: ArtifactConfig,
    /// Performance monitoring configuration.
    pub performance: PerformanceConfig,
    /// Error tracking configuration.
    pub error_tracking: ErrorTrackingConfig,
    /// Tracing and logging configuration.
    pub tracing: TracingConfig,
    /// Web server configuration.
    pub web: WebConfig,
    /// Worker coordination configuration.
    pub worker: WorkerConfig,
    /// General application configuration.
    pub application: ApplicationConfig,
}

/// Database configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database URL for PostgreSQL connection.
    pub url: String,
    /// Maximum number of connections in the pool.
    #[serde(default = "default_db_max_connections")]
    pub max_connections: u32,
    /// Connection timeout in seconds.
    #[serde(default = "default_db_connection_timeout")]
    pub connection_timeout_seconds: u64,
    /// Query timeout in seconds.
    #[serde(default = "default_db_query_timeout")]
    pub query_timeout_seconds: u64,
    /// Enable SQL statement logging.
    #[serde(default)]
    pub enable_sql_logging: bool,
}

/// Redis configuration for coordination.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    /// Redis URL for connection.
    #[serde(default = "default_redis_url")]
    pub url: String,
    /// Connection timeout in seconds.
    #[serde(default = "default_redis_connection_timeout")]
    pub connection_timeout_seconds: u64,
    /// Command timeout in seconds.
    #[serde(default = "default_redis_command_timeout")]
    pub command_timeout_seconds: u64,
    /// Maximum number of connections in the pool.
    #[serde(default = "default_redis_max_connections")]
    pub max_connections: u32,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: default_redis_url(),
            connection_timeout_seconds: default_redis_connection_timeout(),
            command_timeout_seconds: default_redis_command_timeout(),
            max_connections: default_redis_max_connections(),
        }
    }
}

/// VCS management configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VcsConfig {
    /// Git repository base URL.
    pub git_location: Option<String>,
    /// Bazaar repository base URL.
    pub bzr_location: Option<String>,
    /// Public VCS location for workers.
    pub public_vcs_location: Option<String>,
    /// Enable VCS caching.
    #[serde(default = "default_true")]
    pub enable_caching: bool,
    /// VCS operation timeout in seconds.
    #[serde(default = "default_vcs_timeout")]
    pub operation_timeout_seconds: u64,
    /// Hosts to avoid for VCS operations.
    #[serde(default)]
    pub avoid_hosts: Vec<String>,
}

/// Web server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebConfig {
    /// Listen address for the server.
    #[serde(default = "default_listen_address")]
    pub listen_address: String,
    /// Port for the private API.
    #[serde(default = "default_port")]
    pub port: u16,
    /// Port for the public API.
    #[serde(default = "default_public_port")]
    pub public_port: u16,
    /// Request timeout in seconds.
    #[serde(default = "default_request_timeout")]
    pub request_timeout_seconds: u64,
    /// Maximum request body size in bytes.
    #[serde(default = "default_max_request_size")]
    pub max_request_size_bytes: usize,
    /// Enable request logging.
    #[serde(default = "default_true")]
    pub enable_request_logging: bool,
    /// Enable CORS headers.
    #[serde(default)]
    pub enable_cors: bool,
}

/// Worker coordination configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerConfig {
    /// Run timeout in minutes.
    #[serde(default = "default_run_timeout")]
    pub run_timeout_minutes: u64,
    /// Worker health check interval in seconds.
    #[serde(default = "default_health_check_interval")]
    pub health_check_interval_seconds: u64,
    /// Maximum number of retries for failed runs.
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    /// Rate limiting configuration.
    pub rate_limiting: RateLimitingConfig,
    /// Hosts to avoid for scheduling.
    #[serde(default)]
    pub avoid_hosts: Vec<String>,
}

/// Rate limiting configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitingConfig {
    /// Enable rate limiting.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Default rate limit (requests per hour).
    #[serde(default = "default_rate_limit")]
    pub default_limit: u32,
    /// Host-specific rate limits.
    #[serde(default)]
    pub host_limits: std::collections::HashMap<String, u32>,
    /// Rate limit window in seconds.
    #[serde(default = "default_rate_limit_window")]
    pub window_seconds: u64,
}

/// General application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationConfig {
    /// Application name.
    #[serde(default = "default_app_name")]
    pub name: String,
    /// Application version.
    #[serde(default = "default_app_version")]
    pub version: String,
    /// Environment (development, staging, production).
    #[serde(default = "default_environment")]
    pub environment: String,
    /// Debug mode.
    #[serde(default)]
    pub debug: bool,
    /// Enable graceful shutdown.
    #[serde(default = "default_true")]
    pub enable_graceful_shutdown: bool,
    /// Graceful shutdown timeout in seconds.
    #[serde(default = "default_shutdown_timeout")]
    pub shutdown_timeout_seconds: u64,
    /// Backup directory for when storage systems are unavailable.
    pub backup_directory: Option<PathBuf>,
}

// Default value functions
fn default_db_max_connections() -> u32 { 10 }
fn default_db_connection_timeout() -> u64 { 30 }
fn default_db_query_timeout() -> u64 { 30 }
fn default_redis_url() -> String { "redis://localhost:6379".to_string() }
fn default_redis_connection_timeout() -> u64 { 10 }
fn default_redis_command_timeout() -> u64 { 10 }
fn default_redis_max_connections() -> u32 { 10 }
fn default_true() -> bool { true }
fn default_vcs_timeout() -> u64 { 300 }
fn default_listen_address() -> String { "localhost".to_string() }
fn default_port() -> u16 { 9911 }
fn default_public_port() -> u16 { 9919 }
fn default_request_timeout() -> u64 { 60 }
fn default_max_request_size() -> usize { 10 * 1024 * 1024 } // 10MB
fn default_run_timeout() -> u64 { 60 }
fn default_health_check_interval() -> u64 { 30 }
fn default_max_retries() -> u32 { 3 }
fn default_rate_limit() -> u32 { 100 }
fn default_rate_limit_window() -> u64 { 3600 }
fn default_app_name() -> String { "janitor-runner".to_string() }
fn default_app_version() -> String { env!("CARGO_PKG_VERSION").to_string() }
fn default_environment() -> String { "development".to_string() }
fn default_shutdown_timeout() -> u64 { 30 }

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            database: DatabaseConfig::default(),
            redis: None,
            vcs: VcsConfig::default(),
            logs: LogConfig::default(),
            artifacts: ArtifactConfig::default(),
            performance: PerformanceConfig::default(),
            error_tracking: ErrorTrackingConfig::default(),
            tracing: TracingConfig::default(),
            web: WebConfig::default(),
            worker: WorkerConfig::default(),
            application: ApplicationConfig::default(),
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgresql://localhost/janitor".to_string(),
            max_connections: default_db_max_connections(),
            connection_timeout_seconds: default_db_connection_timeout(),
            query_timeout_seconds: default_db_query_timeout(),
            enable_sql_logging: false,
        }
    }
}

impl Default for VcsConfig {
    fn default() -> Self {
        Self {
            git_location: None,
            bzr_location: None,
            public_vcs_location: None,
            enable_caching: default_true(),
            operation_timeout_seconds: default_vcs_timeout(),
            avoid_hosts: Vec::new(),
        }
    }
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            listen_address: default_listen_address(),
            port: default_port(),
            public_port: default_public_port(),
            request_timeout_seconds: default_request_timeout(),
            max_request_size_bytes: default_max_request_size(),
            enable_request_logging: default_true(),
            enable_cors: false,
        }
    }
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            run_timeout_minutes: default_run_timeout(),
            health_check_interval_seconds: default_health_check_interval(),
            max_retries: default_max_retries(),
            rate_limiting: RateLimitingConfig::default(),
            avoid_hosts: Vec::new(),
        }
    }
}

impl Default for RateLimitingConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            default_limit: default_rate_limit(),
            host_limits: std::collections::HashMap::new(),
            window_seconds: default_rate_limit_window(),
        }
    }
}

impl Default for ApplicationConfig {
    fn default() -> Self {
        Self {
            name: default_app_name(),
            version: default_app_version(),
            environment: default_environment(),
            debug: false,
            enable_graceful_shutdown: default_true(),
            shutdown_timeout_seconds: default_shutdown_timeout(),
            backup_directory: None,
        }
    }
}

/// Configuration loading and validation.
impl RunnerConfig {
    /// Load configuration from a file.
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::Io(e))?;
        
        // Try different formats
        if let Ok(config) = toml::from_str(&content) {
            return Ok(config);
        }
        
        if let Ok(config) = serde_json::from_str(&content) {
            return Ok(config);
        }
        
        if let Ok(config) = serde_yaml::from_str(&content) {
            return Ok(config);
        }
        
        Err(ConfigError::Parse("Unable to parse config file as TOML, JSON, or YAML".to_string()))
    }

    /// Load configuration from environment variables.
    pub fn from_env() -> Result<Self, ConfigError> {
        let mut config = Self::default();
        
        // Database configuration
        if let Ok(url) = std::env::var("DATABASE_URL") {
            config.database.url = url;
        }
        if let Ok(max_conn) = std::env::var("DATABASE_MAX_CONNECTIONS") {
            config.database.max_connections = max_conn.parse()
                .map_err(|e| ConfigError::Parse(format!("Invalid DATABASE_MAX_CONNECTIONS: {}", e)))?;
        }
        
        // Redis configuration
        if let Ok(redis_url) = std::env::var("REDIS_URL") {
            config.redis = Some(RedisConfig {
                url: redis_url,
                ..Default::default()
            });
        }
        
        // Web configuration
        if let Ok(port) = std::env::var("PORT") {
            config.web.port = port.parse()
                .map_err(|e| ConfigError::Parse(format!("Invalid PORT: {}", e)))?;
        }
        if let Ok(addr) = std::env::var("LISTEN_ADDRESS") {
            config.web.listen_address = addr;
        }
        
        // Application configuration
        if let Ok(env) = std::env::var("ENVIRONMENT") {
            config.application.environment = env;
        }
        if let Ok(debug) = std::env::var("DEBUG") {
            config.application.debug = debug.parse()
                .map_err(|e| ConfigError::Parse(format!("Invalid DEBUG: {}", e)))?;
        }
        
        Ok(config)
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate database URL
        if self.database.url.is_empty() {
            return Err(ConfigError::Validation("Database URL cannot be empty".to_string()));
        }
        
        // Validate ports
        if self.web.port == 0 {
            return Err(ConfigError::Validation("Web port cannot be 0".to_string()));
        }
        if self.web.public_port == 0 {
            return Err(ConfigError::Validation("Public port cannot be 0".to_string()));
        }
        if self.web.port == self.web.public_port {
            return Err(ConfigError::Validation("Web port and public port cannot be the same".to_string()));
        }
        
        // Validate timeouts
        if self.database.connection_timeout_seconds == 0 {
            return Err(ConfigError::Validation("Database connection timeout must be greater than 0".to_string()));
        }
        if self.worker.run_timeout_minutes == 0 {
            return Err(ConfigError::Validation("Run timeout must be greater than 0".to_string()));
        }
        
        // Validate directories exist if specified
        if let Some(ref backup_dir) = self.application.backup_directory {
            if !backup_dir.exists() {
                return Err(ConfigError::Validation(format!("Backup directory does not exist: {}", backup_dir.display())));
            }
        }
        
        Ok(())
    }

    /// Convert to the legacy janitor config format for compatibility.
    pub fn to_janitor_config(&self) -> janitor::config::Config {
        let mut janitor_config = janitor::config::Config::default();
        
        // Map VCS configuration
        janitor_config.git_location = self.vcs.git_location.clone();
        janitor_config.bzr_location = self.vcs.bzr_location.clone();
        
        // Map database configuration
        janitor_config.database_location = Some(self.database.url.clone());
        
        // Map other fields as needed
        // Note: This is a simplified mapping. In practice, you might need
        // more sophisticated conversion logic.
        
        janitor_config
    }

    /// Get durations from the configuration.
    pub fn run_timeout(&self) -> Duration {
        Duration::from_secs(self.worker.run_timeout_minutes * 60)
    }

    pub fn health_check_interval(&self) -> Duration {
        Duration::from_secs(self.worker.health_check_interval_seconds)
    }

    pub fn shutdown_timeout(&self) -> Duration {
        Duration::from_secs(self.application.shutdown_timeout_seconds)
    }

    pub fn request_timeout(&self) -> Duration {
        Duration::from_secs(self.web.request_timeout_seconds)
    }
}

/// Configuration error types.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Parse error: {0}")]
    Parse(String),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Environment error: {0}")]
    Environment(String),
}

/// Configuration merging utilities.
impl RunnerConfig {
    /// Merge with another configuration, with the other taking precedence.
    pub fn merge_with(mut self, other: RunnerConfig) -> Self {
        // Database config
        if other.database.url != DatabaseConfig::default().url {
            self.database.url = other.database.url;
        }
        if other.database.max_connections != default_db_max_connections() {
            self.database.max_connections = other.database.max_connections;
        }
        
        // Redis config
        if other.redis.is_some() {
            self.redis = other.redis;
        }
        
        // VCS config
        if other.vcs.git_location.is_some() {
            self.vcs.git_location = other.vcs.git_location;
        }
        if other.vcs.bzr_location.is_some() {
            self.vcs.bzr_location = other.vcs.bzr_location;
        }
        
        // Web config
        if other.web.port != default_port() {
            self.web.port = other.web.port;
        }
        if other.web.listen_address != default_listen_address() {
            self.web.listen_address = other.web.listen_address;
        }
        
        // Application config
        if other.application.environment != default_environment() {
            self.application.environment = other.application.environment;
        }
        if other.application.debug {
            self.application.debug = other.application.debug;
        }
        
        // Merge subsystem configs
        self.logs = other.logs;
        self.artifacts = other.artifacts;
        self.performance = other.performance;
        self.error_tracking = other.error_tracking;
        
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = RunnerConfig::default();
        assert_eq!(config.database.url, "postgresql://localhost/janitor");
        assert_eq!(config.web.port, 9911);
        assert_eq!(config.web.public_port, 9919);
        assert_eq!(config.application.name, "janitor-runner");
    }

    #[test]
    fn test_config_validation() {
        let config = RunnerConfig::default();
        assert!(config.validate().is_ok());

        let mut invalid_config = config.clone();
        invalid_config.database.url = "".to_string();
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_config_merge() {
        let mut base_config = RunnerConfig::default();
        let mut override_config = RunnerConfig::default();
        override_config.web.port = 8080;
        override_config.application.debug = true;

        let merged = base_config.merge_with(override_config);
        assert_eq!(merged.web.port, 8080);
        assert_eq!(merged.application.debug, true);
        assert_eq!(merged.database.url, "postgresql://localhost/janitor"); // Should retain base value
    }

    #[test]
    fn test_duration_conversion() {
        let config = RunnerConfig::default();
        assert_eq!(config.run_timeout(), Duration::from_secs(60 * 60)); // 60 minutes
        assert_eq!(config.health_check_interval(), Duration::from_secs(30));
    }
}