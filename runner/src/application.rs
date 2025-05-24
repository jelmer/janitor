//! Application initialization and orchestration for the runner.

use crate::{
    artifacts::{ArtifactConfig, ArtifactManager},
    config::RunnerConfig,
    database::RunnerDatabase,
    error_tracking::{ErrorTracker, ErrorTrackingConfig},
    logs::{LogConfig, LogFileManager},
    metrics::MetricsCollector,
    performance::{PerformanceConfig, PerformanceMonitor},
    vcs::RunnerVcsManager,
    AppState,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;

/// Application configuration combining all subsystem configurations.
#[derive(Debug, Clone)]
pub struct ApplicationConfig {
    /// Database URL for connection.
    pub database_url: String,
    /// Redis URL for coordination (optional).
    pub redis_url: Option<String>,
    /// Log management configuration.
    pub log_config: LogConfig,
    /// Artifact storage configuration.
    pub artifact_config: ArtifactConfig,
    /// Performance monitoring configuration.
    pub performance_config: PerformanceConfig,
    /// Error tracking configuration.
    pub error_tracking_config: ErrorTrackingConfig,
    /// Metrics collection interval.
    pub metrics_interval: Duration,
    /// Enable graceful shutdown handling.
    pub enable_graceful_shutdown: bool,
    /// Shutdown timeout.
    pub shutdown_timeout: Duration,
}

impl Default for ApplicationConfig {
    fn default() -> Self {
        Self {
            database_url: "postgresql://localhost/janitor".to_string(),
            redis_url: None,
            log_config: LogConfig::default(),
            artifact_config: ArtifactConfig::default(),
            performance_config: PerformanceConfig::default(),
            error_tracking_config: ErrorTrackingConfig::default(),
            metrics_interval: Duration::from_secs(30),
            enable_graceful_shutdown: true,
            shutdown_timeout: Duration::from_secs(30),
        }
    }
}

/// Application builder for configuring and initializing the runner.
pub struct ApplicationBuilder {
    config: RunnerConfig,
}

impl ApplicationBuilder {
    /// Create a new application builder with default configuration.
    pub fn new() -> Self {
        Self {
            config: RunnerConfig::default(),
        }
    }

    /// Create a new application builder from configuration.
    pub fn from_config(config: RunnerConfig) -> Self {
        Self { config }
    }

    /// Create a new application builder from a config file.
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self, ApplicationError> {
        let config = RunnerConfig::from_file(path)
            .map_err(|e| ApplicationError::Configuration(format!("Failed to load config: {}", e)))?;
        Ok(Self::from_config(config))
    }

    /// Set the database URL.
    pub fn with_database_url(mut self, url: String) -> Self {
        self.config.database.url = url;
        self
    }

    /// Set the Redis URL for coordination.
    pub fn with_redis_url(mut self, url: Option<String>) -> Self {
        if let Some(url) = url {
            self.config.redis = Some(crate::config::RedisConfig {
                url,
                ..Default::default()
            });
        } else {
            self.config.redis = None;
        }
        self
    }

    /// Set the web server port.
    pub fn with_port(mut self, port: u16) -> Self {
        self.config.web.port = port;
        self
    }

    /// Set the listen address.
    pub fn with_listen_address(mut self, address: String) -> Self {
        self.config.web.listen_address = address;
        self
    }

    /// Enable debug mode.
    pub fn with_debug(mut self, debug: bool) -> Self {
        self.config.application.debug = debug;
        self
    }

    /// Build and initialize the application.
    pub async fn build(self) -> Result<Application, ApplicationError> {
        // Initialize tracing and logging first
        crate::tracing::init_tracing(&self.config.tracing)
            .map_err(|e| ApplicationError::Configuration(format!("Failed to initialize tracing: {}", e)))?;

        log::info!("Initializing Janitor Runner application...");

        // Validate configuration
        self.config.validate()
            .map_err(|e| ApplicationError::Configuration(format!("Configuration validation failed: {}", e)))?;

        // Initialize metrics first so other systems can use them
        log::info!("Initializing metrics collection...");
        let metrics = Arc::new(MetricsCollector {});
        crate::metrics::init_metrics();

        // Initialize error tracking
        log::info!("Initializing error tracking...");
        let error_tracker = Arc::new(ErrorTracker::new(self.config.error_tracking.clone()));

        // Create database connection pool
        log::info!("Initializing database connection...");
        let janitor_config = self.config.to_janitor_config();
        let database_pool = match janitor::state::create_pool(&janitor_config).await {
            Ok(pool) => pool,
            Err(e) => {
                let error = ApplicationError::Database(format!("Failed to create database pool: {}", e));
                error_tracker.track_error(error_tracker.create_tracked_error(
                    &error,
                    crate::error_tracking::ErrorCategory::Database,
                    "application",
                    "initialization",
                )).await;
                return Err(error);
            }
        };

        let database = Arc::new(RunnerDatabase::new_with_redis_url(
            database_pool,
            self.config.redis.as_ref().map(|r| r.url.clone()),
        ).await.map_err(|e| {
            ApplicationError::Database(format!("Failed to initialize database: {}", e))
        })?);

        // Initialize VCS management
        log::info!("Initializing VCS management...");
        let vcs_manager = Arc::new(RunnerVcsManager::from_config(&janitor_config));

        // Initialize log management
        log::info!("Initializing log management...");
        let log_manager = Arc::new(LogFileManager::new(self.config.logs.clone()).await.map_err(|e| {
            ApplicationError::LogManagement(format!("Failed to initialize log manager: {}", e))
        })?);

        // Initialize artifact management
        log::info!("Initializing artifact management...");
        let artifact_manager = Arc::new(ArtifactManager::new(self.config.artifacts.clone()).await.map_err(|e| {
            ApplicationError::ArtifactManagement(format!("Failed to initialize artifact manager: {}", e))
        })?);

        // Initialize performance monitoring
        log::info!("Initializing performance monitoring...");
        let performance_monitor = Arc::new(PerformanceMonitor::new(
            self.config.performance.collection_interval,
        ));

        // Start performance monitoring
        performance_monitor.start_monitoring(self.config.performance.thresholds.clone()).await;

        // Initialize upload processor
        log::info!("Initializing upload processor...");
        let upload_storage_dir = std::path::PathBuf::from("/tmp/janitor-uploads"); // TODO: Make configurable
        let upload_processor = Arc::new(crate::upload::UploadProcessor::new(
            upload_storage_dir,
            100 * 1024 * 1024, // 100MB max file size
            500 * 1024 * 1024, // 500MB max total size
        ));

        // Create application state
        let app_state = Arc::new(AppState {
            database,
            vcs_manager,
            log_manager,
            artifact_manager,
            performance_monitor,
            error_tracker,
            metrics,
            config: Arc::new(janitor_config),
            upload_processor,
        });

        log::info!("Janitor Runner application initialized successfully");

        Ok(Application {
            state: app_state,
            config: self.config,
        })
    }
}

/// Main application struct that manages the runner lifecycle.
pub struct Application {
    /// Application state.
    pub state: Arc<AppState>,
    /// Application configuration.
    config: RunnerConfig,
}

impl Application {
    /// Create a new application builder.
    pub fn builder() -> ApplicationBuilder {
        ApplicationBuilder::new()
    }

    /// Create a new application builder from configuration.
    pub fn builder_from_config(config: RunnerConfig) -> ApplicationBuilder {
        ApplicationBuilder::from_config(config)
    }

    /// Create a new application builder from config file.
    pub fn builder_from_file<P: AsRef<std::path::Path>>(path: P) -> Result<ApplicationBuilder, ApplicationError> {
        ApplicationBuilder::from_file(path)
    }

    /// Get the application state.
    pub fn state(&self) -> &Arc<AppState> {
        &self.state
    }

    /// Run the application with graceful shutdown handling.
    pub async fn run_with_graceful_shutdown<F, Fut>(
        self,
        server_factory: F,
    ) -> Result<(), ApplicationError>
    where
        F: FnOnce(Arc<AppState>) -> Fut,
        Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send + 'static,
    {
        if !self.config.application.enable_graceful_shutdown {
            // Run without graceful shutdown
            return server_factory(self.state).await.map_err(|e| {
                ApplicationError::Runtime(format!("Server error: {}", e))
            });
        }

        // Set up graceful shutdown
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel();

        // Spawn signal handler
        tokio::spawn(async move {
            let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("Failed to install SIGTERM handler");
            let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())
                .expect("Failed to install SIGINT handler");

            tokio::select! {
                _ = sigterm.recv() => {
                    log::info!("Received SIGTERM, initiating graceful shutdown");
                }
                _ = sigint.recv() => {
                    log::info!("Received SIGINT, initiating graceful shutdown");
                }
            }

            let _ = shutdown_tx.send(());
        });

        // Run the server
        let server_handle = tokio::spawn(server_factory(self.state.clone()));

        // Wait for shutdown signal or server completion
        tokio::select! {
            result = server_handle => {
                match result {
                    Ok(Ok(())) => {
                        log::info!("Server completed successfully");
                        Ok(())
                    }
                    Ok(Err(e)) => {
                        log::error!("Server error: {}", e);
                        Err(ApplicationError::Runtime(format!("Server error: {}", e)))
                    }
                    Err(e) => {
                        log::error!("Server task failed: {}", e);
                        Err(ApplicationError::Runtime(format!("Server task failed: {}", e)))
                    }
                }
            }
            _ = &mut shutdown_rx => {
                log::info!("Initiating graceful shutdown...");
                self.shutdown().await?;
                Ok(())
            }
        }
    }

    /// Perform graceful shutdown of all systems.
    async fn shutdown(&self) -> Result<(), ApplicationError> {
        log::info!("Starting graceful shutdown sequence...");

        // Create a timeout for the entire shutdown process
        let shutdown_future = async {
            // 1. Stop accepting new work (this would be handled by the HTTP server)
            log::info!("Stopping new work acceptance...");

            // 2. Wait for active runs to complete or timeout
            log::info!("Waiting for active runs to complete...");
            if let Err(e) = self.wait_for_active_runs().await {
                log::warn!("Error waiting for active runs: {}", e);
            }

            // 3. Cleanup performance monitoring
            log::info!("Stopping performance monitoring...");
            // Performance monitor runs in background tasks that will be cancelled

            // 4. Flush logs and artifacts
            log::info!("Flushing logs and artifacts...");
            if let Err(e) = self.state.log_manager.flush_all().await {
                log::warn!("Error flushing logs: {}", e);
            }
            if let Err(e) = self.state.artifact_manager.flush_all().await {
                log::warn!("Error flushing artifacts: {}", e);
            }

            // 5. Clean up error tracking
            log::info!("Cleaning up error tracking...");
            self.state.error_tracker.cleanup_old_errors(chrono::Duration::hours(24)).await;

            // 6. Close database connections (handled by connection pool drop)
            log::info!("Closing database connections...");

            log::info!("Graceful shutdown completed successfully");
            Ok::<(), ApplicationError>(())
        };

        // Apply timeout to shutdown process
        tokio::time::timeout(self.config.shutdown_timeout(), shutdown_future)
            .await
            .map_err(|_| ApplicationError::ShutdownTimeout)?
    }

    /// Wait for active runs to complete.
    async fn wait_for_active_runs(&self) -> Result<(), ApplicationError> {
        let mut attempts = 0;
        const MAX_ATTEMPTS: u32 = 30; // 30 seconds with 1-second intervals

        while attempts < MAX_ATTEMPTS {
            match self.state.database.get_active_runs().await {
                Ok(active_runs) => {
                    if active_runs.is_empty() {
                        log::info!("All active runs completed");
                        return Ok(());
                    }
                    log::info!("Waiting for {} active runs to complete...", active_runs.len());
                }
                Err(e) => {
                    log::warn!("Error checking active runs: {}", e);
                }
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
            attempts += 1;
        }

        log::warn!("Timeout waiting for active runs to complete");
        Ok(())
    }

    /// Perform health checks on all systems.
    pub async fn health_check(&self) -> HealthCheckResult {
        let mut result = HealthCheckResult {
            overall_healthy: true,
            checks: Vec::new(),
        };

        // Database health check
        let db_health = match self.state.database.health_check().await {
            Ok(()) => {
                ComponentHealth {
                    component: "database".to_string(),
                    healthy: true,
                    message: "Database connection healthy".to_string(),
                }
            }
            Err(e) => {
                result.overall_healthy = false;
                ComponentHealth {
                    component: "database".to_string(),
                    healthy: false,
                    message: format!("Database error: {}", e),
                }
            }
        };
        result.checks.push(db_health);

        // VCS health check
        let vcs_health = self.state.vcs_manager.health_check().await;
        if !vcs_health.overall_healthy {
            result.overall_healthy = false;
        }
        for (vcs_type, health) in vcs_health.vcs_statuses {
            result.checks.push(ComponentHealth {
                component: format!("vcs_{}", vcs_type),
                healthy: matches!(health, crate::vcs::VcsHealth::Healthy),
                message: match health {
                    crate::vcs::VcsHealth::Healthy => "VCS healthy".to_string(),
                    crate::vcs::VcsHealth::Warning(msg) => format!("VCS warning: {}", msg),
                    crate::vcs::VcsHealth::Error(msg) => format!("VCS error: {}", msg),
                },
            });
        }

        // Log manager health check
        let log_health = match self.state.log_manager.health_check().await {
            Ok(()) => ComponentHealth {
                component: "log_manager".to_string(),
                healthy: true,
                message: "Log manager healthy".to_string(),
            },
            Err(e) => {
                result.overall_healthy = false;
                ComponentHealth {
                    component: "log_manager".to_string(),
                    healthy: false,
                    message: format!("Log manager error: {}", e),
                }
            }
        };
        result.checks.push(log_health);

        // Artifact manager health check
        let artifact_health = match self.state.artifact_manager.health_check().await {
            Ok(()) => ComponentHealth {
                component: "artifact_manager".to_string(),
                healthy: true,
                message: "Artifact manager healthy".to_string(),
            },
            Err(e) => {
                result.overall_healthy = false;
                ComponentHealth {
                    component: "artifact_manager".to_string(),
                    healthy: false,
                    message: format!("Artifact manager error: {}", e),
                }
            }
        };
        result.checks.push(artifact_health);

        result
    }
}

/// Health check result for the entire application.
#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    pub overall_healthy: bool,
    pub checks: Vec<ComponentHealth>,
}

/// Health check result for a single component.
#[derive(Debug, Clone)]
pub struct ComponentHealth {
    pub component: String,
    pub healthy: bool,
    pub message: String,
}

/// Application initialization and runtime errors.
#[derive(Debug, thiserror::Error)]
pub enum ApplicationError {
    #[error("Database error: {0}")]
    Database(String),
    
    #[error("Log management error: {0}")]
    LogManagement(String),
    
    #[error("Artifact management error: {0}")]
    ArtifactManagement(String),
    
    #[error("VCS management error: {0}")]
    VcsManagement(String),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    #[error("Runtime error: {0}")]
    Runtime(String),
    
    #[error("Shutdown timeout")]
    ShutdownTimeout,
}

/// Initialize global metrics.
pub fn init_metrics() {
    crate::metrics::MetricsCollector::init_system_info();
    log::info!("Metrics system initialized");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_application_config_default() {
        let config = ApplicationConfig::default();
        assert_eq!(config.database_url, "postgresql://localhost/janitor");
        assert!(config.enable_graceful_shutdown);
    }

    #[tokio::test]
    async fn test_application_builder() {
        let janitor_config = janitor::config::Config::default();
        let builder = Application::builder(janitor_config)
            .with_database_url("postgresql://test/janitor".to_string())
            .with_redis_url(Some("redis://localhost:6379".to_string()));

        // We can't actually build without a real database, but we can test the builder pattern
        assert_eq!(builder.config.database_url, "postgresql://test/janitor");
        assert_eq!(builder.config.redis_url, Some("redis://localhost:6379".to_string()));
    }
}