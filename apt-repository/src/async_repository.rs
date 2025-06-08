//! Async repository generation functionality.

#[cfg(feature = "async")]
use crate::{
    AptRepositoryError, HashedFile, PackageFile, Release, ReleaseBuilder, Repository, Result,
    SourceFile,
};
use std::path::Path;
use tokio::fs;

/// Async version of the Repository with tokio support.
#[derive(Debug, Clone)]
pub struct AsyncRepository {
    inner: Repository,
}

impl AsyncRepository {
    /// Create a new async repository from a regular repository.
    pub fn new(repository: Repository) -> Self {
        Self { inner: repository }
    }

    /// Generate repository metadata files at the specified path asynchronously.
    pub async fn generate_repository<
        P: AsRef<Path>,
        PP: AsyncPackageProvider,
        SP: AsyncSourceProvider,
    >(
        &self,
        base_path: P,
        package_provider: &PP,
        source_provider: &SP,
    ) -> Result<Release> {
        let base_path = base_path.as_ref();
        let mut release_builder = ReleaseBuilder::new()
            .suite(self.inner.suite.clone())
            .architectures(self.inner.architectures.clone())
            .components(self.inner.components.clone())
            .not_automatic(self.inner.not_automatic)
            .but_automatic_upgrades(self.inner.but_automatic_upgrades)
            .acquire_by_hash(self.inner.acquire_by_hash);

        if let Some(ref origin) = self.inner.origin {
            release_builder = release_builder.origin(origin.clone());
        }
        if let Some(ref label) = self.inner.label {
            release_builder = release_builder.label(label.clone());
        }
        if let Some(ref codename) = self.inner.codename {
            release_builder = release_builder.codename(codename.clone());
        }
        if let Some(ref version) = self.inner.version {
            release_builder = release_builder.version(version.clone());
        }
        if let Some(ref description) = self.inner.description {
            release_builder = release_builder.description(description.clone());
        }

        let mut release = release_builder.build()?;

        // Generate files for each component and architecture
        for component in &self.inner.components {
            // Create component directory
            let component_dir = base_path.join(component);
            fs::create_dir_all(&component_dir)
                .await
                .map_err(|e| AptRepositoryError::DirectoryCreation(e.to_string()))?;

            // Generate binary package files
            for arch in &self.inner.architectures {
                let arch_dir = component_dir.join(format!("binary-{}", arch));
                fs::create_dir_all(&arch_dir)
                    .await
                    .map_err(|e| AptRepositoryError::DirectoryCreation(e.to_string()))?;

                let packages = package_provider
                    .get_packages(&self.inner.suite, component, arch)
                    .await?;
                let packages_files = self
                    .write_compressed_file_async(
                        &arch_dir,
                        "Packages",
                        packages.to_string().as_bytes(),
                    )
                    .await?;

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
                if self.inner.acquire_by_hash {
                    self.create_by_hash_links_async(&arch_dir, &packages_files)
                        .await?;
                }
            }

            // Generate source package files
            let source_dir = component_dir.join("source");
            fs::create_dir_all(&source_dir)
                .await
                .map_err(|e| AptRepositoryError::DirectoryCreation(e.to_string()))?;

            let sources = source_provider
                .get_sources(&self.inner.suite, component)
                .await?;
            let sources_files = self
                .write_compressed_file_async(&source_dir, "Sources", sources.to_string().as_bytes())
                .await?;

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
            if self.inner.acquire_by_hash {
                self.create_by_hash_links_async(&source_dir, &sources_files)
                    .await?;
            }
        }

        // Write the Release file
        let release_content = release.to_string();
        fs::write(base_path.join("Release"), &release_content).await?;

        Ok(release)
    }

    /// Write a file with multiple compression formats asynchronously.
    async fn write_compressed_file_async<P: AsRef<Path>>(
        &self,
        dir: P,
        basename: &str,
        content: &[u8],
    ) -> Result<Vec<HashedFile>> {
        let dir = dir.as_ref();
        let mut files = Vec::new();

        for &compression in &self.inner.compressions {
            let filename = format!("{}{}", basename, compression.extension());
            let filepath = dir.join(&filename);

            // Compress the content
            let compressed_content = compression.compress(content)?;

            // Calculate hashes
            let (size, hashes) =
                crate::hash::hash_data(&compressed_content, &self.inner.hash_algorithms);

            // Write the file asynchronously
            fs::write(&filepath, &compressed_content).await?;

            // Create the hashed file record
            let mut hashed_file = HashedFile::new(filename, size);
            hashed_file.hashes = hashes;
            files.push(hashed_file);
        }

        Ok(files)
    }

    /// Create by-hash directory structure and links asynchronously.
    async fn create_by_hash_links_async<P: AsRef<Path>>(
        &self,
        base_dir: P,
        files: &[HashedFile],
    ) -> Result<()> {
        let base_dir = base_dir.as_ref();

        for algorithm in &self.inner.hash_algorithms {
            let by_hash_dir = base_dir.join("by-hash").join(algorithm.as_str());
            fs::create_dir_all(&by_hash_dir)
                .await
                .map_err(|e| AptRepositoryError::DirectoryCreation(e.to_string()))?;

            for file in files {
                if let Some(hash) = file.get_hash(algorithm) {
                    let source_path = base_dir.join(&file.path);
                    let hash_path = by_hash_dir.join(hash);

                    // Copy the file to the by-hash location
                    fs::copy(&source_path, &hash_path).await?;
                }
            }
        }

        Ok(())
    }

    /// Clean up old by-hash files asynchronously.
    pub async fn cleanup_by_hash_files_async<P: AsRef<Path>>(
        &self,
        base_dir: P,
        keep_count: usize,
    ) -> Result<()> {
        let base_dir = base_dir.as_ref();

        for algorithm in &self.inner.hash_algorithms {
            let by_hash_dir = base_dir.join("by-hash").join(algorithm.as_str());
            if !fs::try_exists(&by_hash_dir).await.unwrap_or(false) {
                continue;
            }

            let mut read_dir = fs::read_dir(&by_hash_dir).await?;
            let mut entries = Vec::new();

            while let Some(entry) = read_dir.next_entry().await? {
                if entry.file_type().await?.is_file() {
                    entries.push(entry);
                }
            }

            // Sort by modification time (newest first)
            let mut entries_with_metadata = Vec::new();
            for entry in entries {
                let metadata = entry.metadata().await?;
                let modified = metadata.modified()?;
                entries_with_metadata.push((entry, modified));
            }

            entries_with_metadata.sort_by(|a, b| b.1.cmp(&a.1));

            // Remove old files
            for (entry, _) in entries_with_metadata.iter().skip(keep_count) {
                fs::remove_file(entry.path()).await?;
            }
        }

        Ok(())
    }

    /// Get the inner repository.
    pub fn inner(&self) -> &Repository {
        &self.inner
    }

    /// Get a mutable reference to the inner repository.
    pub fn inner_mut(&mut self) -> &mut Repository {
        &mut self.inner
    }
}

/// Async trait for providing package data to the repository generator.
#[async_trait::async_trait]
pub trait AsyncPackageProvider: Send + Sync {
    /// Get packages for a specific suite, component, and architecture.
    async fn get_packages(
        &self,
        suite: &str,
        component: &str,
        architecture: &str,
    ) -> Result<PackageFile>;
}

/// Async trait for providing source package data to the repository generator.
#[async_trait::async_trait]
pub trait AsyncSourceProvider: Send + Sync {
    /// Get source packages for a specific suite and component.
    async fn get_sources(&self, suite: &str, component: &str) -> Result<SourceFile>;
}

/// Async in-memory package provider for testing.
#[derive(Debug, Clone)]
pub struct AsyncMemoryPackageProvider {
    packages: std::collections::HashMap<(String, String, String), PackageFile>,
}

impl AsyncMemoryPackageProvider {
    /// Create a new empty async memory package provider.
    pub fn new() -> Self {
        Self {
            packages: std::collections::HashMap::new(),
        }
    }

    /// Add packages for a specific suite, component, and architecture.
    pub fn add_packages(
        &mut self,
        suite: &str,
        component: &str,
        architecture: &str,
        packages: PackageFile,
    ) {
        self.packages.insert(
            (
                suite.to_string(),
                component.to_string(),
                architecture.to_string(),
            ),
            packages,
        );
    }
}

impl Default for AsyncMemoryPackageProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl AsyncPackageProvider for AsyncMemoryPackageProvider {
    async fn get_packages(
        &self,
        suite: &str,
        component: &str,
        architecture: &str,
    ) -> Result<PackageFile> {
        Ok(self
            .packages
            .get(&(
                suite.to_string(),
                component.to_string(),
                architecture.to_string(),
            ))
            .cloned()
            .unwrap_or_default())
    }
}

/// Async in-memory source provider for testing.
#[derive(Debug, Clone)]
pub struct AsyncMemorySourceProvider {
    sources: std::collections::HashMap<(String, String), SourceFile>,
}

impl AsyncMemorySourceProvider {
    /// Create a new empty async memory source provider.
    pub fn new() -> Self {
        Self {
            sources: std::collections::HashMap::new(),
        }
    }

    /// Add sources for a specific suite and component.
    pub fn add_sources(&mut self, suite: &str, component: &str, sources: SourceFile) {
        self.sources
            .insert((suite.to_string(), component.to_string()), sources);
    }
}

impl Default for AsyncMemorySourceProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl AsyncSourceProvider for AsyncMemorySourceProvider {
    async fn get_sources(&self, suite: &str, component: &str) -> Result<SourceFile> {
        Ok(self
            .sources
            .get(&(suite.to_string(), component.to_string()))
            .cloned()
            .unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Package, RepositoryBuilder, Source};
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_async_repository_generation() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        let repo = RepositoryBuilder::new()
            .origin("Test")
            .suite("test")
            .architectures(vec!["amd64".to_string()])
            .components(vec!["main".to_string()])
            .build()
            .unwrap();

        let async_repo = AsyncRepository::new(repo);

        let mut package_provider = AsyncMemoryPackageProvider::new();
        let mut packages = PackageFile::new();
        packages.add_package(Package::new("test-pkg", "1.0.0", "amd64", "test.deb", 1024));
        package_provider.add_packages("test", "main", "amd64", packages);

        let source_provider = AsyncMemorySourceProvider::new();

        let release = async_repo
            .generate_repository(repo_path, &package_provider, &source_provider)
            .await
            .unwrap();

        // Check that the Release file was created
        assert!(fs::try_exists(repo_path.join("Release")).await.unwrap());

        // Check that component directories were created
        assert!(fs::try_exists(repo_path.join("main")).await.unwrap());
        assert!(fs::try_exists(repo_path.join("main/binary-amd64"))
            .await
            .unwrap());
        assert!(fs::try_exists(repo_path.join("main/source")).await.unwrap());

        // Check that Packages files were created
        assert!(fs::try_exists(repo_path.join("main/binary-amd64/Packages"))
            .await
            .unwrap());
        assert!(
            fs::try_exists(repo_path.join("main/binary-amd64/Packages.gz"))
                .await
                .unwrap()
        );

        // Check that Sources files were created
        assert!(fs::try_exists(repo_path.join("main/source/Sources"))
            .await
            .unwrap());
        assert!(fs::try_exists(repo_path.join("main/source/Sources.gz"))
            .await
            .unwrap());

        // Check release file content
        assert_eq!(release.origin, Some("Test".to_string()));
        assert_eq!(release.suite, Some("test".to_string()));
        assert!(!release.files.is_empty());
    }

    #[tokio::test]
    async fn test_async_providers() {
        let mut package_provider = AsyncMemoryPackageProvider::new();
        let mut source_provider = AsyncMemorySourceProvider::new();

        // Create test data
        let mut packages = PackageFile::new();
        packages.add_package(Package::new("test-pkg", "1.0.0", "amd64", "test.deb", 1024));

        let mut sources = SourceFile::new();
        sources.add_source(Source::new("test-src", "1.0.0", "any", "pool/main/t/test"));

        package_provider.add_packages("stable", "main", "amd64", packages);
        source_provider.add_sources("stable", "main", sources);

        // Test retrieval
        let retrieved_packages = package_provider
            .get_packages("stable", "main", "amd64")
            .await
            .unwrap();
        assert_eq!(retrieved_packages.len(), 1);
        assert_eq!(retrieved_packages.packages()[0].package, "test-pkg");

        let retrieved_sources = source_provider.get_sources("stable", "main").await.unwrap();
        assert_eq!(retrieved_sources.len(), 1);
        assert_eq!(retrieved_sources.sources()[0].package, "test-src");
    }
}
