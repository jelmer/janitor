//! Redis integration for pub/sub messaging

use futures::StreamExt;
use redis::{aio::MultiplexedConnection, Client};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

use crate::error::{Result, UploadError};

/// Redis pub/sub client for receiving build result messages
pub struct RedisClient {
    /// Redis client connection
    client: Client,
    /// Connection instance
    connection: Option<MultiplexedConnection>,
}

/// Build result message from the runner service
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BuildResultMessage {
    /// Unique identifier for the build run
    pub log_id: String,
    /// Build target information
    pub target: BuildTarget,
    /// Build result status
    pub result: String,
    /// Additional metadata
    #[serde(default)]
    pub metadata: serde_json::Value,
}

/// Build target information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BuildTarget {
    /// Target name (e.g., "debian")
    pub name: String,
    /// Target-specific details
    pub details: BuildTargetDetails,
}

/// Build target details
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BuildTargetDetails {
    /// Build distribution
    pub build_distribution: String,
    /// Additional target-specific fields
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl RedisClient {
    /// Create a new Redis client
    pub async fn new(redis_url: &str) -> Result<Self> {
        info!("Connecting to Redis: {}", redis_url);

        let client = Client::open(redis_url).map_err(|e| UploadError::Redis(e))?;

        let mut redis_client = Self {
            client,
            connection: None,
        };

        // Test the connection
        redis_client.connect().await?;

        Ok(redis_client)
    }

    /// Connect to Redis
    async fn connect(&mut self) -> Result<()> {
        let connection = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| UploadError::Redis(e))?;

        self.connection = Some(connection);

        debug!("Successfully connected to Redis");

        Ok(())
    }

    /// Ensure connection is available
    async fn ensure_connected(&mut self) -> Result<&mut MultiplexedConnection> {
        if self.connection.is_none() {
            self.connect().await?;
        }

        self.connection.as_mut().ok_or_else(|| {
            UploadError::Redis(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Connection not available",
            )))
        })
    }

    /// Subscribe to build result messages
    pub async fn subscribe_to_results<F>(&mut self, handler: F) -> Result<()>
    where
        F: Fn(BuildResultMessage) -> futures::future::BoxFuture<'static, Result<()>>
            + Send
            + Sync
            + 'static,
    {
        info!("Subscribing to Redis pub/sub channel: result");

        let mut pubsub = self
            .client
            .get_async_pubsub()
            .await
            .map_err(|e| UploadError::Redis(e))?;

        pubsub
            .subscribe("result")
            .await
            .map_err(|e| UploadError::Redis(e))?;

        let mut stream = pubsub.into_on_message();

        info!("Listening for Redis messages...");

        while let Some(msg) = stream.next().await {
            let payload: String = msg.get_payload().map_err(|e| UploadError::Redis(e))?;

            debug!("Received Redis message: {}", payload);

            match self.parse_message(&payload).await {
                Ok(build_result) => {
                    info!(
                        log_id = %build_result.log_id,
                        target = %build_result.target.name,
                        distribution = %build_result.target.details.build_distribution,
                        "Processing build result message"
                    );

                    // Handle the message with timeout
                    match timeout(Duration::from_secs(300), handler(build_result)).await {
                        Ok(Ok(_)) => {
                            debug!("Successfully processed message");
                        }
                        Ok(Err(e)) => {
                            error!("Error processing message: {}", e);
                        }
                        Err(_) => {
                            error!("Message processing timed out after 5 minutes");
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to parse Redis message: {} - Error: {}", payload, e);
                }
            }
        }

        warn!("Redis pub/sub stream ended");
        Ok(())
    }

    /// Parse a Redis message into a BuildResultMessage
    async fn parse_message(&self, payload: &str) -> Result<BuildResultMessage> {
        serde_json::from_str(payload).map_err(|e| UploadError::Json(e))
    }

    /// Check Redis connection health
    pub async fn health_check(&mut self) -> Result<()> {
        self.ensure_connected().await?;
        Ok(())
    }
}

/// Redis connection manager for handling reconnections
pub struct RedisConnectionManager {
    /// Redis URL
    redis_url: String,
    /// Current client
    client: Option<RedisClient>,
    /// Reconnection attempts
    reconnect_attempts: u32,
    /// Maximum reconnection attempts
    max_reconnect_attempts: u32,
}

impl RedisConnectionManager {
    /// Create a new connection manager
    pub fn new(redis_url: String) -> Self {
        Self {
            redis_url,
            client: None,
            reconnect_attempts: 0,
            max_reconnect_attempts: 5,
        }
    }

    /// Get or create a Redis client
    pub async fn get_client(&mut self) -> Result<&mut RedisClient> {
        if self.client.is_none() {
            self.connect().await?;
        }

        self.client.as_mut().ok_or_else(|| {
            UploadError::Redis(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Failed to create Redis client",
            )))
        })
    }

    /// Connect to Redis with retry logic
    async fn connect(&mut self) -> Result<()> {
        while self.reconnect_attempts < self.max_reconnect_attempts {
            match RedisClient::new(&self.redis_url).await {
                Ok(client) => {
                    info!("Successfully connected to Redis");
                    self.client = Some(client);
                    self.reconnect_attempts = 0;
                    return Ok(());
                }
                Err(e) => {
                    self.reconnect_attempts += 1;
                    error!(
                        "Failed to connect to Redis (attempt {}/{}): {}",
                        self.reconnect_attempts, self.max_reconnect_attempts, e
                    );

                    if self.reconnect_attempts < self.max_reconnect_attempts {
                        let delay = Duration::from_secs(2u64.pow(self.reconnect_attempts));
                        warn!("Retrying Redis connection in {:?}", delay);
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(UploadError::Redis(redis::RedisError::from((
            redis::ErrorKind::IoError,
            "Exceeded maximum reconnection attempts",
        ))))
    }

    /// Handle connection errors and attempt reconnection
    pub async fn handle_connection_error(&mut self, error: &UploadError) -> Result<()> {
        match error {
            UploadError::Redis(_) => {
                warn!("Redis connection error detected, attempting reconnection");
                self.client = None;
                self.connect().await
            }
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_build_result_message() {
        let json = r#"{
            "log_id": "test-run-123",
            "target": {
                "name": "debian",
                "details": {
                    "build_distribution": "unstable",
                    "extra_field": "extra_value"
                }
            },
            "result": "success"
        }"#;

        let message: BuildResultMessage = serde_json::from_str(json).unwrap();
        assert_eq!(message.log_id, "test-run-123");
        assert_eq!(message.target.name, "debian");
        assert_eq!(message.target.details.build_distribution, "unstable");
        assert_eq!(message.result, "success");
    }

    #[test]
    fn test_redis_connection_manager() {
        let manager = RedisConnectionManager::new("redis://localhost:6379".to_string());
        assert_eq!(manager.redis_url, "redis://localhost:6379");
        assert_eq!(manager.max_reconnect_attempts, 5);
        assert_eq!(manager.reconnect_attempts, 0);
    }
}
