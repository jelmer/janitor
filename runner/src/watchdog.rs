//! Watchdog system for monitoring active runs.

use crate::ActiveRun;
use crate::database::RunnerDatabase;
use chrono::{DateTime, Utc, Duration};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{interval, sleep};

/// Reasons why a run might be terminated.
#[derive(Debug, Clone)]
pub enum TerminationReason {
    /// Run exceeded its maximum allowed duration.
    Timeout,
    /// Worker health check failed.
    HealthCheckFailed,
    /// Run was manually killed.
    ManualKill,
    /// Worker disappeared or stopped responding.
    WorkerDisappeared,
    /// Resource constraints or system issues.
    SystemFailure(String),
}

impl TerminationReason {
    /// Get the result code for this termination reason.
    pub fn result_code(&self) -> &'static str {
        match self {
            TerminationReason::Timeout => "worker-timeout",
            TerminationReason::HealthCheckFailed => "worker-failure", 
            TerminationReason::ManualKill => "killed",
            TerminationReason::WorkerDisappeared => "worker-disappeared",
            TerminationReason::SystemFailure(_) => "system-failure",
        }
    }
    
    /// Get a human-readable description.
    pub fn description(&self) -> String {
        match self {
            TerminationReason::Timeout => "Run exceeded maximum allowed duration".to_string(),
            TerminationReason::HealthCheckFailed => "Worker health check failed".to_string(),
            TerminationReason::ManualKill => "Run was manually terminated".to_string(),
            TerminationReason::WorkerDisappeared => "Worker stopped responding".to_string(),
            TerminationReason::SystemFailure(msg) => format!("System failure: {}", msg),
        }
    }
    
    /// Check if this failure is transient (retriable).
    pub fn is_transient(&self) -> bool {
        match self {
            TerminationReason::Timeout => true,
            TerminationReason::HealthCheckFailed => true,
            TerminationReason::ManualKill => false,
            TerminationReason::WorkerDisappeared => true,
            TerminationReason::SystemFailure(_) => true,
        }
    }
    
    /// Create structured failure details for database storage.
    pub fn create_failure_details(&self, run: &crate::ActiveRun) -> serde_json::Value {
        use serde_json::json;
        use chrono::Utc;
        
        let mut details = json!({
            "termination_reason": self.result_code(),
            "description": self.description(),
            "is_transient": self.is_transient(),
            "worker_name": run.worker_name,
            "codebase": run.codebase,
            "campaign": run.campaign,
            "log_id": run.log_id,
            "terminated_at": Utc::now().to_rfc3339(),
        });
        
        // Add run duration if available
        if let Some(start_time) = run.start_time {
            let duration = Utc::now().signed_duration_since(start_time);
            details["run_duration_seconds"] = json!(duration.num_seconds());
        }
        
        // Add estimated vs actual duration comparison if available
        if let Some(estimated) = run.estimated_duration {
            details["estimated_duration_seconds"] = json!(estimated.as_secs());
        }
        
        // Add backchannel information
        details["backchannel"] = run.backchannel.to_json();
        
        // Add specific details based on termination reason
        match self {
            TerminationReason::Timeout => {
                details["timeout_type"] = json!("watchdog_timeout");
            },
            TerminationReason::HealthCheckFailed => {
                details["health_check_failed"] = json!(true);
            },
            TerminationReason::ManualKill => {
                details["manual_termination"] = json!(true);
            },
            TerminationReason::WorkerDisappeared => {
                details["worker_unreachable"] = json!(true);
            },
            TerminationReason::SystemFailure(msg) => {
                details["system_failure_message"] = json!(msg);
            },
        }
        
        details
    }
}

/// Configuration for the watchdog system.
#[derive(Debug, Clone)]
pub struct WatchdogConfig {
    /// How often to check active runs (in seconds).
    pub check_interval: u64,
    /// Default timeout for runs without explicit timeout (in seconds).
    pub default_timeout: u64,
    /// Maximum allowed timeout for any run (in seconds).
    pub max_timeout: u64,
    /// How long to wait before considering a worker disappeared (in seconds).
    pub worker_heartbeat_timeout: u64,
    /// Maximum number of health check failures before terminating.
    pub max_health_failures: u32,
    /// How often to run maintenance tasks (in seconds).
    pub maintenance_interval: u64,
    /// Maximum age for stale runs before cleanup (in hours).
    pub max_run_age_hours: i64,
    /// Maximum number of retries for failed runs.
    pub max_retries: i32,
    /// Minimum delay before retrying failed runs (in hours).
    pub min_retry_delay_hours: i64,
}

impl Default for WatchdogConfig {
    fn default() -> Self {
        Self {
            check_interval: 30,           // 30 seconds
            default_timeout: 3600,        // 1 hour 
            max_timeout: 14400,           // 4 hours
            worker_heartbeat_timeout: 300, // 5 minutes
            max_health_failures: 3,
            maintenance_interval: 300,    // 5 minutes
            max_run_age_hours: 6,         // 6 hours
            max_retries: 3,
            min_retry_delay_hours: 1,     // 1 hour
        }
    }
}

/// Background watchdog task for monitoring active runs.
pub struct Watchdog {
    database: Arc<RunnerDatabase>,
    config: WatchdogConfig,
    health_failures: HashMap<String, u32>,
}

impl Watchdog {
    /// Create a new watchdog instance.
    pub fn new(database: Arc<RunnerDatabase>, config: WatchdogConfig) -> Self {
        Self {
            database,
            config,
            health_failures: HashMap::new(),
        }
    }

    /// Start the watchdog monitoring loop.
    pub async fn start(&mut self) {
        log::info!("Starting watchdog with check interval {} seconds, maintenance interval {} seconds", 
                  self.config.check_interval, self.config.maintenance_interval);
        
        let mut check_timer = interval(std::time::Duration::from_secs(self.config.check_interval));
        let mut maintenance_timer = interval(std::time::Duration::from_secs(self.config.maintenance_interval));
        
        loop {
            tokio::select! {
                _ = check_timer.tick() => {
                    if let Err(e) = self.check_active_runs().await {
                        log::error!("Watchdog check failed: {}", e);
                    }
                }
                _ = maintenance_timer.tick() => {
                    if let Err(e) = self.run_maintenance().await {
                        log::error!("Watchdog maintenance failed: {}", e);
                    }
                }
            }
        }
    }

    /// Check all active runs for timeouts and health issues.
    async fn check_active_runs(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let active_runs = self.database.get_active_runs().await?;
        let now = Utc::now();
        
        log::debug!("Checking {} active runs", active_runs.len());
        
        for run in active_runs {
            if let Err(e) = self.check_single_run(&run, now).await {
                log::error!("Failed to check run {}: {}", run.log_id, e);
            }
        }
        
        Ok(())
    }

    /// Check a single active run for issues.
    async fn check_single_run(
        &mut self,
        run: &ActiveRun,
        now: DateTime<Utc>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Check for timeout
        if let Some(reason) = self.check_timeout(run, now) {
            log::warn!("Terminating run {} due to: {:?}", run.log_id, reason);
            self.terminate_run(run, reason).await?;
            return Ok(());
        }

        // Check worker health
        if let Some(reason) = self.check_worker_health(run, now).await? {
            log::warn!("Terminating run {} due to health check: {:?}", run.log_id, reason);
            self.terminate_run(run, reason).await?;
            return Ok(());
        }

        Ok(())
    }

    /// Check if a run has exceeded its timeout.
    fn check_timeout(&self, run: &ActiveRun, now: DateTime<Utc>) -> Option<TerminationReason> {
        let timeout_duration = run.estimated_duration
            .map(|d| d.as_secs().min(self.config.max_timeout))
            .unwrap_or(self.config.default_timeout);
            
        let timeout_time = run.start_time + Duration::seconds(timeout_duration as i64);
        
        if now > timeout_time {
            Some(TerminationReason::Timeout)
        } else {
            None
        }
    }

    /// Check worker health via backchannel.
    async fn check_worker_health(
        &mut self,
        run: &ActiveRun,
        now: DateTime<Utc>,
    ) -> Result<Option<TerminationReason>, Box<dyn std::error::Error + Send + Sync>> {
        // Try to get health status via backchannel
        match run.backchannel.get_health_status(&run.log_id).await {
            Ok(health) => {
                // Check heartbeat timestamp if available
                if let Some(last_ping) = health.last_ping {
                    let heartbeat_timeout = Duration::seconds(self.config.worker_heartbeat_timeout as i64);
                    let last_heartbeat_cutoff = now - heartbeat_timeout;
                    
                    if last_ping < last_heartbeat_cutoff {
                        log::warn!("Worker heartbeat timeout for run {}: last ping was {} seconds ago", 
                                 run.log_id, (now - last_ping).num_seconds());
                        
                        let failures = self.health_failures.entry(run.log_id.clone()).or_insert(0);
                        *failures += 1;
                        
                        if *failures >= self.config.max_health_failures {
                            return Ok(Some(TerminationReason::WorkerDisappeared));
                        } else {
                            log::warn!("Heartbeat timeout {}/{} for run {}", 
                                     failures, self.config.max_health_failures, run.log_id);
                            return Ok(None);
                        }
                    }
                }

                // Check worker status
                match health.status.as_str() {
                    "healthy" | "running" | "building" | "completed" => {
                        // Worker is alive and responding properly
                        self.health_failures.remove(&run.log_id);
                        
                        // Additional check: if run is completed but still in active list, 
                        // this might indicate a cleanup issue
                        if health.status == "completed" && health.current_run_id.as_ref() != Some(&run.log_id) {
                            log::warn!("Worker reports completion of different run ({:?}) than expected ({})", 
                                     health.current_run_id, run.log_id);
                            return Ok(Some(TerminationReason::WorkerDisappeared));
                        }
                        
                        Ok(None)
                    }
                    "unhealthy" | "failed" | "aborted" => {
                        let failures = self.health_failures.entry(run.log_id.clone()).or_insert(0);
                        *failures += 1;
                        
                        if *failures >= self.config.max_health_failures {
                            Ok(Some(TerminationReason::HealthCheckFailed))
                        } else {
                            log::warn!("Health check failure {}/{} for run {} (status: {})", 
                                     failures, self.config.max_health_failures, run.log_id, health.status);
                            Ok(None)
                        }
                    }
                    "not-found" | "unreachable" => {
                        // Worker or job no longer exists
                        Ok(Some(TerminationReason::WorkerDisappeared))
                    }
                    "different-run" => {
                        // Worker started processing a different run
                        log::warn!("Worker started processing different run: expected {}, got {:?}", 
                                 run.log_id, health.current_run_id);
                        Ok(Some(TerminationReason::WorkerDisappeared))
                    }
                    _ => {
                        log::warn!("Unknown health status '{}' for run {}", health.status, run.log_id);
                        
                        // Treat unknown status as potential issue but not immediate failure
                        let failures = self.health_failures.entry(run.log_id.clone()).or_insert(0);
                        *failures += 1;
                        
                        if *failures >= self.config.max_health_failures {
                            Ok(Some(TerminationReason::HealthCheckFailed))
                        } else {
                            Ok(None)
                        }
                    }
                }
            }
            Err(e) => {
                log::debug!("Failed to get health status for run {}: {}", run.log_id, e);
                
                // Check for specific error types in the chain
                let error_string = e.to_string();
                let is_fatal_error = error_string.contains("Job not found") 
                    || error_string.contains("Fatal failure")
                    || error_string.contains("not-found");
                let is_unreachable = error_string.contains("Worker unreachable") 
                    || error_string.contains("timeout")
                    || error_string.contains("connection");
                
                if is_fatal_error {
                    log::info!("Fatal error for run {}: {}", run.log_id, e);
                    return Ok(Some(TerminationReason::WorkerDisappeared));
                }
                
                let failure_reason = if is_unreachable {
                    TerminationReason::WorkerDisappeared
                } else {
                    TerminationReason::HealthCheckFailed
                };
                
                // Increment failure count for non-fatal errors
                let failures = self.health_failures.entry(run.log_id.clone()).or_insert(0);
                *failures += 1;
                
                if *failures >= self.config.max_health_failures {
                    Ok(Some(failure_reason))
                } else {
                    log::warn!("Health check error {}/{} for run {}: {}", 
                             failures, self.config.max_health_failures, run.log_id, e);
                    Ok(None)
                }
            }
        }
    }

    /// Terminate a run and clean up its state.
    async fn terminate_run(
        &mut self,
        run: &ActiveRun,
        reason: TerminationReason,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        log::info!("Terminating run {} (worker: {}): {}", 
                  run.log_id, run.worker_name, reason.description());

        // Try to signal the worker to stop via backchannel
        if let Err(e) = run.backchannel.terminate(&run.log_id).await {
            log::warn!("Failed to signal worker termination for run {}: {}", run.log_id, e);
        }

        // Wait a bit for graceful shutdown
        sleep(std::time::Duration::from_secs(5)).await;

        // Create failure result
        let result_code = reason.result_code();
        let description = reason.description();
        let now = Utc::now();

        // Create structured failure details
        let failure_details = reason.create_failure_details(run);
        
        // Update run result in database
        self.database.update_run_result(
            &run.log_id,
            result_code,
            Some(&description),
            Some(&failure_details),
            Some(reason.is_transient()),
            now,
        ).await.map_err(|e| format!("Failed to update run result: {}", e))?;

        // Remove from active runs
        self.database.remove_active_run(&run.log_id).await
            .map_err(|e| format!("Failed to remove active run: {}", e))?;

        // Clean up health failure tracking
        self.health_failures.remove(&run.log_id);

        log::info!("Successfully terminated and cleaned up run {}", run.log_id);
        Ok(())
    }

    /// Run periodic maintenance tasks.
    async fn run_maintenance(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        log::debug!("Running watchdog maintenance tasks");
        
        // Clean up stale active runs
        let cleaned = self.database.cleanup_stale_runs(self.config.max_run_age_hours).await?;
        if cleaned > 0 {
            log::info!("Cleaned up {} stale active runs", cleaned);
        }
        
        // Mark eligible runs for retry
        let retried = self.database.mark_runs_for_retry(
            self.config.max_retries,
            self.config.min_retry_delay_hours,
        ).await?;
        if retried > 0 {
            log::info!("Marked {} runs for retry", retried);
        }
        
        // General database maintenance
        self.database.maintenance_cleanup().await?;
        
        Ok(())
    }

    /// Manually terminate a specific run.
    pub async fn kill_run(
        &mut self,
        run_id: &str,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(run) = self.database.get_active_run(run_id).await? {
            self.terminate_run(&run, TerminationReason::ManualKill).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get current watchdog statistics.
    pub fn get_stats(&self) -> WatchdogStats {
        WatchdogStats {
            runs_with_health_failures: self.health_failures.len(),
            total_health_failures: self.health_failures.values().sum(),
        }
    }

    /// Get detailed health status for all active runs.
    pub async fn get_detailed_health_status(&self) -> Result<Vec<RunHealthStatus>, Box<dyn std::error::Error + Send + Sync>> {
        let active_runs = self.database.get_active_runs().await?;
        let mut health_statuses = Vec::new();
        
        for run in active_runs {
            let health_status = match run.backchannel.get_health_status(&run.log_id).await {
                Ok(health) => Some(health),
                Err(e) => {
                    log::debug!("Failed to get health for run {}: {}", run.log_id, e);
                    None
                }
            };
            
            let failure_count = self.health_failures.get(&run.log_id).copied().unwrap_or(0);
            
            health_statuses.push(RunHealthStatus {
                log_id: run.log_id,
                worker_name: run.worker_name,
                start_time: run.start_time,
                estimated_duration: run.estimated_duration,
                health: health_status,
                failure_count,
                max_failures: self.config.max_health_failures,
            });
        }
        
        Ok(health_statuses)
    }

    /// Force a health check on a specific run.
    pub async fn check_run_health(&mut self, run_id: &str) -> Result<Option<RunHealthStatus>, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(run) = self.database.get_active_run(run_id).await? {
            let now = Utc::now();
            
            // Run the health check
            let termination_reason = self.check_worker_health(&run, now).await?;
            
            // Get current health status
            let health_status = match run.backchannel.get_health_status(&run.log_id).await {
                Ok(health) => Some(health),
                Err(_) => None,
            };
            
            let failure_count = self.health_failures.get(&run.log_id).copied().unwrap_or(0);
            
            // If termination was triggered, handle it
            if let Some(reason) = termination_reason {
                log::info!("Health check triggered termination for run {}: {:?}", run_id, reason);
                self.terminate_run(&run, reason).await?;
            }
            
            Ok(Some(RunHealthStatus {
                log_id: run.log_id,
                worker_name: run.worker_name,
                start_time: run.start_time,
                estimated_duration: run.estimated_duration,
                health: health_status,
                failure_count,
                max_failures: self.config.max_health_failures,
            }))
        } else {
            Ok(None)
        }
    }

    /// Get comprehensive failure and retry statistics.
    pub async fn get_comprehensive_stats(&self) -> Result<HashMap<String, i64>, Box<dyn std::error::Error + Send + Sync>> {
        let mut stats = self.database.get_failure_stats().await?;
        
        // Add watchdog-specific stats
        stats.insert("runs_with_health_failures".to_string(), self.health_failures.len() as i64);
        stats.insert("total_health_failures".to_string(), self.health_failures.values().sum::<u32>() as i64);
        
        Ok(stats)
    }
}

/// Statistics about watchdog operation.
#[derive(Debug, Clone)]
pub struct WatchdogStats {
    /// Number of runs currently experiencing health check failures.
    pub runs_with_health_failures: usize,
    /// Total number of health check failures across all monitored runs.
    pub total_health_failures: u32,
}

/// Detailed health status for a specific run.
#[derive(Debug, Clone)]
pub struct RunHealthStatus {
    /// The log ID of the run.
    pub log_id: String,
    /// Name of the worker processing this run.
    pub worker_name: String,
    /// When the run started.
    pub start_time: DateTime<Utc>,
    /// Estimated duration for the run.
    pub estimated_duration: Option<std::time::Duration>,
    /// Current health status from the backchannel.
    pub health: Option<crate::HealthStatus>,
    /// Number of consecutive health check failures.
    pub failure_count: u32,
    /// Maximum allowed failures before termination.
    pub max_failures: u32,
}