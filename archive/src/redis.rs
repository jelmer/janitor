//! Redis integration for archive service pub/sub messaging.
//!
//! This module provides Redis pub/sub functionality for automatic repository
//! generation triggering. It listens for events from the runner service and
//! other components to trigger repository regeneration when builds complete.

use std::sync::Arc;
use std::time::Duration;

use redis::{aio::ConnectionManager, AsyncCommands, Client, RedisError};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use futures::StreamExt;
use tracing::{debug, error, info, warn};

use crate::error::{ArchiveError, ArchiveResult};
use crate::manager::GeneratorManager;

/// Redis configuration for the archive service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    /// Redis connection URL.
    pub url: String,
    /// Connection timeout in seconds.
    pub connection_timeout_seconds: u64,
    /// Retry attempts for failed connections.
    pub retry_attempts: u32,
    /// Retry delay in seconds.
    pub retry_delay_seconds: u64,
    /// Enable automatic reconnection.
    pub enable_auto_reconnect: bool,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_string(),
            connection_timeout_seconds: 30,
            retry_attempts: 3,
            retry_delay_seconds: 5,
            enable_auto_reconnect: true,
        }
    }
}

/// Events that can trigger repository generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ArchiveEvent {
    /// Build completed successfully.
    BuildCompleted {
        /// Run ID.
        run_id: String,
        /// Codebase name.
        codebase: String,
        /// Campaign/suite name.
        campaign: String,
        /// Build artifacts information.
        artifacts: Vec<String>,
    },
    /// Campaign finished processing.
    CampaignFinished {
        /// Campaign name.
        campaign: String,
        /// Number of successful runs.
        successful_runs: u32,
        /// Total number of runs.
        total_runs: u32,
    },
    /// Manual repository regeneration request.
    ManualRegeneration {
        /// Repository name.
        repository: String,
        /// Campaign name.
        campaign: Option<String>,
        /// User who requested the regeneration.
        requested_by: Option<String>,
    },
    /// Periodic republishing trigger.
    PeriodicRepublish {
        /// Campaign name.
        campaign: String,
        /// Interval type.
        interval_type: String,
    },
}

impl ArchiveEvent {
    /// Get the campaign name associated with this event.
    pub fn campaign(&self) -> Option<&str> {
        match self {
            ArchiveEvent::BuildCompleted { campaign, .. } => Some(campaign),
            ArchiveEvent::CampaignFinished { campaign, .. } => Some(campaign),
            ArchiveEvent::ManualRegeneration { campaign, .. } => campaign.as_deref(),
            ArchiveEvent::PeriodicRepublish { campaign, .. } => Some(campaign),
        }
    }

    /// Get the repository name if specified.
    pub fn repository(&self) -> Option<&str> {
        match self {
            ArchiveEvent::ManualRegeneration { repository, .. } => Some(repository),
            _ => None,
        }
    }
}

/// Redis subscriber for archive events.
pub struct RedisSubscriber {
    /// Redis client.
    client: Client,
    /// Redis configuration.
    config: RedisConfig,
    /// Generator manager.
    generator_manager: Arc<GeneratorManager>,
    /// Channel for shutdown signals.
    shutdown_rx: Option<mpsc::Receiver<()>>,
}

impl RedisSubscriber {
    /// Create a new Redis subscriber.
    pub async fn new(
        config: RedisConfig,
        generator_manager: Arc<GeneratorManager>,
    ) -> ArchiveResult<Self> {
        info!("Creating Redis subscriber with URL: {}", config.url);

        let client = Client::open(config.url.as_str())
            .map_err(|e| ArchiveError::Redis(format!("Failed to create Redis client: {}", e)))?;

        // Test connection
        let mut conn = client
            .get_async_connection()
            .await
            .map_err(|e| ArchiveError::Redis(format!("Failed to connect to Redis: {}", e)))?;

        // Test with a simple ping
        let _result: redis::Value = redis::cmd("PING")
            .query_async(&mut conn)
            .await
            .map_err(|e| ArchiveError::Redis(format!("Redis ping failed: {}", e)))?;

        info!("Successfully connected to Redis");

        Ok(Self {
            client,
            config,
            generator_manager,
            shutdown_rx: None,
        })
    }

    /// Set shutdown channel for graceful shutdown.
    pub fn with_shutdown(mut self, shutdown_rx: mpsc::Receiver<()>) -> Self {
        self.shutdown_rx = Some(shutdown_rx);
        self
    }

    /// Start listening for archive events.
    pub async fn start_listening(&mut self, channels: Vec<String>) -> ArchiveResult<JoinHandle<()>> {
        info!("Starting Redis subscriber for channels: {:?}", channels);

        let client = self.client.clone();
        let config = self.config.clone();
        let generator_manager = Arc::clone(&self.generator_manager);
        let shutdown_rx = self.shutdown_rx.take();

        let handle = tokio::spawn(async move {
            let mut retry_count = 0;

            loop {
                match Self::run_subscriber_loop(
                    &client,
                    &config,
                    &generator_manager,
                    &channels,
                    shutdown_rx.as_ref(),
                )
                .await
                {
                    Ok(_) => {
                        info!("Redis subscriber loop exited normally");
                        break;
                    }
                    Err(e) => {
                        error!("Redis subscriber error: {}", e);
                        retry_count += 1;

                        if retry_count >= config.retry_attempts {
                            error!("Max retry attempts reached, stopping subscriber");
                            break;
                        }

                        if config.enable_auto_reconnect {
                            warn!(
                                "Retrying Redis connection in {} seconds (attempt {}/{})",
                                config.retry_delay_seconds, retry_count, config.retry_attempts
                            );
                            sleep(Duration::from_secs(config.retry_delay_seconds)).await;
                        } else {
                            error!("Auto-reconnect disabled, stopping subscriber");
                            break;
                        }
                    }
                }
            }
        });

        Ok(handle)
    }

    /// Internal subscriber loop.
    async fn run_subscriber_loop(
        client: &Client,
        config: &RedisConfig,
        generator_manager: &Arc<GeneratorManager>,
        channels: &[String],
        shutdown_rx: Option<&mpsc::Receiver<()>>,
    ) -> ArchiveResult<()> {
        let mut conn = client
            .get_async_connection()
            .await
            .map_err(|e| ArchiveError::Redis(format!("Failed to get connection: {}", e)))?;

        let mut pubsub = conn.into_pubsub();

        // Subscribe to channels
        for channel in channels {
            pubsub
                .subscribe(channel)
                .await
                .map_err(|e| ArchiveError::Redis(format!("Failed to subscribe to {}: {}", channel, e)))?;
            info!("Subscribed to Redis channel: {}", channel);
        }

        let mut stream = pubsub.on_message();

        loop {
            tokio::select! {
                // Handle shutdown signal
                shutdown_result = async {
                    match shutdown_rx.as_mut() {
                        Some(rx) => rx.recv().await,
                        None => std::future::pending().await,
                    }
                } => {
                    match shutdown_result {
                        Ok(_) => info!("Received shutdown signal, stopping Redis subscriber"),
                        Err(_) => info!("Shutdown channel closed, stopping Redis subscriber"),
                    }
                    break;
                }

                // Handle Redis messages
                msg_result = stream.next() => {
                    match msg_result {
                        Some(msg) => {
                            let channel: String = msg.get_channel_name().to_string();
                            let payload: String = match msg.get_payload() {
                                Ok(p) => p,
                                Err(e) => {
                                    warn!("Failed to get message payload from {}: {}", channel, e);
                                    continue;
                                }
                            };

                            debug!("Received message from {}: {}", channel, payload);

                            if let Err(e) = Self::handle_message(generator_manager, &channel, &payload).await {
                                error!("Failed to handle message from {}: {}", channel, e);
                            }
                        }
                        None => {
                            info!("Redis pubsub stream ended, stopping subscriber");
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Handle incoming Redis message.
    async fn handle_message(
        generator_manager: &Arc<GeneratorManager>,
        channel: &str,
        payload: &str,
    ) -> ArchiveResult<()> {
        debug!("Processing message from channel {}", channel);

        // Parse the event
        let event: ArchiveEvent = serde_json::from_str(payload)
            .map_err(|e| ArchiveError::Serialization(format!("Failed to parse event: {}", e)))?;

        info!("Processing archive event: {:?}", event);

        match event {
            ArchiveEvent::BuildCompleted { campaign, .. } => {
                info!("Triggering repository generation for build completion in campaign: {}", campaign);
                
                match generator_manager.trigger_campaign(&campaign).await {
                    Ok(job_ids) => {
                        info!("Triggered {} jobs for campaign {}", job_ids.len(), campaign);
                    }
                    Err(e) => {
                        error!("Failed to trigger campaign {}: {}", campaign, e);
                    }
                }
            }
            ArchiveEvent::CampaignFinished { campaign, successful_runs, total_runs } => {
                info!(
                    "Campaign {} finished: {}/{} successful runs",
                    campaign, successful_runs, total_runs
                );

                if successful_runs > 0 {
                    match generator_manager.trigger_campaign(&campaign).await {
                        Ok(job_ids) => {
                            info!("Triggered {} jobs for finished campaign {}", job_ids.len(), campaign);
                        }
                        Err(e) => {
                            error!("Failed to trigger finished campaign {}: {}", campaign, e);
                        }
                    }
                }
            }
            ArchiveEvent::ManualRegeneration { repository, campaign, requested_by } => {
                info!(
                    "Manual regeneration requested for repository {} (campaign: {:?}, by: {:?})",
                    repository, campaign, requested_by
                );

                if let Some(campaign_name) = campaign {
                    match generator_manager.trigger_campaign(&campaign_name).await {
                        Ok(job_ids) => {
                            info!("Triggered {} jobs for manual regeneration", job_ids.len());
                        }
                        Err(e) => {
                            error!("Failed to trigger manual regeneration: {}", e);
                        }
                    }
                }
            }
            ArchiveEvent::PeriodicRepublish { campaign, interval_type } => {
                info!("Periodic republish triggered for campaign {} ({})", campaign, interval_type);

                match generator_manager.trigger_campaign(&campaign).await {
                    Ok(job_ids) => {
                        info!("Triggered {} jobs for periodic republish", job_ids.len());
                    }
                    Err(e) => {
                        error!("Failed to trigger periodic republish: {}", e);
                    }
                }
            }
        }

        Ok(())
    }
}

/// Redis publisher for sending archive events.
pub struct RedisPublisher {
    /// Redis connection manager.
    connection_manager: ConnectionManager,
    /// Default channel for publishing.
    default_channel: String,
}

impl RedisPublisher {
    /// Create a new Redis publisher.
    pub async fn new(config: &RedisConfig, default_channel: String) -> ArchiveResult<Self> {
        info!("Creating Redis publisher for channel: {}", default_channel);

        let client = Client::open(config.url.as_str())
            .map_err(|e| ArchiveError::Redis(format!("Failed to create Redis client: {}", e)))?;

        let connection_manager = ConnectionManager::new(client)
            .await
            .map_err(|e| ArchiveError::Redis(format!("Failed to create connection manager: {}", e)))?;

        Ok(Self {
            connection_manager,
            default_channel,
        })
    }

    /// Publish an archive event.
    pub async fn publish_event(&mut self, event: &ArchiveEvent) -> ArchiveResult<()> {
        self.publish_event_to_channel(&self.default_channel.clone(), event).await
    }

    /// Publish an archive event to a specific channel.
    pub async fn publish_event_to_channel(
        &mut self,
        channel: &str,
        event: &ArchiveEvent,
    ) -> ArchiveResult<()> {
        let payload = serde_json::to_string(event)
            .map_err(|e| ArchiveError::Serialization(format!("Failed to serialize event: {}", e)))?;

        debug!("Publishing event to {}: {}", channel, payload);

        let _: i32 = self
            .connection_manager
            .publish(channel, payload)
            .await
            .map_err(|e| ArchiveError::Redis(format!("Failed to publish to {}: {}", channel, e)))?;

        info!("Published event to channel: {}", channel);
        Ok(())
    }

    /// Publish a build completion event.
    pub async fn publish_build_completed(
        &mut self,
        run_id: String,
        codebase: String,
        campaign: String,
        artifacts: Vec<String>,
    ) -> ArchiveResult<()> {
        let event = ArchiveEvent::BuildCompleted {
            run_id,
            codebase,
            campaign,
            artifacts,
        };

        self.publish_event(&event).await
    }

    /// Publish a campaign finished event.
    pub async fn publish_campaign_finished(
        &mut self,
        campaign: String,
        successful_runs: u32,
        total_runs: u32,
    ) -> ArchiveResult<()> {
        let event = ArchiveEvent::CampaignFinished {
            campaign,
            successful_runs,
            total_runs,
        };

        self.publish_event(&event).await
    }

    /// Health check for Redis connection.
    pub async fn health_check(&mut self) -> ArchiveResult<()> {
        let _result: redis::Value = redis::cmd("PING")
            .query_async(&mut self.connection_manager)
            .await
            .map_err(|e| ArchiveError::Redis(format!("Health check failed: {}", e)))?;

        Ok(())
    }
}

/// Redis manager that coordinates both subscriber and publisher.
pub struct RedisManager {
    /// Redis configuration.
    config: RedisConfig,
    /// Generator manager.
    generator_manager: Arc<GeneratorManager>,
    /// Publisher instance.
    publisher: Option<RedisPublisher>,
    /// Subscriber instance.
    subscriber: Option<RedisSubscriber>,
}

impl RedisManager {
    /// Create a new Redis manager.
    pub async fn new(
        config: RedisConfig,
        generator_manager: Arc<GeneratorManager>,
    ) -> ArchiveResult<Self> {
        info!("Creating Redis manager");

        Ok(Self {
            config,
            generator_manager,
            publisher: None,
            subscriber: None,
        })
    }

    /// Initialize publisher.
    pub async fn init_publisher(&mut self, default_channel: String) -> ArchiveResult<()> {
        self.publisher = Some(RedisPublisher::new(&self.config, default_channel).await?);
        info!("Redis publisher initialized");
        Ok(())
    }

    /// Initialize subscriber.
    pub async fn init_subscriber(&mut self) -> ArchiveResult<()> {
        self.subscriber = Some(RedisSubscriber::new(self.config.clone(), Arc::clone(&self.generator_manager)).await?);
        info!("Redis subscriber initialized");
        Ok(())
    }

    /// Get mutable reference to publisher.
    pub fn publisher_mut(&mut self) -> Option<&mut RedisPublisher> {
        self.publisher.as_mut()
    }

    /// Get mutable reference to subscriber.
    pub fn subscriber_mut(&mut self) -> Option<&mut RedisSubscriber> {
        self.subscriber.as_mut()
    }

    /// Start listening for events.
    pub async fn start_listening(&mut self, channels: Vec<String>) -> ArchiveResult<JoinHandle<()>> {
        match &mut self.subscriber {
            Some(subscriber) => subscriber.start_listening(channels).await,
            None => Err(ArchiveError::Configuration("Subscriber not initialized".to_string())),
        }
    }

    /// Health check for all Redis connections.
    pub async fn health_check(&mut self) -> ArchiveResult<()> {
        if let Some(publisher) = &mut self.publisher {
            publisher.health_check().await?;
        }

        // For subscriber, we can try creating a temporary connection
        let client = Client::open(self.config.url.as_str())
            .map_err(|e| ArchiveError::Redis(format!("Failed to create test client: {}", e)))?;

        let mut conn = client
            .get_async_connection()
            .await
            .map_err(|e| ArchiveError::Redis(format!("Failed to test connection: {}", e)))?;

        let _result: redis::Value = redis::cmd("PING")
            .query_async(&mut conn)
            .await
            .map_err(|e| ArchiveError::Redis(format!("Subscriber health check failed: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redis_config_default() {
        let config = RedisConfig::default();
        
        assert_eq!(config.url, "redis://localhost:6379");
        assert_eq!(config.connection_timeout_seconds, 30);
        assert_eq!(config.retry_attempts, 3);
        assert_eq!(config.retry_delay_seconds, 5);
        assert!(config.enable_auto_reconnect);
    }

    #[test]
    fn test_archive_event_serialization() {
        let event = ArchiveEvent::BuildCompleted {
            run_id: "test-run-123".to_string(),
            codebase: "test-codebase".to_string(),
            campaign: "test-campaign".to_string(),
            artifacts: vec!["artifact1.deb".to_string(), "artifact2.deb".to_string()],
        };

        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: ArchiveEvent = serde_json::from_str(&serialized).unwrap();

        match deserialized {
            ArchiveEvent::BuildCompleted { run_id, codebase, campaign, artifacts } => {
                assert_eq!(run_id, "test-run-123");
                assert_eq!(codebase, "test-codebase");
                assert_eq!(campaign, "test-campaign");
                assert_eq!(artifacts.len(), 2);
            }
            _ => panic!("Wrong event type after deserialization"),
        }
    }

    #[test]
    fn test_archive_event_campaign_extraction() {
        let build_event = ArchiveEvent::BuildCompleted {
            run_id: "test".to_string(),
            codebase: "test".to_string(),
            campaign: "main".to_string(),
            artifacts: vec![],
        };

        let campaign_event = ArchiveEvent::CampaignFinished {
            campaign: "stable".to_string(),
            successful_runs: 5,
            total_runs: 10,
        };

        let manual_event = ArchiveEvent::ManualRegeneration {
            repository: "test-repo".to_string(),
            campaign: Some("testing".to_string()),
            requested_by: Some("admin".to_string()),
        };

        assert_eq!(build_event.campaign(), Some("main"));
        assert_eq!(campaign_event.campaign(), Some("stable"));
        assert_eq!(manual_event.campaign(), Some("testing"));
        assert_eq!(manual_event.repository(), Some("test-repo"));
    }

    #[test]
    fn test_periodic_republish_event() {
        let event = ArchiveEvent::PeriodicRepublish {
            campaign: "nightly".to_string(),
            interval_type: "daily".to_string(),
        };

        assert_eq!(event.campaign(), Some("nightly"));
        assert_eq!(event.repository(), None);

        let serialized = serde_json::to_string(&event).unwrap();
        assert!(serialized.contains("PeriodicRepublish"));
        assert!(serialized.contains("nightly"));
        assert!(serialized.contains("daily"));
    }

    #[tokio::test]
    async fn test_redis_manager_creation() {
        // This test would require a Redis instance running
        // For now, just test configuration
        let config = RedisConfig::default();
        
        // Would need to set up mock GeneratorManager for full test
        // let manager = RedisManager::new(config, generator_manager).await;
        
        assert!(!config.url.is_empty());
        assert!(config.retry_attempts > 0);
    }
}