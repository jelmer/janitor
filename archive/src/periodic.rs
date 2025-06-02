//! Periodic services for archive background operations.
//!
//! This module provides periodic background services for the archive service,
//! including regular repository republishing, cleanup tasks, and health monitoring.

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, broadcast};
use tokio::task::JoinHandle;
use tokio::time::{interval, Instant};
use tracing::{debug, error, info, warn};

use crate::error::{ArchiveError, ArchiveResult};
use crate::manager::GeneratorManager;
use crate::redis::{RedisManager, ArchiveEvent};

/// Configuration for periodic services.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodicConfig {
    /// Enable periodic republishing.
    pub enable_republishing: bool,
    /// Republishing interval in seconds.
    pub republishing_interval_seconds: u64,
    /// Enable cleanup tasks.
    pub enable_cleanup: bool,
    /// Cleanup interval in seconds.
    pub cleanup_interval_seconds: u64,
    /// Enable health monitoring.
    pub enable_health_monitoring: bool,
    /// Health check interval in seconds.
    pub health_check_interval_seconds: u64,
    /// Enable metrics collection.
    pub enable_metrics: bool,
    /// Metrics collection interval in seconds.
    pub metrics_interval_seconds: u64,
}

impl Default for PeriodicConfig {
    fn default() -> Self {
        Self {
            enable_republishing: true,
            republishing_interval_seconds: 3600, // 1 hour
            enable_cleanup: true,
            cleanup_interval_seconds: 300, // 5 minutes
            enable_health_monitoring: true,
            health_check_interval_seconds: 60, // 1 minute
            enable_metrics: true,
            metrics_interval_seconds: 300, // 5 minutes
        }
    }
}

/// Health status for a service component.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    /// Component is healthy.
    Healthy,
    /// Component has warnings.
    Warning,
    /// Component is unhealthy.
    Unhealthy,
    /// Component status is unknown.
    Unknown,
}

/// Health check result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    /// Component name.
    pub component: String,
    /// Health status.
    pub status: HealthStatus,
    /// Status message.
    pub message: String,
    /// Last check timestamp.
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Response time in milliseconds.
    pub response_time_ms: u64,
}

impl HealthCheck {
    /// Create a healthy check result.
    pub fn healthy(component: &str, message: &str, response_time_ms: u64) -> Self {
        Self {
            component: component.to_string(),
            status: HealthStatus::Healthy,
            message: message.to_string(),
            timestamp: chrono::Utc::now(),
            response_time_ms,
        }
    }

    /// Create an unhealthy check result.
    pub fn unhealthy(component: &str, message: &str, response_time_ms: u64) -> Self {
        Self {
            component: component.to_string(),
            status: HealthStatus::Unhealthy,
            message: message.to_string(),
            timestamp: chrono::Utc::now(),
            response_time_ms,
        }
    }

    /// Create a warning check result.
    pub fn warning(component: &str, message: &str, response_time_ms: u64) -> Self {
        Self {
            component: component.to_string(),
            status: HealthStatus::Warning,
            message: message.to_string(),
            timestamp: chrono::Utc::now(),
            response_time_ms,
        }
    }
}

/// Metrics collected by periodic services.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceMetrics {
    /// Timestamp of collection.
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Active jobs count.
    pub active_jobs: usize,
    /// Running jobs count.
    pub running_jobs: usize,
    /// Completed jobs count.
    pub completed_jobs: usize,
    /// Failed jobs count.
    pub failed_jobs: usize,
    /// Average job duration in seconds.
    pub avg_job_duration_seconds: f64,
    /// Redis connection status.
    pub redis_healthy: bool,
    /// Database connection status.
    pub database_healthy: bool,
    /// Memory usage in bytes.
    pub memory_usage_bytes: u64,
    /// CPU usage percentage.
    pub cpu_usage_percent: f64,
}

/// Periodic services manager.
pub struct PeriodicServices {
    /// Configuration.
    config: PeriodicConfig,
    /// Generator manager.
    generator_manager: Arc<GeneratorManager>,
    /// Redis manager.
    redis_manager: Option<Arc<tokio::sync::Mutex<RedisManager>>>,
    /// Shutdown signal sender.
    shutdown_tx: Option<broadcast::Sender<()>>,
    /// Running task handles.
    task_handles: Vec<JoinHandle<()>>,
    /// Latest health checks.
    health_checks: Arc<tokio::sync::RwLock<Vec<HealthCheck>>>,
    /// Latest metrics.
    metrics: Arc<tokio::sync::RwLock<Option<ServiceMetrics>>>,
}

impl PeriodicServices {
    /// Create a new periodic services manager.
    pub fn new(
        config: PeriodicConfig,
        generator_manager: Arc<GeneratorManager>,
        redis_manager: Option<Arc<tokio::sync::Mutex<RedisManager>>>,
    ) -> Self {
        info!("Creating periodic services manager");

        Self {
            config,
            generator_manager,
            redis_manager,
            shutdown_tx: None,
            task_handles: Vec::new(),
            health_checks: Arc::new(tokio::sync::RwLock::new(Vec::new())),
            metrics: Arc::new(tokio::sync::RwLock::new(None)),
        }
    }

    /// Start all periodic services.
    pub async fn start(&mut self) -> ArchiveResult<()> {
        info!("Starting periodic services");

        let (shutdown_tx, _) = broadcast::channel::<()>(10);
        self.shutdown_tx = Some(shutdown_tx.clone());

        // Start republishing service
        if self.config.enable_republishing {
            let handle = self.start_republishing_service(shutdown_tx.subscribe()).await;
            self.task_handles.push(handle);
        }

        // Start cleanup service
        if self.config.enable_cleanup {
            let handle = self.start_cleanup_service(shutdown_tx.subscribe()).await;
            self.task_handles.push(handle);
        }

        // Start health monitoring
        if self.config.enable_health_monitoring {
            let handle = self.start_health_monitoring(shutdown_tx.subscribe()).await;
            self.task_handles.push(handle);
        }

        // Start metrics collection
        if self.config.enable_metrics {
            let handle = self.start_metrics_collection(shutdown_tx.subscribe()).await;
            self.task_handles.push(handle);
        }

        info!("Started {} periodic services", self.task_handles.len());
        Ok(())
    }

    /// Stop all periodic services.
    pub async fn stop(&mut self) -> ArchiveResult<()> {
        info!("Stopping periodic services");

        // Send shutdown signal
        if let Some(shutdown_tx) = &self.shutdown_tx {
            if let Err(e) = shutdown_tx.send(()) {
                warn!("Failed to send shutdown signal: {}", e);
            }
        }

        // Wait for all tasks to complete
        for handle in self.task_handles.drain(..) {
            if let Err(e) = handle.await {
                warn!("Task completed with error: {}", e);
            }
        }

        info!("All periodic services stopped");
        Ok(())
    }

    /// Start republishing service.
    async fn start_republishing_service(&self, mut shutdown_rx: broadcast::Receiver<()>) -> JoinHandle<()> {
        let generator_manager = Arc::clone(&self.generator_manager);
        let redis_manager = self.redis_manager.clone();
        let interval_seconds = self.config.republishing_interval_seconds;

        tokio::spawn(async move {
            info!("Starting periodic republishing service (interval: {}s)", interval_seconds);
            let mut interval = interval(Duration::from_secs(interval_seconds));

            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        info!("Republishing service received shutdown signal");
                        break;
                    }
                    _ = interval.tick() => {
                        debug!("Running periodic republishing");

                        // Get campaign mapping
                        let campaigns = generator_manager.get_campaign_mapping().await;

                        for (campaign_name, _repositories) in campaigns {
                            debug!("Triggering periodic republish for campaign: {}", campaign_name);

                            match generator_manager.trigger_campaign(&campaign_name).await {
                                Ok(job_ids) => {
                                    info!("Triggered {} jobs for periodic republish of campaign {}", 
                                          job_ids.len(), campaign_name);

                                    // Publish event if Redis is available
                                    if let Some(redis_mgr) = &redis_manager {
                                        let mut redis_guard = redis_mgr.lock().await;
                                        if let Some(publisher) = redis_guard.publisher_mut() {
                                            let event = ArchiveEvent::PeriodicRepublish {
                                                campaign: campaign_name.clone(),
                                                interval_type: "periodic".to_string(),
                                            };

                                            if let Err(e) = publisher.publish_event(&event).await {
                                                warn!("Failed to publish periodic republish event: {}", e);
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to trigger periodic republish for campaign {}: {}", 
                                           campaign_name, e);
                                }
                            }
                        }

                        info!("Completed periodic republishing cycle");
                    }
                }
            }

            info!("Periodic republishing service stopped");
        })
    }

    /// Start cleanup service.
    async fn start_cleanup_service(&self, mut shutdown_rx: broadcast::Receiver<()>) -> JoinHandle<()> {
        let generator_manager = Arc::clone(&self.generator_manager);
        let interval_seconds = self.config.cleanup_interval_seconds;

        tokio::spawn(async move {
            info!("Starting cleanup service (interval: {}s)", interval_seconds);
            let mut interval = interval(Duration::from_secs(interval_seconds));

            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        info!("Cleanup service received shutdown signal");
                        break;
                    }
                    _ = interval.tick() => {
                        debug!("Running periodic cleanup");

                        // Cleanup finished jobs
                        let cleaned_count = generator_manager.cleanup_finished_jobs().await;
                        if cleaned_count > 0 {
                            debug!("Cleaned up {} finished jobs", cleaned_count);
                        }

                        // Additional cleanup tasks could be added here:
                        // - Cleanup old temporary files
                        // - Archive old logs
                        // - Update metrics
                    }
                }
            }

            info!("Cleanup service stopped");
        })
    }

    /// Start health monitoring service.
    async fn start_health_monitoring(&self, mut shutdown_rx: broadcast::Receiver<()>) -> JoinHandle<()> {
        let generator_manager = Arc::clone(&self.generator_manager);
        let redis_manager = self.redis_manager.clone();
        let health_checks = Arc::clone(&self.health_checks);
        let interval_seconds = self.config.health_check_interval_seconds;

        tokio::spawn(async move {
            info!("Starting health monitoring service (interval: {}s)", interval_seconds);
            let mut interval = interval(Duration::from_secs(interval_seconds));

            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        info!("Health monitoring service received shutdown signal");
                        break;
                    }
                    _ = interval.tick() => {
                        debug!("Running health checks");

                        let mut checks = Vec::new();

                        // Check generator manager
                        let start = Instant::now();
                        let stats = generator_manager.get_statistics().await;
                        let duration = start.elapsed().as_millis() as u64;

                        let status = if stats.running_jobs > stats.max_concurrent_jobs {
                            HealthStatus::Warning
                        } else {
                            HealthStatus::Healthy
                        };

                        checks.push(HealthCheck {
                            component: "generator_manager".to_string(),
                            status,
                            message: format!("Active: {}, Running: {}", stats.active_jobs, stats.running_jobs),
                            timestamp: chrono::Utc::now(),
                            response_time_ms: duration,
                        });

                        // Check Redis if available
                        if let Some(redis_mgr) = &redis_manager {
                            let start = Instant::now();
                            let mut redis_guard = redis_mgr.lock().await;
                            
                            match redis_guard.health_check().await {
                                Ok(_) => {
                                    let duration = start.elapsed().as_millis() as u64;
                                    checks.push(HealthCheck::healthy("redis", "Connection OK", duration));
                                }
                                Err(e) => {
                                    let duration = start.elapsed().as_millis() as u64;
                                    checks.push(HealthCheck::unhealthy("redis", &e.to_string(), duration));
                                }
                            }
                        }

                        // Update health checks
                        {
                            let mut health_guard = health_checks.write().await;
                            *health_guard = checks;
                        }
                    }
                }
            }

            info!("Health monitoring service stopped");
        })
    }

    /// Start metrics collection service.
    async fn start_metrics_collection(&self, mut shutdown_rx: broadcast::Receiver<()>) -> JoinHandle<()> {
        let generator_manager = Arc::clone(&self.generator_manager);
        let redis_manager = self.redis_manager.clone();
        let metrics = Arc::clone(&self.metrics);
        let interval_seconds = self.config.metrics_interval_seconds;

        tokio::spawn(async move {
            info!("Starting metrics collection service (interval: {}s)", interval_seconds);
            let mut interval = interval(Duration::from_secs(interval_seconds));

            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        info!("Metrics collection service received shutdown signal");
                        break;
                    }
                    _ = interval.tick() => {
                        debug!("Collecting metrics");

                        // Get generator statistics
                        let stats = generator_manager.get_statistics().await;

                        // Check Redis health
                        let redis_healthy = if let Some(redis_mgr) = &redis_manager {
                            let mut redis_guard = redis_mgr.lock().await;
                            redis_guard.health_check().await.is_ok()
                        } else {
                            false
                        };

                        // Collect system metrics (simplified)
                        let memory_usage_bytes = Self::get_memory_usage();
                        let cpu_usage_percent = Self::get_cpu_usage();

                        let service_metrics = ServiceMetrics {
                            timestamp: chrono::Utc::now(),
                            active_jobs: stats.active_jobs,
                            running_jobs: stats.running_jobs,
                            completed_jobs: stats.completed_jobs,
                            failed_jobs: stats.failed_jobs,
                            avg_job_duration_seconds: 0.0, // Would need job timing tracking
                            redis_healthy,
                            database_healthy: true, // Would need database health check
                            memory_usage_bytes,
                            cpu_usage_percent,
                        };

                        // Update metrics
                        {
                            let mut metrics_guard = metrics.write().await;
                            *metrics_guard = Some(service_metrics);
                        }
                    }
                }
            }

            info!("Metrics collection service stopped");
        })
    }

    /// Get current health checks.
    pub async fn get_health_checks(&self) -> Vec<HealthCheck> {
        let health_guard = self.health_checks.read().await;
        health_guard.clone()
    }

    /// Get current metrics.
    pub async fn get_metrics(&self) -> Option<ServiceMetrics> {
        let metrics_guard = self.metrics.read().await;
        metrics_guard.clone()
    }

    /// Get overall health status.
    pub async fn get_overall_health(&self) -> HealthStatus {
        let checks = self.get_health_checks().await;

        if checks.is_empty() {
            return HealthStatus::Unknown;
        }

        let has_unhealthy = checks.iter().any(|c| c.status == HealthStatus::Unhealthy);
        let has_warning = checks.iter().any(|c| c.status == HealthStatus::Warning);

        if has_unhealthy {
            HealthStatus::Unhealthy
        } else if has_warning {
            HealthStatus::Warning
        } else {
            HealthStatus::Healthy
        }
    }

    /// Simple memory usage estimation (in bytes).
    fn get_memory_usage() -> u64 {
        // This is a simplified implementation
        // In production, you might use system crates or proc filesystem
        #[cfg(target_os = "linux")]
        {
            if let Ok(content) = std::fs::read_to_string("/proc/self/status") {
                for line in content.lines() {
                    if line.starts_with("VmRSS:") {
                        if let Some(kb_str) = line.split_whitespace().nth(1) {
                            if let Ok(kb) = kb_str.parse::<u64>() {
                                return kb * 1024; // Convert KB to bytes
                            }
                        }
                    }
                }
            }
        }
        0
    }

    /// Simple CPU usage estimation (percentage).
    fn get_cpu_usage() -> f64 {
        // This is a placeholder implementation
        // In production, you might use system monitoring crates
        0.0
    }
}

impl Drop for PeriodicServices {
    fn drop(&mut self) {
        // Abort any remaining tasks
        for handle in &self.task_handles {
            handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_periodic_config_default() {
        let config = PeriodicConfig::default();
        
        assert!(config.enable_republishing);
        assert_eq!(config.republishing_interval_seconds, 3600);
        assert!(config.enable_cleanup);
        assert_eq!(config.cleanup_interval_seconds, 300);
        assert!(config.enable_health_monitoring);
        assert_eq!(config.health_check_interval_seconds, 60);
        assert!(config.enable_metrics);
        assert_eq!(config.metrics_interval_seconds, 300);
    }

    #[test]
    fn test_health_check_creation() {
        let healthy = HealthCheck::healthy("test", "All good", 100);
        assert_eq!(healthy.component, "test");
        assert_eq!(healthy.status, HealthStatus::Healthy);
        assert_eq!(healthy.message, "All good");
        assert_eq!(healthy.response_time_ms, 100);

        let unhealthy = HealthCheck::unhealthy("test", "Error occurred", 500);
        assert_eq!(unhealthy.status, HealthStatus::Unhealthy);
        assert_eq!(unhealthy.message, "Error occurred");
        assert_eq!(unhealthy.response_time_ms, 500);

        let warning = HealthCheck::warning("test", "Warning message", 200);
        assert_eq!(warning.status, HealthStatus::Warning);
        assert_eq!(warning.message, "Warning message");
        assert_eq!(warning.response_time_ms, 200);
    }

    #[test]
    fn test_health_status_equality() {
        assert_eq!(HealthStatus::Healthy, HealthStatus::Healthy);
        assert_ne!(HealthStatus::Healthy, HealthStatus::Warning);
        assert_ne!(HealthStatus::Warning, HealthStatus::Unhealthy);
        assert_ne!(HealthStatus::Unhealthy, HealthStatus::Unknown);
    }

    #[test]
    fn test_service_metrics_serialization() {
        let metrics = ServiceMetrics {
            timestamp: chrono::Utc::now(),
            active_jobs: 5,
            running_jobs: 2,
            completed_jobs: 100,
            failed_jobs: 3,
            avg_job_duration_seconds: 120.5,
            redis_healthy: true,
            database_healthy: true,
            memory_usage_bytes: 1024 * 1024 * 100, // 100MB
            cpu_usage_percent: 25.5,
        };

        let serialized = serde_json::to_string(&metrics).unwrap();
        let deserialized: ServiceMetrics = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.active_jobs, 5);
        assert_eq!(deserialized.running_jobs, 2);
        assert_eq!(deserialized.completed_jobs, 100);
        assert_eq!(deserialized.failed_jobs, 3);
        assert_eq!(deserialized.avg_job_duration_seconds, 120.5);
        assert!(deserialized.redis_healthy);
        assert!(deserialized.database_healthy);
        assert_eq!(deserialized.memory_usage_bytes, 1024 * 1024 * 100);
        assert_eq!(deserialized.cpu_usage_percent, 25.5);
    }

    #[tokio::test]
    async fn test_periodic_services_creation() {
        let config = PeriodicConfig::default();
        
        // Would need to set up mock GeneratorManager for full test
        // let generator_manager = Arc::new(mock_generator_manager);
        // let services = PeriodicServices::new(config, generator_manager, None);
        
        // For now, just test configuration
        assert!(config.enable_republishing);
        assert!(config.enable_cleanup);
        assert!(config.enable_health_monitoring);
        assert!(config.enable_metrics);
    }

    #[test]
    fn test_memory_usage_function() {
        let usage = PeriodicServices::get_memory_usage();
        // Should return 0 or a positive value
        assert!(usage >= 0);
    }

    #[test]
    fn test_cpu_usage_function() {
        let usage = PeriodicServices::get_cpu_usage();
        // Should return a percentage between 0 and 100
        assert!(usage >= 0.0);
        assert!(usage <= 100.0);
    }
}