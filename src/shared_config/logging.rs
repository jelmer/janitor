//! Shared logging configuration for Janitor services

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::shared_config::{defaults::*, env::EnvParser, ConfigError, FromEnv, ValidationError};
use crate::shared_config::validation::Validator;

/// Logging configuration used across Janitor services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub level: String,
    
    /// Output logs in JSON format
    #[serde(default = "default_false")]
    pub json_format: bool,
    
    /// Enable console output
    #[serde(default = "default_true")]
    pub console_output: bool,
    
    /// File output configuration
    pub file_output: Option<FileOutputConfig>,
    
    /// Enable colored output (only applies to console)
    #[serde(default = "default_true")]
    pub colored_output: bool,
    
    /// Include timestamps in logs
    #[serde(default = "default_true")]
    pub include_timestamps: bool,
    
    /// Include module paths in logs
    #[serde(default = "default_false")]
    pub include_module_path: bool,
    
    /// Include line numbers in logs
    #[serde(default = "default_false")]
    pub include_line_numbers: bool,
    
    /// Maximum log level for specific modules
    pub module_filters: Option<std::collections::HashMap<String, String>>,
}

/// File output configuration for logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileOutputConfig {
    /// Path to the log file
    pub path: PathBuf,
    
    /// Maximum size of log file in bytes before rotation
    pub max_size_bytes: Option<u64>,
    
    /// Maximum number of rotated log files to keep
    pub max_files: Option<u32>,
    
    /// Enable log file compression
    #[serde(default = "default_false")]
    pub compress: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            json_format: default_false(),
            console_output: default_true(),
            file_output: None,
            colored_output: default_true(),
            include_timestamps: default_true(),
            include_module_path: default_false(),
            include_line_numbers: default_false(),
            module_filters: None,
        }
    }
}

impl FromEnv for LoggingConfig {
    fn from_env() -> Result<Self, ConfigError> {
        Self::from_env_with_prefix("")
    }
    
    fn from_env_with_prefix(prefix: &str) -> Result<Self, ConfigError> {
        let parser = EnvParser::with_prefix(prefix);
        
        let mut config = Self {
            level: parser.get_string("LOG_LEVEL")
                .unwrap_or_else(default_log_level),
            json_format: parser.get_bool("LOG_JSON_FORMAT")?
                .unwrap_or_else(default_false),
            console_output: parser.get_bool("LOG_CONSOLE_OUTPUT")?
                .unwrap_or_else(default_true),
            colored_output: parser.get_bool("LOG_COLORED_OUTPUT")?
                .unwrap_or_else(default_true),
            include_timestamps: parser.get_bool("LOG_INCLUDE_TIMESTAMPS")?
                .unwrap_or_else(default_true),
            include_module_path: parser.get_bool("LOG_INCLUDE_MODULE_PATH")?
                .unwrap_or_else(default_false),
            include_line_numbers: parser.get_bool("LOG_INCLUDE_LINE_NUMBERS")?
                .unwrap_or_else(default_false),
            file_output: None,
            module_filters: None,
        };
        
        // File output configuration
        if let Some(log_file) = parser.get_string("LOG_FILE") {
            config.file_output = Some(FileOutputConfig {
                path: PathBuf::from(log_file),
                max_size_bytes: parser.get_u64("LOG_FILE_MAX_SIZE")?,
                max_files: parser.get_u32("LOG_FILE_MAX_FILES")?,
                compress: parser.get_bool("LOG_FILE_COMPRESS")?
                    .unwrap_or_else(default_false),
            });
        }
        
        Ok(config)
    }
}

impl LoggingConfig {
    /// Validate the logging configuration
    pub fn validate(&self) -> Result<(), ValidationError> {
        let mut validator = Validator::new();
        
        // Validate log level
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.level.to_lowercase().as_str()) {
            validator.add_invalid_value(
                "level",
                &format!("Invalid log level '{}', must be one of: {}", 
                        self.level, 
                        valid_levels.join(", "))
            );
        }
        
        // Validate that at least one output is enabled
        if !self.console_output && self.file_output.is_none() {
            validator.add_invalid_value(
                "output",
                "At least one output method (console or file) must be enabled"
            );
        }
        
        // Validate file output configuration
        if let Some(ref file_config) = self.file_output {
            if let Some(parent) = file_config.path.parent() {
                if !parent.exists() {
                    validator.add_invalid_value(
                        "file_output.path",
                        &format!("Parent directory does not exist: {}", parent.display())
                    );
                }
            }
            
            if let Some(max_size) = file_config.max_size_bytes {
                if max_size == 0 {
                    validator.add_invalid_value(
                        "file_output.max_size_bytes",
                        "Maximum file size must be greater than 0"
                    );
                }
            }
            
            if let Some(max_files) = file_config.max_files {
                if max_files == 0 {
                    validator.add_invalid_value(
                        "file_output.max_files",
                        "Maximum number of files must be greater than 0"
                    );
                }
            }
        }
        
        // Validate module filters
        if let Some(ref filters) = self.module_filters {
            for (module, level) in filters {
                if module.is_empty() {
                    validator.add_invalid_value(
                        "module_filters",
                        "Module name cannot be empty"
                    );
                }
                
                if !valid_levels.contains(&level.to_lowercase().as_str()) {
                    validator.add_invalid_value(
                        "module_filters",
                        &format!("Invalid log level '{}' for module '{}', must be one of: {}", 
                                level, module, valid_levels.join(", "))
                    );
                }
            }
        }
        
        validator.finish()
    }
    
    /// Get the log level as a tracing::Level
    pub fn tracing_level(&self) -> Result<tracing::Level, ConfigError> {
        match self.level.to_lowercase().as_str() {
            "trace" => Ok(tracing::Level::TRACE),
            "debug" => Ok(tracing::Level::DEBUG),
            "info" => Ok(tracing::Level::INFO),
            "warn" => Ok(tracing::Level::WARN),
            "error" => Ok(tracing::Level::ERROR),
            _ => Err(ConfigError::ParseError {
                field: "level".to_string(),
                message: format!("Invalid log level: {}", self.level),
            }),
        }
    }
}

/// Initialize logging based on configuration
/// 
/// This is a simplified version that initializes basic tracing.
/// For more advanced logging features, services can implement their own
/// initialization using this config as a base.
pub fn init_logging(config: &LoggingConfig) -> Result<(), LoggingError> {
    use tracing_subscriber::{EnvFilter, fmt, prelude::*};
    
    // Validate configuration first
    config.validate().map_err(LoggingError::ValidationError)?;
    
    // Create environment filter
    let env_filter = if let Some(ref filters) = config.module_filters {
        let mut filter_string = config.level.clone();
        for (module, level) in filters {
            filter_string.push(',');
            filter_string.push_str(&format!("{}={}", module, level));
        }
        EnvFilter::new(filter_string)
    } else {
        EnvFilter::new(&config.level)
    };
    
    // Initialize basic console logging
    if config.console_output {
        if config.json_format {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(
                    fmt::layer()
                        .json()
                        .with_current_span(false)
                        .with_span_list(true)
                )
                .init();
        } else {
            let fmt_layer = fmt::layer()
                .with_ansi(config.colored_output)
                .with_target(config.include_module_path)
                .with_line_number(config.include_line_numbers);
                
            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt_layer)
                .init();
        }
    } else {
        // Even if console output is disabled, we need some form of subscriber
        tracing_subscriber::registry()
            .with(env_filter)
            .init();
    }
    
    // TODO: File output support can be added in future iterations
    // For now, services can implement file logging separately if needed
    if config.file_output.is_some() {
        log::warn!("File output is configured but not yet implemented in shared logging. Services should implement file logging separately.");
    }
    
    Ok(())
}

/// Errors that can occur during logging initialization
#[derive(Debug)]
pub enum LoggingError {
    /// Configuration validation error
    ValidationError(ValidationError),
    
    /// I/O error accessing log file
    IoError {
        path: String,
        message: String,
    },
    
    /// Error creating log filter
    FilterError {
        module: String,
        level: String,
        message: String,
    },
    
    /// Tracing initialization error
    InitError(String),
}

impl std::fmt::Display for LoggingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoggingError::ValidationError(err) => {
                write!(f, "Logging configuration validation error: {}", err)
            }
            LoggingError::IoError { path, message } => {
                write!(f, "I/O error accessing log file '{}': {}", path, message)
            }
            LoggingError::FilterError { module, level, message } => {
                write!(f, "Error creating filter for module '{}' with level '{}': {}", 
                       module, level, message)
            }
            LoggingError::InitError(message) => {
                write!(f, "Tracing initialization error: {}", message)
            }
        }
    }
}

impl std::error::Error for LoggingError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    
    #[test]
    fn test_logging_config_validation() {
        let mut config = LoggingConfig::default();
        assert!(config.validate().is_ok());
        
        // Test invalid log level
        config.level = "invalid".to_string();
        assert!(config.validate().is_err());
        
        // Test no output enabled
        config.level = "info".to_string();
        config.console_output = false;
        config.file_output = None;
        assert!(config.validate().is_err());
        
        // Test invalid module filter
        config.console_output = true;
        let mut filters = HashMap::new();
        filters.insert("test".to_string(), "invalid".to_string());
        config.module_filters = Some(filters);
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_tracing_level_conversion() {
        let config = LoggingConfig {
            level: "debug".to_string(),
            ..LoggingConfig::default()
        };
        
        assert_eq!(config.tracing_level().unwrap(), tracing::Level::DEBUG);
        
        let invalid_config = LoggingConfig {
            level: "invalid".to_string(),
            ..LoggingConfig::default()
        };
        
        assert!(invalid_config.tracing_level().is_err());
    }
}