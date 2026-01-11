//! Shared configuration module for Janitor services
//!
//! This module provides common configuration patterns used across multiple
//! Janitor services to eliminate duplication and ensure consistency.

pub mod database;
pub mod env;
pub mod external;
pub mod http;
pub mod logging;
pub mod parsing;
pub mod redis;
pub mod validation;
pub mod web;

pub use database::{
    create_database_manager_from_env, create_pool_legacy, DatabaseConfig, DatabaseManager,
};
pub use env::{EnvParser, FromEnv};
pub use external::{ExternalService, ExternalServiceConfig};
pub use http::{
    http_client_factory_from_env, HttpClientConfig, HttpClientFactory, HttpCredentials,
};
pub use logging::{init_logging, init_logging_from_simple_args, FileOutputConfig, LoggingConfig};
pub use redis::RedisConfig;
pub use validation::{ConfigError, ValidationError};
pub use web::WebConfig;

/// Base configuration that all service configs should include
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ServiceConfig {
    /// Database configuration (optional for services that don't need database)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database: Option<DatabaseConfig>,

    /// Redis configuration (optional for services that don't need Redis)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redis: Option<RedisConfig>,

    /// Web server configuration (optional for non-web services)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web: Option<WebConfig>,

    /// HTTP client configuration
    #[serde(default)]
    pub http_client: HttpClientConfig,

    /// Logging configuration
    #[serde(default)]
    pub logging: LoggingConfig,

    /// External service URLs
    #[serde(default)]
    pub external_services: ExternalServiceConfig,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            database: None,
            redis: None,
            web: None,
            http_client: HttpClientConfig::default(),
            logging: LoggingConfig::default(),
            external_services: ExternalServiceConfig::default(),
        }
    }
}

impl ServiceConfig {
    /// Create a new ServiceConfig with database
    pub fn with_database(mut self, database_url: String) -> Self {
        self.database = Some(DatabaseConfig {
            url: database_url,
            ..DatabaseConfig::default()
        });
        self
    }

    /// Create a new ServiceConfig with redis
    pub fn with_redis(mut self, redis_url: String) -> Self {
        self.redis = Some(RedisConfig {
            url: redis_url,
            ..RedisConfig::default()
        });
        self
    }

    /// Create a new ServiceConfig with web server
    pub fn with_web(mut self, listen_address: String, port: u16) -> Self {
        let mut web = WebConfig::default();
        web.listen_address = listen_address;
        web.port = port;
        self.web = Some(web);
        self
    }

    /// Merge with another ServiceConfig, giving precedence to other
    pub fn merge(self, other: Self) -> Self {
        Self {
            database: other.database.or(self.database),
            redis: other.redis.or(self.redis),
            web: other.web.or(self.web),
            http_client: other.http_client,
            logging: other.logging,
            external_services: other.external_services,
        }
    }
}

impl FromEnv for ServiceConfig {
    fn from_env() -> Result<Self, ConfigError> {
        Self::from_env_with_prefix("")
    }

    fn from_env_with_prefix(prefix: &str) -> Result<Self, ConfigError> {
        let parser = EnvParser::with_prefix(prefix);

        // Load database config if DATABASE_URL is set
        let database = if parser.get_string("DATABASE_URL").is_some() {
            Some(DatabaseConfig::from_env_with_prefix(prefix)?)
        } else {
            None
        };

        // Load redis config if REDIS_URL is set
        let redis = if parser.get_string("REDIS_URL").is_some() {
            Some(RedisConfig::from_env_with_prefix(prefix)?)
        } else {
            None
        };

        // Load web config if web settings are present
        let web = if parser.get_string("WEB_LISTEN_ADDRESS").is_some()
            || parser.get_u16("WEB_PORT").is_ok()
        {
            Some(WebConfig::from_env_with_prefix(prefix)?)
        } else {
            None
        };

        // Always load logging config
        let logging = LoggingConfig::from_env_with_prefix(prefix)?;

        // Always load external services config
        let external_services = ExternalServiceConfig::from_env_with_prefix(prefix)?;

        // Always load HTTP client config
        let http_client = HttpClientConfig {
            request_timeout: parser.get_u64("HTTP_REQUEST_TIMEOUT")?.unwrap_or(30),
            connect_timeout: parser.get_u64("HTTP_CONNECT_TIMEOUT")?.unwrap_or(10),
            max_redirects: parser.get_u32("HTTP_MAX_REDIRECTS")?.unwrap_or(10),
            http2_prior_knowledge: parser
                .get_bool("HTTP_HTTP2_PRIOR_KNOWLEDGE")?
                .unwrap_or(true),
            compression: parser.get_bool("HTTP_COMPRESSION")?.unwrap_or(true),
        };

        Ok(Self {
            database,
            redis,
            web,
            http_client,
            logging,
            external_services,
        })
    }
}

/// Common configuration loader trait for all services
pub trait ConfigLoader: Sized {
    /// Load configuration from a file (auto-detects format from extension)
    fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self, ConfigError>;

    /// Load configuration from environment variables
    fn from_env() -> Result<Self, ConfigError>;

    /// Load configuration from multiple sources with precedence
    fn from_sources(sources: &[ConfigSource]) -> Result<Self, ConfigError>;

    /// Validate the configuration
    fn validate(&self) -> Result<(), ValidationError>;
}

/// Configuration source for loading configs from multiple places
#[derive(Debug, Clone)]
pub enum ConfigSource {
    File(std::path::PathBuf),
    Environment,
    Defaults,
}

/// Trait for configurations that can be merged together
pub trait Mergeable {
    /// Merge this configuration with another, giving precedence to `other`
    fn merge_with(self, other: Self) -> Self;
}

/// Common default value functions used across services
pub mod defaults {
    /// Default database maximum connections
    pub fn default_db_max_connections() -> u32 {
        10
    }

    /// Default database connection timeout in seconds
    pub fn default_db_connection_timeout() -> u64 {
        30
    }

    /// Default database query timeout in seconds
    pub fn default_db_query_timeout() -> u64 {
        60
    }

    /// Default listen address for web servers
    pub fn default_listen_address() -> String {
        "localhost".to_string()
    }

    /// Default port for web servers
    pub fn default_port() -> u16 {
        8080
    }

    /// Default request timeout in seconds
    pub fn default_request_timeout() -> u64 {
        30
    }

    /// Default maximum request size in bytes
    pub fn default_max_request_size() -> usize {
        16 * 1024 * 1024 // 16MB
    }

    /// Default log level
    pub fn default_log_level() -> String {
        "info".to_string()
    }

    /// Default true value for boolean flags
    pub fn default_true() -> bool {
        true
    }

    /// Default false value for boolean flags
    pub fn default_false() -> bool {
        false
    }
}
