//! Configuration management for the auto-upload service

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::fs;

/// Configuration for the auto-upload service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Redis connection URL
    pub redis_location: String,

    /// Database connection URL
    pub database_location: String,

    /// Artifact storage location
    pub artifact_location: String,
}

impl Config {
    /// Load configuration from a file
    pub async fn load(path: &str) -> Result<Self> {
        let contents = fs::read_to_string(path)
            .await
            .with_context(|| format!("Failed to read config file: {}", path))?;

        // Try to parse as TOML first
        toml::from_str(&contents)
            .or_else(|_| {
                // Fall back to janitor's config format if TOML fails
                Self::parse_janitor_config(&contents)
            })
            .context("Failed to parse configuration")
    }

    /// Parse janitor's custom config format
    fn parse_janitor_config(contents: &str) -> Result<Self> {
        let mut redis_location = None;
        let mut database_location = None;
        let mut artifact_location = None;

        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();

                match key {
                    "redis_location" => redis_location = Some(value.to_string()),
                    "database_location" => database_location = Some(value.to_string()),
                    "artifact_location" => artifact_location = Some(value.to_string()),
                    _ => {} // Ignore unknown keys
                }
            }
        }

        Ok(Config {
            redis_location: redis_location.context("Missing redis_location in config")?,
            database_location: database_location.context("Missing database_location in config")?,
            artifact_location: artifact_location.context("Missing artifact_location in config")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_janitor_config() {
        let config_str = r#"
# Sample janitor config
redis_location = redis://localhost:6379/0
database_location = postgresql://localhost/janitor
artifact_location = file:///var/lib/janitor/artifacts
"#;

        let config = Config::parse_janitor_config(config_str).unwrap();
        assert_eq!(config.redis_location, "redis://localhost:6379/0");
        assert_eq!(config.database_location, "postgresql://localhost/janitor");
        assert_eq!(
            config.artifact_location,
            "file:///var/lib/janitor/artifacts"
        );
    }
}
