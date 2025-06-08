//! External service URL configurations for Janitor services

use serde::{Deserialize, Serialize};
use url::Url;

use crate::shared_config::{env::EnvParser, ConfigError, FromEnv, ValidationError};

/// External service URLs configuration used across Janitor services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalServiceConfig {
    /// Runner service URL for job coordination
    pub runner_url: Option<String>,

    /// Differ service URL for generating diffs
    pub differ_url: Option<String>,

    /// Publisher service URL for publishing changes
    pub publisher_url: Option<String>,

    /// Archive service URL for APT repository operations
    pub archiver_url: Option<String>,

    /// Git store service URL for Git repository access
    pub git_store_url: Option<String>,

    /// Bazaar store service URL for Bazaar repository access  
    pub bzr_store_url: Option<String>,

    /// Site/web interface URL
    pub site_url: Option<String>,

    /// External base URL for public access (used for callbacks, webhooks, etc.)
    pub external_url: Option<String>,

    /// Prometheus gateway URL for metrics pushing
    pub prometheus_gateway_url: Option<String>,

    /// Log management service URL
    pub log_service_url: Option<String>,

    /// Artifact storage service URL
    pub artifact_service_url: Option<String>,
}

impl Default for ExternalServiceConfig {
    fn default() -> Self {
        Self {
            runner_url: Some("http://localhost:9911/".to_string()),
            differ_url: Some("http://localhost:9920/".to_string()),
            publisher_url: Some("http://localhost:9912/".to_string()),
            archiver_url: Some("http://localhost:9913/".to_string()),
            git_store_url: Some("http://localhost:9914/".to_string()),
            bzr_store_url: Some("http://localhost:9915/".to_string()),
            site_url: Some("http://localhost:9910/".to_string()),
            external_url: None,
            prometheus_gateway_url: None,
            log_service_url: None,
            artifact_service_url: None,
        }
    }
}

impl FromEnv for ExternalServiceConfig {
    fn from_env() -> Result<Self, ConfigError> {
        Self::from_env_with_prefix("")
    }

    fn from_env_with_prefix(prefix: &str) -> Result<Self, ConfigError> {
        let parser = EnvParser::with_prefix(prefix);

        Ok(Self {
            runner_url: parser.get_string("RUNNER_URL"),
            differ_url: parser.get_string("DIFFER_URL"),
            publisher_url: parser.get_string("PUBLISHER_URL"),
            archiver_url: parser.get_string("ARCHIVER_URL"),
            git_store_url: parser.get_string("GIT_STORE_URL"),
            bzr_store_url: parser.get_string("BZR_STORE_URL"),
            site_url: parser.get_string("SITE_URL"),
            external_url: parser.get_string("EXTERNAL_URL"),
            prometheus_gateway_url: parser.get_string("PROMETHEUS_GATEWAY_URL"),
            log_service_url: parser.get_string("LOG_SERVICE_URL"),
            artifact_service_url: parser.get_string("ARTIFACT_SERVICE_URL"),
        })
    }
}

impl ExternalServiceConfig {
    /// Validate all configured service URLs
    pub fn validate(&self) -> Result<(), ValidationError> {
        let urls = [
            ("runner_url", &self.runner_url),
            ("differ_url", &self.differ_url),
            ("publisher_url", &self.publisher_url),
            ("archiver_url", &self.archiver_url),
            ("git_store_url", &self.git_store_url),
            ("bzr_store_url", &self.bzr_store_url),
            ("site_url", &self.site_url),
            ("external_url", &self.external_url),
            ("prometheus_gateway_url", &self.prometheus_gateway_url),
            ("log_service_url", &self.log_service_url),
            ("artifact_service_url", &self.artifact_service_url),
        ];

        for (field_name, url_option) in urls {
            if let Some(url_str) = url_option {
                if let Err(_) = Url::parse(url_str) {
                    return Err(ValidationError::InvalidValue {
                        field: field_name.to_string(),
                        message: format!("Invalid URL format: {}", url_str),
                    });
                }
            }
        }

        Ok(())
    }

    /// Get runner URL as parsed Url, if configured
    pub fn runner_url(&self) -> Option<Url> {
        self.runner_url.as_ref().and_then(|s| Url::parse(s).ok())
    }

    /// Get differ URL as parsed Url, if configured
    pub fn differ_url(&self) -> Option<Url> {
        self.differ_url.as_ref().and_then(|s| Url::parse(s).ok())
    }

    /// Get publisher URL as parsed Url, if configured
    pub fn publisher_url(&self) -> Option<Url> {
        self.publisher_url.as_ref().and_then(|s| Url::parse(s).ok())
    }

    /// Get archiver URL as parsed Url, if configured
    pub fn archiver_url(&self) -> Option<Url> {
        self.archiver_url.as_ref().and_then(|s| Url::parse(s).ok())
    }

    /// Get git store URL as parsed Url, if configured
    pub fn git_store_url(&self) -> Option<Url> {
        self.git_store_url.as_ref().and_then(|s| Url::parse(s).ok())
    }

    /// Get bzr store URL as parsed Url, if configured
    pub fn bzr_store_url(&self) -> Option<Url> {
        self.bzr_store_url.as_ref().and_then(|s| Url::parse(s).ok())
    }

    /// Get site URL as parsed Url, if configured
    pub fn site_url(&self) -> Option<Url> {
        self.site_url.as_ref().and_then(|s| Url::parse(s).ok())
    }

    /// Get external URL as parsed Url, if configured
    pub fn external_url(&self) -> Option<Url> {
        self.external_url.as_ref().and_then(|s| Url::parse(s).ok())
    }

    /// Get prometheus gateway URL as parsed Url, if configured
    pub fn prometheus_gateway_url(&self) -> Option<Url> {
        self.prometheus_gateway_url
            .as_ref()
            .and_then(|s| Url::parse(s).ok())
    }

    /// Get log service URL as parsed Url, if configured
    pub fn log_service_url(&self) -> Option<Url> {
        self.log_service_url
            .as_ref()
            .and_then(|s| Url::parse(s).ok())
    }

    /// Get artifact service URL as parsed Url, if configured
    pub fn artifact_service_url(&self) -> Option<Url> {
        self.artifact_service_url
            .as_ref()
            .and_then(|s| Url::parse(s).ok())
    }

    /// Check if a specific service is configured and available
    pub fn has_service(&self, service: ExternalService) -> bool {
        match service {
            ExternalService::Runner => self.runner_url.is_some(),
            ExternalService::Differ => self.differ_url.is_some(),
            ExternalService::Publisher => self.publisher_url.is_some(),
            ExternalService::Archiver => self.archiver_url.is_some(),
            ExternalService::GitStore => self.git_store_url.is_some(),
            ExternalService::BzrStore => self.bzr_store_url.is_some(),
            ExternalService::Site => self.site_url.is_some(),
            ExternalService::Prometheus => self.prometheus_gateway_url.is_some(),
            ExternalService::LogService => self.log_service_url.is_some(),
            ExternalService::ArtifactService => self.artifact_service_url.is_some(),
        }
    }

    /// Get URL for a specific service
    pub fn get_service_url(&self, service: ExternalService) -> Option<&str> {
        match service {
            ExternalService::Runner => self.runner_url.as_deref(),
            ExternalService::Differ => self.differ_url.as_deref(),
            ExternalService::Publisher => self.publisher_url.as_deref(),
            ExternalService::Archiver => self.archiver_url.as_deref(),
            ExternalService::GitStore => self.git_store_url.as_deref(),
            ExternalService::BzrStore => self.bzr_store_url.as_deref(),
            ExternalService::Site => self.site_url.as_deref(),
            ExternalService::Prometheus => self.prometheus_gateway_url.as_deref(),
            ExternalService::LogService => self.log_service_url.as_deref(),
            ExternalService::ArtifactService => self.artifact_service_url.as_deref(),
        }
    }
}

/// Enumeration of external services that can be configured
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExternalService {
    Runner,
    Differ,
    Publisher,
    Archiver,
    GitStore,
    BzrStore,
    Site,
    Prometheus,
    LogService,
    ArtifactService,
}

impl std::fmt::Display for ExternalService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExternalService::Runner => write!(f, "runner"),
            ExternalService::Differ => write!(f, "differ"),
            ExternalService::Publisher => write!(f, "publisher"),
            ExternalService::Archiver => write!(f, "archiver"),
            ExternalService::GitStore => write!(f, "git-store"),
            ExternalService::BzrStore => write!(f, "bzr-store"),
            ExternalService::Site => write!(f, "site"),
            ExternalService::Prometheus => write!(f, "prometheus"),
            ExternalService::LogService => write!(f, "log-service"),
            ExternalService::ArtifactService => write!(f, "artifact-service"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_external_service_config_validation() {
        let mut config = ExternalServiceConfig::default();
        assert!(config.validate().is_ok());

        // Test invalid URL
        config.runner_url = Some("not-a-url".to_string());
        assert!(config.validate().is_err());

        // Test valid URL
        config.runner_url = Some("http://localhost:9911".to_string());
        assert!(config.validate().is_ok());

        // Test HTTPS URL
        config.differ_url = Some("https://differ.example.com/".to_string());
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_external_service_config_parsing() {
        let config = ExternalServiceConfig::default();

        // Test URL parsing
        assert!(config.runner_url().is_some());
        assert!(config.differ_url().is_some());

        let runner_url = config.runner_url().unwrap();
        assert_eq!(runner_url.scheme(), "http");
        assert_eq!(runner_url.host_str(), Some("localhost"));
        assert_eq!(runner_url.port(), Some(9911));
    }

    #[test]
    fn test_has_service() {
        let config = ExternalServiceConfig::default();

        assert!(config.has_service(ExternalService::Runner));
        assert!(config.has_service(ExternalService::Differ));
        assert!(!config.has_service(ExternalService::Prometheus)); // Not set by default
    }

    #[test]
    fn test_get_service_url() {
        let config = ExternalServiceConfig::default();

        assert_eq!(
            config.get_service_url(ExternalService::Runner),
            Some("http://localhost:9911/")
        );
        assert_eq!(config.get_service_url(ExternalService::Prometheus), None);
    }

    #[test]
    fn test_external_service_display() {
        assert_eq!(ExternalService::Runner.to_string(), "runner");
        assert_eq!(ExternalService::GitStore.to_string(), "git-store");
        assert_eq!(ExternalService::BzrStore.to_string(), "bzr-store");
    }
}
