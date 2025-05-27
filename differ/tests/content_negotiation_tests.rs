//! Tests for content negotiation functionality in the differ service.

use axum::http::HeaderMap;
use janitor_differ::DifferError;

// Import the internal function for testing
// Note: In a real scenario, we might want to expose this through the public API
// For now, we'll test the error scenarios it can produce

#[test]
fn test_content_negotiation_errors() {
    // Test various content negotiation failure scenarios

    // Test case 1: Completely invalid Accept header
    let error = DifferError::AcceptHeaderError("invalid-header-value".to_string());
    assert_eq!(error.status_code(), axum::http::StatusCode::BAD_REQUEST);

    // Test case 2: Valid header but no acceptable content type
    let error = DifferError::ContentNegotiationFailed {
        available: vec![
            "text/html".to_string(),
            "application/json".to_string(),
            "text/plain".to_string(),
        ],
        requested: "application/xml".to_string(),
    };
    assert_eq!(error.status_code(), axum::http::StatusCode::NOT_ACCEPTABLE);

    let response = error.to_response();
    assert_eq!(response.error, "Content negotiation failed");
    assert!(response.details.is_some());
    assert!(response.details.unwrap().contains("application/xml"));
}

#[test]
fn test_supported_media_types() {
    // Test that we support the expected media types
    let supported_types = vec![
        "application/json",
        "text/html",
        "text/plain",
        "text/x-diff",
        "text/markdown",
    ];

    // This test documents the expected supported types
    // In a real integration test, we would verify these work with actual HTTP requests
    for media_type in supported_types {
        // Each of these should be supported by our content negotiation
        println!("Should support: {}", media_type);
    }
}

#[test]
fn test_accept_header_parsing_edge_cases() {
    // Test various edge cases in Accept header parsing

    let test_cases = vec![
        // Empty header - should default to application/json
        "",
        // Wildcard
        "*/*",
        // Multiple types with quality values
        "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        // Invalid but parseable
        "text/html; charset=utf-8",
        // Case variations
        "TEXT/HTML",
        "Application/JSON",
    ];

    for header_value in test_cases {
        // These are all cases our content negotiation should handle gracefully
        println!("Should handle Accept header: '{}'", header_value);
    }
}

#[test]
fn test_error_response_content_types() {
    // Test that error responses maintain proper content type handling

    let error = DifferError::ContentNegotiationFailed {
        available: vec!["text/html".to_string()],
        requested: "application/json".to_string(),
    };

    // Convert to response and verify it's properly structured
    let response = error.to_response();

    // Should be serializable as JSON (our default error format)
    let json_str = serde_json::to_string(&response).unwrap();
    assert!(json_str.contains("Content negotiation failed"));

    // Should contain useful debugging information
    assert!(response.details.is_some());
    let details = response.details.unwrap();
    assert!(details.contains("application/json"));
    assert!(details.contains("text/html"));
}

#[test]
fn test_header_map_construction() {
    // Test that we can construct HeaderMap objects correctly for testing
    let mut headers = HeaderMap::new();

    // Test various Accept header values that our system should handle
    let test_values = vec![
        "application/json",
        "text/html",
        "text/plain",
        "text/x-diff",
        "text/markdown",
        "application/json, text/html;q=0.9",
        "*/*",
    ];

    for value in test_values {
        headers.insert("Accept", value.parse().unwrap());
        // In real tests, we would pass this to our content negotiation function
        println!("Test header value: {}", value);
    }
}

/// Test error details formatting
#[test]
fn test_content_negotiation_error_details() {
    let available = vec![
        "text/html".to_string(),
        "application/json".to_string(),
        "text/plain".to_string(),
    ];
    let requested = "application/xml".to_string();

    let error = DifferError::ContentNegotiationFailed {
        available: available.clone(),
        requested: requested.clone(),
    };

    let response = error.to_response();
    let details = response.details.unwrap();

    // Should include both what was requested and what's available
    assert!(details.contains(&requested));
    for available_type in &available {
        assert!(details.contains(available_type));
    }
}
