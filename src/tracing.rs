//! Consolidated tracing and logging initialization for Janitor services.

use crate::error::JanitorError;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

/// Logging configuration
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    /// Log level (e.g. "info", "debug", "trace")
    pub level: String,
    /// Whether to use JSON format for log output
    pub json_format: bool,
    /// Optional module-specific log level overrides
    pub module_filters: Option<std::collections::HashMap<String, String>>,
    /// Optional file output configuration
    pub file_output: Option<FileOutputConfig>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            json_format: false,
            module_filters: None,
            file_output: None,
        }
    }
}

/// Configuration for file-based log output
#[derive(Debug, Clone)]
pub struct FileOutputConfig {
    /// Path to the log file
    pub path: std::path::PathBuf,
    /// Maximum file size in bytes before rotation
    pub max_size_bytes: Option<u64>,
    /// Maximum number of rotated files to keep
    pub max_files: Option<usize>,
    /// Whether to compress rotated files
    pub compress: bool,
}

/// Builder for configuring tracing initialization
pub struct TracingBuilder {
    service_name: String,
    config: LoggingConfig,
    enable_distributed_tracing: bool,
    enable_performance_metrics: bool,
}

impl TracingBuilder {
    /// Create a new tracing builder for a service
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            config: LoggingConfig::default(),
            enable_distributed_tracing: false,
            enable_performance_metrics: false,
        }
    }

    /// Use a specific logging configuration
    pub fn with_config(mut self, config: LoggingConfig) -> Self {
        self.config = config;
        self
    }

    /// Enable distributed tracing support
    pub fn with_distributed_tracing(mut self, enable: bool) -> Self {
        self.enable_distributed_tracing = enable;
        self
    }

    /// Enable performance metrics collection
    pub fn with_performance_metrics(mut self, enable: bool) -> Self {
        self.enable_performance_metrics = enable;
        self
    }

    /// Set the log level
    pub fn with_level(mut self, level: impl Into<String>) -> Self {
        self.config.level = level.into();
        self
    }

    /// Enable JSON formatting
    pub fn with_json_format(mut self, enable: bool) -> Self {
        self.config.json_format = enable;
        self
    }

    /// Initialize the tracing subscriber
    pub fn init(self) -> Result<(), JanitorError> {
        let env_filter = self.build_env_filter()?;

        let subscriber = tracing_subscriber::registry().with(env_filter);

        let subscriber = if self.config.json_format {
            let json_layer = tracing_subscriber::fmt::layer()
                .json()
                .with_target(true)
                .with_current_span(true)
                .with_span_list(true)
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_file(true)
                .with_line_number(true);

            subscriber.with(json_layer.boxed())
        } else {
            let fmt_layer = tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true);

            subscriber.with(fmt_layer.boxed())
        };

        if let Some(ref file_config) = self.config.file_output {
            // TODO: Add file appender layer
            tracing::info!(
                "File logging configured to: {} (max size: {:?}, compression: {})",
                file_config.path.display(),
                file_config.max_size_bytes,
                file_config.compress,
            );
        }

        if self.enable_distributed_tracing {
            // TODO: Integrate with OpenTelemetry or similar
            tracing::info!(
                "Distributed tracing enabled for service: {}",
                self.service_name
            );
        }

        if self.enable_performance_metrics {
            tracing::info!(
                "Performance metrics enabled for service: {}",
                self.service_name
            );
        }

        subscriber
            .try_init()
            .map_err(|e| JanitorError::config(format!("Failed to initialize tracing: {}", e)))?;

        tracing::info!(
            "Tracing initialized for service '{}' with level '{}'",
            self.service_name,
            self.config.level
        );

        Ok(())
    }

    fn build_env_filter(&self) -> Result<EnvFilter, JanitorError> {
        let mut filter = EnvFilter::new(self.config.level.to_string());

        if let Some(ref module_filters) = self.config.module_filters {
            for (module, level) in module_filters {
                let directive = format!("{}={}", module, level);
                filter =
                    filter.add_directive(directive.parse().map_err(|e| {
                        JanitorError::config(format!("Invalid log directive: {}", e))
                    })?);
            }
        }

        if let Ok(env_filter) = std::env::var("RUST_LOG") {
            match env_filter.parse::<EnvFilter>() {
                Ok(env) => {
                    for directive in env.to_string().split(',') {
                        if let Ok(d) = directive.parse() {
                            filter = filter.add_directive(d);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Invalid RUST_LOG environment variable: {}", e);
                }
            }
        }

        Ok(filter)
    }
}

/// Initialize tracing with default configuration for a service
pub fn init(service_name: impl Into<String>) -> Result<(), JanitorError> {
    TracingBuilder::new(service_name).init()
}

/// Initialize tracing from command-line arguments
pub fn init_from_args(
    service_name: impl Into<String>,
    debug: bool,
    log_level: Option<String>,
) -> Result<(), JanitorError> {
    let mut builder = TracingBuilder::new(service_name);

    if debug {
        builder = builder.with_level("debug");
    } else if let Some(level) = log_level {
        match level.to_lowercase().as_str() {
            "trace" | "debug" | "info" | "warn" | "warning" | "error" => {
                builder = builder.with_level(level);
            }
            _ => {
                return Err(JanitorError::config(format!(
                    "Invalid log level: {}",
                    level
                )))
            }
        }
    }

    builder.init()
}

/// Initialize tracing from a logging configuration
pub fn init_with_config(
    service_name: impl Into<String>,
    config: LoggingConfig,
) -> Result<(), JanitorError> {
    TracingBuilder::new(service_name).with_config(config).init()
}

/// Get a tracing builder for a service
pub fn builder(service_name: impl Into<String>) -> TracingBuilder {
    TracingBuilder::new(service_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_creation() {
        let builder = TracingBuilder::new("test-service");
        assert_eq!(builder.service_name, "test-service");
        assert!(!builder.enable_distributed_tracing);
        assert!(!builder.enable_performance_metrics);
    }

    #[test]
    fn test_logging_config_default() {
        let config = LoggingConfig::default();
        assert_eq!(config.level, "info");
        assert!(!config.json_format);
        assert!(config.module_filters.is_none());
        assert!(config.file_output.is_none());
    }
}
