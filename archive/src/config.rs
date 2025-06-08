//! Configuration management for archive service.
//!
//! This module handles archive-specific configuration including repository
//! settings, GPG context setup, and artifact manager integration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::error::{ArchiveError, ArchiveResult};

/// APT repository configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AptRepositoryConfig {
    /// Repository name/identifier.
    pub name: String,
    /// Repository description.
    pub description: String,
    /// Repository origin.
    pub origin: String,
    /// Repository label.
    pub label: String,
    /// Suite name.
    pub suite: String,
    /// Codename (usually same as suite).
    pub codename: String,
    /// Supported architectures.
    pub architectures: Vec<String>,
    /// Repository components.
    pub components: Vec<String>,
    /// Base URL for repository access.
    pub base_url: String,
    /// Local base path for repository files.
    pub base_path: PathBuf,
    /// Whether to generate by-hash files.
    pub by_hash: bool,
}

/// GPG configuration for repository signing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpgConfig {
    /// GPG key ID for signing.
    pub key_id: String,
    /// GPG home directory.
    pub gpg_home: Option<PathBuf>,
    /// Passphrase for the key (if required).
    pub passphrase: Option<String>,
    /// Whether to generate detached signatures.
    pub detached_signature: bool,
    /// Whether to generate clear-text signatures.
    pub clearsign: bool,
}

/// Archive service configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveConfig {
    /// Repositories to manage.
    pub repositories: HashMap<String, AptRepositoryConfig>,
    /// GPG configuration for signing.
    pub gpg: Option<GpgConfig>,
    /// Base path for archive storage.
    pub archive_path: PathBuf,
    /// Artifact manager configuration.
    pub artifact_manager: ArtifactManagerConfig,
    /// Database configuration.
    pub database: DatabaseConfig,
    /// Cache configuration.
    pub cache: CacheConfig,
    /// Server configuration.
    pub server: ServerConfig,
}

/// Artifact manager configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactManagerConfig {
    /// Type of artifact manager (gcs, local, etc.).
    pub manager_type: String,
    /// Base URL for artifact access.
    pub base_url: String,
    /// Timeout for artifact operations (seconds).
    pub timeout: u64,
    /// Maximum concurrent downloads.
    pub max_concurrent: usize,
}

/// Database configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database connection URL.
    pub url: String,
    /// Maximum number of connections in pool.
    pub max_connections: u32,
    /// Connection timeout (seconds).
    pub connection_timeout: u64,
}

/// Cache configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Cache directory.
    pub cache_dir: PathBuf,
    /// Maximum cache size (bytes).
    pub max_size: u64,
    /// Cache entry TTL (seconds).
    pub ttl: u64,
    /// Whether to enable disk caching.
    pub enabled: bool,
}

/// Server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server bind address.
    pub bind_address: String,
    /// Server port.
    pub port: u16,
    /// Number of worker threads.
    pub workers: usize,
    /// Request timeout (seconds).
    pub request_timeout: u64,
}

impl Default for ArchiveConfig {
    fn default() -> Self {
        Self {
            repositories: HashMap::new(),
            gpg: None,
            archive_path: PathBuf::from("/tmp/janitor-archive"),
            artifact_manager: ArtifactManagerConfig::default(),
            database: DatabaseConfig::default(),
            cache: CacheConfig::default(),
            server: ServerConfig::default(),
        }
    }
}

impl Default for ArtifactManagerConfig {
    fn default() -> Self {
        Self {
            manager_type: "local".to_string(),
            base_url: "file:///tmp/artifacts".to_string(),
            timeout: 60 * 30, // 30 minutes
            max_concurrent: 4,
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgresql://localhost/janitor".to_string(),
            max_connections: 10,
            connection_timeout: 30,
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            cache_dir: PathBuf::from("/tmp/janitor-archive-cache"),
            max_size: 1024 * 1024 * 1024, // 1GB
            ttl: 3600,                    // 1 hour
            enabled: true,
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0".to_string(),
            port: 8080,
            workers: num_cpus::get(),
            request_timeout: 30,
        }
    }
}

impl AptRepositoryConfig {
    /// Create a new APT repository configuration.
    pub fn new(
        name: String,
        suite: String,
        architectures: Vec<String>,
        base_path: PathBuf,
    ) -> Self {
        Self {
            description: format!("Janitor APT repository for {}", name),
            origin: "Janitor".to_string(),
            label: "Janitor".to_string(),
            codename: suite.clone(),
            components: vec!["main".to_string()],
            base_url: format!("https://janitor.debian.net/apt/{}", name),
            by_hash: true,
            name,
            suite,
            architectures,
            base_path,
        }
    }

    /// Get the full path for a suite.
    pub fn suite_path(&self) -> PathBuf {
        self.base_path.join("dists").join(&self.suite)
    }

    /// Get the full path for a component and architecture.
    pub fn component_arch_path(&self, component: &str, arch: &str) -> PathBuf {
        self.suite_path()
            .join(component)
            .join(format!("binary-{}", arch))
    }

    /// Get the full path for source packages.
    pub fn source_path(&self, component: &str) -> PathBuf {
        self.suite_path().join(component).join("source")
    }

    /// Validate the configuration.
    pub fn validate(&self) -> ArchiveResult<()> {
        if self.name.is_empty() {
            return Err(ArchiveError::InvalidConfiguration(
                "Repository name cannot be empty".to_string(),
            ));
        }

        if self.suite.is_empty() {
            return Err(ArchiveError::InvalidConfiguration(
                "Suite name cannot be empty".to_string(),
            ));
        }

        if self.architectures.is_empty() {
            return Err(ArchiveError::InvalidConfiguration(
                "At least one architecture must be specified".to_string(),
            ));
        }

        if self.components.is_empty() {
            return Err(ArchiveError::InvalidConfiguration(
                "At least one component must be specified".to_string(),
            ));
        }

        Ok(())
    }
}

impl GpgConfig {
    /// Create a new GPG configuration.
    pub fn new(key_id: String) -> Self {
        Self {
            key_id,
            gpg_home: None,
            passphrase: None,
            detached_signature: true,
            clearsign: true,
        }
    }

    /// Get the GPG home directory or default.
    pub fn gpg_home_dir(&self) -> PathBuf {
        self.gpg_home.clone().unwrap_or_else(|| {
            std::env::var("GNUPGHOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    // Try to get home directory from HOME env var first
                    std::env::var("HOME")
                        .map(PathBuf::from)
                        .unwrap_or_else(|_| PathBuf::from("/tmp"))
                        .join(".gnupg")
                })
        })
    }
}

impl ArchiveConfig {
    /// Load configuration from a file.
    pub fn from_file(path: &std::path::Path) -> ArchiveResult<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            ArchiveError::Configuration(format!("Failed to read config file: {}", e))
        })?;

        let config: ArchiveConfig = serde_json::from_str(&content)
            .map_err(|e| ArchiveError::Configuration(format!("Failed to parse config: {}", e)))?;

        config.validate()?;
        Ok(config)
    }

    /// Save configuration to a file.
    pub fn to_file(&self, path: &std::path::Path) -> ArchiveResult<()> {
        let content = serde_json::to_string_pretty(self).map_err(|e| {
            ArchiveError::Configuration(format!("Failed to serialize config: {}", e))
        })?;

        std::fs::write(path, content).map_err(|e| {
            ArchiveError::Configuration(format!("Failed to write config file: {}", e))
        })?;

        Ok(())
    }

    /// Validate the entire configuration.
    pub fn validate(&self) -> ArchiveResult<()> {
        for (name, repo) in &self.repositories {
            repo.validate().map_err(|e| {
                ArchiveError::InvalidConfiguration(format!("Repository '{}': {}", name, e))
            })?;
        }

        if let Some(gpg) = &self.gpg {
            if gpg.key_id.is_empty() {
                return Err(ArchiveError::InvalidConfiguration(
                    "GPG key ID cannot be empty".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Get repository configuration by name.
    pub fn get_repository(&self, name: &str) -> Option<&AptRepositoryConfig> {
        self.repositories.get(name)
    }

    /// Add a repository configuration.
    pub fn add_repository(&mut self, name: String, config: AptRepositoryConfig) {
        self.repositories.insert(name, config);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_apt_repository_config_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = AptRepositoryConfig::new(
            "test-repo".to_string(),
            "test-suite".to_string(),
            vec!["amd64".to_string(), "arm64".to_string()],
            temp_dir.path().to_path_buf(),
        );

        assert_eq!(config.name, "test-repo");
        assert_eq!(config.suite, "test-suite");
        assert_eq!(config.architectures.len(), 2);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_apt_repository_paths() {
        let temp_dir = TempDir::new().unwrap();
        let config = AptRepositoryConfig::new(
            "test-repo".to_string(),
            "test-suite".to_string(),
            vec!["amd64".to_string()],
            temp_dir.path().to_path_buf(),
        );

        let suite_path = config.suite_path();
        assert!(suite_path.ends_with("dists/test-suite"));

        let arch_path = config.component_arch_path("main", "amd64");
        assert!(arch_path.ends_with("dists/test-suite/main/binary-amd64"));

        let source_path = config.source_path("main");
        assert!(source_path.ends_with("dists/test-suite/main/source"));
    }

    #[test]
    fn test_gpg_config_creation() {
        let gpg_config = GpgConfig::new("12345678".to_string());

        assert_eq!(gpg_config.key_id, "12345678");
        assert!(gpg_config.detached_signature);
        assert!(gpg_config.clearsign);
    }

    #[test]
    fn test_archive_config_validation() {
        let mut config = ArchiveConfig::default();

        // Should validate with empty repositories
        assert!(config.validate().is_ok());

        // Add valid repository
        let repo_config = AptRepositoryConfig::new(
            "test".to_string(),
            "test-suite".to_string(),
            vec!["amd64".to_string()],
            PathBuf::from("/tmp/test"),
        );
        config.add_repository("test".to_string(), repo_config);

        assert!(config.validate().is_ok());
        assert!(config.get_repository("test").is_some());
    }

    #[test]
    fn test_config_file_operations() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let mut config = ArchiveConfig::default();
        let repo_config = AptRepositoryConfig::new(
            "test".to_string(),
            "test-suite".to_string(),
            vec!["amd64".to_string()],
            PathBuf::from("/tmp/test"),
        );
        config.add_repository("test".to_string(), repo_config);

        // Test saving
        assert!(config.to_file(&config_path).is_ok());
        assert!(config_path.exists());

        // Test loading
        let loaded_config = ArchiveConfig::from_file(&config_path).unwrap();
        assert_eq!(loaded_config.repositories.len(), 1);
        assert!(loaded_config.get_repository("test").is_some());
    }
}
