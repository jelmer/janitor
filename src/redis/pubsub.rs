//! Redis pub/sub functionality

use crate::error::JanitorError;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

/// Trait for messages that can be sent via Redis pub/sub
pub trait PubSubMessage: Serialize + for<'de> Deserialize<'de> + Send + Sync {
    /// Get the channel name for this message type
    fn channel() -> &'static str;
}

/// Publisher for sending messages via Redis pub/sub
#[derive(Clone)]
pub struct Publisher {
    manager: super::RedisManager,
}

impl Publisher {
    /// Create a new publisher
    pub(super) fn new(manager: super::RedisManager) -> Self {
        Self { manager }
    }

    /// Publish a message to its channel
    pub async fn publish<T: PubSubMessage>(&self, message: &T) -> Result<(), JanitorError> {
        let channel = T::channel();
        let payload = serde_json::to_string(message)
            .map_err(|e| JanitorError::redis_msg(format!("Failed to serialize message: {}", e)))?;

        let mut cmd = redis::cmd("PUBLISH");
        cmd.arg(channel).arg(&payload);

        let subscribers: i32 = self.manager.execute(&mut cmd).await?;

        debug!(
            "Published message to channel '{}', {} subscribers received it",
            channel, subscribers
        );

        Ok(())
    }

    /// Publish a message to a specific channel
    pub async fn publish_to_channel(
        &self,
        channel: &str,
        message: &str,
    ) -> Result<i32, JanitorError> {
        let mut cmd = redis::cmd("PUBLISH");
        cmd.arg(channel).arg(message);

        let subscribers: i32 = self.manager.execute(&mut cmd).await?;

        debug!(
            "Published raw message to channel '{}', {} subscribers",
            channel, subscribers
        );

        Ok(subscribers)
    }
}

/// Subscriber for receiving messages via Redis pub/sub
pub struct Subscriber {
    manager: super::RedisManager,
}

impl Subscriber {
    /// Create a new subscriber
    pub(super) fn new(manager: super::RedisManager) -> Self {
        Self { manager }
    }

    /// Subscribe to a message type and process messages
    pub async fn subscribe<T, F, Fut>(&self, handler: F) -> Result<(), JanitorError>
    where
        T: PubSubMessage,
        F: Fn(T) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<(), JanitorError>> + Send,
    {
        let channel = T::channel();
        let handler = std::sync::Arc::new(handler);

        self.subscribe_to_channel(channel, move |payload| {
            let handler = handler.clone();
            async move {
                match serde_json::from_str::<T>(&payload) {
                    Ok(message) => handler(message).await,
                    Err(e) => {
                        error!(
                            "Failed to deserialize message from channel '{}': {}",
                            channel, e
                        );
                        Ok(())
                    }
                }
            }
        })
        .await
    }

    /// Subscribe to a specific channel and process raw messages
    pub async fn subscribe_to_channel<F, Fut>(
        &self,
        channel: &str,
        mut handler: F,
    ) -> Result<(), JanitorError>
    where
        F: FnMut(String) -> Fut + Send,
        Fut: std::future::Future<Output = Result<(), JanitorError>> + Send,
    {
        info!("Subscribing to Redis channel: {}", channel);

        let mut pubsub = self
            .manager
            .client()
            .get_async_pubsub()
            .await
            .map_err(|e| JanitorError::Redis(e))?;

        pubsub
            .subscribe(channel)
            .await
            .map_err(|e| JanitorError::Redis(e))?;

        info!("Successfully subscribed to channel: {}", channel);

        let mut stream = pubsub.on_message();

        while let Some(msg) = stream.next().await {
            match msg.get_payload::<String>() {
                Ok(payload) => {
                    debug!(
                        "Received message on channel '{}': {} bytes",
                        channel,
                        payload.len()
                    );
                    if let Err(e) = handler(payload).await {
                        error!("Error handling message: {}", e);
                    }
                }
                Err(e) => {
                    warn!("Failed to get message payload: {}", e);
                }
            }
        }

        warn!("Subscription to channel '{}' ended", channel);
        Ok(())
    }

    /// Subscribe to multiple channels
    pub async fn subscribe_to_channels<F, Fut>(
        &self,
        channels: &[&str],
        mut handler: F,
    ) -> Result<(), JanitorError>
    where
        F: FnMut(String, String) -> Fut + Send,
        Fut: std::future::Future<Output = Result<(), JanitorError>> + Send,
    {
        info!("Subscribing to Redis channels: {:?}", channels);

        let mut pubsub = self
            .manager
            .client()
            .get_async_pubsub()
            .await
            .map_err(|e| JanitorError::Redis(e))?;

        for channel in channels {
            pubsub
                .subscribe(channel)
                .await
                .map_err(|e| JanitorError::Redis(e))?;
        }

        info!("Successfully subscribed to {} channels", channels.len());

        let mut stream = pubsub.on_message();

        while let Some(msg) = stream.next().await {
            let channel = msg.get_channel_name().to_string();
            match msg.get_payload::<String>() {
                Ok(payload) => {
                    debug!(
                        "Received message on channel '{}': {} bytes",
                        channel,
                        payload.len()
                    );
                    if let Err(e) = handler(channel, payload).await {
                        error!("Error handling message: {}", e);
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to get message payload from channel '{}': {}",
                        channel, e
                    );
                }
            }
        }

        warn!("Multi-channel subscription ended");
        Ok(())
    }
}

/// Pattern-based subscriber for dynamic channel subscriptions
pub struct PatternSubscriber {
    manager: super::RedisManager,
}

impl PatternSubscriber {
    /// Create a new pattern subscriber
    pub fn new(manager: super::RedisManager) -> Self {
        Self { manager }
    }

    /// Subscribe to channels matching a pattern
    pub async fn psubscribe<F, Fut>(
        &self,
        pattern: &str,
        mut handler: F,
    ) -> Result<(), JanitorError>
    where
        F: FnMut(String, String) -> Fut + Send,
        Fut: std::future::Future<Output = Result<(), JanitorError>> + Send,
    {
        info!("Subscribing to Redis pattern: {}", pattern);

        let mut pubsub = self
            .manager
            .client()
            .get_async_pubsub()
            .await
            .map_err(|e| JanitorError::Redis(e))?;

        pubsub
            .psubscribe(pattern)
            .await
            .map_err(|e| JanitorError::Redis(e))?;

        info!("Successfully subscribed to pattern: {}", pattern);

        let mut stream = pubsub.on_message();

        while let Some(msg) = stream.next().await {
            let channel = msg.get_channel_name().to_string();
            match msg.get_payload::<String>() {
                Ok(payload) => {
                    debug!(
                        "Received message on channel '{}' (pattern '{}'): {} bytes",
                        channel,
                        pattern,
                        payload.len()
                    );
                    if let Err(e) = handler(channel, payload).await {
                        error!("Error handling message: {}", e);
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to get message payload from channel '{}': {}",
                        channel, e
                    );
                }
            }
        }

        warn!("Pattern subscription to '{}' ended", pattern);
        Ok(())
    }
}

// Example message types for common Janitor operations

/// Build result message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildResultMessage {
    pub run_id: String,
    pub codebase: String,
    pub campaign: String,
    pub status: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl PubSubMessage for BuildResultMessage {
    fn channel() -> &'static str {
        "janitor:build-results"
    }
}

/// Worker status update message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerStatusMessage {
    pub worker_id: String,
    pub status: String,
    pub current_run: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl PubSubMessage for WorkerStatusMessage {
    fn channel() -> &'static str {
        "janitor:worker-status"
    }
}

/// Queue event message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEventMessage {
    pub event_type: String,
    pub queue_item_id: String,
    pub details: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl PubSubMessage for QueueEventMessage {
    fn channel() -> &'static str {
        "janitor:queue-events"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_channels() {
        assert_eq!(BuildResultMessage::channel(), "janitor:build-results");
        assert_eq!(WorkerStatusMessage::channel(), "janitor:worker-status");
        assert_eq!(QueueEventMessage::channel(), "janitor:queue-events");
    }

    #[test]
    fn test_message_serialization() {
        let msg = BuildResultMessage {
            run_id: "test-123".to_string(),
            codebase: "test/repo".to_string(),
            campaign: "lintian-fixes".to_string(),
            status: "success".to_string(),
            timestamp: chrono::Utc::now(),
        };

        let serialized = serde_json::to_string(&msg).unwrap();
        let deserialized: BuildResultMessage = serde_json::from_str(&serialized).unwrap();

        assert_eq!(msg.run_id, deserialized.run_id);
        assert_eq!(msg.codebase, deserialized.codebase);
        assert_eq!(msg.campaign, deserialized.campaign);
        assert_eq!(msg.status, deserialized.status);
    }
}
