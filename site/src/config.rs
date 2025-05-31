use anyhow::{Context, Result};
use chrono::Duration;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;

/// Log level wrapper for serde compatibility
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl From<LogLevel> for tracing::Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Trace => tracing::Level::TRACE,
            LogLevel::Debug => tracing::Level::DEBUG,
            LogLevel::Info => tracing::Level::INFO,
            LogLevel::Warn => tracing::Level::WARN,
            LogLevel::Error => tracing::Level::ERROR,
        }
    }
}

impl FromStr for LogLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "trace" => Ok(LogLevel::Trace),
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            _ => Err(format!("Unknown log level: {}", s)),
        }
    }
}

/// Site-specific configuration that extends the main janitor config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteConfig {
    // Network & Server
    pub listen_address: SocketAddr,
    pub public_listen_address: Option<SocketAddr>,
    pub external_url: Option<String>,
    pub user_agent: String,

    // Database & Storage
    pub database_url: String,
    pub redis_url: Option<String>,

    // Templates & Assets
    pub template_dir: Option<String>,
    pub static_dir: Option<String>,
    pub minified_assets: bool,

    // Debug & Development
    pub debug: bool,
    pub debug_toolbar: bool,
    pub debug_toolbar_allowed_ips: Vec<String>,
    pub log_level: LogLevel,
    pub gcp_logging: bool,

    // Authentication & Security
    pub session_secret: String,
    pub oidc_client_id: Option<String>,
    pub oidc_client_secret: Option<String>,
    pub oidc_issuer_url: Option<String>,
    pub oidc_base_url: Option<String>,
    pub qa_reviewer_group: Option<String>,
    pub admin_group: Option<String>,

    // External Services (with defaults)
    pub runner_url: String,
    pub publisher_url: Option<String>,
    pub archiver_url: Option<String>,
    pub differ_url: String,
    pub git_store_url: Option<String>,
    pub bzr_store_url: Option<String>,

    // Monitoring & Observability
    pub zipkin_address: Option<String>,
    pub zipkin_sample_rate: f64,
    pub metrics_enabled: bool,
    pub request_timeout: Duration,

    // Feature Flags
    pub enable_websockets: bool,
    pub enable_gpg_support: bool,
    pub enable_archive_browsing: bool,
    pub enable_diff_view: bool,

    // Main janitor config integration
    pub janitor_config_path: Option<PathBuf>,
}

/// Combined configuration that includes both site and janitor configs
#[derive(Debug, Clone)]
pub struct Config {
    pub site: SiteConfig,
    pub janitor: Option<janitor::config::Config>,
    // Convenience accessor for campaigns
    pub campaigns: HashMap<String, serde_json::Value>,
}

impl Config {
    pub fn new(site: SiteConfig, janitor: Option<janitor::config::Config>) -> Self {
        let campaigns = janitor.as_ref()
            .map(|j| {
                // Extract campaigns from janitor config
                // TODO: This needs to match the actual janitor config structure
                HashMap::new()
            })
            .unwrap_or_default();
            
        Self {
            site,
            janitor,
            campaigns,
        }
    }
}

/// Environment-specific configuration profiles
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Development,
    Staging,
    Production,
}

/// Configuration builder for hierarchical config loading
#[derive(Debug, Default)]
pub struct ConfigBuilder {
    site_config: Option<SiteConfig>,
    janitor_config_path: Option<PathBuf>,
    environment: Option<Environment>,
    overrides: HashMap<String, String>,
}

impl SiteConfig {
    /// Load configuration from environment variables with sensible defaults
    pub fn from_env() -> Result<Self> {
        let listen_address = env::var("LISTEN_ADDRESS")
            .unwrap_or_else(|_| "127.0.0.1:8080".to_string())
            .parse()
            .context("Invalid LISTEN_ADDRESS format")?;

        let public_listen_address = env::var("PUBLIC_LISTEN_ADDRESS")
            .ok()
            .map(|addr| addr.parse())
            .transpose()
            .context("Invalid PUBLIC_LISTEN_ADDRESS format")?;

        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://localhost/janitor".to_string());

        let session_secret = env::var("SESSION_SECRET").unwrap_or_else(|_| {
            if cfg!(debug_assertions) {
                "debug-secret-key-not-for-production-use-only".to_string()
            } else {
                panic!("SESSION_SECRET environment variable is required in production")
            }
        });

        let debug = env::var("DEBUG")
            .map(|v| v.parse().unwrap_or(false))
            .unwrap_or(cfg!(debug_assertions));

        let log_level = env::var("LOG_LEVEL")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| {
                if debug {
                    LogLevel::Debug
                } else {
                    LogLevel::Info
                }
            });

        let debug_toolbar_allowed_ips = env::var("DEBUG_TOOLBAR_ALLOWED_IPS")
            .map(|ips| ips.split(',').map(|ip| ip.trim().to_string()).collect())
            .unwrap_or_else(|_| vec!["127.0.0.1".to_string(), "::1".to_string()]);

        let zipkin_sample_rate = env::var("ZIPKIN_SAMPLE_RATE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.1);

        let request_timeout = env::var("REQUEST_TIMEOUT_SECONDS")
            .ok()
            .and_then(|v| v.parse().ok())
            .map(Duration::seconds)
            .unwrap_or_else(|| Duration::seconds(30));

        Ok(SiteConfig {
            // Network & Server
            listen_address,
            public_listen_address,
            external_url: env::var("EXTERNAL_URL").ok(),
            user_agent: env::var("USER_AGENT")
                .unwrap_or_else(|_| format!("janitor-site/{}", env!("CARGO_PKG_VERSION"))),

            // Database & Storage
            database_url,
            redis_url: env::var("REDIS_URL").ok(),

            // Templates & Assets
            template_dir: env::var("TEMPLATE_DIR").ok(),
            static_dir: env::var("STATIC_DIR").ok(),
            minified_assets: !debug, // Use minified assets in production

            // Debug & Development
            debug,
            debug_toolbar: env::var("DEBUG_TOOLBAR")
                .map(|v| v.parse().unwrap_or(false))
                .unwrap_or(debug),
            debug_toolbar_allowed_ips,
            log_level,
            gcp_logging: env::var("GCP_LOGGING")
                .map(|v| v.parse().unwrap_or(false))
                .unwrap_or(false),

            // Authentication & Security
            session_secret,
            oidc_client_id: env::var("OAUTH2_CLIENT_ID")
                .or_else(|_| env::var("OIDC_CLIENT_ID"))
                .ok(),
            oidc_client_secret: env::var("OAUTH2_CLIENT_SECRET")
                .or_else(|_| env::var("OIDC_CLIENT_SECRET"))
                .ok(),
            oidc_issuer_url: env::var("OIDC_ISSUER_URL").ok(),
            oidc_base_url: env::var("OIDC_BASE_URL").ok(),
            qa_reviewer_group: env::var("QA_REVIEWER_GROUP").ok(),
            admin_group: env::var("ADMIN_GROUP").ok(),

            // External Services (matching Python defaults)
            runner_url: env::var("RUNNER_URL")
                .unwrap_or_else(|_| "http://localhost:9911/".to_string()),
            publisher_url: env::var("PUBLISHER_URL").ok(),
            archiver_url: env::var("ARCHIVER_URL").ok(),
            differ_url: env::var("DIFFER_URL")
                .unwrap_or_else(|_| "http://localhost:9920/".to_string()),
            git_store_url: env::var("GIT_STORE_URL").ok(),
            bzr_store_url: env::var("BZR_STORE_URL").ok(),

            // Monitoring & Observability
            zipkin_address: env::var("ZIPKIN_ADDRESS").ok(),
            zipkin_sample_rate,
            metrics_enabled: env::var("METRICS_ENABLED")
                .map(|v| v.parse().unwrap_or(true))
                .unwrap_or(true),
            request_timeout,

            // Feature Flags
            enable_websockets: env::var("ENABLE_WEBSOCKETS")
                .map(|v| v.parse().unwrap_or(true))
                .unwrap_or(true),
            enable_gpg_support: env::var("ENABLE_GPG_SUPPORT")
                .map(|v| v.parse().unwrap_or(false))
                .unwrap_or(false),
            enable_archive_browsing: env::var("ENABLE_ARCHIVE_BROWSING")
                .map(|v| v.parse().unwrap_or(true))
                .unwrap_or(true),
            enable_diff_view: env::var("ENABLE_DIFF_VIEW")
                .map(|v| v.parse().unwrap_or(true))
                .unwrap_or(true),

            // Main janitor config integration
            janitor_config_path: env::var("JANITOR_CONFIG")
                .or_else(|_| env::var("CONFIG"))
                .ok()
                .map(PathBuf::from),
        })
    }

    /// Check if authentication is configured
    pub fn has_authentication(&self) -> bool {
        self.oidc_client_id.is_some() && self.oidc_client_secret.is_some()
    }

    /// Check if GPG functionality should be enabled
    pub fn has_gpg_support(&self) -> bool {
        self.enable_gpg_support && self.publisher_url.is_some() && self.archiver_url.is_some()
    }

    /// Get the effective template directory
    pub fn template_directory(&self) -> &str {
        self.template_dir
            .as_deref()
            .unwrap_or("py/janitor/site/templates")
    }

    /// Get the effective static files directory
    pub fn static_directory(&self) -> &str {
        self.static_dir
            .as_deref()
            .unwrap_or("py/janitor/site/_static")
    }

    /// Determine if this is a production environment
    pub fn is_production(&self) -> bool {
        !self.debug && env::var("ENVIRONMENT").as_deref() == Ok("production")
    }
}

impl Config {
    /// Create a new configuration builder
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::default()
    }

    /// Load configuration from environment with janitor config integration
    pub fn from_env() -> Result<Self> {
        let site = SiteConfig::from_env()?;
        let janitor = if let Some(ref config_path) = site.janitor_config_path {
            Some(
                janitor::config::read_file(config_path)
                    .map_err(|e| anyhow::anyhow!("Failed to load janitor config: {}", e))?,
            )
        } else {
            None
        };

        Ok(Config::new(site, janitor))
    }

    /// Access the site configuration
    pub fn site(&self) -> &SiteConfig {
        &self.site
    }

    /// Access the janitor configuration if available
    pub fn janitor(&self) -> Option<&janitor::config::Config> {
        self.janitor.as_ref()
    }

    /// Get the effective database URL (from janitor config if available)
    pub fn database_url(&self) -> &str {
        if let Some(janitor_config) = &self.janitor {
            if let Some(db_location) = &janitor_config.database_location {
                return db_location;
            }
        }
        &self.site.database_url
    }

    /// Get the effective Redis URL (from janitor config if available)
    pub fn redis_url(&self) -> Option<&str> {
        if let Some(janitor_config) = &self.janitor {
            if let Some(redis_location) = &janitor_config.redis_location {
                return Some(redis_location);
            }
        }
        self.site.redis_url.as_deref()
    }

    /// Get external service URLs (using site config only for now)
    pub fn runner_url(&self) -> &str {
        // TODO: Check if janitor config has these fields when available
        &self.site.runner_url
    }

    pub fn publisher_url(&self) -> Option<&str> {
        // TODO: Check if janitor config has these fields when available
        self.site.publisher_url.as_deref()
    }

    pub fn archiver_url(&self) -> Option<&str> {
        // TODO: Check if janitor config has these fields when available
        self.site.archiver_url.as_deref()
    }

    pub fn differ_url(&self) -> Option<&str> {
        // TODO: Check if janitor config has these fields when available
        Some(&self.site.differ_url)
    }

    /// Get external URL for VCS access
    pub fn external_url(&self) -> Option<&str> {
        self.site.external_url.as_deref()
    }

    /// Get log base path
    pub fn log_base_path(&self) -> Option<String> {
        // TODO: Check if janitor config has this field when available
        env::var("LOG_BASE_PATH").ok()
    }

    /// Get OAuth2 configuration from janitor config if available
    pub fn oauth2_config(&self) -> Option<&janitor::config::OAuth2Provider> {
        self.janitor
            .as_ref()
            .and_then(|config| config.oauth2_provider.as_ref())
    }
}

impl Default for SiteConfig {
    fn default() -> Self {
        Self {
            // Network & Server
            listen_address: "127.0.0.1:8080".parse().unwrap(),
            public_listen_address: None,
            external_url: None,
            user_agent: format!("janitor-site/{}", env!("CARGO_PKG_VERSION")),

            // Database & Storage
            database_url: "postgresql://localhost/janitor".to_string(),
            redis_url: None,

            // Templates & Assets
            template_dir: None,
            static_dir: None,
            minified_assets: false,

            // Debug & Development
            debug: true,
            debug_toolbar: true,
            debug_toolbar_allowed_ips: vec!["127.0.0.1".to_string(), "::1".to_string()],
            log_level: LogLevel::Debug,
            gcp_logging: false,

            // Authentication & Security
            session_secret: "debug-secret-key-not-for-production".to_string(),
            oidc_client_id: None,
            oidc_client_secret: None,
            oidc_issuer_url: None,
            oidc_base_url: None,
            qa_reviewer_group: None,
            admin_group: None,

            // External Services
            runner_url: "http://localhost:9911/".to_string(),
            publisher_url: None,
            archiver_url: None,
            differ_url: "http://localhost:9920/".to_string(),
            git_store_url: None,
            bzr_store_url: None,

            // Monitoring & Observability
            zipkin_address: None,
            zipkin_sample_rate: 0.1,
            metrics_enabled: true,
            request_timeout: Duration::seconds(30),

            // Feature Flags
            enable_websockets: true,
            enable_gpg_support: false,
            enable_archive_browsing: true,
            enable_diff_view: true,

            // Main janitor config integration
            janitor_config_path: None,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            site: SiteConfig::default(),
            janitor: None,
            campaigns: HashMap::new(),
        }
    }
}

impl Environment {
    /// Detect environment from environment variables
    pub fn detect() -> Self {
        match env::var("ENVIRONMENT").as_deref() {
            Ok("production") | Ok("prod") => Environment::Production,
            Ok("staging") | Ok("stage") => Environment::Staging,
            _ => Environment::Development,
        }
    }

    /// Get environment-specific defaults
    pub fn defaults(&self) -> HashMap<String, String> {
        let mut defaults = HashMap::new();

        match self {
            Environment::Development => {
                defaults.insert("DEBUG".to_string(), "true".to_string());
                defaults.insert("LOG_LEVEL".to_string(), "debug".to_string());
                defaults.insert("METRICS_ENABLED".to_string(), "true".to_string());
            }
            Environment::Staging => {
                defaults.insert("DEBUG".to_string(), "false".to_string());
                defaults.insert("LOG_LEVEL".to_string(), "info".to_string());
                defaults.insert("METRICS_ENABLED".to_string(), "true".to_string());
            }
            Environment::Production => {
                defaults.insert("DEBUG".to_string(), "false".to_string());
                defaults.insert("LOG_LEVEL".to_string(), "warn".to_string());
                defaults.insert("METRICS_ENABLED".to_string(), "true".to_string());
                defaults.insert("DEBUG_TOOLBAR".to_string(), "false".to_string());
            }
        }

        defaults
    }
}

impl ConfigBuilder {
    /// Set the environment
    pub fn environment(mut self, env: Environment) -> Self {
        self.environment = Some(env);
        self
    }

    /// Load janitor config from path
    pub fn with_janitor_config<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.janitor_config_path = Some(path.into());
        self
    }

    /// Add configuration override
    pub fn override_setting(mut self, key: String, value: String) -> Self {
        self.overrides.insert(key, value);
        self
    }

    /// Build the final configuration
    pub fn build(self) -> Result<Config> {
        // Apply environment defaults first
        let env = self.environment.unwrap_or_else(Environment::detect);
        for (key, value) in env.defaults() {
            if env::var(&key).is_err() {
                env::set_var(&key, &value);
            }
        }

        // Apply manual overrides
        for (key, value) in self.overrides {
            env::set_var(&key, &value);
        }

        // Load site configuration
        let mut site = self
            .site_config
            .unwrap_or_else(|| SiteConfig::from_env().expect("Failed to load site configuration"));

        // Override janitor config path if specified
        if let Some(path) = self.janitor_config_path {
            site.janitor_config_path = Some(path);
        }

        // Load janitor config if path is available
        let janitor = if let Some(ref config_path) = site.janitor_config_path {
            Some(
                janitor::config::read_file(config_path)
                    .map_err(|e| anyhow::anyhow!("Failed to load janitor config: {}", e))?,
            )
        } else {
            None
        };

        Ok(Config::new(site, janitor))
    }
}
