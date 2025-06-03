//! Service orchestration and lifecycle management

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

use crate::backfill::{BackfillProcessor, BackfillConfig};
use crate::config::Config;
use crate::error::Result;
use crate::message_handler::MessageHandler;
use crate::redis_client::RedisConnectionManager;
use crate::upload::UploadConfig;
use crate::web;

/// Service orchestrator for managing auto-upload service lifecycle
pub struct ServiceOrchestrator {
    /// Service configuration
    config: Arc<Config>,
    /// Upload configuration
    upload_config: UploadConfig,
    /// HTTP listen address
    listen_addr: String,
    /// HTTP port
    port: u16,
    /// Whether to run in backfill mode
    backfill: bool,
    /// Shutdown signal broadcaster
    shutdown_tx: broadcast::Sender<()>,
    /// Service health status
    health_status: Arc<ServiceHealth>,
}

/// Service health tracking
pub struct ServiceHealth {
    /// Whether the service is running
    pub running: AtomicBool,
    /// Web server health
    pub web_healthy: AtomicBool,
    /// Redis listener health
    pub redis_healthy: AtomicBool,
    /// Service start time
    pub start_time: Instant,
    /// Total messages processed
    pub messages_processed: AtomicU64,
    /// Last health check time
    pub last_health_check: AtomicU64,
}

impl ServiceHealth {
    /// Create new health tracking
    fn new() -> Self {
        Self {
            running: AtomicBool::new(true),
            web_healthy: AtomicBool::new(false),
            redis_healthy: AtomicBool::new(false),
            start_time: Instant::now(),
            messages_processed: AtomicU64::new(0),
            last_health_check: AtomicU64::new(0),
        }
    }
    
    /// Get service uptime
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }
    
    /// Check if all components are healthy
    pub fn is_healthy(&self) -> bool {
        self.running.load(Ordering::SeqCst)
            && self.web_healthy.load(Ordering::SeqCst)
            && self.redis_healthy.load(Ordering::SeqCst)
    }
    
    /// Update last health check time
    pub fn update_health_check(&self) {
        self.last_health_check.store(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            Ordering::SeqCst,
        );
    }
}

/// Task handle wrapper for lifecycle management
struct TaskHandle {
    /// Task name
    name: String,
    /// Join handle
    handle: JoinHandle<Result<()>>,
    /// Whether this is a critical task
    critical: bool,
}

impl ServiceOrchestrator {
    /// Create a new service orchestrator
    pub fn new(
        config: Config,
        listen_addr: String,
        port: u16,
        dput_host: &str,
        debsign_keyid: Option<&str>,
        source_only: bool,
        distributions: Vec<String>,
        backfill: bool,
    ) -> Self {
        let upload_config = UploadConfig::new(
            dput_host.to_string(),
            debsign_keyid.map(|s| s.to_string()),
            source_only,
            distributions,
        );
        
        let (shutdown_tx, _) = broadcast::channel(1);
        
        Self {
            config: Arc::new(config),
            upload_config,
            listen_addr,
            port,
            backfill,
            shutdown_tx,
            health_status: Arc::new(ServiceHealth::new()),
        }
    }
    
    /// Run the service with proper lifecycle management
    pub async fn run(self) -> Result<()> {
        info!("Starting auto-upload service orchestrator");
        
        // Set up signal handlers
        let shutdown_rx = self.setup_signal_handlers().await?;
        
        // Create tasks
        let mut tasks = Vec::new();
        
        // Start web server task
        tasks.push(self.spawn_web_server().await?);
        
        // Start Redis listener task (only in serve mode)
        if !self.backfill {
            tasks.push(self.spawn_redis_listener().await?);
        }
        
        // Start health check task
        tasks.push(self.spawn_health_check().await?);
        
        // Run service based on mode
        if self.backfill {
            // Run backfill in the main task
            if let Err(e) = self.run_backfill_mode().await {
                error!("Backfill operation failed: {}", e);
                self.initiate_shutdown();
            }
        } else {
            info!("Auto-upload service running in serve mode");
            info!("HTTP server: {}:{}", self.listen_addr, self.port);
            info!("Redis: {}", self.config.redis_location);
        }
        
        // Wait for shutdown signal or task failure
        self.wait_for_completion(tasks, shutdown_rx).await
    }
    
    /// Set up signal handlers for graceful shutdown
    async fn setup_signal_handlers(&self) -> Result<broadcast::Receiver<()>> {
        let mut sigterm = signal(SignalKind::terminate())?;
        let mut sigint = signal(SignalKind::interrupt())?;
        let mut sighup = signal(SignalKind::hangup())?;
        
        let shutdown_tx = self.shutdown_tx.clone();
        let health_status = self.health_status.clone();
        
        tokio::spawn(async move {
            tokio::select! {
                _ = sigterm.recv() => {
                    info!("Received SIGTERM, initiating graceful shutdown");
                }
                _ = sigint.recv() => {
                    info!("Received SIGINT, initiating graceful shutdown");
                }
                _ = sighup.recv() => {
                    info!("Received SIGHUP, initiating graceful shutdown");
                }
            }
            
            health_status.running.store(false, Ordering::SeqCst);
            let _ = shutdown_tx.send(());
        });
        
        Ok(self.shutdown_tx.subscribe())
    }
    
    /// Spawn web server task
    async fn spawn_web_server(&self) -> Result<TaskHandle> {
        let app = web::create_app_with_health(self.config.clone(), Some(self.health_status.clone()));
        let listen_addr = self.listen_addr.clone();
        let port = self.port;
        let health_status = self.health_status.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        
        let handle = tokio::spawn(async move {
            health_status.web_healthy.store(true, Ordering::SeqCst);
            
            tokio::select! {
                result = web::run_web_server(app, &listen_addr, port) => {
                    health_status.web_healthy.store(false, Ordering::SeqCst);
                    match result {
                        Ok(_) => {
                            info!("Web server shut down gracefully");
                            Ok(())
                        }
                        Err(e) => {
                            error!("Web server failed: {}", e);
                            Err(crate::error::UploadError::Config(e.to_string()))
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("Web server received shutdown signal");
                    health_status.web_healthy.store(false, Ordering::SeqCst);
                    Ok(())
                }
            }
        });
        
        Ok(TaskHandle {
            name: "web_server".to_string(),
            handle,
            critical: true,
        })
    }
    
    /// Spawn Redis listener task
    async fn spawn_redis_listener(&self) -> Result<TaskHandle> {
        let config = self.config.clone();
        let upload_config = self.upload_config.clone();
        let health_status = self.health_status.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        
        let handle = tokio::spawn(async move {
            let mut connection_manager = RedisConnectionManager::new(config.redis_location.clone());
            
            loop {
                if !health_status.running.load(Ordering::SeqCst) {
                    info!("Redis listener shutting down");
                    break;
                }
                
                tokio::select! {
                    _ = Self::run_redis_listener_once(
                        &mut connection_manager,
                        &config,
                        &upload_config,
                        &health_status,
                    ) => {
                        // Listener exited, will retry
                        warn!("Redis listener exited, retrying in 5 seconds");
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Redis listener received shutdown signal");
                        break;
                    }
                }
            }
            
            health_status.redis_healthy.store(false, Ordering::SeqCst);
            Ok(())
        });
        
        Ok(TaskHandle {
            name: "redis_listener".to_string(),
            handle,
            critical: true,
        })
    }
    
    /// Run Redis listener once
    async fn run_redis_listener_once(
        connection_manager: &mut RedisConnectionManager,
        config: &Arc<Config>,
        upload_config: &UploadConfig,
        health_status: &Arc<ServiceHealth>,
    ) -> Result<()> {
        info!("Starting Redis listener");
        
        // Create message handler
        let message_handler = match MessageHandler::new(&config.artifact_location, upload_config.clone()).await {
            Ok(handler) => Arc::new(handler),
            Err(e) => {
                error!("Failed to create message handler: {}", e);
                return Err(e);
            }
        };
        
        // Get Redis client
        let redis_client = match connection_manager.get_client().await {
            Ok(client) => {
                health_status.redis_healthy.store(true, Ordering::SeqCst);
                client
            }
            Err(e) => {
                health_status.redis_healthy.store(false, Ordering::SeqCst);
                error!("Failed to get Redis client: {}", e);
                return Err(e);
            }
        };
        
        // Subscribe to messages
        let result = redis_client.subscribe_to_results({
            let handler = message_handler.clone();
            let health_status = health_status.clone();
            move |message| {
                let handler = handler.clone();
                let health_status = health_status.clone();
                Box::pin(async move {
                    let result = handler.handle_message(message).await;
                    health_status.messages_processed.fetch_add(1, Ordering::SeqCst);
                    result
                })
            }
        }).await;
        
        health_status.redis_healthy.store(false, Ordering::SeqCst);
        
        match result {
            Ok(_) => {
                warn!("Redis subscription ended normally");
                Ok(())
            }
            Err(e) => {
                error!("Redis subscription failed: {}", e);
                if let Err(reconnect_err) = connection_manager.handle_connection_error(&e).await {
                    error!("Failed to handle Redis reconnection: {}", reconnect_err);
                }
                Err(e)
            }
        }
    }
    
    /// Spawn health check task
    async fn spawn_health_check(&self) -> Result<TaskHandle> {
        let health_status = self.health_status.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        health_status.update_health_check();
                        
                        let uptime = health_status.uptime();
                        let messages = health_status.messages_processed.load(Ordering::SeqCst);
                        let healthy = health_status.is_healthy();
                        
                        debug!(
                            "Health check - Uptime: {:?}, Messages: {}, Healthy: {}",
                            uptime, messages, healthy
                        );
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Health check received shutdown signal");
                        break;
                    }
                }
            }
            
            Ok(())
        });
        
        Ok(TaskHandle {
            name: "health_check".to_string(),
            handle,
            critical: false,
        })
    }
    
    /// Run in backfill mode
    async fn run_backfill_mode(&self) -> Result<()> {
        info!("Running in backfill mode");
        
        let processor = BackfillProcessor::new(
            &self.config.database_location,
            &self.config.artifact_location,
            self.upload_config.clone(),
        ).await?;
        
        let backfill_config = BackfillConfig {
            distributions: self.upload_config.distributions.clone(),
            ..Default::default()
        };
        
        let summary = processor.run_backfill(backfill_config).await?;
        
        info!("Backfill completed: {}", summary);
        
        if summary.failed_uploads > 0 {
            error!("Some uploads failed during backfill");
            return Err(crate::error::UploadError::Config(
                format!("{} uploads failed", summary.failed_uploads)
            ));
        }
        
        Ok(())
    }
    
    /// Wait for task completion or shutdown
    async fn wait_for_completion(
        &self,
        tasks: Vec<TaskHandle>,
        mut shutdown_rx: broadcast::Receiver<()>,
    ) -> Result<()> {
        let mut handles: Vec<_> = tasks.into_iter().map(|t| t.handle).collect();
        
        // Just wait for the shutdown signal and then wait for tasks
        tokio::select! {
            // Wait for shutdown signal
            _ = shutdown_rx.recv() => {
                info!("Received shutdown signal, initiating graceful shutdown");
            }
        }
        
        // Give tasks time to shut down gracefully  
        info!("Waiting for tasks to complete gracefully");
        
        let timeout = tokio::time::sleep(Duration::from_secs(30));
        tokio::pin!(timeout);
        
        tokio::select! {
            results = futures::future::join_all(handles) => {
                let mut has_errors = false;
                for (i, result) in results.into_iter().enumerate() {
                    match result {
                        Ok(Ok(_)) => {
                            debug!("Task {} completed successfully", i);
                        }
                        Ok(Err(e)) => {
                            warn!("Task {} failed during shutdown: {}", i, e);
                            has_errors = true;
                        }
                        Err(e) => {
                            warn!("Task {} panicked during shutdown: {}", i, e);
                            has_errors = true;
                        }
                    }
                }
                
                if has_errors {
                    warn!("Some tasks had errors during shutdown");
                } else {
                    info!("All tasks shut down gracefully");
                }
            }
            _ = &mut timeout => {
                warn!("Timeout waiting for tasks to complete gracefully");
            }
        }
        
        info!("Service shutdown complete");
        Ok(())
    }
    
    /// Initiate service shutdown
    fn initiate_shutdown(&self) {
        self.health_status.running.store(false, Ordering::SeqCst);
        let _ = self.shutdown_tx.send(());
    }
    
    /// Get service health status
    pub fn health_status(&self) -> &ServiceHealth {
        &self.health_status
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_service_health_new() {
        let health = ServiceHealth::new();
        assert!(health.running.load(Ordering::SeqCst));
        assert!(!health.web_healthy.load(Ordering::SeqCst));
        assert!(!health.redis_healthy.load(Ordering::SeqCst));
        assert_eq!(health.messages_processed.load(Ordering::SeqCst), 0);
    }
    
    #[test]
    fn test_service_health_is_healthy() {
        let health = ServiceHealth::new();
        assert!(!health.is_healthy()); // Not healthy initially
        
        health.web_healthy.store(true, Ordering::SeqCst);
        health.redis_healthy.store(true, Ordering::SeqCst);
        assert!(health.is_healthy()); // Healthy when all components are up
        
        health.running.store(false, Ordering::SeqCst);
        assert!(!health.is_healthy()); // Not healthy when not running
    }
    
    #[tokio::test]
    async fn test_service_orchestrator_new() {
        let config = Config {
            database_location: "postgres://test".to_string(),
            artifact_location: "file:///tmp".to_string(),
            redis_location: "redis://localhost".to_string(),
        };
        
        let orchestrator = ServiceOrchestrator::new(
            config,
            "127.0.0.1".to_string(),
            9933,
            "test-host",
            None,
            false,
            vec![],
            false,
        );
        
        assert_eq!(orchestrator.listen_addr, "127.0.0.1");
        assert_eq!(orchestrator.port, 9933);
        assert!(!orchestrator.backfill);
    }
}