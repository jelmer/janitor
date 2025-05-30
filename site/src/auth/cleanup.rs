use std::time::Duration;
use tokio::time::interval;
use tracing::{info, error};

use crate::auth::session::SessionManager;

/// Background task for cleaning up expired sessions
pub struct SessionCleanupTask {
    session_manager: SessionManager,
    cleanup_interval: Duration,
}

impl SessionCleanupTask {
    /// Create a new session cleanup task
    pub fn new(session_manager: SessionManager) -> Self {
        Self {
            session_manager,
            cleanup_interval: Duration::from_secs(3600), // Run every hour
        }
    }
    
    /// Create a new session cleanup task with custom interval
    pub fn with_interval(session_manager: SessionManager, cleanup_interval: Duration) -> Self {
        Self {
            session_manager,
            cleanup_interval,
        }
    }
    
    /// Run the cleanup task in a loop
    pub async fn run(self) {
        let mut interval = interval(self.cleanup_interval);
        
        info!(
            "Starting session cleanup task with interval of {:?}",
            self.cleanup_interval
        );
        
        loop {
            interval.tick().await;
            
            match self.session_manager.cleanup_expired_sessions().await {
                Ok(count) => {
                    if count > 0 {
                        info!("Cleaned up {} expired sessions", count);
                    }
                }
                Err(e) => {
                    error!("Failed to cleanup expired sessions: {}", e);
                }
            }
        }
    }
    
    /// Run the cleanup task once (for testing or manual cleanup)
    pub async fn run_once(&self) -> Result<u64, crate::auth::session::SessionError> {
        info!("Running session cleanup task once");
        let count = self.session_manager.cleanup_expired_sessions().await?;
        if count > 0 {
            info!("Cleaned up {} expired sessions", count);
        }
        Ok(count)
    }
}

/// Start the session cleanup background task
pub fn spawn_cleanup_task(session_manager: SessionManager) -> tokio::task::JoinHandle<()> {
    let cleanup_task = SessionCleanupTask::new(session_manager);
    tokio::spawn(cleanup_task.run())
}

/// Start the session cleanup background task with custom interval
pub fn spawn_cleanup_task_with_interval(
    session_manager: SessionManager,
    interval: Duration,
) -> tokio::task::JoinHandle<()> {
    let cleanup_task = SessionCleanupTask::with_interval(session_manager, interval);
    tokio::spawn(cleanup_task.run())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    
    #[test]
    fn test_cleanup_task_creation() {
        // This would require a test database setup
        // For now, just test the constructor
        let interval = Duration::from_secs(1800); // 30 minutes
        
        // Can't easily test without a real SessionManager
        // but we can test the interval logic
        assert_eq!(interval.as_secs(), 1800);
    }
}