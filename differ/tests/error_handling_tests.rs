//! Integration tests for error handling in the differ service.

use axum::http::StatusCode;
use axum::response::Response;
use janitor_differ::{DifferError, DifferResult};
use serde_json::Value;

#[tokio::test]
async fn test_error_status_codes() {
    // Test that our error types map to the correct HTTP status codes
    let test_cases = vec![
        (
            DifferError::RunNotFound { run_id: "test123".to_string() },
            StatusCode::NOT_FOUND,
        ),
        (
            DifferError::ArtifactsMissing { run_id: "test123".to_string() },
            StatusCode::NOT_FOUND,
        ),
        (
            DifferError::ContentNegotiationFailed {
                available: vec!["text/html".to_string()],
                requested: "application/json".to_string(),
            },
            StatusCode::NOT_ACCEPTABLE,
        ),
        (
            DifferError::DiffCommandTimeout {
                command: "diffoscope".to_string(),
                timeout: 60,
            },
            StatusCode::GATEWAY_TIMEOUT,
        ),
        (
            DifferError::DiffCommandMemoryError {
                command: "diffoscope".to_string(),
                limit_mb: 1500,
            },
            StatusCode::INSUFFICIENT_STORAGE,
        ),
        (
            DifferError::RunNotSuccessful {
                run_id: "test123".to_string(),
                status: "failed".to_string(),
            },
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
    ];

    for (error, expected_status) in test_cases {
        assert_eq!(error.status_code(), expected_status, "Error: {:?}", error);
    }
}

#[tokio::test]
async fn test_error_responses() {
    // Test that error responses contain the expected structure and data
    let error = DifferError::ArtifactsMissing { 
        run_id: "test123".to_string() 
    };
    
    let response = error.to_response();
    assert_eq!(response.error, "Artifacts missing");
    assert_eq!(response.run_id, Some("test123".to_string()));
    assert!(response.details.is_some());
    assert!(response.details.unwrap().contains("test123"));
}

#[tokio::test]
async fn test_error_additional_headers() {
    // Test that errors include appropriate additional headers
    let error = DifferError::ArtifactsMissing { 
        run_id: "test123".to_string() 
    };
    
    let headers = error.additional_headers();
    assert_eq!(headers.len(), 1);
    assert_eq!(headers[0].0, "unavailable_run_id");
    assert_eq!(headers[0].1, "test123");
}

#[tokio::test] 
async fn test_error_into_response() {
    // Test that errors can be converted to HTTP responses
    let error = DifferError::RunNotFound { 
        run_id: "nonexistent".to_string() 
    };
    
    let response: Response = error.into_response();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    
    // Check that headers are included
    let headers = response.headers();
    assert!(headers.contains_key("unavailable_run_id"));
}

#[test]
fn test_content_negotiation_error() {
    let error = DifferError::ContentNegotiationFailed {
        available: vec!["text/html".to_string(), "application/json".to_string()],
        requested: "text/plain".to_string(),
    };
    
    assert_eq!(error.status_code(), StatusCode::NOT_ACCEPTABLE);
    
    let response = error.to_response();
    assert_eq!(response.error, "Content negotiation failed");
    assert!(response.details.is_some());
    assert!(response.details.unwrap().contains("text/plain"));
}

#[test]
fn test_diff_command_errors() {
    // Test timeout error
    let timeout_error = DifferError::DiffCommandTimeout {
        command: "diffoscope".to_string(),
        timeout: 300,
    };
    
    assert_eq!(timeout_error.status_code(), StatusCode::GATEWAY_TIMEOUT);
    let response = timeout_error.to_response();
    assert_eq!(response.error, "Command timeout");
    assert_eq!(response.command, Some("diffoscope".to_string()));
    
    // Test memory error
    let memory_error = DifferError::DiffCommandMemoryError {
        command: "diffoscope".to_string(),
        limit_mb: 1500,
    };
    
    assert_eq!(memory_error.status_code(), StatusCode::INSUFFICIENT_STORAGE);
    let response = memory_error.to_response();
    assert_eq!(response.error, "Memory limit exceeded");
    assert_eq!(response.command, Some("diffoscope".to_string()));
    
    // Test generic command error
    let command_error = DifferError::DiffCommandError {
        command: "debdiff".to_string(),
        reason: "Process exited with code 2".to_string(),
    };
    
    assert_eq!(command_error.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    let response = command_error.to_response();
    assert_eq!(response.error, "Command failed");
    assert_eq!(response.command, Some("debdiff".to_string()));
}

#[test]
fn test_io_error_conversion() {
    use std::io::{Error as IoError, ErrorKind};
    use std::path::PathBuf;
    
    let io_error = IoError::new(ErrorKind::NotFound, "File not found");
    let differ_error = DifferError::IoError {
        operation: "read_file".to_string(),
        path: PathBuf::from("/test/path"),
        source: io_error,
    };
    
    assert_eq!(differ_error.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    
    let response = differ_error.to_response();
    assert_eq!(response.error, "Internal server error");
    assert!(response.details.is_some());
}

#[test]
fn test_database_error_conversion() {
    use sqlx::Error as SqlxError;
    
    let db_error = SqlxError::RowNotFound;
    let differ_error = DifferError::Database(db_error);
    
    assert_eq!(differ_error.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    
    let response = differ_error.to_response();
    assert_eq!(response.error, "Internal server error");
}

/// Test that error serialization works correctly for JSON responses
#[test]
fn test_error_response_serialization() {
    let error = DifferError::ArtifactsMissing { 
        run_id: "test456".to_string() 
    };
    
    let response = error.to_response();
    let json_str = serde_json::to_string(&response).expect("Failed to serialize error response");
    let parsed: Value = serde_json::from_str(&json_str).expect("Failed to parse JSON");
    
    assert_eq!(parsed["error"], "Artifacts missing");
    assert_eq!(parsed["run_id"], "test456");
    assert!(parsed["details"].is_string());
    assert!(parsed["command"].is_null());
}

/// Test error chaining for complex scenarios
#[test]
fn test_error_chaining() {
    use std::io::{Error as IoError, ErrorKind};
    use std::path::PathBuf;
    
    // Test that we can create errors with sources
    let io_error = IoError::new(ErrorKind::PermissionDenied, "Access denied");
    let differ_error = DifferError::IoError {
        operation: "create_cache_dir".to_string(),
        path: PathBuf::from("/var/cache/differ"),
        source: io_error,
    };
    
    // Verify the error chain
    assert!(differ_error.source().is_some());
    let error_string = format!("{}", differ_error);
    assert!(error_string.contains("create_cache_dir"));
    assert!(error_string.contains("/var/cache/differ"));
}

/// Test that errors can be properly downcasted
#[test]
fn test_error_type_matching() {
    let error1 = DifferError::ArtifactsMissing { run_id: "test".to_string() };
    let error2 = DifferError::RunNotFound { run_id: "test".to_string() };
    let error3 = DifferError::DiffCommandTimeout { command: "diffoscope".to_string(), timeout: 60 };
    
    // Test pattern matching
    match error1 {
        DifferError::ArtifactsMissing { .. } => {}, // Expected
        _ => panic!("Wrong error type"),
    }
    
    match error2 {
        DifferError::RunNotFound { .. } => {}, // Expected
        _ => panic!("Wrong error type"),
    }
    
    match error3 {
        DifferError::DiffCommandTimeout { .. } => {}, // Expected
        _ => panic!("Wrong error type"),
    }
}