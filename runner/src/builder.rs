//! Builder system for the Janitor runner.
//!
//! This module provides the builder trait and implementations for different build types.

use crate::{BuilderResult, QueueItem};
use async_trait::async_trait;
use sqlx::PgConnection;
use std::collections::HashMap;
use std::path::Path;

/// Campaign configuration for builds.
/// This is a simplified version - in a full implementation this would be loaded from config
#[derive(Debug, Clone)]
pub struct CampaignConfig {
    /// Generic build configuration.
    pub generic_build: Option<GenericBuildConfig>,
    /// Debian build configuration.
    pub debian_build: Option<DebianBuildConfig>,
}

/// Generic build configuration.
#[derive(Debug, Clone)]
pub struct GenericBuildConfig {
    /// Chroot to use for builds.
    pub chroot: Option<String>,
}

/// Debian build configuration.
#[derive(Debug, Clone)]
pub struct DebianBuildConfig {
    /// Base distribution for builds.
    pub base_distribution: String,
    /// Extra build distributions.
    pub extra_build_distribution: Vec<String>,
}

/// Abstract builder trait for different build systems.
#[async_trait]
pub trait Builder: Send + Sync {
    /// Get the kind of builder.
    fn kind(&self) -> &'static str;

    /// Generate build configuration.
    async fn config(
        &self,
        conn: &mut PgConnection,
        campaign_config: &CampaignConfig,
        queue_item: &QueueItem,
    ) -> Result<HashMap<String, String>, BuilderError>;

    /// Generate build environment variables.
    async fn build_env(
        &self,
        conn: &mut PgConnection,
        campaign_config: &CampaignConfig,
        queue_item: &QueueItem,
    ) -> Result<HashMap<String, String>, BuilderError>;

    /// Get additional colocated branches for this build type.
    fn additional_colocated_branches(&self, main_branch: &str) -> HashMap<String, String>;

    /// Process build results from a directory.
    fn process_result(&self, output_dir: &Path) -> Result<BuilderResult, BuilderError>;
}

/// Errors that can occur during building.
#[derive(Debug, thiserror::Error)]
pub enum BuilderError {
    /// Database error.
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(String),
    /// Build processing error.
    #[error("Build processing error: {0}")]
    Processing(String),
}

/// Generic builder implementation.
#[derive(Debug, Clone)]
pub struct GenericBuilder {
    /// Dependency server URL.
    pub dep_server_url: Option<String>,
}

impl GenericBuilder {
    /// Create a new generic builder.
    pub fn new(dep_server_url: Option<String>) -> Self {
        Self { dep_server_url }
    }
}

#[async_trait]
impl Builder for GenericBuilder {
    fn kind(&self) -> &'static str {
        "generic"
    }

    async fn config(
        &self,
        _conn: &mut PgConnection,
        campaign_config: &CampaignConfig,
        _queue_item: &QueueItem,
    ) -> Result<HashMap<String, String>, BuilderError> {
        let mut config = HashMap::new();

        if let Some(generic_config) = &campaign_config.generic_build {
            if let Some(chroot) = &generic_config.chroot {
                config.insert("chroot".to_string(), chroot.clone());
            }
        }

        if let Some(dep_server_url) = &self.dep_server_url {
            config.insert("dep_server_url".to_string(), dep_server_url.clone());
        }

        Ok(config)
    }

    async fn build_env(
        &self,
        _conn: &mut PgConnection,
        _campaign_config: &CampaignConfig,
        _queue_item: &QueueItem,
    ) -> Result<HashMap<String, String>, BuilderError> {
        // Generic builds don't need special environment variables
        Ok(HashMap::new())
    }

    fn additional_colocated_branches(&self, _main_branch: &str) -> HashMap<String, String> {
        // Generic builds don't have additional branches
        HashMap::new()
    }

    fn process_result(&self, _output_dir: &Path) -> Result<BuilderResult, BuilderError> {
        // Generic builds produce a simple result
        Ok(BuilderResult::Generic)
    }
}

/// Debian builder implementation.
#[derive(Debug, Clone)]
pub struct DebianBuilder {
    /// Distribution configuration.
    pub distro_config: DistroConfig,
    /// APT archive location.
    pub apt_location: Option<String>,
    /// Dependency server URL.
    pub dep_server_url: Option<String>,
}

/// Distribution configuration for Debian builds.
#[derive(Debug, Clone)]
pub struct DistroConfig {
    /// Lintian profile to use.
    pub lintian_profile: String,
    /// Lintian tags to suppress.
    pub lintian_suppress_tag: Vec<String>,
}

impl DebianBuilder {
    /// Create a new Debian builder.
    pub fn new(
        distro_config: DistroConfig,
        apt_location: Option<String>,
        dep_server_url: Option<String>,
    ) -> Self {
        Self {
            distro_config,
            apt_location,
            dep_server_url,
        }
    }
}

#[async_trait]
impl Builder for DebianBuilder {
    fn kind(&self) -> &'static str {
        "debian"
    }

    async fn config(
        &self,
        _conn: &mut PgConnection,
        campaign_config: &CampaignConfig,
        queue_item: &QueueItem,
    ) -> Result<HashMap<String, String>, BuilderError> {
        let mut config = HashMap::new();

        // Lintian configuration
        config.insert("lintian_profile".to_string(), self.distro_config.lintian_profile.clone());
        
        if !self.distro_config.lintian_suppress_tag.is_empty() {
            config.insert(
                "lintian_suppress_tags".to_string(),
                self.distro_config.lintian_suppress_tag.join(","),
            );
        }

        // Extra repositories
        let mut extra_repositories = Vec::new();
        
        if let Some(debian_config) = &campaign_config.debian_build {
            for dist in &debian_config.extra_build_distribution {
                if let Some(apt_location) = &self.apt_location {
                    extra_repositories.push(format!(
                        "deb [trusted=yes] {} {} main",
                        apt_location, dist
                    ));
                }
            }
        }

        // Add change set specific repository
        if let Some(change_set) = &queue_item.change_set {
            if let Some(apt_location) = &self.apt_location {
                extra_repositories.push(format!(
                    "deb [trusted=yes] {} cs/{} main",
                    apt_location, change_set
                ));
            }
        }

        if !extra_repositories.is_empty() {
            config.insert("build_extra_repositories".to_string(), extra_repositories.join("\n"));
        }

        if let Some(dep_server_url) = &self.dep_server_url {
            config.insert("dep_server_url".to_string(), dep_server_url.clone());
        }

        Ok(config)
    }

    async fn build_env(
        &self,
        _conn: &mut PgConnection,
        _campaign_config: &CampaignConfig,
        _queue_item: &QueueItem,
    ) -> Result<HashMap<String, String>, BuilderError> {
        let mut env = HashMap::new();
        
        // Set Debian-specific environment variables
        env.insert("DEBIAN_FRONTEND".to_string(), "noninteractive".to_string());
        env.insert("DEB_BUILD_OPTIONS".to_string(), "nocheck".to_string());
        
        Ok(env)
    }

    fn additional_colocated_branches(&self, _main_branch: &str) -> HashMap<String, String> {
        // TODO: Implement silver-platter Debian branch picking logic
        // For now, return empty
        HashMap::new()
    }

    fn process_result(&self, output_dir: &Path) -> Result<BuilderResult, BuilderError> {
        // Look for Debian build artifacts
        let changes_summary = match crate::find_changes(output_dir) {
            Ok(summary) => summary,
            Err(_) => {
                // No changes files found, return basic Debian result
                return Ok(BuilderResult::Debian {
                    source: None,
                    build_version: None,
                    build_distribution: None,
                    changes_filenames: None,
                    lintian_result: None,
                    binary_packages: None,
                });
            }
        };

        Ok(BuilderResult::Debian {
            source: Some(changes_summary.source),
            build_version: Some(changes_summary.version.to_string()),
            build_distribution: Some(changes_summary.distribution),
            changes_filenames: Some(changes_summary.names),
            lintian_result: None, // TODO: Process lintian results
            binary_packages: Some(changes_summary.binary_packages),
        })
    }
}

/// Get the appropriate builder for a campaign configuration.
pub fn get_builder(
    campaign_config: &CampaignConfig,
    apt_archive_url: Option<String>,
    dep_server_url: Option<String>,
) -> Result<Box<dyn Builder>, BuilderError> {
    if campaign_config.debian_build.is_some() {
        // Create Debian builder with basic configuration
        let distro_config = DistroConfig {
            lintian_profile: "debian".to_string(),
            lintian_suppress_tag: vec![],
        };
        
        Ok(Box::new(DebianBuilder::new(
            distro_config,
            apt_archive_url,
            dep_server_url,
        )))
    } else {
        // Default to generic builder
        Ok(Box::new(GenericBuilder::new(dep_server_url)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generic_builder_creation() {
        let builder = GenericBuilder::new(Some("http://dep.server".to_string()));
        assert_eq!(builder.kind(), "generic");
        assert_eq!(builder.dep_server_url, Some("http://dep.server".to_string()));
    }

    #[test]
    fn test_debian_builder_creation() {
        let distro_config = DistroConfig {
            lintian_profile: "debian".to_string(),
            lintian_suppress_tag: vec!["tag1".to_string(), "tag2".to_string()],
        };
        
        let builder = DebianBuilder::new(
            distro_config,
            Some("http://apt.archive".to_string()),
            Some("http://dep.server".to_string()),
        );
        
        assert_eq!(builder.kind(), "debian");
        assert_eq!(builder.apt_location, Some("http://apt.archive".to_string()));
        assert_eq!(builder.dep_server_url, Some("http://dep.server".to_string()));
    }

    #[test]
    fn test_builder_factory() {
        let mut config = CampaignConfig {
            generic_build: None,
            debian_build: None,
        };
        
        // Test generic builder selection
        let builder = get_builder(&config, None, None).unwrap();
        assert_eq!(builder.kind(), "generic");
        
        // Test Debian builder selection
        config.debian_build = Some(DebianBuildConfig {
            base_distribution: "bullseye".to_string(),
            extra_build_distribution: vec![],
        });
        
        let builder = get_builder(&config, None, None).unwrap();
        assert_eq!(builder.kind(), "debian");
    }
}