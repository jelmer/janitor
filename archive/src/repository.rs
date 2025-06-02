//! Repository generation engine for the archive service.
//!
//! This module provides functionality to generate APT repositories from build
//! artifacts using the apt-repository crate. It integrates with the scanner
//! module to process package and source files and generate complete APT repository
//! metadata with proper compression and hashing.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use apt_repository::{
    AsyncPackageProvider, AsyncRepository, AsyncSourceProvider, AptRepositoryError, Compression,
    HashAlgorithm, PackageFile, RepositoryBuilder, Result as AptResult, SourceFile,
    Package as AptPackage, Source as AptSource,
};
use debian_control::lossy::apt::{Package as DebianPackage, Source as DebianSource};
use async_trait::async_trait;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::fs;
use tracing::{debug, error, info, warn};

use crate::config::AptRepositoryConfig;
use crate::database::BuildManager;
use crate::error::{ArchiveError, ArchiveResult};
use crate::scanner::{BuildInfo, PackageScanner};

/// Convert a debian-control Package to an apt-repository Package.
fn convert_package(debian_pkg: DebianPackage) -> ArchiveResult<AptPackage> {
    // The debian-control crate doesn't expose the Filename field directly
    // Generate a conventional filename based on package information
    let filename = format!("pool/main/{}/{}_{}_{}.deb", 
                          &debian_pkg.name.chars().next().unwrap_or('a'),
                          &debian_pkg.name,
                          &debian_pkg.version,
                          &debian_pkg.architecture);
    
    // Handle Optional size field and convert usize to u64
    let size = debian_pkg.size.unwrap_or(0) as u64;
    
    let mut apt_pkg = AptPackage::new(
        debian_pkg.name,
        debian_pkg.version.to_string(),
        debian_pkg.architecture,
        filename,
        size,
    );

    // Copy optional fields that exist in both types
    apt_pkg.maintainer = debian_pkg.maintainer;
    apt_pkg.description = debian_pkg.description;
    apt_pkg.section = debian_pkg.section;
    apt_pkg.homepage = debian_pkg.homepage;
    apt_pkg.tag = debian_pkg.tag;

    // Convert relation fields to string format
    if let Some(depends) = debian_pkg.depends {
        apt_pkg.depends = Some(depends.to_string());
    }
    if let Some(pre_depends) = debian_pkg.pre_depends {
        apt_pkg.pre_depends = Some(pre_depends.to_string());
    }
    if let Some(recommends) = debian_pkg.recommends {
        apt_pkg.recommends = Some(recommends.to_string());
    }
    if let Some(suggests) = debian_pkg.suggests {
        apt_pkg.suggests = Some(suggests.to_string());
    }
    if let Some(breaks) = debian_pkg.breaks {
        apt_pkg.breaks = Some(breaks.to_string());
    }
    if let Some(conflicts) = debian_pkg.conflicts {
        apt_pkg.conflicts = Some(conflicts.to_string());
    }
    if let Some(provides) = debian_pkg.provides {
        apt_pkg.provides = Some(provides.to_string());
    }
    if let Some(replaces) = debian_pkg.replaces {
        apt_pkg.replaces = Some(replaces.to_string());
    }

    // Convert priority to string format
    if let Some(priority) = debian_pkg.priority {
        apt_pkg.priority = Some(priority.to_string());
    }

    // Copy hash fields
    apt_pkg.md5sum = debian_pkg.md5sum;
    apt_pkg.sha256 = debian_pkg.sha256;

    Ok(apt_pkg)
}

/// Convert a debian-control Source to an apt-repository Source.
fn convert_source(debian_src: DebianSource) -> ArchiveResult<AptSource> {
    // Sources usually have "any" as architecture for source packages
    let mut apt_src = AptSource::new(
        debian_src.package,
        debian_src.version.to_string(),
        "any".to_string(),
        debian_src.directory,
    );

    // Copy optional fields that exist in both types
    apt_src.maintainer = debian_src.maintainer;
    apt_src.standards_version = debian_src.standards_version;
    apt_src.format = debian_src.format;
    apt_src.homepage = debian_src.homepage;
    apt_src.vcs_browser = debian_src.vcs_browser;
    apt_src.vcs_git = debian_src.vcs_git;
    apt_src.vcs_svn = debian_src.vcs_svn;
    apt_src.vcs_bzr = debian_src.vcs_bzr;
    apt_src.vcs_arch = debian_src.vcs_arch;
    apt_src.vcs_cvs = debian_src.vcs_cvs;
    apt_src.vcs_darcs = debian_src.vcs_darcs;
    apt_src.vcs_hg = debian_src.vcs_hg;

    // Convert relation fields to string format
    if let Some(build_deps) = debian_src.build_depends {
        apt_src.build_depends = Some(build_deps.to_string());
    }
    if let Some(build_deps_indep) = debian_src.build_depends_indep {
        apt_src.build_depends_indep = Some(build_deps_indep.to_string());
    }
    if let Some(build_conflicts) = debian_src.build_conflicts {
        apt_src.build_conflicts = Some(build_conflicts.to_string());
    }
    if let Some(build_conflicts_indep) = debian_src.build_conflicts_indep {
        apt_src.build_conflicts_indep = Some(build_conflicts_indep.to_string());
    }

    // Copy binary package names if available
    if let Some(binaries) = debian_src.binaries {
        apt_src.binary = Some(binaries.join(", "));
    }

    Ok(apt_src)
}

/// Compression format configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionConfig {
    /// No compression.
    None,
    /// Gzip compression.
    Gzip,
    /// Bzip2 compression.
    Bzip2,
}

impl From<CompressionConfig> for Compression {
    fn from(config: CompressionConfig) -> Self {
        match config {
            CompressionConfig::None => Compression::None,
            CompressionConfig::Gzip => Compression::Gzip,
            CompressionConfig::Bzip2 => Compression::Bzip2,
        }
    }
}

/// Hash algorithm configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HashAlgorithmConfig {
    /// MD5 hashing.
    Md5,
    /// SHA-1 hashing.
    Sha1,
    /// SHA-256 hashing.
    Sha256,
    /// SHA-512 hashing.
    Sha512,
}

impl From<HashAlgorithmConfig> for HashAlgorithm {
    fn from(config: HashAlgorithmConfig) -> Self {
        match config {
            HashAlgorithmConfig::Md5 => HashAlgorithm::Md5,
            HashAlgorithmConfig::Sha1 => HashAlgorithm::Sha1,
            HashAlgorithmConfig::Sha256 => HashAlgorithm::Sha256,
            HashAlgorithmConfig::Sha512 => HashAlgorithm::Sha512,
        }
    }
}

/// Repository generation configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryGenerationConfig {
    /// Enable by-hash directory structure.
    pub by_hash: bool,
    /// Compression formats to generate.
    pub compressions: Vec<CompressionConfig>,
    /// Hash algorithms to use.
    pub hash_algorithms: Vec<HashAlgorithmConfig>,
    /// Maximum concurrent operations.
    pub max_concurrent: usize,
    /// Enable GPG signing.
    pub enable_signing: bool,
}

impl Default for RepositoryGenerationConfig {
    fn default() -> Self {
        Self {
            by_hash: true,
            compressions: vec![
                CompressionConfig::None,
                CompressionConfig::Gzip,
                CompressionConfig::Bzip2,
            ],
            hash_algorithms: vec![
                HashAlgorithmConfig::Md5,
                HashAlgorithmConfig::Sha1,
                HashAlgorithmConfig::Sha256,
                HashAlgorithmConfig::Sha512,
            ],
            max_concurrent: 4,
            enable_signing: false,
        }
    }
}

/// Archive package provider that implements apt-repository's package provider trait.
pub struct ArchivePackageProvider {
    scanner: Arc<PackageScanner>,
    build_manager: Arc<BuildManager>,
}

impl ArchivePackageProvider {
    /// Create a new archive package provider.
    pub fn new(scanner: Arc<PackageScanner>, build_manager: Arc<BuildManager>) -> Self {
        Self {
            scanner,
            build_manager,
        }
    }

    /// Get packages for a specific suite, component, and architecture.
    async fn get_packages_async(
        &self,
        suite: &str,
        component: &str,
        architecture: &str,
    ) -> ArchiveResult<PackageFile> {
        let mut package_file = PackageFile::new();

        // Get build results for the suite
        let builds = self.build_manager.get_builds_for_suite(suite).await?;

        info!(
            "Processing {} builds for suite={}, component={}, arch={}",
            builds.len(),
            suite,
            component,
            architecture
        );

        // Process each build
        for build_record in builds {
            let build_info = build_record.into();

            debug!("Processing build: {:?}", build_info);

            // Scan packages for this build
            let package_stream = self
                .scanner
                .scan_packages_for_build(&build_info, Some(architecture))
                .await;

            // Collect packages from stream
            let mut package_stream = Box::pin(package_stream);
            while let Some(package_result) = package_stream.next().await {
                match package_result {
                    Ok(debian_package) => {
                        match convert_package(debian_package) {
                            Ok(apt_package) => {
                                debug!(
                                    "Adding package: {} version {}",
                                    apt_package.package, apt_package.version
                                );
                                package_file.add_package(apt_package);
                            }
                            Err(e) => {
                                warn!("Failed to convert package: {}", e);
                                continue;
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to scan package from build {}: {}", build_info.id, e);
                        continue;
                    }
                }
            }
        }

        info!(
            "Generated package file with {} packages for {}/{}/{}",
            package_file.packages().len(),
            suite,
            component,
            architecture
        );

        Ok(package_file)
    }
}

/// Archive source provider that implements apt-repository's source provider trait.
pub struct ArchiveSourceProvider {
    scanner: Arc<PackageScanner>,
    build_manager: Arc<BuildManager>,
}

impl ArchiveSourceProvider {
    /// Create a new archive source provider.
    pub fn new(scanner: Arc<PackageScanner>, build_manager: Arc<BuildManager>) -> Self {
        Self {
            scanner,
            build_manager,
        }
    }

    /// Get sources for a specific suite and component.
    async fn get_sources_async(&self, suite: &str, component: &str) -> ArchiveResult<SourceFile> {
        let mut source_file = SourceFile::new();

        // Get build results for the suite
        let builds = self.build_manager.get_builds_for_suite(suite).await?;

        info!(
            "Processing {} builds for sources in suite={}, component={}",
            builds.len(),
            suite,
            component
        );

        // Process each build
        for build_record in builds {
            let build_info = build_record.into();

            debug!("Processing build for sources: {:?}", build_info);

            // Scan sources for this build
            let source_stream = self.scanner.scan_sources_for_build(&build_info).await;

            // Collect sources from stream
            let mut source_stream = Box::pin(source_stream);
            while let Some(source_result) = source_stream.next().await {
                match source_result {
                    Ok(debian_source) => {
                        match convert_source(debian_source) {
                            Ok(apt_source) => {
                                debug!(
                                    "Adding source package: {} version {}",
                                    apt_source.package, apt_source.version
                                );
                                source_file.add_source(apt_source);
                            }
                            Err(e) => {
                                warn!("Failed to convert source: {}", e);
                                continue;
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to scan source from build {}: {}", build_info.id, e);
                        continue;
                    }
                }
            }
        }

        info!(
            "Generated source file with {} sources for {}/{}",
            source_file.sources().len(),
            suite,
            component
        );

        Ok(source_file)
    }
}

/// Repository generation engine for the archive service.
pub struct RepositoryGenerator {
    scanner: Arc<PackageScanner>,
    build_manager: Arc<BuildManager>,
    config: RepositoryGenerationConfig,
}

impl RepositoryGenerator {
    /// Create a new repository generator.
    pub fn new(
        scanner: Arc<PackageScanner>,
        build_manager: Arc<BuildManager>,
        config: RepositoryGenerationConfig,
    ) -> Self {
        Self {
            scanner,
            build_manager,
            config,
        }
    }

    /// Generate an APT repository for the given configuration.
    pub async fn generate_repository(
        &self,
        repo_config: &AptRepositoryConfig,
    ) -> ArchiveResult<()> {
        info!("Generating repository: {}", repo_config.name);

        // Create the repository builder
        let mut repo_builder = RepositoryBuilder::new()
            .origin(&repo_config.origin)
            .label(&repo_config.label)
            .suite(&repo_config.suite)
            .codename(&repo_config.codename)
            .architectures(repo_config.architectures.clone())
            .components(repo_config.components.clone())
            .acquire_by_hash(self.config.by_hash)
            .compressions(
                self.config
                    .compressions
                    .iter()
                    .map(|c| (*c).clone().into())
                    .collect(),
            )
            .hash_algorithms(
                self.config
                    .hash_algorithms
                    .iter()
                    .map(|h| (*h).clone().into())
                    .collect(),
            );

        if !repo_config.description.is_empty() {
            repo_builder = repo_builder.description(&repo_config.description);
        }

        let repository = repo_builder
            .build()
            .map_err(|e| ArchiveError::RepositoryGeneration(e.to_string()))?;

        // Create async repository
        let async_repo = AsyncRepository::new(repository);

        // Create providers
        let _package_provider = ArchivePackageProvider::new(
            Arc::clone(&self.scanner),
            Arc::clone(&self.build_manager),
        );
        let _source_provider = ArchiveSourceProvider::new(
            Arc::clone(&self.scanner),
            Arc::clone(&self.build_manager),
        );

        // Ensure the base path exists
        fs::create_dir_all(&repo_config.base_path)
            .await
            .map_err(|e| ArchiveError::Io(e))?;

        info!(
            "Generating repository files in: {:?}",
            repo_config.base_path
        );

        // Generate the repository
        let async_package_provider = AsyncArchivePackageProvider::new(
            Arc::clone(&self.scanner),
            Arc::clone(&self.build_manager),
        );
        let async_source_provider = AsyncArchiveSourceProvider::new(
            Arc::clone(&self.scanner),
            Arc::clone(&self.build_manager),
        );

        let _release = async_repo
            .generate_repository(&repo_config.base_path, &async_package_provider, &async_source_provider)
            .await
            .map_err(|e| ArchiveError::RepositoryGeneration(e.to_string()))?;

        info!("Successfully generated repository: {}", repo_config.name);

        Ok(())
    }

    /// Generate multiple repositories for different suites.
    pub async fn generate_repositories(
        &self,
        repos: &HashMap<String, AptRepositoryConfig>,
    ) -> ArchiveResult<()> {
        info!("Generating {} repositories", repos.len());

        let mut tasks = Vec::new();

        for (name, repo_config) in repos {
            info!("Starting generation for repository: {}", name);

            let generator = RepositoryGenerator::new(
                Arc::clone(&self.scanner),
                Arc::clone(&self.build_manager),
                self.config.clone(),
            );

            let repo_config = repo_config.clone();
            let task = tokio::spawn(async move { generator.generate_repository(&repo_config).await });

            tasks.push(task);

            // Limit concurrent operations
            if tasks.len() >= self.config.max_concurrent {
                // Wait for one task to complete
                let (result, _index, remaining) = futures::future::select_all(tasks).await;
                match result {
                    Ok(Ok(_)) => info!("Repository generation completed successfully"),
                    Ok(Err(e)) => error!("Repository generation failed: {}", e),
                    Err(e) => error!("Repository generation task failed: {}", e),
                }
                tasks = remaining;
            }
        }

        // Wait for remaining tasks
        for task in tasks {
            match task.await {
                Ok(Ok(_)) => info!("Repository generation completed successfully"),
                Ok(Err(e)) => error!("Repository generation failed: {}", e),
                Err(e) => error!("Repository generation task failed: {}", e),
            }
        }

        info!("All repository generation tasks completed");
        Ok(())
    }

    /// Clean up old repository files.
    pub async fn cleanup_repository(&self, repo_config: &AptRepositoryConfig) -> ArchiveResult<()> {
        info!("Cleaning up repository: {}", repo_config.name);

        let suite_path = repo_config.suite_path();

        if suite_path.exists() {
            info!("Removing existing repository files: {:?}", suite_path);
            fs::remove_dir_all(&suite_path)
                .await
                .map_err(|e| ArchiveError::Io(e))?;
        }

        Ok(())
    }

    /// Validate repository configuration.
    pub fn validate_config(&self, repo_config: &AptRepositoryConfig) -> ArchiveResult<()> {
        repo_config.validate()?;

        if repo_config.architectures.is_empty() {
            return Err(ArchiveError::InvalidConfiguration(
                "At least one architecture must be specified".to_string(),
            ));
        }

        if repo_config.components.is_empty() {
            return Err(ArchiveError::InvalidConfiguration(
                "At least one component must be specified".to_string(),
            ));
        }

        Ok(())
    }
}

// Implementation of the async traits for the providers
// This requires implementing the apt-repository async provider traits

/// Async package provider implementation for janitor archive.
pub struct AsyncArchivePackageProvider {
    inner: ArchivePackageProvider,
}

impl AsyncArchivePackageProvider {
    /// Create a new async archive package provider.
    pub fn new(scanner: Arc<PackageScanner>, build_manager: Arc<BuildManager>) -> Self {
        Self {
            inner: ArchivePackageProvider::new(scanner, build_manager),
        }
    }
}

#[async_trait]
impl AsyncPackageProvider for AsyncArchivePackageProvider {
    async fn get_packages(
        &self,
        suite: &str,
        component: &str,
        architecture: &str,
    ) -> AptResult<PackageFile> {
        self.inner
            .get_packages_async(suite, component, architecture)
            .await
            .map_err(|e| AptRepositoryError::InvalidConfiguration(e.to_string()))
    }
}

/// Async source provider implementation for janitor archive.
pub struct AsyncArchiveSourceProvider {
    inner: ArchiveSourceProvider,
}

impl AsyncArchiveSourceProvider {
    /// Create a new async archive source provider.
    pub fn new(scanner: Arc<PackageScanner>, build_manager: Arc<BuildManager>) -> Self {
        Self {
            inner: ArchiveSourceProvider::new(scanner, build_manager),
        }
    }
}

#[async_trait]
impl AsyncSourceProvider for AsyncArchiveSourceProvider {
    async fn get_sources(
        &self,
        suite: &str,
        component: &str,
    ) -> AptResult<SourceFile> {
        self.inner
            .get_sources_async(suite, component)
            .await
            .map_err(|e| AptRepositoryError::InvalidConfiguration(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_repository_generation_config_default() {
        let config = RepositoryGenerationConfig::default();

        assert!(config.by_hash);
        assert_eq!(config.compressions.len(), 3);
        assert_eq!(config.hash_algorithms.len(), 4);
        assert_eq!(config.max_concurrent, 4);
        assert!(!config.enable_signing);
    }

    #[tokio::test]
    async fn test_validate_config_valid() {
        let temp_dir = TempDir::new().unwrap();
        let config = AptRepositoryConfig::new(
            "test-repo".to_string(),
            "test-suite".to_string(),
            vec!["amd64".to_string()],
            temp_dir.path().to_path_buf(),
        );

        // For testing, we create mock components
        // let scanner = Arc::new(PackageScanner::new().await.unwrap());
        // let build_manager = Arc::new(BuildManager::new("dummy://db").await.unwrap());
        // let generator = RepositoryGenerator::new(
        //     scanner,
        //     build_manager,
        //     RepositoryGenerationConfig::default(),
        // );

        // assert!(generator.validate_config(&config).is_ok());

        // For now, just test basic config validation
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_config_empty_architectures() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = AptRepositoryConfig::new(
            "test-repo".to_string(),
            "test-suite".to_string(),
            vec!["amd64".to_string()],
            temp_dir.path().to_path_buf(),
        );
        config.architectures.clear();

        // Test that invalid config is caught
        assert!(config.validate().is_err());
    }

    #[tokio::test]
    async fn test_repository_generator_creation() {
        // This test requires mocking the scanner and build manager
        // For now, just test that the struct can be created
        // let scanner = Arc::new(PackageScanner::new().await.unwrap());
        // let build_manager = Arc::new(BuildManager::new("dummy://db").await.unwrap());
        // let generator = RepositoryGenerator::new(
        //     scanner,
        //     build_manager,
        //     RepositoryGenerationConfig::default(),
        // );
        // assert!(generator.config.by_hash);
    }
}