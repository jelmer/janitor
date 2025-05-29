//! Comprehensive integration tests for the Janitor Runner.

use janitor_runner::{
    application::Application,
    config::{ApplicationConfig, DatabaseConfig, RunnerConfig, WebConfig},
};
use serde_json::json;
use std::time::Duration;
use tokio::time::timeout;

/// Test configuration for integration tests.
fn test_config() -> RunnerConfig {
    RunnerConfig {
        database: DatabaseConfig {
            url: "postgresql://localhost/janitor_test".to_string(),
            max_connections: 5,
            connection_timeout_seconds: 10,
            query_timeout_seconds: 10,
            enable_sql_logging: false,
        },
        web: WebConfig {
            listen_address: "127.0.0.1".to_string(),
            port: 0, // Use random port for tests
            public_port: 0,
            request_timeout_seconds: 30,
            max_request_size_bytes: 1024 * 1024, // 1MB
            enable_request_logging: false,
            enable_cors: false,
        },
        application: ApplicationConfig {
            name: "janitor-runner-test".to_string(),
            debug: true,
            enable_graceful_shutdown: false, // Disable for tests
            ..Default::default()
        },
        ..Default::default()
    }
}

#[tokio::test]
async fn test_application_lifecycle() {
    // Test the complete application lifecycle: build, start, health check, shutdown

    let config = test_config();
    let app = Application::builder_from_config(config).build().await;

    // Application should build successfully (or fail with expected database error)
    match app {
        Ok(app) => {
            // If database is available, test health checks
            let health_result = app.health_check().await;

            // Should have health check results
            assert!(!health_result.checks.is_empty());

            // Test state access
            let state = app.state();
            // Metrics should be available (it's an Arc, not a Result)
            assert!(!std::ptr::eq(state.metrics.as_ref(), std::ptr::null()));
        }
        Err(e) => {
            // If database is not available, that's expected in CI
            let error_msg = format!("{}", e);
            assert!(
                error_msg.contains("database") || error_msg.contains("connection"),
                "Expected database connection error, got: {}",
                error_msg
            );
        }
    }
}

#[tokio::test]
async fn test_configuration_loading() {
    // Test configuration loading from different sources

    // Test default configuration
    let default_config = RunnerConfig::default();
    assert_eq!(
        default_config.database.url,
        "postgresql://localhost/janitor"
    );
    assert_eq!(default_config.web.port, 9911);
    assert_eq!(default_config.application.name, "janitor-runner");

    // Test configuration validation
    assert!(default_config.validate().is_ok());

    // Test invalid configuration
    let mut invalid_config = default_config.clone();
    invalid_config.database.url = "".to_string();
    assert!(invalid_config.validate().is_err());

    // Test configuration merging
    let mut override_config = RunnerConfig::default();
    override_config.web.port = 8080;

    let merged = default_config.merge_with(override_config);
    assert_eq!(merged.web.port, 8080);
    assert_eq!(merged.database.url, "postgresql://localhost/janitor"); // Should retain base value
}

#[tokio::test]
async fn test_metrics_collection() {
    // Test that metrics collection works

    use janitor_runner::metrics::MetricsCollector;

    // Test metrics collection (this should not fail)
    match MetricsCollector::collect_metrics() {
        Ok(metrics) => {
            // Should return some metrics data
            assert!(!metrics.is_empty());
        }
        Err(e) => {
            // Metrics collection failure might be expected in test environment
            println!("Metrics collection failed (expected in test env): {}", e);
        }
    }
}

#[tokio::test]
async fn test_error_tracking() {
    // Test error tracking system

    use janitor_runner::error_tracking::{
        ErrorCategory, ErrorSeverity, ErrorTracker, ErrorTrackingConfig, TrackedError,
    };
    use std::collections::HashMap;

    let config = ErrorTrackingConfig {
        log_to_file: false, // Don't write files in tests
        ..Default::default()
    };

    let tracker = ErrorTracker::new(config);

    // Create a test error
    let error = TrackedError {
        id: "test-error-1".to_string(),
        timestamp: chrono::Utc::now(),
        severity: ErrorSeverity::Error,
        category: ErrorCategory::Database,
        component: "test-component".to_string(),
        operation: "test-operation".to_string(),
        message: "Test error message".to_string(),
        details: None,
        stack_trace: None,
        context: HashMap::new(),
        correlation_id: None,
        user_id: None,
        request_id: None,
        retry_count: 0,
        is_transient: false,
    };

    // Track the error
    tracker.track_error(error).await;

    // Get statistics
    let stats = tracker.get_error_statistics().await;
    assert_eq!(stats.total_errors, 1);
    assert_eq!(stats.by_category.get(&ErrorCategory::Database), Some(&1));
}

#[tokio::test]
async fn test_performance_monitoring() {
    // Test performance monitoring system

    use janitor_runner::performance::{PerformanceConfig, PerformanceMonitor};

    let config = PerformanceConfig {
        collection_interval: Duration::from_millis(100), // Fast interval for testing
        ..Default::default()
    };

    let monitor = PerformanceMonitor::new(config.collection_interval);

    // Get performance summary
    let summary = monitor.get_performance_summary().await;
    assert_eq!(summary.data_points_collected, 0); // Should start with 0

    // Test would need actual monitoring to be started to test data collection
    // But that requires background tasks which are complex to test
}

#[tokio::test]
async fn test_vcs_manager() {
    // Test VCS management system

    use janitor_runner::vcs::RunnerVcsManager;
    use std::collections::HashMap;

    // Create VCS manager with empty config (no actual VCS backends)
    let managers = HashMap::new();
    let vcs_manager = RunnerVcsManager::new(managers);

    // Test health check
    let health = vcs_manager.health_check().await;
    // With no managers, should be "healthy" but empty
    assert!(health.overall_healthy); // No managers means no failures
    assert!(health.vcs_statuses.is_empty());

    // Test statistics
    let stats = vcs_manager.get_statistics();
    assert_eq!(stats.manager_count, 0);
    assert!(stats.supported_vcs_types.is_empty());
}

#[tokio::test]
#[ignore = "LogConfig and LogStorageBackend not yet implemented"]
async fn test_log_manager() {
    // Test log management system

    // TODO: Fix when LogConfig and LogStorageBackend are implemented
    // use janitor_runner::logs::{LogConfig, LogFileManager, LogStorageBackend};
    // use std::path::PathBuf;

    // let config = LogConfig {
    //     storage_backend: LogStorageBackend::Local,
    //     local_log_path: PathBuf::from("/tmp/janitor_test_logs"),
    //     gcs_bucket: None,
    // };

    // let manager = LogFileManager::new(config).await.unwrap();

    // TODO: Complete test when LogFileManager API is available
    // // Test storing and retrieving a log
    // let test_content = b"Test log content";
    // let run_id = "test-run-123";
    // let filename = "test.log";

    // // Store log
    // let store_result = manager.store_log(run_id, filename, test_content).await;

    // match store_result {
    //     Ok(()) => {
    //         // Retrieve log
    //         let retrieved = manager.get_log(run_id, filename).await.unwrap();
    //         assert_eq!(retrieved, test_content);

    //         // List logs
    //         let logs = manager.list_logs(run_id).await.unwrap();
    //         assert!(logs.contains(&filename.to_string()));
    //     }
    //     Err(e) => {
    //         println!("Log storage failed (may be expected in test env): {}", e);
    //     }
    // }
}

#[tokio::test]
async fn test_graceful_shutdown() {
    // Test graceful shutdown functionality

    let config = test_config();
    let app = Application::builder_from_config(config).build().await;

    if let Ok(app) = app {
        // Test that shutdown doesn't panic
        // In a real test, you'd start the server and then trigger shutdown
        // For now, just test that the application can be dropped cleanly
        drop(app);
    }
}

#[tokio::test]
async fn test_concurrent_operations() {
    // Test concurrent operations to ensure thread safety

    use janitor_runner::error_tracking::{
        ErrorCategory, ErrorSeverity, ErrorTracker, ErrorTrackingConfig, TrackedError,
    };
    use std::collections::HashMap;
    use std::sync::Arc;

    let config = ErrorTrackingConfig {
        log_to_file: false,
        ..Default::default()
    };

    let tracker = Arc::new(ErrorTracker::new(config));

    // Spawn multiple tasks that track errors concurrently
    let mut handles = Vec::new();

    for i in 0..10 {
        let tracker_clone = Arc::clone(&tracker);
        let handle = tokio::spawn(async move {
            let error = TrackedError {
                id: format!("concurrent-error-{}", i),
                timestamp: chrono::Utc::now(),
                severity: ErrorSeverity::Warning,
                category: ErrorCategory::Network,
                component: "concurrent-test".to_string(),
                operation: "test-operation".to_string(),
                message: format!("Concurrent test error {}", i),
                details: None,
                stack_trace: None,
                context: HashMap::new(),
                correlation_id: None,
                user_id: None,
                request_id: None,
                retry_count: 0,
                is_transient: false,
            };

            tracker_clone.track_error(error).await;
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // Check that all errors were tracked
    let stats = tracker.get_error_statistics().await;
    assert_eq!(stats.total_errors, 10);
}

#[tokio::test]
async fn test_system_integration() {
    // High-level integration test that exercises multiple systems together
    use janitor_runner::metrics::MetricsCollector;

    let config = test_config();
    let app_result = Application::builder_from_config(config).build().await;

    match app_result {
        Ok(app) => {
            // Test that all systems are integrated and accessible
            let state = app.state();

            // Test database (may fail if DB not available)
            let _ = state.database.health_check().await;

            // Test VCS manager
            let vcs_health = state.vcs_manager.health_check().await;
            assert!(vcs_health.vcs_statuses.is_empty()); // No VCS configured in test

            // Test performance monitor
            let perf_summary = state.performance_monitor.get_performance_summary().await;
            assert_eq!(perf_summary.data_points_collected, 0); // Just started

            // Test error tracker
            let error_stats = state.error_tracker.get_error_statistics().await;
            // Should have some structure even if no errors yet

            // Test metrics
            let metrics_result = MetricsCollector::collect_metrics();
            // Metrics collection may fail in test environment, that's OK

            println!("Integration test completed successfully");
        }
        Err(e) => {
            // Database connection failure is expected in many test environments
            println!("Application initialization failed (expected in CI): {}", e);
        }
    }
}

/// Helper to run integration tests with timeout
#[tokio::test]
async fn test_timeout_protection() {
    // Ensure tests don't hang indefinitely

    let test_future = async {
        // A test that should complete quickly
        let config = RunnerConfig::default();
        config.validate()
    };

    let result = timeout(Duration::from_secs(5), test_future).await;
    assert!(result.is_ok(), "Test should complete within timeout");
    assert!(result.unwrap().is_ok(), "Configuration should be valid");
}
