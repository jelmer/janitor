//! File upload processing for worker results.

use axum::extract::Multipart;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use crate::{BuilderResult, WorkerResult};

/// File uploaded by a worker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadedFile {
    /// Original filename.
    pub filename: String,
    /// Content type if provided.
    pub content_type: Option<String>,
    /// File size in bytes.
    pub size: u64,
    /// Path where file was stored.
    pub stored_path: PathBuf,
    /// Upload timestamp.
    pub uploaded_at: DateTime<Utc>,
}

/// Complete worker result uploaded via multipart form.
#[derive(Debug, Serialize, Deserialize)]
pub struct UploadedWorkerResult {
    /// Worker result metadata.
    pub worker_result: WorkerResult,
    /// Log files uploaded.
    pub log_files: Vec<UploadedFile>,
    /// Artifact files uploaded.
    pub artifact_files: Vec<UploadedFile>,
    /// Build output files (for builder results).
    pub build_files: Vec<UploadedFile>,
    /// Additional metadata files.
    pub metadata_files: Vec<UploadedFile>,
}

/// Processor for multipart uploads from workers.
pub struct UploadProcessor {
    /// Base directory for storing uploaded files.
    storage_dir: PathBuf,
    /// Maximum file size allowed (in bytes).
    max_file_size: u64,
    /// Maximum total upload size (in bytes).
    max_total_size: u64,
}

impl UploadProcessor {
    /// Create a new upload processor.
    pub fn new(storage_dir: PathBuf, max_file_size: u64, max_total_size: u64) -> Self {
        Self {
            storage_dir,
            max_file_size,
            max_total_size,
        }
    }

    /// Process a multipart upload from a worker.
    pub async fn process_upload(
        &self,
        mut multipart: Multipart,
        run_id: &str,
    ) -> Result<UploadedWorkerResult, UploadError> {
        let mut worker_result: Option<WorkerResult> = None;
        let mut log_files = Vec::new();
        let mut artifact_files = Vec::new();
        let mut build_files = Vec::new();
        let mut metadata_files = Vec::new();
        let mut total_size = 0u64;

        // Create run-specific directory
        let run_dir = self.storage_dir.join(run_id);
        tokio::fs::create_dir_all(&run_dir).await
            .map_err(|e| UploadError::Storage(format!("Failed to create directory: {}", e)))?;

        while let Some(field) = multipart.next_field().await
            .map_err(|e| UploadError::Multipart(format!("Failed to read field: {}", e)))? {
            
            let field_name = field.name()
                .ok_or_else(|| UploadError::Multipart("Field missing name".to_string()))?
                .to_string();

            match field_name.as_str() {
                "worker_result" => {
                    // Process JSON worker result
                    let data = field.bytes().await
                        .map_err(|e| UploadError::Multipart(format!("Failed to read worker_result: {}", e)))?;
                    
                    worker_result = Some(serde_json::from_slice(&data)
                        .map_err(|e| UploadError::Parse(format!("Invalid worker_result JSON: {}", e)))?);
                }
                field_name if field_name.starts_with("log_") => {
                    // Process log file
                    let file = self.save_field_to_file(field, &run_dir, "logs", &mut total_size).await?;
                    log_files.push(file);
                }
                field_name if field_name.starts_with("artifact_") => {
                    // Process artifact file
                    let file = self.save_field_to_file(field, &run_dir, "artifacts", &mut total_size).await?;
                    artifact_files.push(file);
                }
                field_name if field_name.starts_with("build_") => {
                    // Process build output file
                    let file = self.save_field_to_file(field, &run_dir, "build", &mut total_size).await?;
                    build_files.push(file);
                }
                field_name if field_name.starts_with("metadata_") => {
                    // Process metadata file
                    let file = self.save_field_to_file(field, &run_dir, "metadata", &mut total_size).await?;
                    metadata_files.push(file);
                }
                _ => {
                    // Unknown field, skip
                    log::warn!("Skipping unknown field: {}", field_name);
                }
            }

            // Check total size limit
            if total_size > self.max_total_size {
                return Err(UploadError::SizeLimit(format!(
                    "Total upload size {} exceeds limit {}", 
                    total_size, 
                    self.max_total_size
                )));
            }
        }

        let worker_result = worker_result
            .ok_or_else(|| UploadError::Validation("Missing worker_result field".to_string()))?;

        Ok(UploadedWorkerResult {
            worker_result,
            log_files,
            artifact_files,
            build_files,
            metadata_files,
        })
    }

    /// Save a multipart field to a file.
    async fn save_field_to_file<'a>(
        &self,
        field: axum::extract::multipart::Field<'a>,
        run_dir: &PathBuf,
        category: &str,
        total_size: &mut u64,
    ) -> Result<UploadedFile, UploadError> {
        let filename = field.file_name()
            .unwrap_or("unknown")
            .to_string();
        
        let content_type = field.content_type()
            .map(|ct| ct.to_string());

        // Create category directory
        let category_dir = run_dir.join(category);
        tokio::fs::create_dir_all(&category_dir).await
            .map_err(|e| UploadError::Storage(format!("Failed to create {} directory: {}", category, e)))?;

        // Create safe filename
        let safe_filename = sanitize_filename(&filename);
        let file_path = category_dir.join(&safe_filename);

        // Stream file to disk
        let data = field.bytes().await
            .map_err(|e| UploadError::Multipart(format!("Failed to read file data: {}", e)))?;

        let file_size = data.len() as u64;

        // Check individual file size limit
        if file_size > self.max_file_size {
            return Err(UploadError::SizeLimit(format!(
                "File {} size {} exceeds limit {}", 
                filename, 
                file_size, 
                self.max_file_size
            )));
        }

        *total_size += file_size;

        // Write file
        tokio::fs::write(&file_path, &data).await
            .map_err(|e| UploadError::Storage(format!("Failed to write file {}: {}", filename, e)))?;

        log::info!("Uploaded file: {} ({} bytes) -> {:?}", filename, file_size, file_path);

        Ok(UploadedFile {
            filename,
            content_type,
            size: file_size,
            stored_path: file_path,
            uploaded_at: Utc::now(),
        })
    }

    /// Process uploaded worker result and extract builder result if present.
    pub fn extract_builder_result(
        &self,
        uploaded: &UploadedWorkerResult,
    ) -> Result<Option<BuilderResult>, UploadError> {
        if let Some(ref builder_result) = uploaded.worker_result.builder_result {
            match builder_result {
                BuilderResult::Debian { .. } => {
                    // For Debian builds, we might need to parse additional information
                    // from uploaded files (e.g., changes files, lintian output)
                    self.extract_debian_builder_result(uploaded)
                }
                BuilderResult::Generic => {
                    // Generic builds don't need additional processing
                    Ok(Some(BuilderResult::Generic))
                }
            }
        } else {
            Ok(None)
        }
    }

    /// Extract Debian-specific builder result from uploaded files.
    fn extract_debian_builder_result(
        &self,
        uploaded: &UploadedWorkerResult,
    ) -> Result<Option<BuilderResult>, UploadError> {
        // Look for changes files and lintian output
        let mut changes_filenames = Vec::new();
        let mut lintian_result = None;
        let binary_packages = None;

        for file in &uploaded.build_files {
            if file.filename.ends_with(".changes") {
                changes_filenames.push(file.filename.clone());
            } else if file.filename == "lintian.txt" {
                // Parse lintian output from file
                // For now, just record that we have it
                lintian_result = Some(serde_json::json!({
                    "file": file.stored_path.to_string_lossy()
                }));
            }
        }

        // Extract source and version from worker result if available
        let (source, build_version, build_distribution) = if let Some(ref result) = uploaded.worker_result.builder_result {
            match result {
                BuilderResult::Debian { source, build_version, build_distribution, .. } => {
                    (source.clone(), build_version.clone(), build_distribution.clone())
                }
                _ => (None, None, None)
            }
        } else {
            (None, None, None)
        };

        Ok(Some(BuilderResult::Debian {
            source,
            build_version,
            build_distribution,
            changes_filenames: if changes_filenames.is_empty() { None } else { Some(changes_filenames) },
            lintian_result,
            binary_packages,
        }))
    }

    /// Get storage statistics.
    pub async fn get_storage_stats(&self) -> Result<StorageStats, UploadError> {
        let mut total_files = 0;
        let mut total_size = 0;
        let mut categories = HashMap::new();

        if self.storage_dir.exists() {
            let mut entries = tokio::fs::read_dir(&self.storage_dir).await
                .map_err(|e| UploadError::Storage(format!("Failed to read storage directory: {}", e)))?;

            while let Some(entry) = entries.next_entry().await
                .map_err(|e| UploadError::Storage(format!("Failed to read directory entry: {}", e)))? {
                
                if entry.file_type().await.map_err(|e| UploadError::Storage(e.to_string()))?.is_dir() {
                    // This is a run directory
                    let run_stats = self.get_run_storage_stats(&entry.path()).await?;
                    total_files += run_stats.total_files;
                    total_size += run_stats.total_size;
                    
                    for (category, count) in run_stats.files_by_category {
                        *categories.entry(category).or_insert(0) += count;
                    }
                }
            }
        }

        Ok(StorageStats {
            total_files,
            total_size,
            files_by_category: categories,
        })
    }

    /// Get storage statistics for a specific run.
    async fn get_run_storage_stats(&self, run_dir: &PathBuf) -> Result<RunStorageStats, UploadError> {
        let mut total_files = 0;
        let mut total_size = 0;
        let mut files_by_category = HashMap::new();

        let mut entries = tokio::fs::read_dir(run_dir).await
            .map_err(|e| UploadError::Storage(format!("Failed to read run directory: {}", e)))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| UploadError::Storage(format!("Failed to read directory entry: {}", e)))? {
            
            if entry.file_type().await.map_err(|e| UploadError::Storage(e.to_string()))?.is_dir() {
                let category = entry.file_name().to_string_lossy().to_string();
                let category_stats = self.get_category_storage_stats(&entry.path()).await?;
                
                total_files += category_stats.0;
                total_size += category_stats.1;
                files_by_category.insert(category, category_stats.0);
            }
        }

        Ok(RunStorageStats {
            total_files,
            total_size,
            files_by_category,
        })
    }

    /// Get storage statistics for a category directory.
    async fn get_category_storage_stats(&self, category_dir: &PathBuf) -> Result<(u64, u64), UploadError> {
        let mut file_count = 0;
        let mut total_size = 0;

        let mut entries = tokio::fs::read_dir(category_dir).await
            .map_err(|e| UploadError::Storage(format!("Failed to read category directory: {}", e)))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| UploadError::Storage(format!("Failed to read directory entry: {}", e)))? {
            
            if entry.file_type().await.map_err(|e| UploadError::Storage(e.to_string()))?.is_file() {
                let metadata = entry.metadata().await
                    .map_err(|e| UploadError::Storage(format!("Failed to read file metadata: {}", e)))?;
                
                file_count += 1;
                total_size += metadata.len();
            }
        }

        Ok((file_count, total_size))
    }
}

/// Storage statistics.
#[derive(Debug, Serialize)]
pub struct StorageStats {
    /// Total number of files.
    pub total_files: u64,
    /// Total size in bytes.
    pub total_size: u64,
    /// Files by category.
    pub files_by_category: HashMap<String, u64>,
}

/// Storage statistics for a specific run.
#[derive(Debug)]
struct RunStorageStats {
    total_files: u64,
    total_size: u64,
    files_by_category: HashMap<String, u64>,
}

/// Errors that can occur during upload processing.
#[derive(Debug, thiserror::Error)]
pub enum UploadError {
    /// Multipart parsing error.
    #[error("Multipart error: {0}")]
    Multipart(String),
    
    /// JSON parsing error.
    #[error("Parse error: {0}")]
    Parse(String),
    
    /// File storage error.
    #[error("Storage error: {0}")]
    Storage(String),
    
    /// Size limit exceeded.
    #[error("Size limit exceeded: {0}")]
    SizeLimit(String),
    
    /// Validation error.
    #[error("Validation error: {0}")]
    Validation(String),
}

/// Sanitize a filename for safe storage.
fn sanitize_filename(filename: &str) -> String {
    // Replace potentially dangerous characters
    filename
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect::<String>()
        .trim_matches('.')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("normal.txt"), "normal.txt");
        assert_eq!(sanitize_filename("with/slash.txt"), "with_slash.txt");
        assert_eq!(sanitize_filename("with:colon.txt"), "with_colon.txt");
        assert_eq!(sanitize_filename("..dangerous"), "dangerous");
        assert_eq!(sanitize_filename("dangerous.."), "dangerous");
    }
}