//! Comprehensive configuration management for the runner.

use crate::{
    artifacts::ArtifactConfig, error_tracking::ErrorTrackingConfig, performance::PerformanceConfig,
    tracing::TracingConfig,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Log management configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LogConfig {
    /// Storage backend type (filesystem, gcs).
    #[serde(default = "default_log_backend")]
    pub backend: String,
    /// Base path for filesystem logs.
    pub filesystem_base_path: Option<PathBuf>,
    /// GCS bucket for logs.
    pub gcs_bucket: Option<String>,
    /// Log retention days.
    #[serde(default = "default_log_retention_days")]
    pub retention_days: u32,
    /// Enable log compression.
    #[serde(default = "default_true")]
    pub enable_compression: bool,
}

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
    /// Security configuration.
    #[serde(default)]
    pub security: crate::auth::SecurityConfig,
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

/// Configuration source types for priority-based loading.
#[derive(Debug, Clone)]
pub enum ConfigSource {
    /// Load from a configuration file.
    File(PathBuf),
    /// Load from environment variables.
    Environment,
    /// Use default values.
    Defaults,
}

/// Configuration manager with hot-reloading support.
pub struct ConfigManager {
    config: Arc<RwLock<RunnerConfig>>,
    file_path: Option<PathBuf>,
    reload_enabled: bool,
}

impl ConfigManager {
    /// Create a new configuration manager.
    pub fn new(config: RunnerConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            file_path: None,
            reload_enabled: false,
        }
    }

    /// Create a configuration manager from a file with hot-reloading.
    pub async fn from_file<P: AsRef<Path>>(
        path: P,
        enable_reload: bool,
    ) -> Result<Self, ConfigError> {
        let path = path.as_ref().to_path_buf();
        let config = RunnerConfig::from_file_with_env(&path)?;

        let mut manager = Self {
            config: Arc::new(RwLock::new(config)),
            file_path: Some(path),
            reload_enabled: enable_reload,
        };

        if enable_reload {
            manager.start_file_watcher().await?;
        }

        Ok(manager)
    }

    /// Get the current configuration.
    pub async fn get(&self) -> RunnerConfig {
        self.config.read().await.clone()
    }

    /// Update the configuration.
    pub async fn update(&self, new_config: RunnerConfig) -> Result<(), ConfigError> {
        new_config.validate()?;
        *self.config.write().await = new_config;
        Ok(())
    }

    /// Reload configuration from file.
    pub async fn reload(&self) -> Result<(), ConfigError> {
        if let Some(ref path) = self.file_path {
            let new_config = RunnerConfig::from_file_with_env(path)?;
            self.update(new_config).await?;
            log::info!("Configuration reloaded from {}", path.display());
        }
        Ok(())
    }

    /// Start file watcher for hot-reloading (simplified version).
    async fn start_file_watcher(&mut self) -> Result<(), ConfigError> {
        if let Some(ref path) = self.file_path {
            let config_manager = self.config.clone();
            let watch_path = path.clone();

            tokio::spawn(async move {
                // This is a simplified implementation
                // In a full implementation, you would use a file watcher like `notify`
                let mut interval = tokio::time::interval(Duration::from_secs(10));
                let mut last_modified = std::fs::metadata(&watch_path)
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

                loop {
                    interval.tick().await;

                    if let Ok(metadata) = std::fs::metadata(&watch_path) {
                        if let Ok(modified) = metadata.modified() {
                            if modified > last_modified {
                                last_modified = modified;

                                match RunnerConfig::from_file_with_env(&watch_path) {
                                    Ok(new_config) => {
                                        if new_config.validate().is_ok() {
                                            *config_manager.write().await = new_config;
                                            log::info!(
                                                "Configuration hot-reloaded from {}",
                                                watch_path.display()
                                            );
                                        } else {
                                            log::error!("Invalid configuration in {}, keeping current config", watch_path.display());
                                        }
                                    }
                                    Err(e) => {
                                        log::error!(
                                            "Failed to reload configuration from {}: {}",
                                            watch_path.display(),
                                            e
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            });
        }
        Ok(())
    }
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
    /// Upload storage directory for temporary file uploads.
    #[serde(default = "default_upload_storage_dir")]
    pub upload_storage_dir: PathBuf,
}

// Default value functions
fn default_db_max_connections() -> u32 {
    10
}
fn default_db_connection_timeout() -> u64 {
    30
}
fn default_db_query_timeout() -> u64 {
    30
}
fn default_redis_url() -> String {
    "redis://localhost:6379".to_string()
}
fn default_redis_connection_timeout() -> u64 {
    10
}
fn default_redis_command_timeout() -> u64 {
    10
}
fn default_redis_max_connections() -> u32 {
    10
}
fn default_true() -> bool {
    true
}
fn default_vcs_timeout() -> u64 {
    300
}
fn default_listen_address() -> String {
    "localhost".to_string()
}
fn default_port() -> u16 {
    9911
}
fn default_public_port() -> u16 {
    9919
}
fn default_request_timeout() -> u64 {
    60
}
fn default_max_request_size() -> usize {
    10 * 1024 * 1024
} // 10MB
fn default_run_timeout() -> u64 {
    60
}
fn default_health_check_interval() -> u64 {
    30
}
fn default_log_backend() -> String {
    "filesystem".to_string()
}
fn default_log_retention_days() -> u32 {
    30
}
fn default_max_retries() -> u32 {
    3
}
fn default_rate_limit() -> u32 {
    100
}
fn default_rate_limit_window() -> u64 {
    3600
}
fn default_app_name() -> String {
    "janitor-runner".to_string()
}
fn default_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
fn default_environment() -> String {
    "development".to_string()
}
fn default_shutdown_timeout() -> u64 {
    30
}
fn default_upload_storage_dir() -> PathBuf {
    PathBuf::from("/tmp/janitor-uploads")
}

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
            security: crate::auth::SecurityConfig::default(),
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
            upload_storage_dir: default_upload_storage_dir(),
        }
    }
}

/// Configuration loading and validation.
impl RunnerConfig {
    /// Load configuration from a file with enhanced error reporting.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|e| ConfigError::Io(e))?;

        Self::from_content(&content, path)
    }

    /// Load configuration from file content with format detection.
    pub fn from_content(content: &str, path: &Path) -> Result<Self, ConfigError> {
        let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

        // Try format based on file extension first
        let config = match extension {
            "toml" => toml::from_str(content)
                .map_err(|e| ConfigError::Parse(format!("TOML parse error: {}", e)))?,
            "json" => serde_json::from_str(content)
                .map_err(|e| ConfigError::Parse(format!("JSON parse error: {}", e)))?,
            "yaml" | "yml" => serde_yaml::from_str(content)
                .map_err(|e| ConfigError::Parse(format!("YAML parse error: {}", e)))?,
            _ => {
                // Try different formats when extension is unknown
                if let Ok(config) = toml::from_str(content) {
                    config
                } else if let Ok(config) = serde_json::from_str(content) {
                    config
                } else if let Ok(config) = serde_yaml::from_str(content) {
                    config
                } else {
                    return Err(ConfigError::Parse(
                        "Unable to parse config file as TOML, JSON, or YAML".to_string(),
                    ));
                }
            }
        };

        Ok(config)
    }

    /// Load configuration with environment variable overrides.
    pub fn from_file_with_env<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let mut config = Self::from_file(path)?;
        config.apply_env_overrides()?;
        Ok(config)
    }

    /// Load configuration from multiple sources with priority.
    pub fn from_sources(sources: &[ConfigSource]) -> Result<Self, ConfigError> {
        let mut config = Self::default();

        for source in sources {
            let source_config = match source {
                ConfigSource::File(path) => Self::from_file(path)?,
                ConfigSource::Environment => Self::from_env()?,
                ConfigSource::Defaults => Self::default(),
            };
            config = config.merge_with(source_config);
        }

        config.validate()?;
        Ok(config)
    }

    /// Load configuration from environment variables.
    pub fn from_env() -> Result<Self, ConfigError> {
        let mut config = Self::default();
        config.apply_env_overrides()?;
        Ok(config)
    }

    /// Apply environment variable overrides to existing configuration.
    pub fn apply_env_overrides(&mut self) -> Result<(), ConfigError> {
        // Database configuration
        if let Ok(url) = std::env::var("DATABASE_URL") {
            self.database.url = url;
        }
        if let Ok(max_conn) = std::env::var("DATABASE_MAX_CONNECTIONS") {
            self.database.max_connections = max_conn.parse().map_err(|e| {
                ConfigError::Environment(format!("Invalid DATABASE_MAX_CONNECTIONS: {}", e))
            })?;
        }
        if let Ok(timeout) = std::env::var("DATABASE_CONNECTION_TIMEOUT") {
            self.database.connection_timeout_seconds = timeout.parse().map_err(|e| {
                ConfigError::Environment(format!("Invalid DATABASE_CONNECTION_TIMEOUT: {}", e))
            })?;
        }
        if let Ok(sql_logging) = std::env::var("DATABASE_ENABLE_SQL_LOGGING") {
            self.database.enable_sql_logging = sql_logging.parse().map_err(|e| {
                ConfigError::Environment(format!("Invalid DATABASE_ENABLE_SQL_LOGGING: {}", e))
            })?;
        }

        // Redis configuration
        if let Ok(redis_url) = std::env::var("REDIS_URL") {
            self.redis = Some(RedisConfig {
                url: redis_url,
                ..self.redis.clone().unwrap_or_default()
            });
        }
        if let Ok(redis_timeout) = std::env::var("REDIS_CONNECTION_TIMEOUT") {
            if let Some(ref mut redis) = self.redis {
                redis.connection_timeout_seconds = redis_timeout.parse().map_err(|e| {
                    ConfigError::Environment(format!("Invalid REDIS_CONNECTION_TIMEOUT: {}", e))
                })?;
            }
        }

        // VCS configuration
        if let Ok(git_location) = std::env::var("GIT_LOCATION") {
            self.vcs.git_location = Some(git_location);
        }
        if let Ok(bzr_location) = std::env::var("BZR_LOCATION") {
            self.vcs.bzr_location = Some(bzr_location);
        }
        if let Ok(public_vcs) = std::env::var("PUBLIC_VCS_LOCATION") {
            self.vcs.public_vcs_location = Some(public_vcs);
        }
        if let Ok(avoid_hosts) = std::env::var("VCS_AVOID_HOSTS") {
            self.vcs.avoid_hosts = avoid_hosts
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
        }

        // Web configuration
        if let Ok(port) = std::env::var("PORT") {
            self.web.port = port
                .parse()
                .map_err(|e| ConfigError::Environment(format!("Invalid PORT: {}", e)))?;
        }
        if let Ok(public_port) = std::env::var("PUBLIC_PORT") {
            self.web.public_port = public_port
                .parse()
                .map_err(|e| ConfigError::Environment(format!("Invalid PUBLIC_PORT: {}", e)))?;
        }
        if let Ok(addr) = std::env::var("LISTEN_ADDRESS") {
            self.web.listen_address = addr;
        }
        if let Ok(cors) = std::env::var("ENABLE_CORS") {
            self.web.enable_cors = cors
                .parse()
                .map_err(|e| ConfigError::Environment(format!("Invalid ENABLE_CORS: {}", e)))?;
        }

        // Worker configuration
        if let Ok(timeout) = std::env::var("RUN_TIMEOUT_MINUTES") {
            self.worker.run_timeout_minutes = timeout.parse().map_err(|e| {
                ConfigError::Environment(format!("Invalid RUN_TIMEOUT_MINUTES: {}", e))
            })?;
        }
        if let Ok(interval) = std::env::var("HEALTH_CHECK_INTERVAL") {
            self.worker.health_check_interval_seconds = interval.parse().map_err(|e| {
                ConfigError::Environment(format!("Invalid HEALTH_CHECK_INTERVAL: {}", e))
            })?;
        }
        if let Ok(retries) = std::env::var("MAX_RETRIES") {
            self.worker.max_retries = retries
                .parse()
                .map_err(|e| ConfigError::Environment(format!("Invalid MAX_RETRIES: {}", e)))?;
        }
        if let Ok(avoid_hosts) = std::env::var("WORKER_AVOID_HOSTS") {
            self.worker.avoid_hosts = avoid_hosts
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
        }

        // Rate limiting configuration
        if let Ok(enabled) = std::env::var("RATE_LIMITING_ENABLED") {
            self.worker.rate_limiting.enabled = enabled.parse().map_err(|e| {
                ConfigError::Environment(format!("Invalid RATE_LIMITING_ENABLED: {}", e))
            })?;
        }
        if let Ok(limit) = std::env::var("RATE_LIMITING_DEFAULT_LIMIT") {
            self.worker.rate_limiting.default_limit = limit.parse().map_err(|e| {
                ConfigError::Environment(format!("Invalid RATE_LIMITING_DEFAULT_LIMIT: {}", e))
            })?;
        }

        // Application configuration
        if let Ok(env) = std::env::var("ENVIRONMENT") {
            self.application.environment = env;
        }
        if let Ok(debug) = std::env::var("DEBUG") {
            self.application.debug = debug
                .parse()
                .map_err(|e| ConfigError::Environment(format!("Invalid DEBUG: {}", e)))?;
        }
        if let Ok(app_name) = std::env::var("APP_NAME") {
            self.application.name = app_name;
        }
        if let Ok(backup_dir) = std::env::var("BACKUP_DIRECTORY") {
            self.application.backup_directory = Some(PathBuf::from(backup_dir));
        }

        Ok(())
    }

    /// Validate the configuration with comprehensive checks.
    pub fn validate(&self) -> Result<(), ConfigError> {
        let mut errors = Vec::new();

        // Validate database configuration
        if self.database.url.is_empty() {
            errors.push("Database URL cannot be empty".to_string());
        }
        if !self.database.url.starts_with("postgresql://")
            && !self.database.url.starts_with("postgres://")
        {
            errors.push("Database URL must be a PostgreSQL connection string".to_string());
        }
        if self.database.max_connections == 0 {
            errors.push("Database max connections must be greater than 0".to_string());
        }
        if self.database.connection_timeout_seconds == 0 {
            errors.push("Database connection timeout must be greater than 0".to_string());
        }
        if self.database.query_timeout_seconds == 0 {
            errors.push("Database query timeout must be greater than 0".to_string());
        }

        // Validate Redis configuration if present
        if let Some(ref redis) = self.redis {
            if redis.url.is_empty() {
                errors.push("Redis URL cannot be empty when Redis is configured".to_string());
            }
            if !redis.url.starts_with("redis://") && !redis.url.starts_with("rediss://") {
                errors.push("Redis URL must be a valid Redis connection string".to_string());
            }
            if redis.connection_timeout_seconds == 0 {
                errors.push("Redis connection timeout must be greater than 0".to_string());
            }
            if redis.max_connections == 0 {
                errors.push("Redis max connections must be greater than 0".to_string());
            }
        }

        // Validate web configuration
        if self.web.port == 0 {
            errors.push("Web port cannot be 0".to_string());
        }
        if self.web.public_port == 0 {
            errors.push("Public port cannot be 0".to_string());
        }
        if self.web.port == self.web.public_port {
            errors.push("Web port and public port cannot be the same".to_string());
        }
        if self.web.port > 65535 || self.web.public_port > 65535 {
            errors.push("Ports must be in the range 1-65535".to_string());
        }
        if self.web.request_timeout_seconds == 0 {
            errors.push("Web request timeout must be greater than 0".to_string());
        }
        if self.web.max_request_size_bytes == 0 {
            errors.push("Web max request size must be greater than 0".to_string());
        }

        // Validate worker configuration
        if self.worker.run_timeout_minutes == 0 {
            errors.push("Run timeout must be greater than 0".to_string());
        }
        if self.worker.health_check_interval_seconds == 0 {
            errors.push("Health check interval must be greater than 0".to_string());
        }
        if self.worker.rate_limiting.window_seconds == 0 {
            errors.push("Rate limiting window must be greater than 0".to_string());
        }

        // Validate VCS configuration
        if self.vcs.operation_timeout_seconds == 0 {
            errors.push("VCS operation timeout must be greater than 0".to_string());
        }

        // Validate application configuration
        if self.application.name.is_empty() {
            errors.push("Application name cannot be empty".to_string());
        }
        if self.application.shutdown_timeout_seconds == 0 {
            errors.push("Shutdown timeout must be greater than 0".to_string());
        }

        // Validate directories exist if specified
        if let Some(ref backup_dir) = self.application.backup_directory {
            if !backup_dir.exists() {
                errors.push(format!(
                    "Backup directory does not exist: {}",
                    backup_dir.display()
                ));
            } else if !backup_dir.is_dir() {
                errors.push(format!(
                    "Backup directory path is not a directory: {}",
                    backup_dir.display()
                ));
            }
        }

        // Check for reasonable timeout values
        if self.worker.run_timeout_minutes > 1440 {
            // 24 hours
            errors.push("Run timeout seems unreasonably high (>24 hours)".to_string());
        }
        if self.database.connection_timeout_seconds > 300 {
            // 5 minutes
            errors.push(
                "Database connection timeout seems unreasonably high (>5 minutes)".to_string(),
            );
        }

        if !errors.is_empty() {
            return Err(ConfigError::Validation(format!(
                "Configuration validation failed:\n- {}",
                errors.join("\n- ")
            )));
        }

        Ok(())
    }

    /// Load configuration for a specific profile (development, staging, production).
    pub fn for_profile(profile: &str) -> Result<Self, ConfigError> {
        let mut config = Self::default();

        // Apply profile-specific defaults
        match profile {
            "development" => {
                config.application.environment = "development".to_string();
                config.application.debug = true;
                config.database.enable_sql_logging = true;
                config.tracing.log_level = "debug".to_string();
                config.web.enable_request_logging = true;
            }
            "staging" => {
                config.application.environment = "staging".to_string();
                config.application.debug = false;
                config.database.enable_sql_logging = false;
                config.tracing.log_level = "info".to_string();
                config.web.enable_request_logging = true;
            }
            "production" => {
                config.application.environment = "production".to_string();
                config.application.debug = false;
                config.database.enable_sql_logging = false;
                config.tracing.log_level = "warn".to_string();
                config.web.enable_request_logging = false;
                config.worker.rate_limiting.enabled = true;
            }
            _ => {
                return Err(ConfigError::Validation(format!(
                    "Unknown profile: {}",
                    profile
                )));
            }
        }

        // Try to load profile-specific config file
        let profile_config_path = format!("janitor.{}.conf", profile);
        if std::path::Path::new(&profile_config_path).exists() {
            let profile_config = Self::from_file(&profile_config_path)?;
            config = config.merge_with(profile_config);
        }

        // Apply environment overrides
        config.apply_env_overrides()?;

        config.validate()?;
        Ok(config)
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

    /// Get health check interval as a Duration.
    pub fn health_check_interval(&self) -> Duration {
        Duration::from_secs(self.worker.health_check_interval_seconds)
    }

    /// Get shutdown timeout as a Duration.
    pub fn shutdown_timeout(&self) -> Duration {
        Duration::from_secs(self.application.shutdown_timeout_seconds)
    }

    /// Get request timeout as a Duration.
    pub fn request_timeout(&self) -> Duration {
        Duration::from_secs(self.web.request_timeout_seconds)
    }
}

/// Configuration error types.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// IO error reading configuration files.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Error parsing configuration content.
    #[error("Parse error: {0}")]
    Parse(String),

    /// Configuration validation error.
    #[error("Validation error: {0}")]
    Validation(String),

    /// Environment variable parsing error.
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

    #[test]
    fn test_enhanced_validation() {
        let mut config = RunnerConfig::default();
        assert!(config.validate().is_ok());

        // Test invalid database URL
        config.database.url = "invalid-url".to_string();
        assert!(config.validate().is_err());

        // Reset and test invalid ports
        config = RunnerConfig::default();
        config.web.port = 0;
        assert!(config.validate().is_err());

        // Test port conflict
        config = RunnerConfig::default();
        config.web.port = 8080;
        config.web.public_port = 8080;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_env_overrides() {
        std::env::set_var("DATABASE_URL", "postgresql://test:5432/test");
        std::env::set_var("PORT", "8080");
        std::env::set_var("DEBUG", "true");

        let mut config = RunnerConfig::default();
        config.apply_env_overrides().unwrap();

        assert_eq!(config.database.url, "postgresql://test:5432/test");
        assert_eq!(config.web.port, 8080);
        assert_eq!(config.application.debug, true);

        // Cleanup
        std::env::remove_var("DATABASE_URL");
        std::env::remove_var("PORT");
        std::env::remove_var("DEBUG");
    }

    #[test]
    fn test_profile_loading() {
        let dev_config = RunnerConfig::for_profile("development").unwrap();
        assert_eq!(dev_config.application.environment, "development");
        assert_eq!(dev_config.application.debug, true);

        let prod_config = RunnerConfig::for_profile("production").unwrap();
        assert_eq!(prod_config.application.environment, "production");
        assert_eq!(prod_config.application.debug, false);
        assert_eq!(prod_config.worker.rate_limiting.enabled, true);

        // Test invalid profile
        assert!(RunnerConfig::for_profile("invalid").is_err());
    }

    #[test]
    fn test_config_sources() {
        let sources = vec![ConfigSource::Defaults, ConfigSource::Environment];

        // This should not fail even though we don't have actual files
        std::env::set_var("DATABASE_URL", "postgresql://localhost/test");
        let config = RunnerConfig::from_sources(&sources).unwrap();
        assert_eq!(config.database.url, "postgresql://localhost/test");
        std::env::remove_var("DATABASE_URL");
    }

    #[test]
    fn test_config_content_parsing() {
        let toml_content = r#"
[database]
url = "postgresql://localhost/janitor_test"
max_connections = 5

[web]
port = 8080
"#;

        let config =
            RunnerConfig::from_content(toml_content, std::path::Path::new("test.toml")).unwrap();
        assert_eq!(config.database.url, "postgresql://localhost/janitor_test");
        assert_eq!(config.database.max_connections, 5);
        assert_eq!(config.web.port, 8080);
    }

    #[tokio::test]
    async fn test_config_manager() {
        let config = RunnerConfig::default();
        let manager = ConfigManager::new(config.clone());

        let retrieved_config = manager.get().await;
        assert_eq!(retrieved_config.database.url, config.database.url);

        // Test update
        let mut new_config = config.clone();
        new_config.web.port = 9000;
        manager.update(new_config).await.unwrap();

        let updated_config = manager.get().await;
        assert_eq!(updated_config.web.port, 9000);
    }
}
