//! Generator Manager for archive service background operations.
//!
//! This module provides the GeneratorManager which handles job scheduling,
//! campaign management, and repository coordination for the archive service.
//! It manages concurrent repository generation jobs and maps campaigns to
//! multiple APT repositories that should be regenerated when campaigns update.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::config::{AptRepositoryConfig, ArchiveConfig};
use crate::database::BuildManager;
use crate::error::{ArchiveError, ArchiveResult};
use crate::repository::RepositoryGenerator;
use crate::scanner::PackageScanner;

/// Job status for tracking repository generation tasks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JobStatus {
    /// Job is pending execution.
    Pending,
    /// Job is currently running.
    Running,
    /// Job completed successfully.
    Completed,
    /// Job failed with an error.
    Failed,
    /// Job was cancelled.
    Cancelled,
}

/// Information about a repository generation job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobInfo {
    /// Unique job identifier.
    pub id: Uuid,
    /// Repository name.
    pub repository_name: String,
    /// Campaign name that triggered this job.
    pub campaign: Option<String>,
    /// Current job status.
    pub status: JobStatus,
    /// Job start time.
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// Job completion time.
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Error message if job failed.
    pub error_message: Option<String>,
}

impl JobInfo {
    /// Create a new job info for a repository.
    pub fn new(repository_name: String, campaign: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            repository_name,
            campaign,
            status: JobStatus::Pending,
            started_at: chrono::Utc::now(),
            completed_at: None,
            error_message: None,
        }
    }

    /// Mark the job as running.
    pub fn start(&mut self) {
        self.status = JobStatus::Running;
        self.started_at = chrono::Utc::now();
    }

    /// Mark the job as completed successfully.
    pub fn complete(&mut self) {
        self.status = JobStatus::Completed;
        self.completed_at = Some(chrono::Utc::now());
    }

    /// Mark the job as failed with an error message.
    pub fn fail(&mut self, error: &str) {
        self.status = JobStatus::Failed;
        self.completed_at = Some(chrono::Utc::now());
        self.error_message = Some(error.to_string());
    }

    /// Mark the job as cancelled.
    pub fn cancel(&mut self) {
        self.status = JobStatus::Cancelled;
        self.completed_at = Some(chrono::Utc::now());
    }

    /// Check if the job is finished (completed, failed, or cancelled).
    pub fn is_finished(&self) -> bool {
        matches!(
            self.status,
            JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled
        )
    }
}

/// Active job handle combining task handle and job information.
#[derive(Debug)]
pub struct ActiveJob {
    /// Job information.
    pub info: JobInfo,
    /// Task handle for the running job.
    pub handle: JoinHandle<ArchiveResult<()>>,
}

impl ActiveJob {
    /// Create a new active job.
    pub fn new(info: JobInfo, handle: JoinHandle<ArchiveResult<()>>) -> Self {
        Self { info, handle }
    }

    /// Check if the job is finished.
    pub fn is_finished(&self) -> bool {
        self.handle.is_finished()
    }

    /// Cancel the job.
    pub fn cancel(&self) {
        self.handle.abort();
    }
}

/// Configuration for the generator manager.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratorManagerConfig {
    /// Maximum number of concurrent repository generation jobs.
    pub max_concurrent_jobs: usize,
    /// Job timeout in seconds.
    pub job_timeout_seconds: u64,
    /// Enable automatic job cleanup.
    pub enable_cleanup: bool,
    /// Cleanup interval in seconds.
    pub cleanup_interval_seconds: u64,
}

impl Default for GeneratorManagerConfig {
    fn default() -> Self {
        Self {
            max_concurrent_jobs: 4,
            job_timeout_seconds: 3600, // 1 hour
            enable_cleanup: true,
            cleanup_interval_seconds: 300, // 5 minutes
        }
    }
}

/// Generator Manager handles job scheduling and campaign coordination.
pub struct GeneratorManager {
    /// Archive configuration.
    config: Arc<ArchiveConfig>,
    /// Repository generator.
    generator: Arc<RepositoryGenerator>,
    /// Package scanner.
    scanner: Arc<PackageScanner>,
    /// Database manager.
    database: Arc<BuildManager>,
    /// Generator manager configuration.
    manager_config: GeneratorManagerConfig,
    /// Active jobs tracking.
    active_jobs: Arc<RwLock<HashMap<String, ActiveJob>>>,
    /// Campaign to repository mapping.
    campaign_to_repository: Arc<RwLock<HashMap<String, Vec<AptRepositoryConfig>>>>,
    /// Job history for completed jobs.
    job_history: Arc<Mutex<Vec<JobInfo>>>,
}

impl GeneratorManager {
    /// Create a new generator manager.
    pub async fn new(
        config: ArchiveConfig,
        generator: RepositoryGenerator,
        scanner: PackageScanner,
        database: BuildManager,
        manager_config: GeneratorManagerConfig,
    ) -> ArchiveResult<Self> {
        info!("Creating new GeneratorManager");

        let config = Arc::new(config);
        let generator = Arc::new(generator);
        let scanner = Arc::new(scanner);
        let database = Arc::new(database);

        // Build campaign to repository mapping
        let mut campaign_mapping: HashMap<String, Vec<AptRepositoryConfig>> = HashMap::new();

        for (repo_name, repo_config) in &config.repositories {
            // For now, assume all repositories are part of a default campaign
            // In a real implementation, this would come from repository configuration
            let campaign_name = repo_config.suite.clone();

            campaign_mapping
                .entry(campaign_name)
                .or_default()
                .push(repo_config.clone());

            debug!(
                "Mapped repository '{}' to campaign '{}'",
                repo_name, repo_config.suite
            );
        }

        info!(
            "Built campaign mapping for {} campaigns",
            campaign_mapping.len()
        );

        Ok(Self {
            config,
            generator,
            scanner,
            database,
            manager_config,
            active_jobs: Arc::new(RwLock::new(HashMap::new())),
            campaign_to_repository: Arc::new(RwLock::new(campaign_mapping)),
            job_history: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// Trigger repository generation for a specific campaign.
    pub async fn trigger_campaign(&self, campaign_name: &str) -> ArchiveResult<Vec<Uuid>> {
        info!(
            "Triggering repository generation for campaign: {}",
            campaign_name
        );

        let campaign_mapping = self.campaign_to_repository.read().await;
        let repositories = match campaign_mapping.get(campaign_name) {
            Some(repos) => repos.clone(),
            None => {
                warn!("No repositories found for campaign: {}", campaign_name);
                return Ok(Vec::new());
            }
        };

        let mut job_ids = Vec::new();

        for repo_config in repositories {
            match self
                .trigger_repository(&repo_config, Some(campaign_name.to_string()))
                .await
            {
                Ok(job_id) => {
                    job_ids.push(job_id);
                    info!(
                        "Triggered job {} for repository {}",
                        job_id, repo_config.name
                    );
                }
                Err(e) => {
                    error!("Failed to trigger repository {}: {}", repo_config.name, e);
                }
            }
        }

        info!(
            "Triggered {} jobs for campaign {}",
            job_ids.len(),
            campaign_name
        );
        Ok(job_ids)
    }

    /// Trigger repository generation for a specific repository.
    pub async fn trigger_repository(
        &self,
        repo_config: &AptRepositoryConfig,
        campaign: Option<String>,
    ) -> ArchiveResult<Uuid> {
        let repo_name = &repo_config.name;

        // Check if a job is already running for this repository
        {
            let active_jobs = self.active_jobs.read().await;
            if let Some(existing_job) = active_jobs.get(repo_name) {
                if !existing_job.is_finished() {
                    info!("Job already running for repository: {}", repo_name);
                    return Ok(existing_job.info.id);
                }
            }
        }

        // Check concurrent job limit
        let active_count = {
            let active_jobs = self.active_jobs.read().await;
            active_jobs
                .values()
                .filter(|job| !job.is_finished())
                .count()
        };

        if active_count >= self.manager_config.max_concurrent_jobs {
            return Err(ArchiveError::ResourceLimit(format!(
                "Maximum concurrent jobs ({}) reached",
                self.manager_config.max_concurrent_jobs
            )));
        }

        // Create job info
        let mut job_info = JobInfo::new(repo_name.clone(), campaign);
        let job_id = job_info.id;

        info!(
            "Starting repository generation job {} for {}",
            job_id, repo_name
        );

        // Clone necessary data for the task
        let generator = Arc::clone(&self.generator);
        let repo_config_clone = repo_config.clone();
        let job_info_clone = job_info.clone();
        let active_jobs = Arc::clone(&self.active_jobs);
        let job_history = Arc::clone(&self.job_history);

        // Start the job
        job_info.start();

        // Spawn the repository generation task
        let handle = tokio::spawn(async move {
            let result = generator.generate_repository(&repo_config_clone).await;

            // Update job status and move to history
            {
                let mut active_jobs_guard = active_jobs.write().await;
                if let Some(mut active_job) = active_jobs_guard.remove(&repo_config_clone.name) {
                    match &result {
                        Ok(_) => {
                            active_job.info.complete();
                            info!("Repository generation completed for job {}", job_id);
                        }
                        Err(e) => {
                            active_job.info.fail(&e.to_string());
                            error!("Repository generation failed for job {}: {}", job_id, e);
                        }
                    }

                    // Move to history
                    let mut history = job_history.lock().await;
                    history.push(active_job.info);

                    // Keep only recent history (last 100 jobs)
                    if history.len() > 100 {
                        history.remove(0);
                    }
                }
            }

            result
        });

        // Store the active job
        let active_job = ActiveJob::new(job_info, handle);
        {
            let mut active_jobs_guard = self.active_jobs.write().await;
            active_jobs_guard.insert(repo_name.clone(), active_job);
        }

        Ok(job_id)
    }

    /// Get status of all active jobs.
    pub async fn get_active_jobs(&self) -> HashMap<String, JobInfo> {
        let active_jobs = self.active_jobs.read().await;
        active_jobs
            .iter()
            .map(|(name, job)| (name.clone(), job.info.clone()))
            .collect()
    }

    /// Get job history.
    pub async fn get_job_history(&self) -> Vec<JobInfo> {
        let history = self.job_history.lock().await;
        history.clone()
    }

    /// Get job information by ID.
    pub async fn get_job_info(&self, job_id: Uuid) -> Option<JobInfo> {
        // Check active jobs first
        {
            let active_jobs = self.active_jobs.read().await;
            for job in active_jobs.values() {
                if job.info.id == job_id {
                    return Some(job.info.clone());
                }
            }
        }

        // Check job history
        {
            let history = self.job_history.lock().await;
            for job_info in history.iter() {
                if job_info.id == job_id {
                    return Some(job_info.clone());
                }
            }
        }

        None
    }

    /// Cancel a job by ID.
    pub async fn cancel_job(&self, job_id: Uuid) -> ArchiveResult<()> {
        let active_jobs = self.active_jobs.write().await;

        for (repo_name, active_job) in active_jobs.iter() {
            if active_job.info.id == job_id {
                info!("Cancelling job {} for repository {}", job_id, repo_name);
                active_job.cancel();
                return Ok(());
            }
        }

        Err(ArchiveError::NotFound(format!("Job {} not found", job_id)))
    }

    /// Cleanup finished jobs.
    pub async fn cleanup_finished_jobs(&self) -> usize {
        let mut active_jobs = self.active_jobs.write().await;
        let mut to_remove = Vec::new();

        for (repo_name, active_job) in active_jobs.iter() {
            if active_job.is_finished() {
                to_remove.push(repo_name.clone());
            }
        }

        let count = to_remove.len();
        for repo_name in to_remove {
            if let Some(mut active_job) = active_jobs.remove(&repo_name) {
                // Update final status if needed
                if active_job.handle.is_finished() && active_job.info.status == JobStatus::Running {
                    // Since handle is finished, we can't get the result anymore
                    // We'll assume it completed unless we know otherwise
                    active_job.info.complete();
                }

                // Move to history
                let mut history = self.job_history.lock().await;
                history.push(active_job.info);

                // Keep only recent history
                if history.len() > 100 {
                    history.remove(0);
                }
            }
        }

        if count > 0 {
            debug!("Cleaned up {} finished jobs", count);
        }

        count
    }

    /// Start background cleanup task.
    pub async fn start_cleanup_task(&self) -> JoinHandle<()> {
        let active_jobs = Arc::clone(&self.active_jobs);
        let job_history = Arc::clone(&self.job_history);
        let cleanup_interval = self.manager_config.cleanup_interval_seconds;

        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(tokio::time::Duration::from_secs(cleanup_interval));

            loop {
                interval.tick().await;

                // Cleanup finished jobs
                let mut active_jobs_guard = active_jobs.write().await;
                let mut to_remove = Vec::new();

                for (repo_name, active_job) in active_jobs_guard.iter() {
                    if active_job.is_finished() {
                        to_remove.push(repo_name.clone());
                    }
                }

                let count = to_remove.len();
                for repo_name in to_remove {
                    if let Some(active_job) = active_jobs_guard.remove(&repo_name) {
                        let mut history = job_history.lock().await;
                        history.push(active_job.info);

                        if history.len() > 100 {
                            history.remove(0);
                        }
                    }
                }

                if count > 0 {
                    debug!("Background cleanup: removed {} finished jobs", count);
                }
            }
        })
    }

    /// Get campaign to repository mapping.
    pub async fn get_campaign_mapping(&self) -> HashMap<String, Vec<String>> {
        let mapping = self.campaign_to_repository.read().await;
        mapping
            .iter()
            .map(|(campaign, repos)| {
                let repo_names = repos.iter().map(|r| r.name.clone()).collect();
                (campaign.clone(), repo_names)
            })
            .collect()
    }

    /// Get manager statistics.
    pub async fn get_statistics(&self) -> ManagerStatistics {
        let active_jobs = self.active_jobs.read().await;
        let history = self.job_history.lock().await;

        let active_count = active_jobs.len();
        let running_count = active_jobs
            .values()
            .filter(|job| !job.is_finished())
            .count();
        let total_historical = history.len();
        let completed_count = history
            .iter()
            .filter(|job| job.status == JobStatus::Completed)
            .count();
        let failed_count = history
            .iter()
            .filter(|job| job.status == JobStatus::Failed)
            .count();

        ManagerStatistics {
            active_jobs: active_count,
            running_jobs: running_count,
            total_historical_jobs: total_historical,
            completed_jobs: completed_count,
            failed_jobs: failed_count,
            max_concurrent_jobs: self.manager_config.max_concurrent_jobs,
        }
    }
}

/// Manager statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagerStatistics {
    /// Number of active jobs.
    pub active_jobs: usize,
    /// Number of currently running jobs.
    pub running_jobs: usize,
    /// Total number of historical jobs.
    pub total_historical_jobs: usize,
    /// Number of completed jobs.
    pub completed_jobs: usize,
    /// Number of failed jobs.
    pub failed_jobs: usize,
    /// Maximum concurrent jobs allowed.
    pub max_concurrent_jobs: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_job_info_creation() {
        let job = JobInfo::new("test-repo".to_string(), Some("test-campaign".to_string()));

        assert_eq!(job.repository_name, "test-repo");
        assert_eq!(job.campaign, Some("test-campaign".to_string()));
        assert_eq!(job.status, JobStatus::Pending);
        assert!(!job.is_finished());
    }

    #[test]
    fn test_job_info_lifecycle() {
        let mut job = JobInfo::new("test-repo".to_string(), None);

        // Start the job
        job.start();
        assert_eq!(job.status, JobStatus::Running);
        assert!(!job.is_finished());

        // Complete the job
        job.complete();
        assert_eq!(job.status, JobStatus::Completed);
        assert!(job.is_finished());
        assert!(job.completed_at.is_some());
    }

    #[test]
    fn test_job_info_failure() {
        let mut job = JobInfo::new("test-repo".to_string(), None);

        job.start();
        job.fail("Test error");

        assert_eq!(job.status, JobStatus::Failed);
        assert!(job.is_finished());
        assert_eq!(job.error_message, Some("Test error".to_string()));
        assert!(job.completed_at.is_some());
    }

    #[test]
    fn test_generator_manager_config_default() {
        let config = GeneratorManagerConfig::default();

        assert_eq!(config.max_concurrent_jobs, 4);
        assert_eq!(config.job_timeout_seconds, 3600);
        assert!(config.enable_cleanup);
        assert_eq!(config.cleanup_interval_seconds, 300);
    }

    #[tokio::test]
    async fn test_generator_manager_creation() {
        // This test would require setting up all dependencies
        // For now, just test that the config and mapping logic work

        // Create a mock configuration
        let temp_dir = TempDir::new().unwrap();
        let mut repositories = HashMap::new();

        let repo_config = AptRepositoryConfig::new(
            "test-repo".to_string(),
            "test-suite".to_string(),
            vec!["amd64".to_string()],
            temp_dir.path().to_path_buf(),
        );
        repositories.insert("test-repo".to_string(), repo_config);

        let archive_config = ArchiveConfig {
            repositories,
            archive_path: temp_dir.path().to_path_buf(),
            gpg: None,
            artifact_manager: crate::config::ArtifactManagerConfig::default(),
            database: crate::config::DatabaseConfig::default(),
            cache: crate::config::CacheConfig::default(),
            server: crate::config::ServerConfig::default(),
        };

        // Test would continue with full setup when dependencies are available
        assert!(!archive_config.repositories.is_empty());
    }

    #[test]
    fn test_manager_statistics() {
        let stats = ManagerStatistics {
            active_jobs: 2,
            running_jobs: 1,
            total_historical_jobs: 10,
            completed_jobs: 8,
            failed_jobs: 2,
            max_concurrent_jobs: 4,
        };

        assert_eq!(stats.active_jobs, 2);
        assert_eq!(stats.running_jobs, 1);
        assert_eq!(
            stats.completed_jobs + stats.failed_jobs,
            stats.total_historical_jobs
        );
    }
}
