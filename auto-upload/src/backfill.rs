//! Backfill functionality for uploading historical builds

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, warn};

use crate::artifacts::ArtifactProcessor;
use crate::database::{DatabaseClient, DebianBuild};
use crate::error::Result;
use crate::process::upload_build_result;
use crate::upload::UploadConfig;

/// Backfill processor for uploading historical builds
pub struct BackfillProcessor {
    /// Database client for querying builds
    db_client: DatabaseClient,
    /// Artifact processor for retrieving build artifacts
    artifact_processor: ArtifactProcessor,
    /// Upload configuration
    upload_config: UploadConfig,
    /// Progress tracking
    progress: Arc<BackfillProgress>,
}

/// Backfill progress tracking
#[derive(Debug, Default)]
pub struct BackfillProgress {
    /// Total builds found
    pub total_builds: AtomicU64,
    /// Builds processed successfully
    pub successful_uploads: AtomicU64,
    /// Builds that failed to upload
    pub failed_uploads: AtomicU64,
    /// Builds skipped (no artifacts, etc.)
    pub skipped_builds: AtomicU64,
}

/// Backfill configuration options
#[derive(Debug, Clone)]
pub struct BackfillConfig {
    /// Distributions to process (empty means all)
    pub distributions: Vec<String>,
    /// Source packages to process (empty means all)
    pub source_packages: Vec<String>,
    /// Maximum number of builds to process
    pub max_builds: Option<u64>,
    /// Delay between uploads (to avoid overwhelming target)
    pub upload_delay: Duration,
    /// Number of retries for failed uploads
    pub max_retries: u32,
    /// Batch size for processing
    pub batch_size: u32,
    /// Dry run mode (don't actually upload)
    pub dry_run: bool,
}

impl Default for BackfillConfig {
    fn default() -> Self {
        Self {
            distributions: Vec::new(),
            source_packages: Vec::new(),
            max_builds: None,
            upload_delay: Duration::from_secs(1),
            max_retries: 3,
            batch_size: 100,
            dry_run: false,
        }
    }
}

impl BackfillProcessor {
    /// Create a new backfill processor
    pub async fn new(
        database_url: &str,
        artifact_location: &str,
        upload_config: UploadConfig,
    ) -> Result<Self> {
        let db_client = DatabaseClient::new(database_url).await?;
        let artifact_processor = ArtifactProcessor::new(artifact_location).await?;
        
        Ok(Self {
            db_client,
            artifact_processor,
            upload_config,
            progress: Arc::new(BackfillProgress::default()),
        })
    }
    
    /// Run backfill operation
    pub async fn run_backfill(&self, config: BackfillConfig) -> Result<BackfillSummary> {
        info!("Starting backfill operation");
        info!("Configuration: {:?}", config);
        
        // Query builds from database
        let builds = self.query_builds(&config).await?;
        let total_builds = builds.len() as u64;
        
        self.progress.total_builds.store(total_builds, Ordering::SeqCst);
        
        info!("Found {} builds for backfill", total_builds);
        
        if config.dry_run {
            info!("Running in dry-run mode - no actual uploads will be performed");
        }
        
        // Process builds in batches
        let mut processed = 0u64;
        let batch_size = config.batch_size as usize;
        
        for (batch_idx, batch) in builds.chunks(batch_size).enumerate() {
            info!("Processing batch {} ({} builds)", batch_idx + 1, batch.len());
            
            for build in batch {
                if let Some(max_builds) = config.max_builds {
                    if processed >= max_builds {
                        info!("Reached maximum build limit: {}", max_builds);
                        break;
                    }
                }
                
                match self.process_build(build, &config).await {
                    Ok(ProcessResult::Uploaded) => {
                        self.progress.successful_uploads.fetch_add(1, Ordering::SeqCst);
                        info!(
                            run_id = %build.run_id,
                            distribution = %build.distribution,
                            source = %build.source,
                            "Successfully uploaded build"
                        );
                    }
                    Ok(ProcessResult::Skipped(reason)) => {
                        self.progress.skipped_builds.fetch_add(1, Ordering::SeqCst);
                        info!(
                            run_id = %build.run_id,
                            reason = %reason,
                            "Skipped build"
                        );
                    }
                    Err(e) => {
                        self.progress.failed_uploads.fetch_add(1, Ordering::SeqCst);
                        error!(
                            run_id = %build.run_id,
                            error = %e,
                            "Failed to process build"
                        );
                    }
                }
                
                processed += 1;
                
                // Progress reporting
                if processed % 10 == 0 {
                    self.log_progress(processed, total_builds);
                }
                
                // Rate limiting
                if config.upload_delay > Duration::ZERO {
                    sleep(config.upload_delay).await;
                }
            }
            
            // Brief pause between batches
            if batch_idx < builds.len() / batch_size {
                sleep(Duration::from_millis(100)).await;
            }
        }
        
        let summary = self.create_summary();
        info!("Backfill operation completed: {:?}", summary);
        
        Ok(summary)
    }
    
    /// Query builds based on configuration
    async fn query_builds(&self, config: &BackfillConfig) -> Result<Vec<DebianBuild>> {
        let distributions = if config.distributions.is_empty() {
            None
        } else {
            Some(config.distributions.as_slice())
        };
        
        let mut builds = self.db_client.get_backfill_builds(distributions).await?;
        
        // Filter by source packages if specified
        if !config.source_packages.is_empty() {
            builds.retain(|build| config.source_packages.contains(&build.source));
        }
        
        // Apply limit if specified
        if let Some(max_builds) = config.max_builds {
            builds.truncate(max_builds as usize);
        }
        
        Ok(builds)
    }
    
    /// Process a single build
    async fn process_build(&self, build: &DebianBuild, config: &BackfillConfig) -> Result<ProcessResult> {
        // Check if artifacts exist
        if !self.artifact_processor.artifacts_exist(&build.run_id).await {
            return Ok(ProcessResult::Skipped("No artifacts found".to_string()));
        }
        
        // Check distribution filter
        if !self.upload_config.should_upload_distribution(&build.distribution) {
            return Ok(ProcessResult::Skipped(format!(
                "Distribution {} not in allowed list", 
                build.distribution
            )));
        }
        
        if config.dry_run {
            info!(
                run_id = %build.run_id,
                "Would upload build (dry run mode)"
            );
            return Ok(ProcessResult::Uploaded);
        }
        
        // Attempt upload with retries
        for attempt in 1..=config.max_retries {
            match upload_build_result(
                &build.run_id,
                &self.artifact_processor,
                &self.upload_config,
            ).await {
                Ok(_) => return Ok(ProcessResult::Uploaded),
                Err(e) => {
                    warn!(
                        run_id = %build.run_id,
                        attempt = attempt,
                        max_retries = config.max_retries,
                        error = %e,
                        "Upload attempt failed"
                    );
                    
                    if attempt < config.max_retries {
                        let delay = Duration::from_secs(2u64.pow(attempt));
                        sleep(delay).await;
                    } else {
                        return Err(e);
                    }
                }
            }
        }
        
        unreachable!()
    }
    
    /// Log progress information
    fn log_progress(&self, processed: u64, total: u64) {
        let successful = self.progress.successful_uploads.load(Ordering::SeqCst);
        let failed = self.progress.failed_uploads.load(Ordering::SeqCst);
        let skipped = self.progress.skipped_builds.load(Ordering::SeqCst);
        
        let percentage = if total > 0 {
            (processed as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        
        info!(
            "Progress: {}/{} ({:.1}%) - Success: {}, Failed: {}, Skipped: {}",
            processed, total, percentage, successful, failed, skipped
        );
    }
    
    /// Create backfill summary
    fn create_summary(&self) -> BackfillSummary {
        BackfillSummary {
            total_builds: self.progress.total_builds.load(Ordering::SeqCst),
            successful_uploads: self.progress.successful_uploads.load(Ordering::SeqCst),
            failed_uploads: self.progress.failed_uploads.load(Ordering::SeqCst),
            skipped_builds: self.progress.skipped_builds.load(Ordering::SeqCst),
        }
    }
    
    /// Get current progress
    pub fn get_progress(&self) -> BackfillSummary {
        self.create_summary()
    }
    
    /// Check database connection health
    pub async fn health_check(&self) -> Result<()> {
        self.db_client.health_check().await
    }
}

/// Result of processing a single build
#[derive(Debug)]
enum ProcessResult {
    /// Build was successfully uploaded
    Uploaded,
    /// Build was skipped with reason
    Skipped(String),
}

/// Summary of backfill operation
#[derive(Debug, Clone)]
pub struct BackfillSummary {
    /// Total builds found
    pub total_builds: u64,
    /// Builds processed successfully
    pub successful_uploads: u64,
    /// Builds that failed to upload
    pub failed_uploads: u64,
    /// Builds skipped
    pub skipped_builds: u64,
}

impl BackfillSummary {
    /// Calculate success rate
    pub fn success_rate(&self) -> f64 {
        let attempted = self.successful_uploads + self.failed_uploads;
        if attempted > 0 {
            (self.successful_uploads as f64 / attempted as f64) * 100.0
        } else {
            0.0
        }
    }
    
    /// Check if backfill was successful
    pub fn is_successful(&self) -> bool {
        self.failed_uploads == 0 && self.total_builds > 0
    }
    
    /// Get total processed builds
    pub fn total_processed(&self) -> u64 {
        self.successful_uploads + self.failed_uploads + self.skipped_builds
    }
}

impl std::fmt::Display for BackfillSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Backfill Summary: {} total, {} uploaded ({:.1}%), {} failed, {} skipped",
            self.total_builds,
            self.successful_uploads,
            self.success_rate(),
            self.failed_uploads,
            self.skipped_builds
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_backfill_config_default() {
        let config = BackfillConfig::default();
        assert!(config.distributions.is_empty());
        assert!(config.source_packages.is_empty());
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.batch_size, 100);
        assert!(!config.dry_run);
    }
    
    #[test]
    fn test_backfill_summary_success_rate() {
        let summary = BackfillSummary {
            total_builds: 100,
            successful_uploads: 80,
            failed_uploads: 20,
            skipped_builds: 0,
        };
        
        assert_eq!(summary.success_rate(), 80.0);
        assert!(!summary.is_successful());
    }
    
    #[test]
    fn test_backfill_summary_perfect_success() {
        let summary = BackfillSummary {
            total_builds: 50,
            successful_uploads: 40,
            failed_uploads: 0,
            skipped_builds: 10,
        };
        
        assert_eq!(summary.success_rate(), 100.0);
        assert!(summary.is_successful());
    }
    
    #[test]
    fn test_backfill_summary_display() {
        let summary = BackfillSummary {
            total_builds: 100,
            successful_uploads: 75,
            failed_uploads: 15,
            skipped_builds: 10,
        };
        
        let display = format!("{}", summary);
        assert!(display.contains("100 total"));
        assert!(display.contains("75 uploaded"));
        assert!(display.contains("15 failed"));
        assert!(display.contains("10 skipped"));
    }
}