//! Test utilities for the differ crate.

use janitor::artifacts::ArtifactManager;
use std::path::PathBuf;
use std::sync::Arc;

/// Test application state for integration tests
pub struct TestAppState {
    /// Database connection pool
    pub pool: sqlx::PgPool,
    /// Artifact manager instance
    pub artifact_manager: Arc<Box<dyn ArtifactManager>>,
    /// Memory limit for tasks in MB
    pub task_memory_limit: Option<usize>,
    /// Timeout for tasks in seconds
    pub task_timeout: Option<usize>,
    /// Path to diffoscope cache
    pub diffoscope_cache_path: Option<PathBuf>,
    /// Path to debdiff cache
    pub debdiff_cache_path: Option<PathBuf>,
    /// Command to run diffoscope
    pub diffoscope_command: String,
}

impl TestAppState {
    /// Create a new test app state with default values
    pub fn new(pool: sqlx::PgPool, artifact_manager: Box<dyn ArtifactManager>) -> Self {
        Self {
            pool,
            artifact_manager: Arc::new(artifact_manager),
            task_memory_limit: Some(1500),
            task_timeout: Some(60),
            diffoscope_cache_path: None,
            debdiff_cache_path: None,
            diffoscope_command: "diffoscope".to_string(),
        }
    }
}
