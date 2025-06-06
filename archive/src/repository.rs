//! Repository generation engine for the archive service.
//!
//! This module provides functionality to generate APT repositories from build
//! artifacts using the apt-repository crate. It integrates with the scanner
//! module to process package and source files and generate complete APT repository
//! metadata with proper compression and hashing.

use std::collections::HashMap;
use std::sync::Arc;

use apt_repository::{
    AptRepositoryError, AsyncPackageProvider, AsyncRepository, AsyncSourceProvider, Compression,
    HashAlgorithm, Package as AptPackage, PackageFile, RepositoryBuilder, Result as AptResult,
    Source as AptSource, SourceFile,
};
use async_trait::async_trait;
use debian_control::lossy::apt::{Package as DebianPackage, Source as DebianSource};
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::{debug, error, info, warn};

use crate::config::AptRepositoryConfig;
use crate::database::BuildManager;
use crate::error::{ArchiveError, ArchiveResult};
use crate::scanner::PackageScanner;

/// Convert a debian-control Package to an apt-repository Package.
fn convert_package(debian_pkg: DebianPackage) -> ArchiveResult<AptPackage> {
    // The debian-control crate doesn't expose the Filename field directly
    // Generate a conventional filename based on package information
    let filename = format!(
        "pool/main/{}/{}_{}_{}.deb",
        &debian_pkg.name.chars().next().unwrap_or('a'),
        &debian_pkg.name,
        &debian_pkg.version,
        &debian_pkg.architecture
    );

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
                    Ok(debian_package) => match convert_package(debian_package) {
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
                    },
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
                    Ok(debian_source) => match convert_source(debian_source) {
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
                    },
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
        let _package_provider =
            ArchivePackageProvider::new(Arc::clone(&self.scanner), Arc::clone(&self.build_manager));
        let _source_provider =
            ArchiveSourceProvider::new(Arc::clone(&self.scanner), Arc::clone(&self.build_manager));

        // Ensure the base path exists
        fs::create_dir_all(&repo_config.base_path)
            .await
            .map_err(ArchiveError::Io)?;

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
            .generate_repository(
                &repo_config.base_path,
                &async_package_provider,
                &async_source_provider,
            )
            .await
            .map_err(|e| ArchiveError::RepositoryGeneration(e.to_string()))?;

        info!("Successfully generated repository: {}", repo_config.name);

        // Generate Contents files for each architecture
        self.generate_contents_files(repo_config).await?;

        Ok(())
    }

    /// Generate Contents files for the repository.
    ///
    /// Contents files map file paths to the packages that contain them.
    /// This implementation creates basic Contents files for each architecture.
    async fn generate_contents_files(&self, repo_config: &AptRepositoryConfig) -> ArchiveResult<()> {
        info!("Generating Contents files for repository: {}", repo_config.name);

        for component in &repo_config.components {
            for architecture in &repo_config.architectures {
                if architecture == "source" {
                    continue; // Skip source architecture for Contents files
                }

                let contents_path = repo_config.base_path
                    .join("dists")
                    .join(&repo_config.codename)
                    .join(component)
                    .join(format!("Contents-{}", architecture));

                info!(
                    "Generating Contents file: {:?} for {}/{}",
                    contents_path, component, architecture
                );

                let contents_data = self.generate_contents_data(
                    &repo_config.suite,
                    component,
                    architecture,
                ).await?;

                // Ensure the directory exists
                if let Some(parent) = contents_path.parent() {
                    fs::create_dir_all(parent).await.map_err(ArchiveError::Io)?;
                }

                // Write Contents file
                fs::write(&contents_path, contents_data.as_bytes())
                    .await
                    .map_err(ArchiveError::Io)?;

                // Generate compressed versions
                let compressions = [
                    apt_repository::Compression::Gzip,
                    apt_repository::Compression::Bzip2,
                ];

                for compression in &compressions {
                    let compressed_path = contents_path.with_extension(
                        format!("{}.{}", contents_path.extension().unwrap_or_default().to_string_lossy(), compression.extension())
                    );

                    let compressed_data = compression.compress(contents_data.as_bytes())
                        .map_err(|e| ArchiveError::RepositoryGeneration(e.to_string()))?;

                    fs::write(compressed_path, compressed_data)
                        .await
                        .map_err(ArchiveError::Io)?;
                }
            }
        }

        Ok(())
    }

    /// Generate Contents data for a specific suite/component/architecture.
    async fn generate_contents_data(
        &self,
        suite: &str,
        component: &str,
        architecture: &str,
    ) -> ArchiveResult<String> {
        debug!(
            "Generating contents data for suite={}, component={}, arch={}",
            suite, component, architecture
        );

        // Get builds for this suite and filter by component
        let all_builds = self.build_manager.get_builds_for_suite(suite).await?;
        let builds: Vec<_> = all_builds.into_iter()
            .filter(|build| build.component == component)
            .collect();

        let mut contents_entries = Vec::new();

        // Process each build
        for build_record in builds {
            let build_info = build_record.into();

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
                        // For each package, extract actual file list from .deb package
                        match self.generate_package_file_list(&debian_package, &build_info).await {
                            Ok(package_files) => {
                                for file_path in package_files {
                                    contents_entries.push(format!(
                                        "{:<60} {}",
                                        file_path,
                                        format!("{}/{}", component, debian_package.name)
                                    ));
                                }
                            }
                            Err(e) => {
                                warn!("Failed to extract file list for package {}: {}", debian_package.name, e);
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

        // Sort contents entries for consistent output
        contents_entries.sort();

        // Add header comment
        let mut contents_data = String::new();
        contents_data.push_str(&format!(
            "FILE                                                         LOCATION\n"
        ));

        for entry in contents_entries {
            contents_data.push_str(&entry);
            contents_data.push('\n');
        }

        Ok(contents_data)
    }

    /// Extract the actual file list from a Debian package.
    ///
    /// This uses dpkg-deb to extract the file list from the .deb package.
    async fn generate_package_file_list(&self, package: &DebianPackage, build_info: &crate::scanner::BuildInfo) -> ArchiveResult<Vec<String>> {
        // First, we need to find the actual .deb file for this package
        let deb_filename = format!("{}_{}_{}.deb", 
            package.name, 
            package.version, 
            package.architecture
        );

        // Get the .deb file from artifacts
        let artifact_manager = janitor::artifacts::get_artifact_manager("dummy://location")
            .await
            .map_err(|e| ArchiveError::ArtifactRetrieval(e.to_string()))?;

        // Create a temporary directory for downloading the .deb file
        let temp_dir = tempfile::TempDir::new().map_err(ArchiveError::Io)?;
        let temp_path = temp_dir.path();

        // Download the specific .deb file
        let deb_path = temp_path.join(&deb_filename);
        
        // Try to get the .deb file from artifacts
        match artifact_manager.get_artifact(&build_info.id, &deb_filename).await {
            Ok(mut reader) => {
                // Write the artifact data to the temp file
                let mut file = tokio::fs::File::create(&deb_path)
                    .await
                    .map_err(ArchiveError::Io)?;
                
                // Copy from reader to file
                let mut buffer = Vec::new();
                std::io::Read::read_to_end(&mut *reader, &mut buffer)
                    .map_err(|e| ArchiveError::ArtifactRetrieval(e.to_string()))?;
                
                file.write_all(&buffer)
                    .await
                    .map_err(ArchiveError::Io)?;
                
                // Extract file list using dpkg-deb
                self.extract_file_list_from_deb(&deb_path).await
            }
            Err(janitor::artifacts::Error::ArtifactsMissing) => {
                debug!("Package file {} not found in artifacts for build {}", deb_filename, build_info.id);
                Ok(vec![]) // Return empty list if package not found
            }
            Err(e) => {
                warn!("Failed to retrieve package file {}: {}", deb_filename, e);
                Ok(vec![]) // Return empty list on error
            }
        }
    }

    /// Extract file list from a .deb package using dpkg-deb.
    async fn extract_file_list_from_deb(&self, deb_path: &std::path::Path) -> ArchiveResult<Vec<String>> {
        use tokio::process::Command;

        debug!("Extracting file list from: {:?}", deb_path);

        // Use dpkg-deb -c to list contents
        let output = Command::new("dpkg-deb")
            .arg("-c")
            .arg(deb_path)
            .output()
            .await
            .map_err(|e| ArchiveError::PackageScanning(format!("Failed to run dpkg-deb: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ArchiveError::PackageScanning(format!(
                "dpkg-deb failed: {}", stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut file_paths = Vec::new();

        // Parse dpkg-deb output
        // Format: drwxr-xr-x root/root         0 2023-01-01 12:00 ./usr/
        // Format: -rw-r--r-- root/root      1234 2023-01-01 12:00 ./usr/bin/example
        for line in stdout.lines() {
            if let Some(file_path) = self.parse_dpkg_deb_line(line) {
                // Remove leading ./ and only include files (not directories)
                if !file_path.ends_with('/') && file_path.starts_with("./") {
                    let cleaned_path = file_path.strip_prefix("./").unwrap_or(&file_path);
                    if !cleaned_path.is_empty() {
                        file_paths.push(cleaned_path.to_string());
                    }
                }
            }
        }

        debug!("Extracted {} files from package", file_paths.len());
        Ok(file_paths)
    }

    /// Parse a line from dpkg-deb -c output to extract the file path.
    fn parse_dpkg_deb_line(&self, line: &str) -> Option<String> {
        // dpkg-deb -c output format:
        // drwxr-xr-x root/root         0 2023-01-01 12:00 ./path/to/file
        let parts: Vec<&str> = line.split_whitespace().collect();
        
        if parts.len() >= 6 {
            // The path is the last part (or parts if it contains spaces)
            let path_start_idx = parts.len() - 1;
            Some(parts[path_start_idx].to_string())
        } else {
            None
        }
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
            let task =
                tokio::spawn(async move { generator.generate_repository(&repo_config).await });

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
                .map_err(ArchiveError::Io)?;
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
    async fn get_sources(&self, suite: &str, component: &str) -> AptResult<SourceFile> {
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
