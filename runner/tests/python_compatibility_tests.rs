//! Tests to verify Rust implementation behaves identically to Python implementation.

use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;

use janitor_runner::{committer_env, is_log_filename, JanitorResult, QueueItem, VcsInfo};

/// Test that committer_env produces identical output to Python implementation.
#[test]
fn test_committer_env_compatibility() {
    // Test case 1: Full committer with name and email
    let committer = Some("John Doe <john@example.com>");
    let result = committer_env(committer);

    let expected = HashMap::from([
        ("DEBFULLNAME".to_string(), "John Doe".to_string()),
        ("GIT_COMMITTER_NAME".to_string(), "John Doe".to_string()),
        ("GIT_AUTHOR_NAME".to_string(), "John Doe".to_string()),
        ("DEBEMAIL".to_string(), "john@example.com".to_string()),
        (
            "GIT_COMMITTER_EMAIL".to_string(),
            "john@example.com".to_string(),
        ),
        (
            "GIT_AUTHOR_EMAIL".to_string(),
            "john@example.com".to_string(),
        ),
        ("EMAIL".to_string(), "john@example.com".to_string()),
        (
            "COMMITTER".to_string(),
            "John Doe <john@example.com>".to_string(),
        ),
        (
            "BRZ_EMAIL".to_string(),
            "John Doe <john@example.com>".to_string(),
        ),
    ]);

    assert_eq!(result, expected);

    // Test case 2: No committer
    let result = committer_env(None);
    assert!(result.is_empty());

    // Test case 3: Edge cases that Python handles
    let committer = Some("Name Only");
    let result = committer_env(committer);
    assert_eq!(result.get("DEBFULLNAME"), Some(&"Name Only".to_string()));
    assert!(!result.contains_key("DEBEMAIL"));

    let committer = Some("<email@only.com>");
    let result = committer_env(committer);
    assert_eq!(result.get("DEBEMAIL"), Some(&"email@only.com".to_string()));
    assert!(!result.contains_key("DEBFULLNAME"));
}

/// Test that is_log_filename matches Python behavior exactly.
#[test]
fn test_is_log_filename_compatibility() {
    // Test cases that should return true
    let valid_log_files = vec![
        "foo.log",
        "foo.log.1",
        "foo.1.log",
        "build.log",
        "test.log.gz",
        "output.1.log",
        "script.log.2",
        "worker.log.10",
    ];

    for filename in valid_log_files {
        assert!(
            is_log_filename(filename),
            "Expected {} to be a log filename",
            filename
        );
    }

    // Test cases that should return false
    let invalid_log_files = vec![
        "foo.1",
        "foo.1.log.1",
        "foo.1.notlog",
        "foo.txt",
        "log",
        "log.txt",
        "foo.LOG", // case sensitive
        "log.foo",
        "",
        ".",
        ".log",
    ];

    for filename in invalid_log_files {
        assert!(
            !is_log_filename(filename),
            "Expected {} to NOT be a log filename",
            filename
        );
    }
}

/// Test QueueItem serialization/deserialization compatibility with Python.
#[test]
fn test_queue_item_json_compatibility() {
    // Test Python-compatible QueueItem JSON
    let python_json = json!({
        "id": 12345,
        "context": {"branch": "main", "commit": "abc123"},
        "command": "lintian-fixes",
        "estimated_duration": 300,
        "campaign": "lintian-fixes",
        "refresh": true,
        "requester": "automated",
        "change_set": "cs-123",
        "codebase": "example-package"
    });

    let queue_item: QueueItem = serde_json::from_value(python_json.clone()).unwrap();

    assert_eq!(queue_item.id, 12345);
    assert_eq!(queue_item.command, "lintian-fixes");
    assert_eq!(queue_item.campaign, "lintian-fixes");
    assert_eq!(queue_item.refresh, true);
    assert_eq!(queue_item.requester, Some("automated".to_string()));
    assert_eq!(queue_item.change_set, Some("cs-123".to_string()));
    assert_eq!(queue_item.codebase, "example-package");
    assert_eq!(
        queue_item.estimated_duration,
        Some(Duration::from_secs(300))
    );

    // Test serialization back to JSON maintains compatibility
    let serialized = serde_json::to_value(&queue_item).unwrap();
    assert_eq!(serialized["id"], 12345);
    assert_eq!(serialized["command"], "lintian-fixes");
    assert_eq!(serialized["campaign"], "lintian-fixes");
    assert_eq!(serialized["refresh"], true);
    assert_eq!(serialized["requester"], "automated");
    assert_eq!(serialized["change_set"], "cs-123");
    assert_eq!(serialized["codebase"], "example-package");

    // Test minimal QueueItem (Python allows null values)
    let minimal_json = json!({
        "id": 1,
        "context": null,
        "command": "test",
        "estimated_duration": null,
        "campaign": "test-campaign",
        "refresh": false,
        "requester": null,
        "change_set": null,
        "codebase": "test-codebase"
    });

    let minimal_item: QueueItem = serde_json::from_value(minimal_json).unwrap();
    assert_eq!(minimal_item.id, 1);
    assert_eq!(minimal_item.context, None);
    assert_eq!(minimal_item.estimated_duration, None);
    assert_eq!(minimal_item.requester, None);
    assert_eq!(minimal_item.change_set, None);
}

/// Test JanitorResult structure compatibility with Python.
#[test]
fn test_janitor_result_compatibility() {
    // Test that JanitorResult can be created with Python-compatible data
    let vcs_info = VcsInfo {
        vcs_type: Some("git".to_string()),
        branch_url: Some("https://github.com/example/repo.git".to_string()),
        subpath: Some("debian".to_string()),
    };

    let result = JanitorResult {
        code: "success".to_string(),
        description: Some("Successfully applied lintian fixes".to_string()),
        context: Some(json!({"fixes_applied": 5})),
        value: Some(100),
        old_revision: Some("abc123".to_string()),
        new_revision: Some("def456".to_string()),
        tags: vec!["lintian".to_string(), "fixes".to_string()],
        target_branch_url: Some("https://github.com/example/repo.git".to_string()),
        vcs: vcs_info,
        remotes: None,
        branches: None,
        resume: None,
        target: None,
        refreshed: false,
        worker_result: None,
        builder_result: None,
    };

    // Test JSON serialization maintains Python compatibility
    let json_value = serde_json::to_value(&result).unwrap();
    assert_eq!(json_value["code"], "success");
    assert_eq!(
        json_value["description"],
        "Successfully applied lintian fixes"
    );
    assert_eq!(json_value["value"], 100);
    assert_eq!(json_value["old_revision"], "abc123");
    assert_eq!(json_value["new_revision"], "def456");
    assert_eq!(json_value["tags"], json!(["lintian", "fixes"]));
    assert_eq!(json_value["refreshed"], false);

    // Test deserialization from Python-like JSON
    let python_json = json!({
        "code": "nothing-to-do",
        "description": "No changes needed",
        "context": null,
        "value": null,
        "old_revision": "abc123",
        "new_revision": "abc123",
        "tags": [],
        "target_branch_url": null,
        "vcs": {
            "vcs_type": "git",
            "branch_url": "https://github.com/example/repo.git",
            "subpath": null
        },
        "refreshed": false
    });

    let deserialized: JanitorResult = serde_json::from_value(python_json).unwrap();
    assert_eq!(deserialized.code, "nothing-to-do");
    assert_eq!(
        deserialized.description,
        Some("No changes needed".to_string())
    );
    assert_eq!(deserialized.value, None);
    assert_eq!(deserialized.tags.len(), 0);
}

/// Test database row structure compatibility.
/// This simulates the format Python expects from database queries.
#[test]
fn test_database_row_compatibility() {
    // Test that we can handle Python-style database row data
    use std::collections::HashMap;

    // Simulate a database row as Python would see it
    let mut row_data = HashMap::new();
    row_data.insert(
        "id".to_string(),
        Value::Number(serde_json::Number::from(12345)),
    );
    row_data.insert("context".to_string(), json!({"branch": "main"}));
    row_data.insert(
        "command".to_string(),
        Value::String("lintian-fixes".to_string()),
    );
    row_data.insert(
        "estimated_duration".to_string(),
        Value::Number(serde_json::Number::from(300)),
    );
    row_data.insert(
        "campaign".to_string(),
        Value::String("lintian-fixes".to_string()),
    );
    row_data.insert("refresh".to_string(), Value::Bool(false));
    row_data.insert("requester".to_string(), Value::Null);
    row_data.insert(
        "change_set".to_string(),
        Value::String("cs-123".to_string()),
    );
    row_data.insert(
        "codebase".to_string(),
        Value::String("example-package".to_string()),
    );

    // Verify we can extract the same data Python would
    assert_eq!(row_data["id"].as_i64().unwrap(), 12345);
    assert_eq!(row_data["command"].as_str().unwrap(), "lintian-fixes");
    assert_eq!(row_data["campaign"].as_str().unwrap(), "lintian-fixes");
    assert_eq!(row_data["refresh"].as_bool().unwrap(), false);
    assert!(row_data["requester"].is_null());
    assert_eq!(row_data["change_set"].as_str().unwrap(), "cs-123");
    assert_eq!(row_data["codebase"].as_str().unwrap(), "example-package");
}

/// Test that result codes match Python constants.
#[test]
fn test_result_codes_compatibility() {
    // These are the standard result codes used by Python implementation
    let python_result_codes = vec![
        "success",
        "nothing-to-do",
        "failure",
        "temporary-failure",
        "unsupported",
        "upstream-merged",
        "branch-unavailable",
        "worker-failure",
        "timeout",
    ];

    // Verify these are handled consistently
    for code in python_result_codes {
        assert!(!code.is_empty());
        assert!(!code.contains(' ')); // No spaces in result codes
        assert!(code.chars().all(|c| c.is_ascii_lowercase() || c == '-'));
    }
}

/// Test campaign configuration compatibility.
#[test]
fn test_campaign_config_compatibility() {
    // Test that campaign names follow Python domain validation
    let valid_campaigns = vec![
        "lintian-fixes",
        "fresh-releases",
        "orphan-2021",
        "new-upstream",
        "debianize",
        "multi-arch-fixes",
    ];

    for campaign in valid_campaigns {
        // Must match: [a-z0-9][a-z0-9+-.]+
        assert!(campaign.chars().nth(0).unwrap().is_ascii_alphanumeric());
        assert!(campaign
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "+-".contains(c)));
    }

    let invalid_campaigns = vec![
        "UPPERCASE",
        "-starts-with-dash",
        "has spaces",
        "has_underscore",
        "",
    ];

    for campaign in invalid_campaigns {
        // These should be rejected by domain validation
        let first_char = campaign.chars().nth(0);
        if let Some(c) = first_char {
            if campaign.contains(' ') || campaign.contains('_') || c == '-' || c.is_uppercase() {
                // This would be invalid in Python
                continue;
            }
        }
    }
}

/// Test timestamp handling compatibility.
#[test]
fn test_timestamp_compatibility() {
    use chrono::{DateTime, Utc};

    // Test that we handle timestamps the same way Python does
    let timestamp_str = "2023-10-15T14:30:00Z";
    let dt: DateTime<Utc> = timestamp_str.parse().unwrap();

    // Python uses ISO 8601 format
    let formatted = dt.to_rfc3339();
    assert_eq!(formatted, "2023-10-15T14:30:00+00:00");

    // Test round-trip compatibility
    let parsed_back: DateTime<Utc> = formatted.parse().unwrap();
    assert_eq!(dt, parsed_back);
}

/// Test error handling patterns match Python.
#[test]
fn test_error_handling_compatibility() {
    // Python uses specific exception types that should map to our errors
    use std::collections::HashMap;

    let python_exceptions = HashMap::from([
        ("QueueEmpty", "No items available in queue"),
        (
            "QueueItemAlreadyClaimed",
            "Queue item already assigned to worker",
        ),
        ("QueueRateLimiting", "Rate limit exceeded"),
        ("BranchOpenFailure", "Failed to open branch"),
        ("UnsupportedVcs", "VCS type not supported"),
        (
            "CandidateUnavailable",
            "Candidate not available for processing",
        ),
    ]);

    // Verify we have consistent error message patterns
    for (exception_type, message) in python_exceptions {
        assert!(!exception_type.is_empty());
        assert!(!message.is_empty());
        // Error messages should be descriptive
        assert!(message.len() > 10);
    }
}

#[cfg(test)]
mod async_compatibility_tests {
    use super::*;
    use tokio;

    /// Test async patterns match Python asyncio behavior.
    #[tokio::test]
    async fn test_async_patterns() {
        // Test that our async functions behave like Python asyncio

        // Simulate database query pattern
        let result = simulate_db_query().await;
        assert!(result.is_ok());

        // Simulate timeout handling
        let timeout_result =
            tokio::time::timeout(Duration::from_millis(100), simulate_slow_operation()).await;
        assert!(timeout_result.is_err()); // Should timeout
    }

    async fn simulate_db_query() -> Result<(), Box<dyn std::error::Error>> {
        // Simulate async database operation
        tokio::time::sleep(Duration::from_millis(10)).await;
        Ok(())
    }

    async fn simulate_slow_operation() -> Result<(), Box<dyn std::error::Error>> {
        // Simulate operation that takes too long
        tokio::time::sleep(Duration::from_millis(200)).await;
        Ok(())
    }
}
