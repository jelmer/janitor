//! Message processing and routing for build results

use tracing::{debug, info, warn};

use crate::artifacts::ArtifactProcessor;
use crate::error::Result;
use crate::process::upload_build_result;
use crate::redis_client::BuildResultMessage;
use crate::upload::UploadConfig;

/// Message handler for processing build result messages
pub struct MessageHandler {
    /// Artifact processor for retrieving build artifacts
    artifact_processor: ArtifactProcessor,
    /// Upload configuration
    upload_config: UploadConfig,
}

impl MessageHandler {
    /// Create a new message handler
    pub async fn new(
        artifact_location: &str,
        upload_config: UploadConfig,
    ) -> Result<Self> {
        let artifact_processor = ArtifactProcessor::new(artifact_location).await?;
        
        Ok(Self {
            artifact_processor,
            upload_config,
        })
    }
    
    /// Process a build result message
    pub async fn handle_message(&self, message: BuildResultMessage) -> Result<()> {
        info!(
            log_id = %message.log_id,
            target = %message.target.name,
            distribution = %message.target.details.build_distribution,
            result = %message.result,
            "Received build result message"
        );
        
        // Apply filters before processing
        if !self.should_process_message(&message).await? {
            debug!(
                log_id = %message.log_id,
                "Skipping message due to filters"
            );
            return Ok(());
        }
        
        // Check if this is a successful build
        if !self.is_successful_build(&message) {
            debug!(
                log_id = %message.log_id,
                result = %message.result,
                "Skipping non-successful build"
            );
            return Ok(());
        }
        
        // Process the upload
        match upload_build_result(
            &message.log_id,
            &self.artifact_processor,
            &self.upload_config,
        ).await {
            Ok(_) => {
                info!(
                    log_id = %message.log_id,
                    "Successfully processed upload"
                );
                Ok(())
            }
            Err(e) => {
                warn!(
                    log_id = %message.log_id,
                    error = %e,
                    "Failed to process upload: {}",
                    e
                );
                Err(e)
            }
        }
    }
    
    /// Check if a message should be processed based on filters
    async fn should_process_message(&self, message: &BuildResultMessage) -> Result<bool> {
        // Check target name filter (only process "debian" targets)
        if message.target.name != "debian" {
            debug!(
                target = %message.target.name,
                "Skipping non-debian target"
            );
            return Ok(false);
        }
        
        // Check distribution filter
        if !self.upload_config.should_upload_distribution(&message.target.details.build_distribution) {
            debug!(
                distribution = %message.target.details.build_distribution,
                "Skipping distribution not in allowed list"
            );
            return Ok(false);
        }
        
        // Check if artifacts exist (optional optimization)
        if !self.artifact_processor.artifacts_exist(&message.log_id).await {
            warn!(
                log_id = %message.log_id,
                "Artifacts not found for build"
            );
            return Ok(false);
        }
        
        Ok(true)
    }
    
    /// Check if this represents a successful build that should be uploaded
    fn is_successful_build(&self, message: &BuildResultMessage) -> bool {
        // Check common success indicators
        matches!(
            message.result.to_lowercase().as_str(),
            "success" | "successful" | "ok" | "passed"
        )
    }
    
    /// Get upload statistics
    pub async fn get_upload_stats(&self) -> UploadStats {
        // In a real implementation, this would track statistics
        // For now, return default stats
        UploadStats::default()
    }
}

/// Upload statistics for monitoring
#[derive(Debug, Default)]
pub struct UploadStats {
    /// Total messages processed
    pub messages_processed: u64,
    /// Successful uploads
    pub successful_uploads: u64,
    /// Failed uploads
    pub failed_uploads: u64,
    /// Messages skipped due to filters
    pub messages_skipped: u64,
}

/// Message filter configuration
#[derive(Debug, Clone)]
pub struct MessageFilter {
    /// Target names to process
    pub allowed_targets: Vec<String>,
    /// Distributions to process
    pub allowed_distributions: Vec<String>,
    /// Results to process
    pub allowed_results: Vec<String>,
}

impl Default for MessageFilter {
    fn default() -> Self {
        Self {
            allowed_targets: vec!["debian".to_string()],
            allowed_distributions: vec![], // Empty means all
            allowed_results: vec![
                "success".to_string(),
                "successful".to_string(),
                "ok".to_string(),
                "passed".to_string(),
            ],
        }
    }
}

impl MessageFilter {
    /// Check if a message passes the filter
    pub fn should_process(&self, message: &BuildResultMessage) -> bool {
        // Check target
        if !self.allowed_targets.is_empty() && !self.allowed_targets.contains(&message.target.name) {
            return false;
        }
        
        // Check distribution
        if !self.allowed_distributions.is_empty() 
            && !self.allowed_distributions.contains(&message.target.details.build_distribution) {
            return false;
        }
        
        // Check result
        if !self.allowed_results.is_empty() && !self.allowed_results.contains(&message.result) {
            return false;
        }
        
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::redis_client::{BuildTarget, BuildTargetDetails};
    
    fn create_test_message(target: &str, distribution: &str, result: &str) -> BuildResultMessage {
        BuildResultMessage {
            log_id: "test-123".to_string(),
            target: BuildTarget {
                name: target.to_string(),
                details: BuildTargetDetails {
                    build_distribution: distribution.to_string(),
                    extra: serde_json::Map::new(),
                },
            },
            result: result.to_string(),
            metadata: serde_json::Value::Null,
        }
    }
    
    #[test]
    fn test_message_filter_default() {
        let filter = MessageFilter::default();
        
        // Should process debian targets
        let debian_msg = create_test_message("debian", "unstable", "success");
        assert!(filter.should_process(&debian_msg));
        
        // Should not process non-debian targets
        let ubuntu_msg = create_test_message("ubuntu", "focal", "success");
        assert!(!filter.should_process(&ubuntu_msg));
        
        // Should not process failed builds
        let failed_msg = create_test_message("debian", "unstable", "failed");
        assert!(!filter.should_process(&failed_msg));
    }
    
    #[test]
    fn test_message_filter_custom() {
        let filter = MessageFilter {
            allowed_targets: vec!["debian".to_string()],
            allowed_distributions: vec!["unstable".to_string(), "experimental".to_string()],
            allowed_results: vec!["success".to_string()],
        };
        
        // Should process allowed combination
        let allowed_msg = create_test_message("debian", "unstable", "success");
        assert!(filter.should_process(&allowed_msg));
        
        // Should not process disallowed distribution
        let wrong_dist = create_test_message("debian", "stable", "success");
        assert!(!filter.should_process(&wrong_dist));
    }
}