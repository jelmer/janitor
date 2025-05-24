//! Log file management for the runner.

use crate::metrics::MetricsCollector;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Log storage backend configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogStorageBackend {
    /// Local filesystem storage.
    Local,
    /// Google Cloud Storage.
    Gcs,
}

impl Default for LogStorageBackend {
    fn default() -> Self {
        Self::Local
    }
}

/// Log management configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    /// Storage backend to use.
    #[serde(default)]
    pub storage_backend: LogStorageBackend,
    /// Local log storage path.
    #[serde(default = "default_local_log_path")]
    pub local_log_path: PathBuf,
    /// GCS bucket for log storage.
    pub gcs_bucket: Option<String>,
    /// Maximum log file size in bytes.
    #[serde(default = "default_max_log_size")]
    pub max_log_size: usize,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            storage_backend: LogStorageBackend::Local,
            local_log_path: default_local_log_path(),
            gcs_bucket: None,
            max_log_size: default_max_log_size(),
        }
    }
}

fn default_local_log_path() -> PathBuf {
    PathBuf::from("./logs")
}

fn default_max_log_size() -> usize {
    50 * 1024 * 1024 // 50MB
}

/// Error types for log management operations.
#[derive(Debug, thiserror::Error)]
pub enum LogError {
    /// IO error during log operations.
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    /// Log file not found.
    #[error("Log file not found: {0}")]
    NotFound(String),
    
    /// Invalid log filename.
    #[error("Invalid log filename: {0}")]
    InvalidFilename(String),
    
    /// Storage backend error.
    #[error("Storage backend error: {0}")]
    StorageError(String),
    
    /// Validation error.
    #[error("Validation error: {0}")]
    Validation(String),
}

/// Log storage backend trait.
#[async_trait::async_trait]
pub trait LogStorage: Send + Sync {
    /// Store a log file.
    async fn store_log(&self, run_id: &str, filename: &str, content: &[u8]) -> Result<(), LogError>;
    
    /// Retrieve a log file.
    async fn get_log(&self, run_id: &str, filename: &str) -> Result<Vec<u8>, LogError>;
    
    /// List log files for a run.
    async fn list_logs(&self, run_id: &str) -> Result<Vec<String>, LogError>;
    
    /// Delete log files for a run.
    async fn delete_logs(&self, run_id: &str) -> Result<(), LogError>;
    
    /// Delete a specific log file.
    async fn delete_log(&self, run_id: &str, filename: &str) -> Result<(), LogError>;
    
    /// Get storage type name for metrics.
    fn storage_type(&self) -> &'static str;
}

/// Local filesystem log storage.
pub struct LocalLogStorage {
    base_path: PathBuf,
}

impl LocalLogStorage {
    /// Create a new local log storage.
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }
    
    /// Get the path for a run's log directory.
    fn run_log_path(&self, run_id: &str) -> PathBuf {
        self.base_path.join(run_id)
    }
    
    /// Get the path for a specific log file.
    fn log_file_path(&self, run_id: &str, filename: &str) -> PathBuf {
        self.run_log_path(run_id).join(filename)
    }
}

#[async_trait::async_trait]
impl LogStorage for LocalLogStorage {
    async fn store_log(&self, run_id: &str, filename: &str, content: &[u8]) -> Result<(), LogError> {
        if !is_valid_log_filename(filename) {
            return Err(LogError::InvalidFilename(filename.to_string()));
        }
        
        let run_dir = self.run_log_path(run_id);
        fs::create_dir_all(&run_dir).await?;
        
        let file_path = self.log_file_path(run_id, filename);
        let mut file = fs::File::create(file_path).await?;
        file.write_all(content).await?;
        file.flush().await?;
        
        MetricsCollector::record_database_operation("store_log", true, 0.0);
        Ok(())
    }
    
    async fn get_log(&self, run_id: &str, filename: &str) -> Result<Vec<u8>, LogError> {
        if !is_valid_log_filename(filename) {
            return Err(LogError::InvalidFilename(filename.to_string()));
        }
        
        let file_path = self.log_file_path(run_id, filename);
        if !file_path.exists() {
            return Err(LogError::NotFound(format!("{}/{}", run_id, filename)));
        }
        
        let mut file = fs::File::open(file_path).await?;
        let mut content = Vec::new();
        file.read_to_end(&mut content).await?;
        
        MetricsCollector::record_database_operation("get_log", true, 0.0);
        Ok(content)
    }
    
    async fn list_logs(&self, run_id: &str) -> Result<Vec<String>, LogError> {
        let run_dir = self.run_log_path(run_id);
        if !run_dir.exists() {
            return Ok(Vec::new());
        }
        
        let mut entries = fs::read_dir(run_dir).await?;
        let mut log_files = Vec::new();
        
        while let Some(entry) = entries.next_entry().await? {
            if let Some(filename) = entry.file_name().to_str() {
                if is_valid_log_filename(filename) {
                    log_files.push(filename.to_string());
                }
            }
        }
        
        log_files.sort();
        Ok(log_files)
    }
    
    async fn delete_logs(&self, run_id: &str) -> Result<(), LogError> {
        let run_dir = self.run_log_path(run_id);
        if run_dir.exists() {
            fs::remove_dir_all(run_dir).await?;
        }
        Ok(())
    }
    
    async fn delete_log(&self, run_id: &str, filename: &str) -> Result<(), LogError> {
        let log_path = self.log_file_path(run_id, filename);
        if log_path.exists() {
            fs::remove_file(log_path).await?;
        }
        Ok(())
    }
    
    fn storage_type(&self) -> &'static str {
        "local"
    }
}

/// Google Cloud Storage log storage.
pub struct GcsLogStorage {
    bucket: String,
    client: Option<()>, // TODO: Add actual GCS client when available
}

impl GcsLogStorage {
    /// Create a new GCS log storage.
    pub fn new(bucket: String) -> Self {
        Self {
            bucket,
            client: None, // TODO: Initialize GCS client
        }
    }
    
    /// Get the GCS object path for a log file.
    fn log_object_path(&self, run_id: &str, filename: &str) -> String {
        format!("logs/{}/{}", run_id, filename)
    }
}

#[async_trait::async_trait]
impl LogStorage for GcsLogStorage {
    async fn store_log(&self, run_id: &str, filename: &str, content: &[u8]) -> Result<(), LogError> {
        if !is_valid_log_filename(filename) {
            return Err(LogError::InvalidFilename(filename.to_string()));
        }
        
        // TODO: Implement actual GCS upload
        let _object_path = self.log_object_path(run_id, filename);
        let _content_len = content.len();
        
        log::info!("Would upload {} bytes to GCS: {}/{}", content.len(), run_id, filename);
        
        // For now, return an error indicating GCS is not yet implemented
        Err(LogError::StorageError("GCS storage not yet implemented".to_string()))
    }
    
    async fn get_log(&self, run_id: &str, filename: &str) -> Result<Vec<u8>, LogError> {
        if !is_valid_log_filename(filename) {
            return Err(LogError::InvalidFilename(filename.to_string()));
        }
        
        // TODO: Implement actual GCS download
        let _object_path = self.log_object_path(run_id, filename);
        
        log::info!("Would download from GCS: {}/{}", run_id, filename);
        
        // For now, return an error indicating GCS is not yet implemented
        Err(LogError::StorageError("GCS storage not yet implemented".to_string()))
    }
    
    async fn list_logs(&self, run_id: &str) -> Result<Vec<String>, LogError> {
        // TODO: Implement actual GCS listing
        log::info!("Would list logs from GCS for run: {}", run_id);
        
        // For now, return an error indicating GCS is not yet implemented
        Err(LogError::StorageError("GCS storage not yet implemented".to_string()))
    }
    
    async fn delete_logs(&self, run_id: &str) -> Result<(), LogError> {
        // TODO: Implement actual GCS deletion
        log::info!("Would delete logs from GCS for run: {}", run_id);
        
        // For now, return an error indicating GCS is not yet implemented
        Err(LogError::StorageError("GCS storage not yet implemented".to_string()))
    }
    
    async fn delete_log(&self, run_id: &str, filename: &str) -> Result<(), LogError> {
        // TODO: Implement actual GCS deletion
        log::info!("Would delete log {} from GCS for run: {}", filename, run_id);
        
        // For now, return an error indicating GCS is not yet implemented
        Err(LogError::StorageError("GCS storage not yet implemented".to_string()))
    }
    
    fn storage_type(&self) -> &'static str {
        "gcs"
    }
}

/// Log file manager with pluggable storage backends.
pub struct LogFileManager {
    storage: Box<dyn LogStorage>,
}

impl LogFileManager {
    /// Create a new log file manager with local storage.
    pub fn new_local(base_path: PathBuf) -> Self {
        Self {
            storage: Box::new(LocalLogStorage::new(base_path)),
        }
    }
    
    /// Create a new log file manager with GCS storage.
    pub fn new_gcs(bucket: String) -> Self {
        Self {
            storage: Box::new(GcsLogStorage::new(bucket)),
        }
    }

    /// Create a new log file manager from configuration.
    pub async fn new(config: LogConfig) -> Result<Self, LogError> {
        match config.storage_backend {
            LogStorageBackend::Local => Ok(Self::new_local(config.local_log_path)),
            LogStorageBackend::Gcs => Ok(Self::new_gcs(config.gcs_bucket.unwrap_or_default())),
        }
    }

    /// Perform a health check on the log storage.
    pub async fn health_check(&self) -> Result<(), LogError> {
        // Try to store and retrieve a small test file
        let test_content = b"health_check";
        let test_run_id = "health_test";
        let test_filename = format!("health_check_{}.log", chrono::Utc::now().timestamp());
        
        self.storage.store_log(test_run_id, &test_filename, test_content).await?;
        let retrieved = self.storage.get_log(test_run_id, &test_filename).await?;
        
        if retrieved != test_content {
            return Err(LogError::Validation("Health check data mismatch".to_string()));
        }
        
        // Clean up test file (ignore errors)
        let _ = self.storage.delete_log(test_run_id, &test_filename).await;
        
        Ok(())
    }

    /// Flush all pending logs.
    pub async fn flush_all(&self) -> Result<(), LogError> {
        // For now, this is a no-op as our storage backends are synchronous
        // In a real implementation, this would flush any buffered writes
        Ok(())
    }
    
    /// Store a log file.
    pub async fn store_log(&self, run_id: &str, filename: &str, content: &[u8]) -> Result<(), LogError> {
        let start = std::time::Instant::now();
        let result = self.storage.store_log(run_id, filename, content).await;
        let duration = start.elapsed().as_secs_f64();
        
        let success = result.is_ok();
        MetricsCollector::record_database_operation("store_log", success, duration);
        
        if success {
            log::info!("Stored log file {}/{} ({} bytes)", run_id, filename, content.len());
        } else {
            log::error!("Failed to store log file {}/{}: {:?}", run_id, filename, result);
        }
        
        result
    }
    
    /// Retrieve a log file.
    pub async fn get_log(&self, run_id: &str, filename: &str) -> Result<Vec<u8>, LogError> {
        let start = std::time::Instant::now();
        let result = self.storage.get_log(run_id, filename).await;
        let duration = start.elapsed().as_secs_f64();
        
        let success = result.is_ok();
        MetricsCollector::record_database_operation("get_log", success, duration);
        
        result
    }
    
    /// List log files for a run.
    pub async fn list_logs(&self, run_id: &str) -> Result<Vec<String>, LogError> {
        self.storage.list_logs(run_id).await
    }
    
    /// Delete log files for a run.
    pub async fn delete_logs(&self, run_id: &str) -> Result<(), LogError> {
        let start = std::time::Instant::now();
        let result = self.storage.delete_logs(run_id).await;
        let duration = start.elapsed().as_secs_f64();
        
        let success = result.is_ok();
        MetricsCollector::record_database_operation("delete_logs", success, duration);
        
        result
    }
    
    /// Get storage type for metrics.
    pub fn storage_type(&self) -> &'static str {
        self.storage.storage_type()
    }
}

/// Check if a filename is a valid log filename.
/// This matches the logic from the Python implementation.
pub fn is_valid_log_filename(filename: &str) -> bool {
    // Basic validation - log files should have common extensions
    // and not contain path traversal characters
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return false;
    }
    
    // Check for common log file extensions
    filename.ends_with(".log") || 
    filename.ends_with(".txt") || 
    filename.ends_with(".out") || 
    filename.ends_with(".err") ||
    filename == "worker.log" ||
    filename == "build.log"
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_is_valid_log_filename() {
        assert!(is_valid_log_filename("worker.log"));
        assert!(is_valid_log_filename("build.log"));
        assert!(is_valid_log_filename("test.txt"));
        assert!(is_valid_log_filename("output.out"));
        
        assert!(!is_valid_log_filename("../etc/passwd"));
        assert!(!is_valid_log_filename("path/to/file.log"));
        assert!(!is_valid_log_filename("file\\with\\backslash.log"));
    }
    
    #[tokio::test]
    async fn test_local_log_storage() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalLogStorage::new(temp_dir.path().to_path_buf());
        
        let run_id = "test-run-123";
        let filename = "worker.log";
        let content = b"Test log content\nLine 2\n";
        
        // Test store
        storage.store_log(run_id, filename, content).await.unwrap();
        
        // Test get
        let retrieved = storage.get_log(run_id, filename).await.unwrap();
        assert_eq!(retrieved, content);
        
        // Test list
        let logs = storage.list_logs(run_id).await.unwrap();
        assert_eq!(logs, vec!["worker.log"]);
        
        // Test delete
        storage.delete_logs(run_id).await.unwrap();
        let logs_after_delete = storage.list_logs(run_id).await.unwrap();
        assert!(logs_after_delete.is_empty());
    }
}