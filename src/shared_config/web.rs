//! Shared web server configuration for Janitor services

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::Duration;

use crate::shared_config::{defaults::*, env::EnvParser, ConfigError, FromEnv, ValidationError};

/// Web server configuration used across Janitor services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebConfig {
    /// Address to bind the web server to
    #[serde(default = "default_listen_address")]
    pub listen_address: String,

    /// Port to bind the web server to
    #[serde(default = "default_port")]
    pub port: u16,

    /// Public port for external access (if different from bind port)
    pub public_port: Option<u16>,

    /// Request timeout in seconds
    #[serde(default = "default_request_timeout")]
    pub request_timeout_seconds: u64,

    /// Maximum request size in bytes
    #[serde(default = "default_max_request_size")]
    pub max_request_size_bytes: usize,

    /// Enable request logging
    #[serde(default = "default_true")]
    pub enable_request_logging: bool,

    /// Enable CORS headers
    #[serde(default = "default_false")]
    pub enable_cors: bool,

    /// Number of worker threads
    pub workers: Option<usize>,

    /// Keep-alive timeout in seconds
    pub keep_alive_seconds: Option<u64>,

    /// Enable compression
    #[serde(default = "default_true")]
    pub enable_compression: bool,

    /// Enable HTTP/2
    #[serde(default = "default_true")]
    pub enable_http2: bool,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            listen_address: default_listen_address(),
            port: default_port(),
            public_port: None,
            request_timeout_seconds: default_request_timeout(),
            max_request_size_bytes: default_max_request_size(),
            enable_request_logging: default_true(),
            enable_cors: default_false(),
            workers: None,
            keep_alive_seconds: None,
            enable_compression: default_true(),
            enable_http2: default_true(),
        }
    }
}

impl FromEnv for WebConfig {
    fn from_env() -> Result<Self, ConfigError> {
        Self::from_env_with_prefix("")
    }

    fn from_env_with_prefix(prefix: &str) -> Result<Self, ConfigError> {
        let parser = EnvParser::with_prefix(prefix);

        Ok(Self {
            listen_address: parser
                .get_string("LISTEN_ADDRESS")
                .unwrap_or_else(default_listen_address),
            port: parser.get_u16("PORT")?.unwrap_or_else(default_port),
            public_port: parser.get_u16("PUBLIC_PORT")?,
            request_timeout_seconds: parser
                .get_u64("REQUEST_TIMEOUT_SECONDS")?
                .unwrap_or_else(default_request_timeout),
            max_request_size_bytes: parser
                .get_usize("MAX_REQUEST_SIZE_BYTES")?
                .unwrap_or_else(default_max_request_size),
            enable_request_logging: parser
                .get_bool("ENABLE_REQUEST_LOGGING")?
                .unwrap_or_else(default_true),
            enable_cors: parser
                .get_bool("ENABLE_CORS")?
                .unwrap_or_else(default_false),
            workers: parser.get_usize("WORKERS")?,
            keep_alive_seconds: parser.get_u64("KEEP_ALIVE_SECONDS")?,
            enable_compression: parser
                .get_bool("ENABLE_COMPRESSION")?
                .unwrap_or_else(default_true),
            enable_http2: parser
                .get_bool("ENABLE_HTTP2")?
                .unwrap_or_else(default_true),
        })
    }
}

impl WebConfig {
    /// Validate the web configuration
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.listen_address.is_empty() {
            return Err(ValidationError::InvalidValue {
                field: "listen_address".to_string(),
                message: "Listen address cannot be empty".to_string(),
            });
        }

        if self.port == 0 {
            return Err(ValidationError::InvalidValue {
                field: "port".to_string(),
                message: "Port must be greater than 0".to_string(),
            });
        }

        if let Some(public_port) = self.public_port {
            if public_port == 0 {
                return Err(ValidationError::InvalidValue {
                    field: "public_port".to_string(),
                    message: "Public port must be greater than 0".to_string(),
                });
            }
        }

        if let Some(workers) = self.workers {
            if workers == 0 {
                return Err(ValidationError::InvalidValue {
                    field: "workers".to_string(),
                    message: "Number of workers must be greater than 0".to_string(),
                });
            }
        }

        Ok(())
    }

    /// Get the socket address for binding
    pub fn socket_addr(&self) -> Result<SocketAddr, ConfigError> {
        let addr = format!("{}:{}", self.listen_address, self.port);
        addr.parse().map_err(|e| ConfigError::ParseError {
            field: "listen_address:port".to_string(),
            message: format!("Invalid socket address '{}': {}", addr, e),
        })
    }

    /// Get the public port (falls back to main port if not specified)
    pub fn public_port(&self) -> u16 {
        self.public_port.unwrap_or(self.port)
    }

    /// Get request timeout as Duration
    pub fn request_timeout(&self) -> Duration {
        Duration::from_secs(self.request_timeout_seconds)
    }

    /// Get keep-alive timeout as Duration
    pub fn keep_alive_timeout(&self) -> Option<Duration> {
        self.keep_alive_seconds.map(Duration::from_secs)
    }
}

/// CORS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    /// Allowed origins (empty means all origins)
    pub allowed_origins: Vec<String>,

    /// Allowed methods
    pub allowed_methods: Vec<String>,

    /// Allowed headers
    pub allowed_headers: Vec<String>,

    /// Maximum age for preflight requests in seconds
    pub max_age_seconds: Option<u64>,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: vec!["*".to_string()],
            allowed_methods: vec![
                "GET".to_string(),
                "POST".to_string(),
                "PUT".to_string(),
                "DELETE".to_string(),
                "OPTIONS".to_string(),
            ],
            allowed_headers: vec![
                "Content-Type".to_string(),
                "Authorization".to_string(),
                "Accept".to_string(),
            ],
            max_age_seconds: Some(3600), // 1 hour
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_config_validation() {
        let mut config = WebConfig::default();
        assert!(config.validate().is_ok());

        // Test empty listen address
        config.listen_address = "".to_string();
        assert!(config.validate().is_err());

        // Test zero port
        config.listen_address = "localhost".to_string();
        config.port = 0;
        assert!(config.validate().is_err());

        // Test zero workers
        config.port = 8080;
        config.workers = Some(0);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_web_config_socket_addr() {
        let config = WebConfig {
            listen_address: "127.0.0.1".to_string(),
            port: 8080,
            ..WebConfig::default()
        };

        let addr = config.socket_addr().unwrap();
        assert_eq!(addr.to_string(), "127.0.0.1:8080");
    }

    #[test]
    fn test_web_config_public_port() {
        let mut config = WebConfig::default();
        config.port = 8080;

        // Should fall back to main port
        assert_eq!(config.public_port(), 8080);

        // Should use specified public port
        config.public_port = Some(443);
        assert_eq!(config.public_port(), 443);
    }
}
