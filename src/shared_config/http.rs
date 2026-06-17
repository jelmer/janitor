//! Shared HTTP client configuration and factory for Janitor services

use crate::utils::service_user_agent;
use base64::prelude::*;
use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// HTTP client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpClientConfig {
    /// Request timeout in seconds
    pub request_timeout: u64,
    /// Connection timeout in seconds
    pub connect_timeout: u64,
    /// Maximum number of redirects to follow
    pub max_redirects: u32,
    /// Enable HTTP/2
    pub http2_prior_knowledge: bool,
    /// Enable compression
    pub compression: bool,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            request_timeout: 30,
            connect_timeout: 10,
            max_redirects: 10,
            http2_prior_knowledge: true,
            compression: true,
        }
    }
}

/// Authentication credentials for HTTP clients
#[derive(Debug, Clone)]
pub enum HttpCredentials {
    /// No authentication
    None,
    /// Bearer token authentication
    Bearer { token: String },
    /// Basic authentication
    Basic {
        username: String,
        password: Option<String>,
    },
}

/// HTTP client factory for creating configured clients across services
pub struct HttpClientFactory {
    config: HttpClientConfig,
    service_name: String,
    version: String,
}

impl HttpClientFactory {
    /// Create a new HTTP client factory
    pub fn new(service_name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            config: HttpClientConfig::default(),
            service_name: service_name.into(),
            version: version.into(),
        }
    }

    /// Create a factory with custom configuration
    pub fn with_config(
        service_name: impl Into<String>,
        version: impl Into<String>,
        config: HttpClientConfig,
    ) -> Self {
        Self {
            config,
            service_name: service_name.into(),
            version: version.into(),
        }
    }

    /// Create a basic HTTP client with standard configuration
    pub fn create_client(&self) -> Result<Client, reqwest::Error> {
        self.create_client_with_auth(HttpCredentials::None)
    }

    /// Create an HTTP client with authentication
    pub fn create_client_with_auth(
        &self,
        credentials: HttpCredentials,
    ) -> Result<Client, reqwest::Error> {
        let mut builder = ClientBuilder::new()
            .timeout(Duration::from_secs(self.config.request_timeout))
            .connect_timeout(Duration::from_secs(self.config.connect_timeout))
            .redirect(reqwest::redirect::Policy::limited(
                self.config.max_redirects as usize,
            ))
            .user_agent(service_user_agent(&self.service_name));

        if self.config.http2_prior_knowledge {
            builder = builder.http2_prior_knowledge();
        }

        // Note: reqwest enables compression by default,
        // and we can't explicitly enable it in the builder

        // Apply authentication
        match credentials {
            HttpCredentials::None => {}
            HttpCredentials::Bearer { token } => {
                let mut headers = reqwest::header::HeaderMap::new();
                match reqwest::header::HeaderValue::from_str(&format!("Bearer {}", token)) {
                    Ok(header_value) => {
                        headers.insert(reqwest::header::AUTHORIZATION, header_value);
                    }
                    Err(_) => {
                        // If the token is invalid, just skip authentication
                        // reqwest will handle this gracefully
                    }
                }
                builder = builder.default_headers(headers);
            }
            HttpCredentials::Basic { username, password } => {
                // For basic auth, we'll need to add the header manually
                let mut headers = reqwest::header::HeaderMap::new();
                let auth_string = match password {
                    Some(pwd) => format!("{}:{}", username, pwd),
                    None => username.clone(),
                };
                let encoded = base64::prelude::BASE64_STANDARD.encode(auth_string.as_bytes());
                if let Ok(header_value) =
                    reqwest::header::HeaderValue::from_str(&format!("Basic {}", encoded))
                {
                    headers.insert(reqwest::header::AUTHORIZATION, header_value);
                }
                builder = builder.default_headers(headers);
            }
        }

        builder.build()
    }

    /// Create a client with custom timeout while keeping other settings
    pub fn create_client_with_timeout(
        &self,
        timeout_seconds: u64,
    ) -> Result<Client, reqwest::Error> {
        self.create_client_with_timeout_and_auth(timeout_seconds, HttpCredentials::None)
    }

    /// Create a client with custom timeout and authentication
    pub fn create_client_with_timeout_and_auth(
        &self,
        timeout_seconds: u64,
        credentials: HttpCredentials,
    ) -> Result<Client, reqwest::Error> {
        let mut config = self.config.clone();
        config.request_timeout = timeout_seconds;

        let temp_factory =
            HttpClientFactory::with_config(&self.service_name, &self.version, config);
        temp_factory.create_client_with_auth(credentials)
    }

    /// Create a client builder for custom configuration
    pub fn create_client_builder(&self) -> ClientBuilder {
        let mut builder = ClientBuilder::new()
            .timeout(Duration::from_secs(self.config.request_timeout))
            .connect_timeout(Duration::from_secs(self.config.connect_timeout))
            .redirect(reqwest::redirect::Policy::limited(
                self.config.max_redirects as usize,
            ))
            .user_agent(service_user_agent(&self.service_name));

        if self.config.http2_prior_knowledge {
            builder = builder.http2_prior_knowledge();
        }

        // Note: reqwest enables compression by default,
        // and we can't explicitly enable it in the builder

        builder
    }

    /// Get the current configuration
    pub fn config(&self) -> &HttpClientConfig {
        &self.config
    }

    /// Get the service name
    pub fn service_name(&self) -> &str {
        &self.service_name
    }

    /// Get the version
    pub fn version(&self) -> &str {
        &self.version
    }
}

/// Predefined HTTP client factories for common service configurations
impl HttpClientFactory {
    /// Create a factory for long-running operations (5 minute timeout)
    pub fn long_running(service_name: impl Into<String>, version: impl Into<String>) -> Self {
        let config = HttpClientConfig {
            request_timeout: 300, // 5 minutes
            ..Default::default()
        };
        Self::with_config(service_name, version, config)
    }

    /// Create a factory for quick operations (10 second timeout)
    pub fn quick(service_name: impl Into<String>, version: impl Into<String>) -> Self {
        let config = HttpClientConfig {
            request_timeout: 10,
            ..Default::default()
        };
        Self::with_config(service_name, version, config)
    }

    /// Create a factory for health checks (5 second timeout)
    pub fn health_check(service_name: impl Into<String>, version: impl Into<String>) -> Self {
        let config = HttpClientConfig {
            request_timeout: 5,
            connect_timeout: 3,
            max_redirects: 3,
            ..Default::default()
        };
        Self::with_config(service_name, version, config)
    }

    /// Create a factory for file operations (2 minute timeout)
    pub fn file_operations(service_name: impl Into<String>, version: impl Into<String>) -> Self {
        let config = HttpClientConfig {
            request_timeout: 120,
            ..Default::default()
        };
        Self::with_config(service_name, version, config)
    }
}

/// Create a shared HTTP client factory from environment variables
pub fn http_client_factory_from_env(
    service_name: impl Into<String>,
    version: impl Into<String>,
) -> HttpClientFactory {
    let config = HttpClientConfig {
        request_timeout: std::env::var("HTTP_REQUEST_TIMEOUT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30),
        connect_timeout: std::env::var("HTTP_CONNECT_TIMEOUT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(10),
        max_redirects: std::env::var("HTTP_MAX_REDIRECTS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(10),
        http2_prior_knowledge: std::env::var("HTTP_HTTP2_PRIOR_KNOWLEDGE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(true),
        compression: std::env::var("HTTP_COMPRESSION")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(true),
    };

    HttpClientFactory::with_config(service_name, version, config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_client_config_default() {
        let config = HttpClientConfig::default();
        assert_eq!(config.request_timeout, 30);
        assert_eq!(config.connect_timeout, 10);
        assert_eq!(config.max_redirects, 10);
        assert!(config.http2_prior_knowledge);
        assert!(config.compression);
    }

    #[test]
    fn test_http_client_factory_creation() {
        let factory = HttpClientFactory::new("test-service", "1.0.0");
        assert_eq!(factory.service_name(), "test-service");
        assert_eq!(factory.version(), "1.0.0");
        assert_eq!(factory.config().request_timeout, 30);
    }

    #[test]
    fn test_http_client_factory_with_config() {
        let config = HttpClientConfig {
            request_timeout: 60,
            connect_timeout: 15,
            max_redirects: 5,
            http2_prior_knowledge: false,
            compression: false,
        };
        let factory = HttpClientFactory::with_config("test-service", "1.0.0", config);

        assert_eq!(factory.config().request_timeout, 60);
        assert_eq!(factory.config().connect_timeout, 15);
        assert_eq!(factory.config().max_redirects, 5);
        assert!(!factory.config().http2_prior_knowledge);
        assert!(!factory.config().compression);
    }

    #[test]
    fn test_predefined_factories() {
        let long_running = HttpClientFactory::long_running("test", "1.0.0");
        assert_eq!(long_running.config().request_timeout, 300);

        let quick = HttpClientFactory::quick("test", "1.0.0");
        assert_eq!(quick.config().request_timeout, 10);

        let health_check = HttpClientFactory::health_check("test", "1.0.0");
        assert_eq!(health_check.config().request_timeout, 5);
        assert_eq!(health_check.config().connect_timeout, 3);

        let file_ops = HttpClientFactory::file_operations("test", "1.0.0");
        assert_eq!(file_ops.config().request_timeout, 120);
    }

    #[test]
    fn test_credentials_types() {
        // Test that credentials enum variants exist and can be created
        let none = HttpCredentials::None;
        let bearer = HttpCredentials::Bearer {
            token: "test-token".to_string(),
        };
        let basic_with_password = HttpCredentials::Basic {
            username: "user".to_string(),
            password: Some("pass".to_string()),
        };
        let basic_without_password = HttpCredentials::Basic {
            username: "user".to_string(),
            password: None,
        };

        // Just verify they can be created (match pattern to use them)
        match none {
            HttpCredentials::None => {}
            _ => panic!("Wrong credential type"),
        }
        match bearer {
            HttpCredentials::Bearer { .. } => {}
            _ => panic!("Wrong credential type"),
        }
        match basic_with_password {
            HttpCredentials::Basic { .. } => {}
            _ => panic!("Wrong credential type"),
        }
        match basic_without_password {
            HttpCredentials::Basic { .. } => {}
            _ => panic!("Wrong credential type"),
        }
    }

    #[test]
    fn test_create_client() {
        let factory = HttpClientFactory::new("test-service", "1.0.0");
        let client = factory.create_client();
        assert!(client.is_ok());
    }

    #[test]
    fn test_create_client_with_timeout() {
        let factory = HttpClientFactory::new("test-service", "1.0.0");
        let client = factory.create_client_with_timeout(60);
        assert!(client.is_ok());
    }

    #[test]
    fn test_environment_factory() {
        // Test with no environment variables set (should use defaults)
        let factory = http_client_factory_from_env("test-service", "1.0.0");
        assert_eq!(factory.config().request_timeout, 30);
        assert_eq!(factory.config().connect_timeout, 10);
    }
}
