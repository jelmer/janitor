use anyhow::Result;
use tracing::Level;
use tracing_subscriber::{
    filter::{EnvFilter, LevelFilter},
    fmt::{self, writer::MakeWriterExt},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    Registry,
};

use crate::config::{LogLevel, SiteConfig};

/// Initialize logging based on configuration
pub fn init_logging(config: &SiteConfig) -> Result<()> {
    let level: tracing::Level = config.log_level.into();
    if config.gcp_logging {
        // TODO: Implement Google Cloud Logging integration
        // This would require the google-cloud-logging crate
        init_standard_logging(config)
    } else {
        init_standard_logging(config)
    }
}

/// Initialize standard tracing-based logging
fn init_standard_logging(config: &SiteConfig) -> Result<()> {
    let filter = if config.debug {
        // In debug mode, show all logs from our crate and info+ from others
        EnvFilter::default()
            .add_directive("janitor=trace".parse()?)
            .add_directive("janitor_site=trace".parse()?)
            .add_directive(LevelFilter::INFO.into())
    } else {
        // In production, respect the configured log level
        EnvFilter::default()
            .add_directive(format!("janitor={}", log_level_to_string(config.log_level)).parse()?)
            .add_directive(format!("janitor_site={}", log_level_to_string(config.log_level)).parse()?)
            .add_directive(LevelFilter::WARN.into())
    };

    let fmt_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(config.debug)
        .with_thread_names(config.debug)
        .with_file(config.debug)
        .with_line_number(config.debug);

    let registry = Registry::default()
        .with(filter)
        .with(fmt_layer);

    if config.debug {
        // In debug mode, also log to stderr for development
        registry
            .with(
                fmt::layer()
                    .with_writer(std::io::stderr.with_max_level(tracing::Level::DEBUG))
                    .with_ansi(true),
            )
            .init();
    } else {
        registry.init();
    }

    Ok(())
}

/// Log level conversion utilities
fn log_level_to_string(level: LogLevel) -> &'static str {
    match level {
        LogLevel::Trace => "trace",
        LogLevel::Debug => "debug",
        LogLevel::Info => "info",
        LogLevel::Warn => "warn",
        LogLevel::Error => "error",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SiteConfig;

    #[test]
    fn test_logging_init() {
        let config = SiteConfig::default();
        // This test just ensures the logging initialization doesn't panic
        init_logging(&config).expect("Failed to initialize logging");
    }

    #[test]
    fn test_level_conversion() {
        assert_eq!(log_level_to_string(LogLevel::Debug), "debug");
        assert_eq!(log_level_to_string(LogLevel::Info), "info");
        assert_eq!(log_level_to_string(LogLevel::Warn), "warn");
    }
}