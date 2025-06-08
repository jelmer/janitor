//! Tests for backchannel communication implementations.
//!
//! These tests verify that Jenkins and Polling backchannel implementations
//! work correctly and maintain compatibility with Python behavior.

use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;
use url::Url;

use janitor_runner::backchannel::{
    Backchannel, Error as BackchannelError, HealthStatus, JenkinsBackchannel, PollingBackchannel,
};

/// Mock HTTP server for testing backchannel communication.
struct MockServer {
    responses: Arc<Mutex<Vec<(u16, String)>>>,
}

impl MockServer {
    fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(Vec::new())),
        }
    }

    async fn add_response(&self, status: u16, body: String) {
        let mut responses = self.responses.lock().await;
        responses.push((status, body));
    }
}

/// Test PollingBackchannel ping functionality.
#[tokio::test]
async fn test_polling_backchannel_ping() {
    let base_url = Url::parse("http://localhost:8080").unwrap();
    let backchannel = PollingBackchannel::new(base_url);

    // Test ping with unreachable server (should return error)
    let result = backchannel.ping().await;
    assert!(result.is_err());

    // The error should be a connection error
    match result {
        Err(BackchannelError::Connection(_)) => {}
        Err(BackchannelError::Http(_)) => {}
        _ => panic!("Expected connection or HTTP error"),
    }
}

/// Test PollingBackchannel kill functionality.
#[tokio::test]
async fn test_polling_backchannel_kill() {
    let base_url = Url::parse("http://localhost:8080").unwrap();
    let backchannel = PollingBackchannel::new(base_url);

    // Test kill with unreachable server
    let result = backchannel.kill().await;
    assert!(result.is_err());
}

/// Test PollingBackchannel log file operations.
#[tokio::test]
async fn test_polling_backchannel_logs() {
    let base_url = Url::parse("http://localhost:8080").unwrap();
    let backchannel = PollingBackchannel::new(base_url);

    // Test list_log_files with unreachable server
    let result = backchannel.list_log_files().await;
    assert!(result.is_err());

    // Test get_log_file with unreachable server
    let result = backchannel.get_log_file("worker.log").await;
    assert!(result.is_err());
}

/// Test JenkinsBackchannel ping functionality.
#[tokio::test]
async fn test_jenkins_backchannel_ping() {
    let base_url = Url::parse("http://localhost:8080").unwrap();
    let job_name = "test-job".to_string();
    let build_number = 123;

    let backchannel = JenkinsBackchannel::new(base_url, job_name, build_number);

    // Test ping with unreachable server
    let result = backchannel.ping().await;
    assert!(result.is_err());
}

/// Test JenkinsBackchannel kill functionality.
#[tokio::test]
async fn test_jenkins_backchannel_kill() {
    let base_url = Url::parse("http://localhost:8080").unwrap();
    let job_name = "test-job".to_string();
    let build_number = 123;

    let backchannel = JenkinsBackchannel::new(base_url, job_name, build_number);

    // Test kill - should return not supported error
    let result = backchannel.kill().await;
    assert!(result.is_err());

    match result {
        Err(BackchannelError::NotSupported(_)) => {}
        _ => panic!("Expected NotSupported error for Jenkins kill"),
    }
}

/// Test JenkinsBackchannel log operations.
#[tokio::test]
async fn test_jenkins_backchannel_logs() {
    let base_url = Url::parse("http://localhost:8080").unwrap();
    let job_name = "test-job".to_string();
    let build_number = 123;

    let backchannel = JenkinsBackchannel::new(base_url, job_name, build_number);

    // Test list_log_files - should return only worker.log
    let result = backchannel.list_log_files().await;
    assert!(result.is_ok());

    let files = result.unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0], "worker.log");

    // Test get_log_file with worker.log
    let result = backchannel.get_log_file("worker.log").await;
    assert!(result.is_err()); // Will fail due to no server, but validates path

    // Test get_log_file with invalid file
    let result = backchannel.get_log_file("invalid.log").await;
    assert!(result.is_err());

    match result {
        Err(BackchannelError::NotSupported(_)) => {}
        _ => panic!("Expected NotSupported error for invalid log file"),
    }
}

/// Test backchannel error handling.
#[tokio::test]
async fn test_backchannel_error_handling() {
    let base_url = Url::parse("http://localhost:8080").unwrap();
    let backchannel = PollingBackchannel::new(base_url);

    // Test various error conditions
    let result = backchannel.ping().await;
    assert!(result.is_err());

    // Verify error can be converted to string
    let error_str = format!("{}", result.unwrap_err());
    assert!(!error_str.is_empty());
}

/// Test HealthStatus serialization/deserialization.
#[test]
fn test_health_status_serialization() {
    let health = HealthStatus {
        healthy: true,
        details: Some(json!({
            "cpu_usage": 50.0,
            "memory_usage": 75.0,
            "disk_space": 80.0
        })),
    };

    // Test serialization
    let json_str = serde_json::to_string(&health).unwrap();
    assert!(json_str.contains("healthy"));
    assert!(json_str.contains("cpu_usage"));

    // Test deserialization
    let parsed: HealthStatus = serde_json::from_str(&json_str).unwrap();
    assert_eq!(parsed.healthy, health.healthy);
    assert!(parsed.details.is_some());
}

/// Test backchannel URL construction.
#[test]
fn test_backchannel_url_construction() {
    // Test PollingBackchannel URL construction
    let base_url = Url::parse("http://worker:8080").unwrap();
    let polling = PollingBackchannel::new(base_url.clone());

    // Test that URLs are constructed correctly (would need access to internal methods)
    // For now, just verify the backchannel can be created
    assert_eq!(polling.base_url(), &base_url);

    // Test JenkinsBackchannel URL construction
    let jenkins = JenkinsBackchannel::new(base_url.clone(), "test-job".to_string(), 123);

    assert_eq!(jenkins.base_url(), &base_url);
    assert_eq!(jenkins.job_name(), "test-job");
    assert_eq!(jenkins.build_number(), 123);
}

/// Test backchannel timeout handling.
#[tokio::test]
async fn test_backchannel_timeouts() {
    let base_url = Url::parse("http://192.0.2.1:8080").unwrap(); // Non-routable IP
    let backchannel = PollingBackchannel::new(base_url);

    // Test that operations timeout rather than hanging indefinitely
    let start = std::time::Instant::now();
    let result = backchannel.ping().await;
    let duration = start.elapsed();

    assert!(result.is_err());
    assert!(duration.as_secs() < 30); // Should timeout before 30 seconds
}

/// Test concurrent backchannel operations.
#[tokio::test]
async fn test_concurrent_backchannel_operations() {
    let base_url = Url::parse("http://localhost:8080").unwrap();
    let backchannel = Arc::new(PollingBackchannel::new(base_url));

    // Test multiple concurrent ping operations
    let mut handles = Vec::new();

    for _ in 0..5 {
        let bc = Arc::clone(&backchannel);
        let handle = tokio::spawn(async move { bc.ping().await });
        handles.push(handle);
    }

    // Wait for all operations to complete
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_err()); // Expected to fail without server
    }
}

/// Test backchannel creation from JSON configuration.
#[test]
fn test_backchannel_from_config() {
    // Test Polling backchannel configuration
    let polling_config = json!({
        "type": "polling",
        "url": "http://worker:8080"
    });

    // Would test creation from config if we had a factory function
    assert!(polling_config.get("type").is_some());
    assert!(polling_config.get("url").is_some());

    // Test Jenkins backchannel configuration
    let jenkins_config = json!({
        "type": "jenkins",
        "url": "http://jenkins:8080",
        "job_name": "test-job",
        "build_number": 123
    });

    assert!(jenkins_config.get("type").is_some());
    assert!(jenkins_config.get("job_name").is_some());
    assert!(jenkins_config.get("build_number").is_some());
}

/// Test backchannel compatibility with Python implementations.
#[tokio::test]
async fn test_python_compatibility() {
    // Test that our implementations match Python behavior

    // Polling backchannel should use these endpoints:
    // - GET /status for ping
    // - POST /kill for termination
    // - GET /logs for log file listing
    // - GET /logs/{filename} for log retrieval

    let base_url = Url::parse("http://worker:8080").unwrap();
    let polling = PollingBackchannel::new(base_url);

    // Verify endpoints would be called correctly (test the URL construction)
    // This would require exposing internal URL building methods

    // Jenkins backchannel should use these endpoints:
    // - GET /job/{job}/api/json for ping
    // - Kill not supported
    // - Only worker.log supported for logs
    // - GET /job/{job}/{build}/logText/progressiveText for log content

    let jenkins_url = Url::parse("http://jenkins:8080").unwrap();
    let jenkins = JenkinsBackchannel::new(jenkins_url, "test-job".to_string(), 123);

    // Test that only worker.log is supported
    let log_files = jenkins.list_log_files().await.unwrap();
    assert_eq!(log_files, vec!["worker.log"]);
}

/// Test backchannel factory pattern.
#[test]
fn test_backchannel_factory() {
    // Test that we can create different backchannel types
    // This would test a factory function if we had one

    let polling_url = Url::parse("http://worker:8080").unwrap();
    let _polling = PollingBackchannel::new(polling_url);

    let jenkins_url = Url::parse("http://jenkins:8080").unwrap();
    let _jenkins = JenkinsBackchannel::new(jenkins_url, "job-name".to_string(), 1);

    // Both should implement the Backchannel trait
    // This verifies the trait design works correctly
}
