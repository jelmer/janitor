//! Unit tests for core runner functionality.
//!
//! These tests verify that individual components work correctly
//! and maintain compatibility with Python behavior.

use chrono::{DateTime, Duration, Utc};
use serde_json::json;
use std::collections::HashMap;
use uuid::Uuid;

use janitor_runner::{
    backchannel::{Backchannel, HealthStatus, JenkinsBackchannel, PollingBackchannel},
    builder::{get_builder, CampaignConfig, DebianBuildConfig, GenericBuildConfig},
    committer_env,
    watchdog::{TerminationReason, Watchdog, WatchdogConfig},
    ActiveRun, JanitorResult, QueueAssignment, WorkerResult,
};

/// Test committer_env function compatibility with Python version.
#[test]
fn test_committer_env_compatibility() {
    // Test with full committer string
    let env = committer_env(Some("John Doe <john@example.com>"));

    assert_eq!(env.get("DEBFULLNAME"), Some(&"John Doe".to_string()));
    assert_eq!(env.get("DEBEMAIL"), Some(&"john@example.com".to_string()));
    assert_eq!(env.get("GIT_COMMITTER_NAME"), Some(&"John Doe".to_string()));
    assert_eq!(
        env.get("GIT_COMMITTER_EMAIL"),
        Some(&"john@example.com".to_string())
    );
    assert_eq!(env.get("GIT_AUTHOR_NAME"), Some(&"John Doe".to_string()));
    assert_eq!(
        env.get("GIT_AUTHOR_EMAIL"),
        Some(&"john@example.com".to_string())
    );
    assert_eq!(env.get("EMAIL"), Some(&"john@example.com".to_string()));
    assert_eq!(
        env.get("COMMITTER"),
        Some(&"John Doe <john@example.com>".to_string())
    );
    assert_eq!(
        env.get("BRZ_EMAIL"),
        Some(&"John Doe <john@example.com>".to_string())
    );

    // Test with None
    let env = committer_env(None);
    assert!(env.is_empty());

    // Test with malformed committer
    let env = committer_env(Some("invalid"));
    assert_eq!(env.get("COMMITTER"), Some(&"invalid".to_string()));
    assert_eq!(env.get("BRZ_EMAIL"), Some(&"invalid".to_string()));
}

/// Test JanitorResult serialization/deserialization compatibility.
#[test]
fn test_janitor_result_serialization() {
    let result = JanitorResult {
        log_id: "test-log-123".to_string(),
        branch_url: "https://github.com/test/repo".to_string(),
        subpath: Some("subdir".to_string()),
        code: "success".to_string(),
        transient: Some(false),
        codebase: "test-codebase".to_string(),
        campaign: "test-campaign".to_string(),
        description: Some("Test successful".to_string()),
        worker_result: None,
        logfilenames: vec!["worker.log".to_string()],
        start_time: Some(Utc::now()),
        finish_time: Some(Utc::now()),
        remotes: None,
        target: None,
        queue_id: Some(123),
        builder_result: None,
    };

    // Test serialization
    let json_str = serde_json::to_string(&result).unwrap();
    assert!(json_str.contains("test-log-123"));
    assert!(json_str.contains("success"));

    // Test deserialization
    let parsed: JanitorResult = serde_json::from_str(&json_str).unwrap();
    assert_eq!(parsed.log_id, result.log_id);
    assert_eq!(parsed.code, result.code);
    assert_eq!(parsed.codebase, result.codebase);
}

/// Test WorkerResult compatibility with Python dataclass.
#[test]
fn test_worker_result_compatibility() {
    let worker_result = WorkerResult {
        code: "success".to_string(),
        description: Some("Build successful".to_string()),
        context: Some(json!({"test": true})),
        codemod: Some(json!({"applied": ["fix1", "fix2"]})),
        main_branch_revision: None,
        revision: None,
        value: Some(100),
        branches: Some(vec![(
            Some("main".to_string()),
            Some("feature".to_string()),
            None,
            None,
        )]),
        tags: Some(vec![("v1.0".to_string(), None)]),
        remotes: Some({
            let mut map = HashMap::new();
            map.insert("origin".to_string(), {
                let mut remote = HashMap::new();
                remote.insert("url".to_string(), json!("https://github.com/test/repo"));
                remote
            });
            map
        }),
        details: Some(json!({"duration": 300})),
        stage: Some("build".to_string()),
        builder_result: Some(json!({"artifacts": ["file1", "file2"]})),
        start_time: Some(Utc::now()),
        finish_time: Some(Utc::now()),
        queue_id: Some(456),
    };

    // Test all fields are preserved in serialization
    let json_str = serde_json::to_string(&worker_result).unwrap();
    let parsed: WorkerResult = serde_json::from_str(&json_str).unwrap();

    assert_eq!(parsed.code, worker_result.code);
    assert_eq!(parsed.value, worker_result.value);
    assert_eq!(parsed.stage, worker_result.stage);
    assert_eq!(parsed.queue_id, worker_result.queue_id);
}

/// Test ActiveRun structure matches Python implementation.
#[test]
fn test_active_run_structure() {
    use janitor::queue::VcsInfo;

    let active_run = ActiveRun {
        worker_name: "test-worker".to_string(),
        worker_link: Some("http://worker:8080".to_string()),
        queue_id: 789,
        log_id: "run-log-456".to_string(),
        start_time: Utc::now(),
        finish_time: None,
        estimated_duration: Some(std::time::Duration::from_secs(300)),
        campaign: "test-campaign".to_string(),
        change_set: Some("changeset-123".to_string()),
        command: "test-command".to_string(),
        codebase: "test-codebase".to_string(),
        requester: Some("user@example.com".to_string()),
        refresh: false,
        backchannel: Some(json!({
            "type": "polling",
            "url": "http://worker:8080"
        })),
        vcs_info: VcsInfo {
            branch_url: Some("https://github.com/test/repo".to_string()),
            subpath: Some("src".to_string()),
            vcs_type: Some("git".to_string()),
        },
    };

    // Test serialization includes all Python fields
    let json_str = serde_json::to_string(&active_run).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    assert!(parsed.get("worker_name").is_some());
    assert!(parsed.get("queue_id").is_some());
    assert!(parsed.get("log_id").is_some());
    assert!(parsed.get("start_time").is_some());
    assert!(parsed.get("finish_time").is_some());
    assert!(parsed.get("campaign").is_some());
    assert!(parsed.get("vcs_info").is_some());
}

/// Test backchannel implementations.
#[tokio::test]
async fn test_backchannel_implementations() {
    // Test PollingBackchannel
    let polling = PollingBackchannel::new("http://worker:8080".parse().unwrap());

    // Test ping functionality (will fail in test but should not panic)
    let health = polling.ping().await;
    assert!(health.is_err()); // Expected to fail without real server

    // Test Jenkins backchannel
    let jenkins = JenkinsBackchannel::new(
        "http://jenkins:8080".parse().unwrap(),
        "test-job".to_string(),
        123,
    );

    let health = jenkins.ping().await;
    assert!(health.is_err()); // Expected to fail without real server
}

/// Test builder configuration generation.
#[test]
fn test_builder_configuration() {
    // Test generic builder
    let generic_config = CampaignConfig {
        generic_build: Some(GenericBuildConfig {
            chroot: Some("ubuntu:20.04".to_string()),
        }),
        debian_build: None,
    };

    let builder = get_builder(&generic_config, "dep-server-url");
    assert!(builder.is_ok());

    // Test Debian builder
    let debian_config = CampaignConfig {
        generic_build: None,
        debian_build: Some(DebianBuildConfig {
            distribution: "unstable".to_string(),
            build_suffix: None,
            build_command: None,
            apt_repository: None,
            apt_repository_key: None,
        }),
    };

    let builder = get_builder(&debian_config, "dep-server-url");
    assert!(builder.is_ok());
}

/// Test watchdog functionality.
#[test]
fn test_watchdog_functionality() {
    let config = WatchdogConfig {
        check_interval: std::time::Duration::from_secs(30),
        max_failures: 3,
        timeout_multiplier: 2.0,
        enable_termination: true,
    };

    let watchdog = Watchdog::new(config);

    // Test creating failure details
    let active_run = ActiveRun {
        worker_name: "test-worker".to_string(),
        worker_link: None,
        queue_id: 123,
        log_id: "test-log".to_string(),
        start_time: Utc::now(),
        finish_time: None,
        estimated_duration: Some(std::time::Duration::from_secs(300)),
        campaign: "test".to_string(),
        change_set: None,
        command: "test".to_string(),
        codebase: "test".to_string(),
        requester: None,
        refresh: false,
        backchannel: None,
        vcs_info: janitor::queue::VcsInfo {
            branch_url: None,
            subpath: None,
            vcs_type: None,
        },
    };

    let failure_details = watchdog.create_failure_details(&active_run);
    assert!(failure_details.get("worker_name").is_some());
    assert!(failure_details.get("log_id").is_some());
    assert!(failure_details.get("termination_reason").is_some());
}

/// Test error handling and validation.
#[test]
fn test_error_handling() {
    // Test invalid JSON parsing
    let invalid_json = r#"{"invalid": json"#;
    let result: Result<WorkerResult, _> = serde_json::from_str(invalid_json);
    assert!(result.is_err());

    // Test missing required fields
    let incomplete_json = r#"{"code": "success"}"#;
    let result: Result<WorkerResult, _> = serde_json::from_str(incomplete_json);
    assert!(result.is_err());
}

/// Test URL and network utilities.
#[test]
fn test_url_utilities() {
    use url::Url;

    // Test URL parsing for backchannel
    let valid_url = "http://worker:8080/status";
    let parsed = Url::parse(valid_url);
    assert!(parsed.is_ok());

    let invalid_url = "not-a-url";
    let parsed = Url::parse(invalid_url);
    assert!(parsed.is_err());
}

/// Test date/time handling compatibility.
#[test]
fn test_datetime_handling() {
    let now = Utc::now();

    // Test serialization to ISO 8601 (Python compatible)
    let json_str = serde_json::to_string(&now).unwrap();
    assert!(json_str.contains("T"));
    assert!(json_str.contains("Z"));

    // Test parsing from Python format
    let python_format = r#""2023-01-01T12:00:00Z""#;
    let parsed: DateTime<Utc> = serde_json::from_str(python_format).unwrap();
    assert_eq!(parsed.year(), 2023);
}

/// Test configuration validation.
#[test]
fn test_configuration_validation() {
    // Test valid campaign configuration
    let valid_config = CampaignConfig {
        generic_build: Some(GenericBuildConfig {
            chroot: Some("ubuntu:20.04".to_string()),
        }),
        debian_build: None,
    };

    // Should be able to get builder without errors
    let builder = get_builder(&valid_config, "http://dep-server");
    assert!(builder.is_ok());

    // Test empty configuration
    let empty_config = CampaignConfig {
        generic_build: None,
        debian_build: None,
    };

    let builder = get_builder(&empty_config, "http://dep-server");
    assert!(builder.is_err());
}
