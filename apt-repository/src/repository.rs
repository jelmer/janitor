//! Main repository generation functionality.

use crate::{
    AptRepositoryError, Compression, HashAlgorithm, HashedFile, PackageFile,
    Release, ReleaseBuilder, Result, SourceFile, DEFAULT_COMPRESSIONS, DEFAULT_HASH_ALGORITHMS,
};
use crate::hash::MultiHasher;
use std::fs;
use std::path::Path;

/// An APT repository that can generate metadata files.
#[derive(Debug, Clone)]
pub struct Repository {
    /// Origin of the repository.
    pub origin: Option<String>,
    /// Label for the repository.
    pub label: Option<String>,
    /// Suite name.
    pub suite: String,
    /// Codename.
    pub codename: Option<String>,
    /// Version.
    pub version: Option<String>,
    /// Supported architectures.
    pub architectures: Vec<String>,
    /// Repository components.
    pub components: Vec<String>,
    /// Description.
    pub description: Option<String>,
    /// Whether packages require authentication.
    pub not_automatic: bool,
    /// Whether automatic upgrades are allowed.
    pub but_automatic_upgrades: bool,
    /// Whether by-hash is supported.
    pub acquire_by_hash: bool,
    /// Compression formats to use.
    pub compressions: Vec<Compression>,
    /// Hash algorithms to use.
    pub hash_algorithms: Vec<HashAlgorithm>,
}

impl Repository {
    /// Generate repository metadata files at the specified path.
    pub fn generate_repository<P: AsRef<Path>>(
        &self,
        base_path: P,
        package_provider: &dyn PackageProvider,
        source_provider: &dyn SourceProvider,
    ) -> Result<Release> {
        let base_path = base_path.as_ref();
        let mut release_builder = ReleaseBuilder::new()
            .suite(self.suite.clone())
            .architectures(self.architectures.clone())
            .components(self.components.clone())
            .not_automatic(self.not_automatic)
            .but_automatic_upgrades(self.but_automatic_upgrades)
            .acquire_by_hash(self.acquire_by_hash);

        if let Some(ref origin) = self.origin {
            release_builder = release_builder.origin(origin.clone());
        }
        if let Some(ref label) = self.label {
            release_builder = release_builder.label(label.clone());
        }
        if let Some(ref codename) = self.codename {
            release_builder = release_builder.codename(codename.clone());
        }
        if let Some(ref version) = self.version {
            release_builder = release_builder.version(version.clone());
        }
        if let Some(ref description) = self.description {
            release_builder = release_builder.description(description.clone());
        }

        let mut release = release_builder.build()?;

        // Generate files for each component and architecture
        for component in &self.components {
            // Create component directory
            let component_dir = base_path.join(component);
            fs::create_dir_all(&component_dir)
                .map_err(|e| AptRepositoryError::DirectoryCreation(e.to_string()))?;

            // Generate binary package files
            for arch in &self.architectures {
                let arch_dir = component_dir.join(format!("binary-{}", arch));
                fs::create_dir_all(&arch_dir)
                    .map_err(|e| AptRepositoryError::DirectoryCreation(e.to_string()))?;

                let packages = package_provider.get_packages(&self.suite, component, arch)?;
                let packages_files = self.write_compressed_file(
                    &arch_dir,
                    "Packages",
                    packages.to_string().as_bytes(),
                )?;

                for file in &packages_files {
                    let relative_path = component_dir
                        .join(format!("binary-{}", arch))
                        .join(&file.path)
                        .strip_prefix(base_path)
                        .unwrap()
                        .to_string_lossy()
                        .to_string();
                    let mut hashed_file = HashedFile::new(relative_path, file.size);
                    hashed_file.hashes = file.hashes.clone();
                    release.add_file(hashed_file);
                }

                // Create by-hash directory structure if enabled
                if self.acquire_by_hash {
                    self.create_by_hash_links(&arch_dir, &packages_files)?;
                }
            }

            // Generate source package files
            let source_dir = component_dir.join("source");
            fs::create_dir_all(&source_dir)
                .map_err(|e| AptRepositoryError::DirectoryCreation(e.to_string()))?;

            let sources = source_provider.get_sources(&self.suite, component)?;
            let sources_files = self.write_compressed_file(
                &source_dir,
                "Sources",
                sources.to_string().as_bytes(),
            )?;

            for file in &sources_files {
                let relative_path = component_dir
                    .join("source")
                    .join(&file.path)
                    .strip_prefix(base_path)
                    .unwrap()
                    .to_string_lossy()
                    .to_string();
                let mut hashed_file = HashedFile::new(relative_path, file.size);
                hashed_file.hashes = file.hashes.clone();
                release.add_file(hashed_file);
            }

            // Create by-hash directory structure if enabled
            if self.acquire_by_hash {
                self.create_by_hash_links(&source_dir, &sources_files)?;
            }
        }

        // Write the Release file
        let release_content = release.to_string();
        fs::write(base_path.join("Release"), &release_content)?;

        Ok(release)
    }

    /// Write a file with multiple compression formats and return the hashed files.
    fn write_compressed_file<P: AsRef<Path>>(
        &self,
        dir: P,
        basename: &str,
        content: &[u8],
    ) -> Result<Vec<HashedFile>> {
        let dir = dir.as_ref();
        let mut files = Vec::new();

        for &compression in &self.compressions {
            let filename = format!("{}{}", basename, compression.extension());
            let filepath = dir.join(&filename);

            // Compress the content
            let compressed_content = compression.compress(content)?;

            // Calculate hashes
            let (size, hashes) = crate::hash::hash_data(&compressed_content, &self.hash_algorithms);

            // Write the file
            fs::write(&filepath, &compressed_content)?;

            // Create the hashed file record
            let mut hashed_file = HashedFile::new(filename, size);
            hashed_file.hashes = hashes;
            files.push(hashed_file);
        }

        Ok(files)
    }

    /// Create by-hash directory structure and links.
    fn create_by_hash_links<P: AsRef<Path>>(
        &self,
        base_dir: P,
        files: &[HashedFile],
    ) -> Result<()> {
        let base_dir = base_dir.as_ref();

        for algorithm in &self.hash_algorithms {
            let by_hash_dir = base_dir.join("by-hash").join(algorithm.as_str());
            fs::create_dir_all(&by_hash_dir)
                .map_err(|e| AptRepositoryError::DirectoryCreation(e.to_string()))?;

            for file in files {
                if let Some(hash) = file.get_hash(algorithm) {
                    let source_path = base_dir.join(&file.path);
                    let hash_path = by_hash_dir.join(hash);

                    // Copy the file to the by-hash location
                    fs::copy(&source_path, &hash_path)?;
                }
            }
        }

        Ok(())
    }

    /// Clean up old by-hash files, keeping only the specified number.
    pub fn cleanup_by_hash_files<P: AsRef<Path>>(
        &self,
        base_dir: P,
        keep_count: usize,
    ) -> Result<()> {
        let base_dir = base_dir.as_ref();

        for algorithm in &self.hash_algorithms {
            let by_hash_dir = base_dir.join("by-hash").join(algorithm.as_str());
            if !by_hash_dir.exists() {
                continue;
            }

            let mut entries: Vec<_> = fs::read_dir(&by_hash_dir)?
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.file_type().map_or(false, |ft| ft.is_file()))
                .collect();

            // Sort by modification time (newest first)
            entries.sort_by_key(|entry| {
                entry
                    .metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::UNIX_EPOCH)
            });
            entries.reverse();

            // Remove old files
            for entry in entries.iter().skip(keep_count) {
                fs::remove_file(entry.path())?;
            }
        }

        Ok(())
    }
}

/// Trait for providing package data to the repository generator.
pub trait PackageProvider {
    /// Get packages for a specific suite, component, and architecture.
    fn get_packages(&self, suite: &str, component: &str, architecture: &str) -> Result<PackageFile>;
}

/// Trait for providing source package data to the repository generator.
pub trait SourceProvider {
    /// Get source packages for a specific suite and component.
    fn get_sources(&self, suite: &str, component: &str) -> Result<SourceFile>;
}

/// Builder for creating Repository instances.
#[derive(Debug, Clone)]
pub struct RepositoryBuilder {
    repository: Repository,
}

impl RepositoryBuilder {
    /// Create a new Repository builder.
    pub fn new() -> Self {
        Self {
            repository: Repository {
                origin: None,
                label: None,
                suite: "stable".to_string(),
                codename: None,
                version: None,
                architectures: vec!["amd64".to_string()],
                components: vec!["main".to_string()],
                description: None,
                not_automatic: true,
                but_automatic_upgrades: true,
                acquire_by_hash: true,
                compressions: DEFAULT_COMPRESSIONS.to_vec(),
                hash_algorithms: DEFAULT_HASH_ALGORITHMS.to_vec(),
            },
        }
    }

    /// Set the origin.
    pub fn origin<S: Into<String>>(mut self, origin: S) -> Self {
        self.repository.origin = Some(origin.into());
        self
    }

    /// Set the label.
    pub fn label<S: Into<String>>(mut self, label: S) -> Self {
        self.repository.label = Some(label.into());
        self
    }

    /// Set the suite.
    pub fn suite<S: Into<String>>(mut self, suite: S) -> Self {
        self.repository.suite = suite.into();
        self
    }

    /// Set the codename.
    pub fn codename<S: Into<String>>(mut self, codename: S) -> Self {
        self.repository.codename = Some(codename.into());
        self
    }

    /// Set the version.
    pub fn version<S: Into<String>>(mut self, version: S) -> Self {
        self.repository.version = Some(version.into());
        self
    }

    /// Set the architectures.
    pub fn architectures(mut self, architectures: Vec<String>) -> Self {
        self.repository.architectures = architectures;
        self
    }

    /// Set the components.
    pub fn components(mut self, components: Vec<String>) -> Self {
        self.repository.components = components;
        self
    }

    /// Set the description.
    pub fn description<S: Into<String>>(mut self, description: S) -> Self {
        self.repository.description = Some(description.into());
        self
    }

    /// Set not automatic flag.
    pub fn not_automatic(mut self, not_automatic: bool) -> Self {
        self.repository.not_automatic = not_automatic;
        self
    }

    /// Set but automatic upgrades flag.
    pub fn but_automatic_upgrades(mut self, but_automatic_upgrades: bool) -> Self {
        self.repository.but_automatic_upgrades = but_automatic_upgrades;
        self
    }

    /// Set acquire by hash flag.
    pub fn acquire_by_hash(mut self, acquire_by_hash: bool) -> Self {
        self.repository.acquire_by_hash = acquire_by_hash;
        self
    }

    /// Set compression formats.
    pub fn compressions(mut self, compressions: Vec<Compression>) -> Self {
        self.repository.compressions = compressions;
        self
    }

    /// Set hash algorithms.
    pub fn hash_algorithms(mut self, hash_algorithms: Vec<HashAlgorithm>) -> Self {
        self.repository.hash_algorithms = hash_algorithms;
        self
    }

    /// Build the Repository.
    pub fn build(self) -> Result<Repository> {
        if self.repository.suite.is_empty() {
            return Err(AptRepositoryError::invalid_config("Suite cannot be empty"));
        }
        if self.repository.architectures.is_empty() {
            return Err(AptRepositoryError::invalid_config("At least one architecture must be specified"));
        }
        if self.repository.components.is_empty() {
            return Err(AptRepositoryError::invalid_config("At least one component must be specified"));
        }

        Ok(self.repository)
    }
}

impl Default for RepositoryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple in-memory package provider for testing.
#[derive(Debug, Clone)]
pub struct MemoryPackageProvider {
    packages: std::collections::HashMap<(String, String, String), PackageFile>,
}

impl MemoryPackageProvider {
    /// Create a new empty memory package provider.
    pub fn new() -> Self {
        Self {
            packages: std::collections::HashMap::new(),
        }
    }

    /// Add packages for a specific suite, component, and architecture.
    pub fn add_packages(&mut self, suite: &str, component: &str, architecture: &str, packages: PackageFile) {
        self.packages.insert(
            (suite.to_string(), component.to_string(), architecture.to_string()),
            packages,
        );
    }
}

impl Default for MemoryPackageProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl PackageProvider for MemoryPackageProvider {
    fn get_packages(&self, suite: &str, component: &str, architecture: &str) -> Result<PackageFile> {
        Ok(self.packages
            .get(&(suite.to_string(), component.to_string(), architecture.to_string()))
            .cloned()
            .unwrap_or_default())
    }
}

/// Simple in-memory source provider for testing.
#[derive(Debug, Clone)]
pub struct MemorySourceProvider {
    sources: std::collections::HashMap<(String, String), SourceFile>,
}

impl MemorySourceProvider {
    /// Create a new empty memory source provider.
    pub fn new() -> Self {
        Self {
            sources: std::collections::HashMap::new(),
        }
    }

    /// Add sources for a specific suite and component.
    pub fn add_sources(&mut self, suite: &str, component: &str, sources: SourceFile) {
        self.sources.insert(
            (suite.to_string(), component.to_string()),
            sources,
        );
    }
}

impl Default for MemorySourceProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceProvider for MemorySourceProvider {
    fn get_sources(&self, suite: &str, component: &str) -> Result<SourceFile> {
        Ok(self.sources
            .get(&(suite.to_string(), component.to_string()))
            .cloned()
            .unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Package, Source};
    use tempfile::TempDir;

    #[test]
    fn test_repository_builder() {
        let repo = RepositoryBuilder::new()
            .origin("Test Origin")
            .label("Test Repository")
            .suite("stable")
            .architectures(vec!["amd64".to_string(), "i386".to_string()])
            .components(vec!["main".to_string(), "contrib".to_string()])
            .build()
            .unwrap();

        assert_eq!(repo.origin, Some("Test Origin".to_string()));
        assert_eq!(repo.label, Some("Test Repository".to_string()));
        assert_eq!(repo.suite, "stable");
        assert_eq!(repo.architectures, vec!["amd64", "i386"]);
        assert_eq!(repo.components, vec!["main", "contrib"]);
    }

    #[test]
    fn test_repository_builder_validation() {
        // Empty suite should fail
        let result = RepositoryBuilder::new()
            .suite("")
            .build();
        assert!(result.is_err());

        // Empty architectures should fail
        let result = RepositoryBuilder::new()
            .architectures(vec![])
            .build();
        assert!(result.is_err());

        // Empty components should fail
        let result = RepositoryBuilder::new()
            .components(vec![])
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn test_memory_providers() {
        let mut package_provider = MemoryPackageProvider::new();
        let mut source_provider = MemorySourceProvider::new();

        // Create test data
        let mut packages = PackageFile::new();
        packages.add_package(Package::new("test-pkg", "1.0.0", "amd64", "test.deb", 1024));

        let mut sources = SourceFile::new();
        sources.add_source(Source::new("test-src", "1.0.0", "any", "pool/main/t/test"));

        package_provider.add_packages("stable", "main", "amd64", packages);
        source_provider.add_sources("stable", "main", sources);

        // Test retrieval
        let retrieved_packages = package_provider.get_packages("stable", "main", "amd64").unwrap();
        assert_eq!(retrieved_packages.len(), 1);
        assert_eq!(retrieved_packages.packages()[0].package, "test-pkg");

        let retrieved_sources = source_provider.get_sources("stable", "main").unwrap();
        assert_eq!(retrieved_sources.len(), 1);
        assert_eq!(retrieved_sources.sources()[0].package, "test-src");
    }

    #[test]
    fn test_repository_generation() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        let repo = RepositoryBuilder::new()
            .origin("Test")
            .suite("test")
            .architectures(vec!["amd64".to_string()])
            .components(vec!["main".to_string()])
            .build()
            .unwrap();

        let mut package_provider = MemoryPackageProvider::new();
        let mut packages = PackageFile::new();
        packages.add_package(Package::new("test-pkg", "1.0.0", "amd64", "test.deb", 1024));
        package_provider.add_packages("test", "main", "amd64", packages);

        let source_provider = MemorySourceProvider::new();

        let release = repo.generate_repository(repo_path, &package_provider, &source_provider).unwrap();

        // Check that the Release file was created
        assert!(repo_path.join("Release").exists());

        // Check that component directories were created
        assert!(repo_path.join("main").exists());
        assert!(repo_path.join("main/binary-amd64").exists());
        assert!(repo_path.join("main/source").exists());

        // Check that Packages files were created
        assert!(repo_path.join("main/binary-amd64/Packages").exists());
        assert!(repo_path.join("main/binary-amd64/Packages.gz").exists());

        // Check that Sources files were created
        assert!(repo_path.join("main/source/Sources").exists());
        assert!(repo_path.join("main/source/Sources.gz").exists());

        // Check release file content
        assert_eq!(release.origin, Some("Test".to_string()));
        assert_eq!(release.suite, Some("test".to_string()));
        assert!(!release.files.is_empty());
    }
}