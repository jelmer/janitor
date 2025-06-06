//! Redis integration for publish service.
//!
//! This module provides Redis pub/sub functionality for communicating with
//! other services, particularly the runner service.

use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use sqlx::Row;

// Type alias for connection manager
/// Type alias for Redis connection manager used throughout the publish service.
pub type RedisConnectionManager = ConnectionManager;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Redis publisher for sending messages to other services.
pub struct RedisPublisher {
    /// Redis connection manager.
    redis: ConnectionManager,
}

impl RedisPublisher {
    /// Create a new Redis publisher.
    ///
    /// # Arguments
    /// * `redis` - Redis connection manager
    ///
    /// # Returns
    /// A new RedisPublisher instance
    pub fn new(redis: ConnectionManager) -> Self {
        Self { redis }
    }

    /// Publish a message to a Redis channel.
    ///
    /// # Arguments
    /// * `channel` - The channel to publish to
    /// * `message` - The message to publish
    ///
    /// # Returns
    /// Ok(()) if successful, or a redis::RedisError
    pub async fn publish(&mut self, channel: &str, message: &str) -> Result<(), redis::RedisError> {
        let _: () = self.redis.publish(channel, message).await?;
        log::debug!("Published message to channel '{}': {}", channel, message);
        Ok(())
    }

    /// Publish a publish event to the "publish" channel.
    ///
    /// # Arguments
    /// * `event` - The publish event data
    ///
    /// # Returns
    /// Ok(()) if successful, or a redis::RedisError
    pub async fn publish_event(&mut self, event: &PublishEvent) -> Result<(), redis::RedisError> {
        let message = serde_json::to_string(event).map_err(|e| {
            redis::RedisError::from((
                redis::ErrorKind::IoError,
                "JSON serialization failed",
                e.to_string(),
            ))
        })?;

        self.publish("publish", &message).await
    }

    /// Publish a merge proposal event to the "merge-proposal" channel.
    ///
    /// # Arguments
    /// * `event` - The merge proposal event data
    ///
    /// # Returns
    /// Ok(()) if successful, or a redis::RedisError
    pub async fn publish_merge_proposal(
        &mut self,
        event: &MergeProposalEvent,
    ) -> Result<(), redis::RedisError> {
        let message = serde_json::to_string(event).map_err(|e| {
            redis::RedisError::from((
                redis::ErrorKind::IoError,
                "JSON serialization failed",
                e.to_string(),
            ))
        })?;

        self.publish("merge-proposal", &message).await
    }
}

/// Event data for publish notifications.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PublishEvent {
    /// The codebase that was published.
    pub codebase: String,
    /// The campaign/suite that was published.
    pub campaign: String,
    /// The publish mode used.
    pub mode: String,
    /// The result code of the publish operation.
    pub result_code: String,
    /// Optional description of the result.
    pub description: Option<String>,
    /// Optional URL of the merge proposal created.
    pub proposal_url: Option<String>,
    /// Optional web URL of the merge proposal.
    pub proposal_web_url: Option<String>,
    /// The target branch URL.
    pub target_branch_url: Option<String>,
    /// The branch name that was published.
    pub branch_name: String,
    /// The revision that was published.
    pub revision: Option<String>,
    /// Timestamp of the event.
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Event data for merge proposal notifications.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MergeProposalEvent {
    /// URL of the merge proposal.
    pub url: String,
    /// Web URL of the merge proposal.
    pub web_url: Option<String>,
    /// Status of the merge proposal.
    pub status: String,
    /// The codebase this proposal belongs to.
    pub codebase: String,
    /// The campaign/suite this proposal belongs to.
    pub campaign: String,
    /// The target branch URL.
    pub target_branch_url: String,
    /// Optional target branch web URL.
    pub target_branch_web_url: Option<String>,
    /// Timestamp of the event.
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Redis subscriber for receiving messages from other services.
pub struct RedisSubscriber {
    /// Redis client for creating pubsub connections.
    redis_client: redis::Client,
    /// Channel for receiving shutdown signals.
    shutdown_rx: mpsc::Receiver<()>,
}

impl RedisSubscriber {
    /// Create a new Redis subscriber.
    ///
    /// # Arguments
    /// * `redis_client` - Redis client for creating connections
    /// * `shutdown_rx` - Channel for receiving shutdown signals
    ///
    /// # Returns
    /// A new RedisSubscriber instance
    pub fn new(redis_client: redis::Client, shutdown_rx: mpsc::Receiver<()>) -> Self {
        Self { redis_client, shutdown_rx }
    }

    /// Listen to the runner service for new runs to publish.
    ///
    /// This function subscribes to the "runner" Redis channel and processes
    /// messages about new runs that are ready for publishing.
    ///
    /// # Arguments
    /// * `state` - Application state
    ///
    /// # Returns
    /// Ok(()) when the listener is shut down, or an error
    pub async fn listen_to_runner(
        mut self,
        state: Arc<crate::AppState>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        log::info!("Starting Redis listener for runner messages");

        use futures::StreamExt;
        
        // Create a pubsub connection
        let mut pubsub = self.redis_client.get_async_pubsub().await?;
        pubsub.subscribe("runner").await?;
        
        let mut pubsub_stream = pubsub.into_on_message();

        loop {
            tokio::select! {
                // Check for shutdown signal
                result = self.shutdown_rx.recv() => {
                    match result {
                        Some(()) => {
                            log::info!("Received shutdown signal, stopping Redis listener");
                            break;
                        }
                        None => {
                            log::warn!("Shutdown channel closed, stopping Redis listener");
                            break;
                        }
                    }
                }
                
                // Process Redis messages
                msg_result = pubsub_stream.next() => {
                    match msg_result {
                        Some(msg) => {
                            if let Err(e) = self.process_runner_message(&state, &msg).await {
                                log::error!("Error processing runner message: {}", e);
                            }
                        }
                        None => {
                            log::warn!("Redis pubsub stream ended, reconnecting...");
                            // Attempt to reconnect
                            match self.redis_client.get_async_pubsub().await {
                                Ok(mut new_pubsub) => {
                                    if let Err(e) = new_pubsub.subscribe("runner").await {
                                        log::error!("Failed to resubscribe to runner channel: {}", e);
                                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                                        continue;
                                    }
                                    pubsub_stream = new_pubsub.into_on_message();
                                }
                                Err(e) => {
                                    log::error!("Failed to reconnect to Redis: {}", e);
                                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                                }
                            }
                        }
                    }
                }
            }
        }

        log::info!("Redis listener stopped");
        Ok(())
    }

    /// Process a message from the runner service.
    ///
    /// # Arguments
    /// * `state` - Application state
    /// * `msg` - The Redis message
    ///
    /// # Returns
    /// Ok(()) if successful, or an error
    async fn process_runner_message(
        &self,
        state: &Arc<crate::AppState>,
        msg: &redis::Msg,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let payload: String = msg.get_payload()?;
        log::debug!("Received runner message: {}", payload);

        // Parse the message as JSON
        let message: serde_json::Value = serde_json::from_str(&payload)?;

        // Extract run information from the message
        if let Some(run_data) = message.as_object() {
            // Check if this is a "run-finished" event
            if let Some(event_type) = run_data.get("event").and_then(|v| v.as_str()) {
                if event_type == "run-finished" {
                    if let Some(run_id) = run_data.get("run_id").and_then(|v| v.as_str()) {
                        log::info!("Processing run-finished event for run {}", run_id);

                        // Process this run for potential publishing
                        if let Err(e) = self.process_finished_run(state, run_id).await {
                            log::error!("Error processing finished run {}: {}", run_id, e);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Process a finished run for potential publishing.
    ///
    /// # Arguments
    /// * `state` - Application state
    /// * `run_id` - The ID of the finished run
    ///
    /// # Returns
    /// Ok(()) if successful, or an error
    async fn process_finished_run(
        &self,
        state: &Arc<crate::AppState>,
        run_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Get the run details from the database
        let run = sqlx::query_as::<_, janitor::state::Run>("SELECT * FROM run WHERE id = $1")
            .bind(run_id)
            .fetch_optional(&state.conn)
            .await?;

        // Get the rate limit bucket separately
        let rate_limit_bucket = sqlx::query(
            "SELECT COALESCE(rate_limit_bucket, 'default') as rate_limit_bucket FROM run WHERE id = $1"
        )
        .bind(run_id)
        .fetch_optional(&state.conn)
        .await?
        .map(|row| row.get::<String, _>("rate_limit_bucket"))
        .unwrap_or_else(|| "default".to_string());

        let run = match run {
            Some(run) => run,
            None => {
                log::warn!("Run {} not found in database", run_id);
                return Ok(());
            }
        };

        // Only process successful runs
        if run.result_code != "success" {
            log::debug!(
                "Run {} is not successful ({}), skipping",
                run_id,
                run.result_code
            );
            return Ok(());
        }

        // Get unpublished branches for this run
        let unpublished_branches = sqlx::query_as::<_, crate::state::UnpublishedBranch>(
            r#"
            SELECT 
                role,
                remote_name,
                base_revision,
                revision,
                publish_mode,
                max_frequency_days,
                name
            FROM new_result_branch 
            LEFT JOIN publish ON new_result_branch.run_id = publish.run_id 
                AND new_result_branch.role = publish.branch_name
                AND new_result_branch.revision = publish.revision
            WHERE new_result_branch.run_id = $1 
                AND publish.id IS NULL
            "#,
        )
        .bind(run_id)
        .fetch_all(&state.conn)
        .await?;

        if unpublished_branches.is_empty() {
            log::debug!("No unpublished branches found for run {}", run_id);
            return Ok(());
        }

        // rate_limit_bucket is already defined above
        let command = run.command.clone();

        log::info!(
            "Processing run {} with {} unpublished branches",
            run_id,
            unpublished_branches.len()
        );

        // Consider publishing this run
        match crate::consider_publish_run(
            &state.conn,
            state.redis.clone(),
            state.config,
            &state.publish_worker,
            &state.vcs_managers,
            &state.bucket_rate_limiter,
            &run,
            &rate_limit_bucket,
            &unpublished_branches,
            &command,
            state.push_limit,
            state.require_binary_diff,
        )
        .await
        {
            Ok(results) => {
                log::info!(
                    "Successfully considered run {} for publishing: {:?}",
                    run_id,
                    results
                );
            }
            Err(e) => {
                log::error!("Error considering run {} for publishing: {}", run_id, e);
            }
        }

        Ok(())
    }
}

/// Listen to the runner for new changes to publish.
///
/// This function sets up a Redis subscriber to listen for notifications
/// from the runner service about completed runs that may need publishing.
///
/// # Arguments
/// * `state` - The application state
/// * `shutdown_rx` - Channel for receiving shutdown signals
///
/// # Returns
/// A future that resolves when the listener is shut down
pub async fn listen_to_runner(
    state: Arc<crate::AppState>,
    shutdown_rx: mpsc::Receiver<()>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let redis_url = match &state.config.redis_location {
        Some(url) => url,
        None => {
            log::warn!("Redis not configured, runner listener disabled");
            return Ok(());
        }
    };

    // Create a Redis client for pubsub operations
    let redis_client = redis::Client::open(redis_url.to_string())?;

    let subscriber = RedisSubscriber::new(redis_client, shutdown_rx);
    subscriber.listen_to_runner(state).await
}

/// Publish a success event to Redis.
///
/// # Arguments
/// * `redis` - Optional Redis connection
/// * `event` - The publish event to send
///
/// # Returns
/// Ok(()) if successful or Redis not configured, or an error
pub async fn pubsub_publish_event(
    redis: Option<&mut ConnectionManager>,
    event: &PublishEvent,
) -> Result<(), redis::RedisError> {
    if let Some(redis) = redis {
        let mut publisher = RedisPublisher::new(redis.clone());
        publisher.publish_event(event).await?;
    }
    Ok(())
}

/// Publish a merge proposal event to Redis.
///
/// # Arguments
/// * `redis` - Optional Redis connection
/// * `event` - The merge proposal event to send
///
/// # Returns
/// Ok(()) if successful or Redis not configured, or an error
pub async fn pubsub_publish_merge_proposal(
    redis: Option<&mut ConnectionManager>,
    event: &MergeProposalEvent,
) -> Result<(), redis::RedisError> {
    if let Some(redis) = redis {
        let mut publisher = RedisPublisher::new(redis.clone());
        publisher.publish_merge_proposal(event).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_publish_event_serialization() {
        let event = PublishEvent {
            codebase: "test-codebase".to_string(),
            campaign: "test-campaign".to_string(),
            mode: "propose".to_string(),
            result_code: "success".to_string(),
            description: Some("Test publish".to_string()),
            proposal_url: Some("https://github.com/test/repo/pull/123".to_string()),
            proposal_web_url: Some("https://github.com/test/repo/pull/123".to_string()),
            target_branch_url: Some("https://github.com/test/repo".to_string()),
            branch_name: "main".to_string(),
            revision: Some("abc123".to_string()),
            timestamp: Utc::now(),
        };

        let json = serde_json::to_string(&event).unwrap();
        let parsed: PublishEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(event.codebase, parsed.codebase);
        assert_eq!(event.campaign, parsed.campaign);
        assert_eq!(event.mode, parsed.mode);
    }

    #[test]
    fn test_merge_proposal_event_serialization() {
        let event = MergeProposalEvent {
            url: "https://github.com/test/repo/pull/123".to_string(),
            web_url: Some("https://github.com/test/repo/pull/123".to_string()),
            status: "open".to_string(),
            codebase: "test-codebase".to_string(),
            campaign: "test-campaign".to_string(),
            target_branch_url: "https://github.com/test/repo".to_string(),
            target_branch_web_url: Some("https://github.com/test/repo".to_string()),
            timestamp: Utc::now(),
        };

        let json = serde_json::to_string(&event).unwrap();
        let parsed: MergeProposalEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(event.url, parsed.url);
        assert_eq!(event.status, parsed.status);
        assert_eq!(event.codebase, parsed.codebase);
    }
}
