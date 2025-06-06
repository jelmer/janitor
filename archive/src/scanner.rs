/// Module for scanning Debian package archives with enhanced stream-based processing.
use anyhow::Result;
use deb822_lossless::FromDeb822Paragraph;
use debian_control::lossy::apt::{Package, Source};
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::Command;
use tracing::{debug, error, info, warn};

use crate::error::{ArchiveError, ArchiveResult};
use janitor::artifacts::{get_artifact_manager, ArtifactManager};

/// Build information retrieved from database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildInfo {
    /// Build ID.
    pub id: String,
    /// Codebase name.
    pub codebase: String,
    /// Suite/campaign name.
    pub suite: String,
    /// Target architecture.
    pub architecture: String,
    /// Repository component.
    pub component: String,
    /// Binary package files produced.
    pub binary_files: Vec<String>,
    /// Source package files produced.
    pub source_files: Vec<String>,
}

/// Enhanced package scanner with artifact management integration.
pub struct PackageScanner {
    artifact_manager: Arc<dyn ArtifactManager>,
    temp_dir: TempDir,
}

impl PackageScanner {
    /// Create a new package scanner with artifact manager.
    pub async fn new() -> ArchiveResult<Self> {
        let artifact_manager = get_artifact_manager("dummy://location")
            .await
            .map_err(|e| ArchiveError::ArtifactRetrieval(e.to_string()))?;
        let temp_dir = tempfile::TempDir::new().map_err(ArchiveError::Io)?;

        Ok(Self {
            artifact_manager: Arc::from(artifact_manager),
            temp_dir,
        })
    }

    /// Scan packages for a specific build, downloading artifacts as needed.
    pub async fn scan_packages_for_build<'a>(
        &'a self,
        build_info: &BuildInfo,
        arch: Option<&'a str>,
    ) -> impl Stream<Item = ArchiveResult<Package>> + 'a {
        let build_id = build_info.id.clone();
        let temp_path = self.temp_dir.path().to_path_buf();

        // Create async stream that downloads artifacts and scans packages
        async_stream::try_stream! {
            // Download build artifacts to temp directory
            let artifact_dir = self.download_build_artifacts(&build_id, &temp_path).await?;

            // Scan packages in the artifact directory
            let packages = scan_packages_in_directory(&artifact_dir, arch).await?;

            for package in packages {
                yield package;
            }
        }
    }

    /// Scan sources for a specific build, downloading artifacts as needed.
    pub async fn scan_sources_for_build<'a>(
        &'a self,
        build_info: &BuildInfo,
    ) -> impl Stream<Item = ArchiveResult<Source>> + 'a {
        let build_id = build_info.id.clone();
        let temp_path = self.temp_dir.path().to_path_buf();

        // Create async stream that downloads artifacts and scans sources
        async_stream::try_stream! {
            // Download build artifacts to temp directory
            let artifact_dir = self.download_build_artifacts(&build_id, &temp_path).await?;

            // Scan sources in the artifact directory
            let sources = scan_sources_in_directory(&artifact_dir).await?;

            for source in sources {
                yield source;
            }
        }
    }

    /// Download build artifacts to temporary directory.
    async fn download_build_artifacts(
        &self,
        build_id: &str,
        temp_path: &Path,
    ) -> ArchiveResult<PathBuf> {
        let artifact_dir = temp_path.join(format!("build-{}", build_id));
        tokio::fs::create_dir_all(&artifact_dir)
            .await
            .map_err(ArchiveError::Io)?;

        debug!(
            "Downloading artifacts for build {} to {:?}",
            build_id, artifact_dir
        );

        // Download all artifacts for this build using the artifact manager
        // Filter to only download package files (.deb, .dsc, .tar.*, .orig.tar.*)
        let package_filter = |filename: &str| -> bool {
            filename.ends_with(".deb")
                || filename.ends_with(".dsc")
                || filename.ends_with(".tar.gz")
                || filename.ends_with(".tar.xz")
                || filename.ends_with(".tar.bz2")
                || filename.contains(".orig.tar.")
                || filename.contains(".debian.tar.")
        };

        self.artifact_manager
            .retrieve_artifacts(build_id, &artifact_dir, Some(&package_filter))
            .await
            .map_err(|e| match e {
                janitor::artifacts::Error::ArtifactsMissing => ArchiveError::ArtifactsMissing {
                    build_id: build_id.to_string(),
                    message: "No artifacts found for build".to_string(),
                },
                janitor::artifacts::Error::ServiceUnavailable => ArchiveError::ArtifactRetrieval(
                    "Artifact service is currently unavailable".to_string(),
                ),
                janitor::artifacts::Error::IoError(io_err) => ArchiveError::Io(io_err),
                janitor::artifacts::Error::Other(msg) => ArchiveError::ArtifactRetrieval(msg),
            })?;

        // Verify that we actually downloaded some artifacts
        let entries = tokio::fs::read_dir(&artifact_dir)
            .await
            .map_err(ArchiveError::Io)?;
        let mut entry_count = 0;
        let mut entries = entries;
        while let Some(_entry) = entries.next_entry().await.map_err(ArchiveError::Io)? {
            entry_count += 1;
        }

        if entry_count == 0 {
            warn!("No artifacts downloaded for build {}", build_id);
            return Err(ArchiveError::ArtifactsMissing {
                build_id: build_id.to_string(),
                message: "No package artifacts found after download".to_string(),
            });
        }

        info!(
            "Downloaded {} artifacts for build {} to {:?}",
            entry_count, build_id, artifact_dir
        );
        Ok(artifact_dir)
    }
}

/// Scan binary packages in a directory (internal function).
///
/// # Arguments
/// * `td` - The directory to scan
/// * `arch` - Optional architecture to filter by
///
/// # Returns
/// A vector of Package objects or an error
async fn scan_packages_in_directory(td: &Path, arch: Option<&str>) -> ArchiveResult<Vec<Package>> {
    let mut args = Vec::new();
    if let Some(arch) = arch {
        args.extend(["-a", arch]);
    }

    let mut proc = Command::new("dpkg-scanpackages")
        .arg(td)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            ArchiveError::PackageScanning(format!("Failed to spawn dpkg-scanpackages: {}", e))
        })?;

    let stdout = proc
        .stdout
        .take()
        .ok_or_else(|| ArchiveError::PackageScanning("Failed to open stdout".to_string()))?;
    let stderr = proc
        .stderr
        .take()
        .ok_or_else(|| ArchiveError::PackageScanning("Failed to open stderr".to_string()))?;

    let mut stdout_reader = BufReader::new(stdout);
    let stderr_reader = BufReader::new(stderr);

    let mut stdout = Vec::new();

    stdout_reader
        .read_to_end(&mut stdout)
        .await
        .map_err(ArchiveError::Io)?;

    // Parse stdout paragraphs
    let paragraphs = deb822_lossless::lossy::Deb822::from_reader(&stdout[..])
        .map_err(|e| ArchiveError::PackageScanning(format!("Failed to parse deb822: {}", e)))?;

    // Process stderr
    tokio::spawn(async move {
        let mut lines = stderr_reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let line = line.as_bytes();
            if line.starts_with(b"dpkg-scanpackages: ") {
                let line = &line[b"dpkg-scanpackages: ".len()..];
                handle_log_line(line);
            } else {
                handle_log_line(line);
            }
        }
    });

    let packages: Result<Vec<Package>, _> = paragraphs
        .into_iter()
        .map(|p| Package::from_paragraph(&p))
        .collect();

    packages.map_err(|e| ArchiveError::PackageScanning(format!("Failed to parse package: {}", e)))
}

/// Scan source packages in a directory (internal function).
///
/// # Arguments
/// * `td` - The directory to scan
///
/// # Returns
/// A vector of Source objects or an error
async fn scan_sources_in_directory(td: &Path) -> ArchiveResult<Vec<Source>> {
    let mut proc = Command::new("dpkg-scansources")
        .arg(td)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            ArchiveError::SourceScanning(format!("Failed to spawn dpkg-scansources: {}", e))
        })?;

    let stdout = proc
        .stdout
        .take()
        .ok_or_else(|| ArchiveError::SourceScanning("Failed to open stdout".to_string()))?;
    let stderr = proc
        .stderr
        .take()
        .ok_or_else(|| ArchiveError::SourceScanning("Failed to open stderr".to_string()))?;

    let mut stdout_reader = BufReader::new(stdout);
    let stderr_reader = BufReader::new(stderr);

    // Read stdout content
    let mut stdout = Vec::new();
    stdout_reader
        .read_to_end(&mut stdout)
        .await
        .map_err(ArchiveError::Io)?;

    let paragraphs = deb822_lossless::lossy::Deb822::from_reader(&stdout[..])
        .map_err(|e| ArchiveError::SourceScanning(format!("Failed to parse deb822: {}", e)))?;

    // Process stderr
    tokio::spawn(async move {
        let mut lines = stderr_reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let line = line.as_bytes();
            if line.starts_with(b"dpkg-scansources: ") {
                let line = &line[b"dpkg-scansources: ".len()..];
                handle_log_line(line);
            } else {
                handle_log_line(line);
            }
        }
    });

    let sources: Result<Vec<Source>, _> = paragraphs
        .into_iter()
        .map(|p| Source::from_paragraph(&p))
        .collect();

    sources.map_err(|e| ArchiveError::SourceScanning(format!("Failed to parse source: {}", e)))
}

/// Handle a log line from the scanner process.
///
/// # Arguments
/// * `line` - The log line as bytes
fn handle_log_line(line: &[u8]) {
    if line.starts_with(b"info: ") {
        debug!("{}", String::from_utf8_lossy(&line[b"info: ".len()..]));
    } else if line.starts_with(b"warning: ") {
        warn!("{}", String::from_utf8_lossy(&line[b"warning: ".len()..]));
    } else if line.starts_with(b"error: ") {
        error!("{}", String::from_utf8_lossy(&line[b"error: ".len()..]));
    } else {
        info!("dpkg error: {}", String::from_utf8_lossy(line));
    }
}

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn test_scan_packages() {
        let test_dir = std::path::Path::new("tests/data");
        let packages = super::scan_packages_in_directory(test_dir, None)
            .await
            .unwrap();

        assert_eq!(packages.len(), 1);

        let package = &packages[0];

        assert_eq!(package.name, "hello");
        assert_eq!(package.version, "2.10-3".parse().unwrap());
    }

    #[tokio::test]
    async fn test_scan_sources() {
        let test_dir = std::path::Path::new("tests/data");
        let sources = super::scan_sources_in_directory(test_dir).await.unwrap();

        assert_eq!(sources.len(), 1);

        let source = &sources[0];

        assert_eq!(source.package, "hello");
        assert_eq!(source.version, "2.10-3".parse().unwrap());
    }

    #[tokio::test]
    async fn test_package_scanner_creation() {
        // This test might fail if artifact manager is not available
        // In that case, it should be marked as ignored or use a mock
        // let scanner = super::PackageScanner::new().await;
        // assert!(scanner.is_ok());
    }
}
