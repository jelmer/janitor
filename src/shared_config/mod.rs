//! Shared configuration module for Janitor services
//!
//! This module provides common configuration patterns used across multiple
//! Janitor services to eliminate duplication and ensure consistency.

pub mod database;
pub mod env;
pub mod external;
pub mod logging;
pub mod redis;
pub mod validation;
pub mod web;

pub use database::DatabaseConfig;
pub use env::{EnvParser, FromEnv};
pub use external::{ExternalService, ExternalServiceConfig};
pub use logging::{init_logging, LoggingConfig};
pub use redis::RedisConfig;
pub use validation::{ConfigError, ValidationError};
pub use web::WebConfig;

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
