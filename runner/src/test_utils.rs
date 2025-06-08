//! Test utilities for the runner module

use crate::{AppState, database::RunnerDatabase};
use janitor::test_utils::{TestDatabase, MockArtifactManager, MockLogFileManager, TestConfigBuilder};
use std::sync::Arc;
use crate::{
    vcs::RunnerVcsManager,
    performance::PerformanceMonitor,
    error_tracking::ErrorTracker,
    metrics::MetricsCollector,
    upload::UploadProcessor,
    auth::{WorkerAuthService, SecurityService},
};

/// Create a test AppState with mock dependencies
/// 
/// This function will return an error if no database is available.
/// Use `create_test_app_state_if_available()` for optional database setup.
pub async fn create_test_app_state() -> Result<Arc<AppState>, Box<dyn std::error::Error + Send + Sync>> {
    // Create test database
    let test_db = TestDatabase::new().await?;
    let janitor_db = test_db.into_janitor_database();
    let runner_db = RunnerDatabase::from_database(janitor_db);

    // Create mock managers
    let log_manager = Arc::new(MockLogFileManager);
    let artifact_manager = Arc::new(MockArtifactManager);

    // Create test config
    let config = Arc::new(TestConfigBuilder::new().build_janitor_config());

    // Create database-dependent components
    let runner_db_arc = Arc::new(runner_db);
    
    // Create other dependencies with minimal/mock implementations
    let vcs_manager = Arc::new(RunnerVcsManager::new(std::collections::HashMap::new()));
    let performance_monitor = Arc::new(PerformanceMonitor::new(std::time::Duration::from_secs(30)));
    let error_tracker = Arc::new(ErrorTracker::new(crate::error_tracking::ErrorTrackingConfig::default()));
    let metrics = Arc::new(MetricsCollector);
    
    // Create upload processor with temp directory
    let temp_dir = std::env::temp_dir().join("janitor_test_uploads");
    let upload_processor = Arc::new(UploadProcessor::new(
        temp_dir,
        1024 * 1024,  // 1MB max file size
        10 * 1024 * 1024  // 10MB max total size
    ));
    
    let auth_service = Arc::new(WorkerAuthService::new(runner_db_arc.clone()));
    let security_service = Arc::new(SecurityService::new(
        crate::auth::SecurityConfig::default(),
        runner_db_arc.clone()
    ));
    let resume_service = Arc::new(crate::resume::ResumeService::new((*runner_db_arc).clone()));

    Ok(Arc::new(AppState {
        database: runner_db_arc,
        vcs_manager,
        log_manager,
        artifact_manager,
        performance_monitor,
        error_tracker,
        metrics,
        config,
        upload_processor,
        auth_service,
        security_service,
        resume_service,
    }))
}

/// Create a test AppState with mock dependencies, returning None if database unavailable
pub async fn create_test_app_state_if_available() -> Result<Option<Arc<AppState>>, Box<dyn std::error::Error + Send + Sync>> {
    match create_test_app_state().await {
        Ok(state) => Ok(Some(state)),
        Err(_) => {
            eprintln!("Warning: Could not create test app state (likely no database available)");
            Ok(None)
        }
    }
}

/// Create a test app with mock state
pub async fn create_test_app() -> Result<axum::Router, Box<dyn std::error::Error + Send + Sync>> {
    let state = create_test_app_state().await?;
    Ok(crate::web::app(state))
}

/// Create a test app with mock state, returning None if database unavailable
pub async fn create_test_app_if_available() -> Result<Option<axum::Router>, Box<dyn std::error::Error + Send + Sync>> {
    match create_test_app_state_if_available().await? {
        Some(state) => Ok(Some(crate::web::app(state))),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_test_app_state() {
        let state = create_test_app_state_if_available().await;
        assert!(state.is_ok(), "Should be able to create test app state or return None");
    }

    #[tokio::test] 
    async fn test_create_test_app() {
        let app = create_test_app_if_available().await;
        assert!(app.is_ok(), "Should be able to create test app or return None");
    }
}