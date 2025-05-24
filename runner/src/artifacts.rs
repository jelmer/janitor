//! Artifact storage management for the runner.

use crate::metrics::MetricsCollector;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Artifact storage backend configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArtifactStorageBackend {
    /// Local filesystem storage.
    Local,
    /// Google Cloud Storage.
    Gcs,
}

impl Default for ArtifactStorageBackend {
    fn default() -> Self {
        Self::Local
    }
}

/// Artifact storage configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactConfig {
    /// Storage backend to use.
    #[serde(default)]
    pub storage_backend: ArtifactStorageBackend,
    /// Local artifact storage path.
    #[serde(default = "default_local_artifact_path")]
    pub local_artifact_path: PathBuf,
    /// GCS bucket for artifact storage.
    pub gcs_bucket: Option<String>,
    /// Maximum artifact size in bytes.
    #[serde(default = "default_max_artifact_size")]
    pub max_artifact_size: usize,
}

impl Default for ArtifactConfig {
    fn default() -> Self {
        Self {
            storage_backend: ArtifactStorageBackend::Local,
            local_artifact_path: default_local_artifact_path(),
            gcs_bucket: None,
            max_artifact_size: default_max_artifact_size(),
        }
    }
}

fn default_local_artifact_path() -> PathBuf {
    PathBuf::from("./artifacts")
}

fn default_max_artifact_size() -> usize {
    100 * 1024 * 1024 // 100MB
}

/// Error types for artifact storage operations.
#[derive(Debug, thiserror::Error)]
pub enum ArtifactError {
    /// IO error during artifact operations.
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    /// Artifact not found.
    #[error("Artifact not found: {0}")]
    NotFound(String),
    
    /// Invalid artifact name.
    #[error("Invalid artifact name: {0}")]
    InvalidName(String),
    
    /// Storage backend error.
    #[error("Storage backend error: {0}")]
    StorageError(String),
    
    /// Artifact too large.
    #[error("Artifact too large: {0} bytes (max: {1} bytes)")]
    TooLarge(u64, u64),
    
    /// Validation error.
    #[error("Validation error: {0}")]
    Validation(String),
}

/// Artifact metadata.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ArtifactMetadata {
    /// Artifact name/filename.
    pub name: String,
    /// Size in bytes.
    pub size: u64,
    /// Content type/MIME type.
    pub content_type: String,
    /// Upload timestamp.
    pub uploaded_at: chrono::DateTime<chrono::Utc>,
    /// Additional metadata.
    pub metadata: HashMap<String, String>,
}

/// Artifact storage backend trait.
#[async_trait::async_trait]
pub trait ArtifactStorage: Send + Sync {
    /// Store an artifact.
    async fn store_artifact(
        &self, 
        run_id: &str, 
        name: &str, 
        content: &[u8], 
        content_type: &str,
        metadata: Option<HashMap<String, String>>
    ) -> Result<(), ArtifactError>;
    
    /// Retrieve an artifact.
    async fn get_artifact(&self, run_id: &str, name: &str) -> Result<Vec<u8>, ArtifactError>;
    
    /// Get artifact metadata.
    async fn get_artifact_metadata(&self, run_id: &str, name: &str) -> Result<ArtifactMetadata, ArtifactError>;
    
    /// List artifacts for a run.
    async fn list_artifacts(&self, run_id: &str) -> Result<Vec<ArtifactMetadata>, ArtifactError>;
    
    /// Delete an artifact.
    async fn delete_artifact(&self, run_id: &str, name: &str) -> Result<(), ArtifactError>;
    
    /// Delete all artifacts for a run.
    async fn delete_run_artifacts(&self, run_id: &str) -> Result<(), ArtifactError>;
    
    /// Get storage type name for metrics.
    fn storage_type(&self) -> &'static str;
}

/// Local filesystem artifact storage.
pub struct LocalArtifactStorage {
    base_path: PathBuf,
    max_size: u64,
}

impl LocalArtifactStorage {
    /// Create a new local artifact storage.
    pub fn new(base_path: PathBuf, max_size: u64) -> Self {
        Self { base_path, max_size }
    }
    
    /// Get the path for a run's artifact directory.
    fn run_artifact_path(&self, run_id: &str) -> PathBuf {
        self.base_path.join(run_id)
    }
    
    /// Get the path for a specific artifact file.
    fn artifact_file_path(&self, run_id: &str, name: &str) -> PathBuf {
        self.run_artifact_path(run_id).join(name)
    }
    
    /// Get the path for artifact metadata file.
    fn metadata_file_path(&self, run_id: &str, name: &str) -> PathBuf {
        self.run_artifact_path(run_id).join(format!("{}.meta", name))
    }
}

#[async_trait::async_trait]
impl ArtifactStorage for LocalArtifactStorage {
    async fn store_artifact(
        &self, 
        run_id: &str, 
        name: &str, 
        content: &[u8], 
        content_type: &str,
        metadata: Option<HashMap<String, String>>
    ) -> Result<(), ArtifactError> {
        if !is_valid_artifact_name(name) {
            return Err(ArtifactError::InvalidName(name.to_string()));
        }
        
        if content.len() as u64 > self.max_size {
            return Err(ArtifactError::TooLarge(content.len() as u64, self.max_size));
        }
        
        let run_dir = self.run_artifact_path(run_id);
        fs::create_dir_all(&run_dir).await?;
        
        // Store the artifact content
        let file_path = self.artifact_file_path(run_id, name);
        let mut file = fs::File::create(file_path).await?;
        file.write_all(content).await?;
        file.flush().await?;
        
        // Store metadata
        let artifact_metadata = ArtifactMetadata {
            name: name.to_string(),
            size: content.len() as u64,
            content_type: content_type.to_string(),
            uploaded_at: chrono::Utc::now(),
            metadata: metadata.unwrap_or_default(),
        };
        
        let metadata_path = self.metadata_file_path(run_id, name);
        let metadata_json = serde_json::to_string_pretty(&artifact_metadata)
            .map_err(|e| ArtifactError::StorageError(format!("Failed to serialize metadata: {}", e)))?;
        
        let mut metadata_file = fs::File::create(metadata_path).await?;
        metadata_file.write_all(metadata_json.as_bytes()).await?;
        metadata_file.flush().await?;
        
        MetricsCollector::record_artifact_upload(self.storage_type(), true, content.len() as f64);
        Ok(())
    }
    
    async fn get_artifact(&self, run_id: &str, name: &str) -> Result<Vec<u8>, ArtifactError> {
        if !is_valid_artifact_name(name) {
            return Err(ArtifactError::InvalidName(name.to_string()));
        }
        
        let file_path = self.artifact_file_path(run_id, name);
        if !file_path.exists() {
            return Err(ArtifactError::NotFound(format!("{}/{}", run_id, name)));
        }
        
        let mut file = fs::File::open(file_path).await?;
        let mut content = Vec::new();
        file.read_to_end(&mut content).await?;
        
        Ok(content)
    }
    
    async fn get_artifact_metadata(&self, run_id: &str, name: &str) -> Result<ArtifactMetadata, ArtifactError> {
        if !is_valid_artifact_name(name) {
            return Err(ArtifactError::InvalidName(name.to_string()));
        }
        
        let metadata_path = self.metadata_file_path(run_id, name);
        if !metadata_path.exists() {
            return Err(ArtifactError::NotFound(format!("{}/{}.meta", run_id, name)));
        }
        
        let mut metadata_file = fs::File::open(metadata_path).await?;
        let mut metadata_json = String::new();
        metadata_file.read_to_string(&mut metadata_json).await?;
        
        let metadata: ArtifactMetadata = serde_json::from_str(&metadata_json)
            .map_err(|e| ArtifactError::StorageError(format!("Failed to deserialize metadata: {}", e)))?;
        
        Ok(metadata)
    }
    
    async fn list_artifacts(&self, run_id: &str) -> Result<Vec<ArtifactMetadata>, ArtifactError> {
        let run_dir = self.run_artifact_path(run_id);
        if !run_dir.exists() {
            return Ok(Vec::new());
        }
        
        let mut entries = fs::read_dir(run_dir).await?;
        let mut artifacts = Vec::new();
        
        while let Some(entry) = entries.next_entry().await? {
            if let Some(filename) = entry.file_name().to_str() {
                // Skip metadata files
                if filename.ends_with(".meta") {
                    continue;
                }
                
                if is_valid_artifact_name(filename) {
                    match self.get_artifact_metadata(run_id, filename).await {
                        Ok(metadata) => artifacts.push(metadata),
                        Err(_) => {
                            // If no metadata file exists, create basic metadata
                            let file_metadata = entry.metadata().await?;
                            artifacts.push(ArtifactMetadata {
                                name: filename.to_string(),
                                size: file_metadata.len(),
                                content_type: "application/octet-stream".to_string(),
                                uploaded_at: chrono::Utc::now(), // Approximate
                                metadata: HashMap::new(),
                            });
                        }
                    }
                }
            }
        }
        
        artifacts.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(artifacts)
    }
    
    async fn delete_artifact(&self, run_id: &str, name: &str) -> Result<(), ArtifactError> {
        if !is_valid_artifact_name(name) {
            return Err(ArtifactError::InvalidName(name.to_string()));
        }
        
        let file_path = self.artifact_file_path(run_id, name);
        let metadata_path = self.metadata_file_path(run_id, name);
        
        if file_path.exists() {
            fs::remove_file(file_path).await?;
        }
        if metadata_path.exists() {
            fs::remove_file(metadata_path).await?;
        }
        
        Ok(())
    }
    
    async fn delete_run_artifacts(&self, run_id: &str) -> Result<(), ArtifactError> {
        let run_dir = self.run_artifact_path(run_id);
        if run_dir.exists() {
            fs::remove_dir_all(run_dir).await?;
        }
        Ok(())
    }
    
    fn storage_type(&self) -> &'static str {
        "local"
    }
}

/// Google Cloud Storage artifact storage.
pub struct GcsArtifactStorage {
    bucket: String,
    max_size: u64,
    client: Option<()>, // TODO: Add actual GCS client when available
}

impl GcsArtifactStorage {
    /// Create a new GCS artifact storage.
    pub fn new(bucket: String, max_size: u64) -> Self {
        Self {
            bucket,
            max_size,
            client: None, // TODO: Initialize GCS client
        }
    }
    
    /// Get the GCS object path for an artifact.
    fn artifact_object_path(&self, run_id: &str, name: &str) -> String {
        format!("artifacts/{}/{}", run_id, name)
    }
}

#[async_trait::async_trait]
impl ArtifactStorage for GcsArtifactStorage {
    async fn store_artifact(
        &self, 
        run_id: &str, 
        name: &str, 
        content: &[u8], 
        content_type: &str,
        metadata: Option<HashMap<String, String>>
    ) -> Result<(), ArtifactError> {
        if !is_valid_artifact_name(name) {
            return Err(ArtifactError::InvalidName(name.to_string()));
        }
        
        if content.len() as u64 > self.max_size {
            return Err(ArtifactError::TooLarge(content.len() as u64, self.max_size));
        }
        
        // TODO: Implement actual GCS upload
        let _object_path = self.artifact_object_path(run_id, name);
        let _content_len = content.len();
        let _content_type = content_type;
        let _metadata = metadata;
        
        log::info!("Would upload artifact to GCS: {}/{} ({} bytes)", run_id, name, content.len());
        
        // For now, return an error indicating GCS is not yet implemented
        Err(ArtifactError::StorageError("GCS storage not yet implemented".to_string()))
    }
    
    async fn get_artifact(&self, run_id: &str, name: &str) -> Result<Vec<u8>, ArtifactError> {
        if !is_valid_artifact_name(name) {
            return Err(ArtifactError::InvalidName(name.to_string()));
        }
        
        // TODO: Implement actual GCS download
        let _object_path = self.artifact_object_path(run_id, name);
        
        log::info!("Would download artifact from GCS: {}/{}", run_id, name);
        
        // For now, return an error indicating GCS is not yet implemented
        Err(ArtifactError::StorageError("GCS storage not yet implemented".to_string()))
    }
    
    async fn get_artifact_metadata(&self, run_id: &str, name: &str) -> Result<ArtifactMetadata, ArtifactError> {
        if !is_valid_artifact_name(name) {
            return Err(ArtifactError::InvalidName(name.to_string()));
        }
        
        // TODO: Implement actual GCS metadata retrieval
        log::info!("Would get artifact metadata from GCS: {}/{}", run_id, name);
        
        // For now, return an error indicating GCS is not yet implemented
        Err(ArtifactError::StorageError("GCS storage not yet implemented".to_string()))
    }
    
    async fn list_artifacts(&self, run_id: &str) -> Result<Vec<ArtifactMetadata>, ArtifactError> {
        // TODO: Implement actual GCS listing
        log::info!("Would list artifacts from GCS for run: {}", run_id);
        
        // For now, return an error indicating GCS is not yet implemented
        Err(ArtifactError::StorageError("GCS storage not yet implemented".to_string()))
    }
    
    async fn delete_artifact(&self, run_id: &str, name: &str) -> Result<(), ArtifactError> {
        if !is_valid_artifact_name(name) {
            return Err(ArtifactError::InvalidName(name.to_string()));
        }
        
        // TODO: Implement actual GCS deletion
        log::info!("Would delete artifact from GCS: {}/{}", run_id, name);
        
        // For now, return an error indicating GCS is not yet implemented
        Err(ArtifactError::StorageError("GCS storage not yet implemented".to_string()))
    }
    
    async fn delete_run_artifacts(&self, run_id: &str) -> Result<(), ArtifactError> {
        // TODO: Implement actual GCS deletion
        log::info!("Would delete all artifacts from GCS for run: {}", run_id);
        
        // For now, return an error indicating GCS is not yet implemented
        Err(ArtifactError::StorageError("GCS storage not yet implemented".to_string()))
    }
    
    fn storage_type(&self) -> &'static str {
        "gcs"
    }
}

/// Artifact manager with pluggable storage backends.
pub struct ArtifactManager {
    storage: Box<dyn ArtifactStorage>,
}

impl ArtifactManager {
    /// Create a new artifact manager with local storage.
    pub fn new_local(base_path: PathBuf, max_size: u64) -> Self {
        Self {
            storage: Box::new(LocalArtifactStorage::new(base_path, max_size)),
        }
    }
    
    /// Create a new artifact manager with GCS storage.
    pub fn new_gcs(bucket: String, max_size: u64) -> Self {
        Self {
            storage: Box::new(GcsArtifactStorage::new(bucket, max_size)),
        }
    }

    /// Create a new artifact manager from configuration.
    pub async fn new(config: ArtifactConfig) -> Result<Self, ArtifactError> {
        match config.storage_backend {
            ArtifactStorageBackend::Local => Ok(Self::new_local(config.local_artifact_path, config.max_artifact_size as u64)),
            ArtifactStorageBackend::Gcs => Ok(Self::new_gcs(config.gcs_bucket.unwrap_or_default(), config.max_artifact_size as u64)),
        }
    }

    /// Perform a health check on the artifact storage.
    pub async fn health_check(&self) -> Result<(), ArtifactError> {
        // Try to store and retrieve a small test artifact
        let test_content = b"health_check";
        let test_run_id = "health_test";
        let test_name = format!("health_check_{}.txt", chrono::Utc::now().timestamp());
        
        self.storage.store_artifact(test_run_id, &test_name, test_content, "text/plain", None).await?;
        let retrieved = self.storage.get_artifact(test_run_id, &test_name).await?;
        
        if retrieved != test_content {
            return Err(ArtifactError::Validation("Health check data mismatch".to_string()));
        }
        
        // Clean up test artifact (ignore errors)
        let _ = self.storage.delete_artifact(test_run_id, &test_name).await;
        
        Ok(())
    }

    /// Flush all pending artifacts.
    pub async fn flush_all(&self) -> Result<(), ArtifactError> {
        // For now, this is a no-op as our storage backends are synchronous
        // In a real implementation, this would flush any buffered writes
        Ok(())
    }
    
    /// Store an artifact.
    pub async fn store_artifact(
        &self, 
        run_id: &str, 
        name: &str, 
        content: &[u8], 
        content_type: &str,
        metadata: Option<HashMap<String, String>>
    ) -> Result<(), ArtifactError> {
        let start = std::time::Instant::now();
        let result = self.storage.store_artifact(run_id, name, content, content_type, metadata).await;
        let duration = start.elapsed().as_secs_f64();
        
        let success = result.is_ok();
        MetricsCollector::record_artifact_upload(self.storage.storage_type(), success, content.len() as f64);
        
        if success {
            log::info!("Stored artifact {}/{} ({} bytes)", run_id, name, content.len());
        } else {
            log::error!("Failed to store artifact {}/{}: {:?}", run_id, name, result);
        }
        
        result
    }
    
    /// Retrieve an artifact.
    pub async fn get_artifact(&self, run_id: &str, name: &str) -> Result<Vec<u8>, ArtifactError> {
        self.storage.get_artifact(run_id, name).await
    }
    
    /// Get artifact metadata.
    pub async fn get_artifact_metadata(&self, run_id: &str, name: &str) -> Result<ArtifactMetadata, ArtifactError> {
        self.storage.get_artifact_metadata(run_id, name).await
    }
    
    /// List artifacts for a run.
    pub async fn list_artifacts(&self, run_id: &str) -> Result<Vec<ArtifactMetadata>, ArtifactError> {
        self.storage.list_artifacts(run_id).await
    }
    
    /// Delete an artifact.
    pub async fn delete_artifact(&self, run_id: &str, name: &str) -> Result<(), ArtifactError> {
        self.storage.delete_artifact(run_id, name).await
    }
    
    /// Delete all artifacts for a run.
    pub async fn delete_run_artifacts(&self, run_id: &str) -> Result<(), ArtifactError> {
        self.storage.delete_run_artifacts(run_id).await
    }
    
    /// Get storage type for metrics.
    pub fn storage_type(&self) -> &'static str {
        self.storage.storage_type()
    }
}

/// Check if an artifact name is valid.
pub fn is_valid_artifact_name(name: &str) -> bool {
    // Basic validation - artifact names should not contain path traversal characters
    // and should have reasonable length
    if name.is_empty() || name.len() > 255 {
        return false;
    }
    
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return false;
    }
    
    // Don't allow names starting with dots (hidden files)
    if name.starts_with('.') {
        return false;
    }
    
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_is_valid_artifact_name() {
        assert!(is_valid_artifact_name("artifact.tar.gz"));
        assert!(is_valid_artifact_name("package.deb"));
        assert!(is_valid_artifact_name("results.json"));
        
        assert!(!is_valid_artifact_name(""));
        assert!(!is_valid_artifact_name("../etc/passwd"));
        assert!(!is_valid_artifact_name("path/to/file"));
        assert!(!is_valid_artifact_name("file\\with\\backslash"));
        assert!(!is_valid_artifact_name(".hidden"));
    }
    
    #[tokio::test]
    async fn test_local_artifact_storage() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalArtifactStorage::new(temp_dir.path().to_path_buf(), 1024 * 1024); // 1MB max
        
        let run_id = "test-run-123";
        let name = "artifact.txt";
        let content = b"Test artifact content\nLine 2\n";
        let content_type = "text/plain";
        
        // Test store
        storage.store_artifact(run_id, name, content, content_type, None).await.unwrap();
        
        // Test get
        let retrieved = storage.get_artifact(run_id, name).await.unwrap();
        assert_eq!(retrieved, content);
        
        // Test metadata
        let metadata = storage.get_artifact_metadata(run_id, name).await.unwrap();
        assert_eq!(metadata.name, name);
        assert_eq!(metadata.size, content.len() as u64);
        assert_eq!(metadata.content_type, content_type);
        
        // Test list
        let artifacts = storage.list_artifacts(run_id).await.unwrap();
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].name, name);
        
        // Test delete
        storage.delete_artifact(run_id, name).await.unwrap();
        let artifacts_after_delete = storage.list_artifacts(run_id).await.unwrap();
        assert!(artifacts_after_delete.is_empty());
    }
}