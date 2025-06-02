use anyhow::{Context, Result};
use redis::{AsyncCommands, Client as RedisClient};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tokio_stream::StreamExt;
use tracing::{debug, error, info, warn};

/// Real-time event types for the Janitor system
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", content = "data")]
pub enum RealtimeEvent {
    /// Queue status updates
    QueueStatusUpdate {
        queue_size: i64,
        active_runs: i64,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Run status changes
    RunStatusChange {
        run_id: String,
        old_status: String,
        new_status: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Worker status updates
    WorkerStatusUpdate {
        worker_id: String,
        status: String,
        current_task: Option<String>,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// System health changes
    SystemHealthChange {
        component: String,
        old_status: String,
        new_status: String,
        details: Option<serde_json::Value>,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Publishing events
    PublishEvent {
        codebase: String,
        action: String,
        result: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Campaign updates
    CampaignUpdate {
        campaign: String,
        update_type: String,
        data: serde_json::Value,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
}

impl RealtimeEvent {
    /// Get the event type as a string for filtering
    pub fn event_type(&self) -> &'static str {
        match self {
            RealtimeEvent::QueueStatusUpdate { .. } => "queue_status_update",
            RealtimeEvent::RunStatusChange { .. } => "run_status_change",
            RealtimeEvent::WorkerStatusUpdate { .. } => "worker_status_update",
            RealtimeEvent::SystemHealthChange { .. } => "system_health_change",
            RealtimeEvent::PublishEvent { .. } => "publish_event",
            RealtimeEvent::CampaignUpdate { .. } => "campaign_update",
        }
    }

    /// Get the target channel for this event
    pub fn channel(&self) -> String {
        match self {
            RealtimeEvent::QueueStatusUpdate { .. } => "janitor:queue:status".to_string(),
            RealtimeEvent::RunStatusChange { run_id, .. } => format!("janitor:run:{}", run_id),
            RealtimeEvent::WorkerStatusUpdate { worker_id, .. } => {
                format!("janitor:worker:{}", worker_id)
            }
            RealtimeEvent::SystemHealthChange { component, .. } => {
                format!("janitor:system:{}", component)
            }
            RealtimeEvent::PublishEvent { codebase, .. } => format!("janitor:publish:{}", codebase),
            RealtimeEvent::CampaignUpdate { campaign, .. } => {
                format!("janitor:campaign:{}", campaign)
            }
        }
    }
}

/// Configuration for real-time features
#[derive(Debug, Clone)]
pub struct RealtimeConfig {
    /// Redis key prefix for pub/sub channels
    pub channel_prefix: String,
    /// Maximum number of events to buffer per channel
    pub buffer_size: usize,
    /// Event expiration time in seconds
    pub event_expiry_seconds: u64,
    /// Enable real-time features
    pub enabled: bool,
}

impl Default for RealtimeConfig {
    fn default() -> Self {
        Self {
            channel_prefix: "janitor".to_string(),
            buffer_size: 1000,
            event_expiry_seconds: 3600, // 1 hour
            enabled: true,
        }
    }
}

/// Manager for real-time features including Redis pub/sub
pub struct RealtimeManager {
    redis_client: Option<RedisClient>,
    config: RealtimeConfig,
    /// Broadcast channels for different event types
    event_broadcasters: Arc<RwLock<HashMap<String, broadcast::Sender<RealtimeEvent>>>>,
    /// Statistics tracking
    stats: Arc<RwLock<RealtimeStats>>,
}

#[derive(Debug, Default)]
pub struct RealtimeStats {
    pub events_published: u64,
    pub events_received: u64,
    pub active_subscribers: u64,
    pub channels_active: u64,
    pub last_event_time: Option<chrono::DateTime<chrono::Utc>>,
}

impl RealtimeManager {
    /// Create a new realtime manager
    pub fn new(redis_client: Option<RedisClient>, config: RealtimeConfig) -> Self {
        Self {
            redis_client,
            config,
            event_broadcasters: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(RealtimeStats::default())),
        }
    }

    /// Start the real-time manager and background tasks
    pub async fn start(&self) -> Result<()> {
        if !self.config.enabled {
            info!("Real-time features disabled in configuration");
            return Ok(());
        }

        if let Some(redis_client) = &self.redis_client {
            info!("Starting real-time Redis pub/sub listener");
            self.start_redis_subscriber(redis_client.clone()).await?;
        } else {
            warn!("No Redis client configured, real-time features will use in-memory only");
        }

        Ok(())
    }

    /// Start Redis pub/sub subscriber
    async fn start_redis_subscriber(&self, redis_client: RedisClient) -> Result<()> {
        let broadcasters = self.event_broadcasters.clone();
        let stats = self.stats.clone();
        let channel_prefix = self.config.channel_prefix.clone();

        tokio::spawn(async move {
            loop {
                match Self::redis_subscriber_loop(
                    &redis_client,
                    &broadcasters,
                    &stats,
                    &channel_prefix,
                )
                .await
                {
                    Ok(_) => {
                        warn!("Redis subscriber loop ended, restarting...");
                    }
                    Err(e) => {
                        error!("Redis subscriber error: {}, retrying in 5 seconds...", e);
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    }
                }
            }
        });

        Ok(())
    }

    /// Redis subscriber loop
    async fn redis_subscriber_loop(
        redis_client: &RedisClient,
        broadcasters: &Arc<RwLock<HashMap<String, broadcast::Sender<RealtimeEvent>>>>,
        stats: &Arc<RwLock<RealtimeStats>>,
        channel_prefix: &str,
    ) -> Result<()> {
        let conn = redis_client
            .get_async_connection()
            .await
            .context("Failed to connect to Redis for pub/sub")?;

        let pattern = format!("{}:*", channel_prefix);
        debug!("Subscribing to Redis pattern: {}", pattern);

        let mut pubsub = conn.into_pubsub();
        pubsub
            .psubscribe(pattern.as_str())
            .await
            .context("Failed to subscribe to Redis pattern")?;

        let mut stream = pubsub.on_message();

        while let Some(msg) = stream.next().await {
            if let Ok(payload) = msg.get_payload::<String>() {
                if let Ok(event) = serde_json::from_str::<RealtimeEvent>(&payload) {
                    debug!("Received real-time event: {}", event.event_type());

                    // Update stats
                    {
                        let mut stats_guard = stats.write().await;
                        stats_guard.events_received += 1;
                        stats_guard.last_event_time = Some(chrono::Utc::now());
                    }

                    // Broadcast to local subscribers
                    let channel = event.channel();
                    let broadcasters_guard = broadcasters.read().await;
                    if let Some(broadcaster) = broadcasters_guard.get(&channel) {
                        if let Err(_) = broadcaster.send(event) {
                            debug!("No active subscribers for channel: {}", channel);
                        }
                    }
                } else {
                    warn!("Failed to parse real-time event payload: {}", payload);
                }
            }
        }

        Ok(())
    }

    /// Publish an event to Redis and local subscribers
    pub async fn publish_event(&self, channel: &str, event_data: &serde_json::Value) -> Result<()> {
        let event = RealtimeEvent::CampaignUpdate {
            campaign: channel.to_string(),
            update_type: "generic".to_string(),
            data: event_data.clone(),
            timestamp: chrono::Utc::now(),
        };
        self.publish_real_event(event).await
    }

    /// Publish a RealtimeEvent to Redis and local subscribers
    pub async fn publish_real_event(&self, event: RealtimeEvent) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let channel = event.channel();
        let payload = serde_json::to_string(&event).context("Failed to serialize event")?;

        // Publish to Redis if available
        if let Some(redis_client) = &self.redis_client {
            let mut conn = redis_client
                .get_async_connection()
                .await
                .context("Failed to connect to Redis for publishing")?;

            let _: i32 = conn
                .publish(&channel, &payload)
                .await
                .context("Failed to publish to Redis")?;

            debug!("Published event to Redis channel: {}", channel);
        }

        // Publish to local subscribers
        self.publish_local_event(event).await;

        // Update stats
        {
            let mut stats_guard = self.stats.write().await;
            stats_guard.events_published += 1;
            stats_guard.last_event_time = Some(chrono::Utc::now());
        }

        Ok(())
    }

    /// Publish event to local subscribers only
    pub async fn publish_local_event(&self, event: RealtimeEvent) {
        let channel = event.channel();
        let event_type = event.event_type();

        // Get or create broadcaster for this channel
        let broadcaster = {
            let mut broadcasters_guard = self.event_broadcasters.write().await;
            broadcasters_guard
                .entry(channel.clone())
                .or_insert_with(|| {
                    debug!("Creating new broadcast channel: {}", channel);
                    broadcast::channel(self.config.buffer_size).0
                })
                .clone()
        };

        // Send to subscribers
        match broadcaster.send(event) {
            Ok(subscriber_count) => {
                debug!(
                    "Sent {} event to {} local subscribers on channel: {}",
                    event_type, subscriber_count, channel
                );
            }
            Err(_) => {
                debug!(
                    "No local subscribers for {} event on channel: {}",
                    event_type, channel
                );
            }
        }
    }

    /// Subscribe to real-time events for a specific channel
    pub async fn subscribe(&self, channel: &str) -> broadcast::Receiver<RealtimeEvent> {
        let mut broadcasters_guard = self.event_broadcasters.write().await;
        let broadcaster = broadcasters_guard
            .entry(channel.to_string())
            .or_insert_with(|| {
                debug!(
                    "Creating new broadcast channel for subscription: {}",
                    channel
                );
                broadcast::channel(self.config.buffer_size).0
            });

        let receiver = broadcaster.subscribe();

        // Update stats
        {
            let mut stats_guard = self.stats.write().await;
            stats_guard.active_subscribers += 1;
            stats_guard.channels_active = broadcasters_guard.len() as u64;
        }

        debug!("New subscriber for channel: {}", channel);
        receiver
    }

    /// Get current real-time statistics
    pub async fn get_stats(&self) -> RealtimeStats {
        self.stats.read().await.clone()
    }

    /// Health check for real-time features
    pub async fn health_check(&self) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        if let Some(redis_client) = &self.redis_client {
            let mut conn = redis_client
                .get_async_connection()
                .await
                .context("Failed to connect to Redis for health check")?;

            let _: String = redis::cmd("PING")
                .query_async(&mut conn)
                .await
                .context("Redis PING failed")?;
        }

        Ok(())
    }
}

impl Clone for RealtimeStats {
    fn clone(&self) -> Self {
        Self {
            events_published: self.events_published,
            events_received: self.events_received,
            active_subscribers: self.active_subscribers,
            channels_active: self.channels_active,
            last_event_time: self.last_event_time,
        }
    }
}

/// Helper functions for common real-time events
impl RealtimeManager {
    /// Publish queue status update
    pub async fn publish_queue_status(&self, queue_size: i64, active_runs: i64) -> Result<()> {
        let event = RealtimeEvent::QueueStatusUpdate {
            queue_size,
            active_runs,
            timestamp: chrono::Utc::now(),
        };
        self.publish_real_event(event).await
    }

    /// Publish run status change
    pub async fn publish_run_status_change(
        &self,
        run_id: String,
        old_status: String,
        new_status: String,
    ) -> Result<()> {
        let event = RealtimeEvent::RunStatusChange {
            run_id,
            old_status,
            new_status,
            timestamp: chrono::Utc::now(),
        };
        self.publish_real_event(event).await
    }

    /// Publish worker status update
    pub async fn publish_worker_status(
        &self,
        worker_id: String,
        status: String,
        current_task: Option<String>,
    ) -> Result<()> {
        let event = RealtimeEvent::WorkerStatusUpdate {
            worker_id,
            status,
            current_task,
            timestamp: chrono::Utc::now(),
        };
        self.publish_real_event(event).await
    }

    /// Publish system health change
    pub async fn publish_system_health(
        &self,
        component: String,
        old_status: String,
        new_status: String,
        details: Option<serde_json::Value>,
    ) -> Result<()> {
        let event = RealtimeEvent::SystemHealthChange {
            component,
            old_status,
            new_status,
            details,
            timestamp: chrono::Utc::now(),
        };
        self.publish_real_event(event).await
    }

    /// Publish publishing event
    pub async fn publish_publish_event(
        &self,
        codebase: String,
        action: String,
        result: String,
    ) -> Result<()> {
        let event = RealtimeEvent::PublishEvent {
            codebase,
            action,
            result,
            timestamp: chrono::Utc::now(),
        };
        self.publish_real_event(event).await
    }

    /// Publish campaign update
    pub async fn publish_campaign_update(
        &self,
        campaign: String,
        update_type: String,
        data: serde_json::Value,
    ) -> Result<()> {
        let event = RealtimeEvent::CampaignUpdate {
            campaign,
            update_type,
            data,
            timestamp: chrono::Utc::now(),
        };
        self.publish_real_event(event).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_realtime_event_serialization() {
        let event = RealtimeEvent::QueueStatusUpdate {
            queue_size: 10,
            active_runs: 5,
            timestamp: chrono::Utc::now(),
        };

        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: RealtimeEvent = serde_json::from_str(&serialized).unwrap();

        assert_eq!(event.event_type(), deserialized.event_type());
    }

    #[test]
    fn test_event_channels() {
        let queue_event = RealtimeEvent::QueueStatusUpdate {
            queue_size: 10,
            active_runs: 5,
            timestamp: chrono::Utc::now(),
        };
        assert_eq!(queue_event.channel(), "janitor:queue:status");

        let run_event = RealtimeEvent::RunStatusChange {
            run_id: "test-run-123".to_string(),
            old_status: "running".to_string(),
            new_status: "completed".to_string(),
            timestamp: chrono::Utc::now(),
        };
        assert_eq!(run_event.channel(), "janitor:run:test-run-123");
    }

    #[tokio::test]
    async fn test_realtime_manager_creation() {
        let config = RealtimeConfig::default();
        let manager = RealtimeManager::new(None, config);

        let stats = manager.get_stats().await;
        assert_eq!(stats.events_published, 0);
        assert_eq!(stats.events_received, 0);
    }

    #[tokio::test]
    async fn test_local_event_publishing() {
        let config = RealtimeConfig::default();
        let manager = RealtimeManager::new(None, config);

        let mut receiver = manager.subscribe("janitor:queue:status").await;

        let event = RealtimeEvent::QueueStatusUpdate {
            queue_size: 15,
            active_runs: 8,
            timestamp: chrono::Utc::now(),
        };

        manager.publish_local_event(event.clone()).await;

        let received_event = receiver.recv().await.unwrap();
        assert_eq!(received_event.event_type(), event.event_type());
    }
}
