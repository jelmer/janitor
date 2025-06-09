// Integration tests disabled due to dependencies
// These would require proper database setup and service mocking

#[cfg(test)]
mod unit_tests {
    use crate::api::types::*;
    use serde_json::json;

    #[test]
    fn test_api_response_success() {
        let response = ApiResponse::success(json!({"test": "data"}));
        assert!(response.data.is_some());
        assert!(response.error.is_none());
        assert_eq!(response.data.unwrap()["test"], "data");
    }

    #[test]
    fn test_api_response_error() {
        let response: ApiResponse<()> =
            ApiResponse::error("Test error".to_string(), Some("reason".to_string()));
        assert!(response.data.is_none());
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap(), "Test error");
        assert_eq!(response.reason.unwrap(), "reason");
    }

    #[test]
    fn test_queue_status_serialization() {
        let status = QueueStatus {
            total_candidates: 100,
            pending_candidates: 95,
            active_runs: 5,
            campaigns: vec![],
        };

        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"total_candidates\":100"));
        assert!(json.contains("\"active_runs\":5"));
    }

    #[test]
    fn test_pagination_params() {
        let params = PaginationParams {
            page: Some(2),
            limit: Some(20),
            offset: None,
        };

        assert_eq!(params.get_offset(), 20); // (2-1) * 20
        assert_eq!(params.get_limit(), 20);
    }
}

#[cfg(test)]
mod bulk_operations_tests {
    use crate::api::schemas::BulkUserOperationRequest;
    use serde_json::json;

    #[test]
    fn test_bulk_operation_request_serialization() {
        let request = BulkUserOperationRequest {
            user_ids: vec!["user1".to_string(), "user2".to_string()],
            operation: "revoke_sessions".to_string(),
            parameters: None,
            requester: Some("admin@example.com".to_string()),
        };

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["user_ids"].as_array().unwrap().len(), 2);
        assert_eq!(json["operation"], "revoke_sessions");
        assert_eq!(json["requester"], "admin@example.com");
    }

    #[test]
    fn test_bulk_operation_with_parameters() {
        let request = BulkUserOperationRequest {
            user_ids: vec!["user1".to_string()],
            operation: "update_role".to_string(),
            parameters: Some(json!({
                "role": "Admin"
            })),
            requester: Some("admin@example.com".to_string()),
        };

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["parameters"]["role"], "Admin");
    }

    #[test]
    fn test_bulk_operation_validation() {
        use validator::Validate;
        
        // Test empty user_ids validation
        let request = BulkUserOperationRequest {
            user_ids: vec![],
            operation: "revoke_sessions".to_string(),
            parameters: None,
            requester: None,
        };
        
        // This should fail validation due to empty user_ids
        assert!(request.validate().is_err());
        
        // Test with too many user_ids
        let many_users: Vec<String> = (0..101).map(|i| format!("user{}", i)).collect();
        let request = BulkUserOperationRequest {
            user_ids: many_users,
            operation: "revoke_sessions".to_string(),
            parameters: None,
            requester: None,
        };
        
        // This should fail validation due to too many user_ids (max 100)
        assert!(request.validate().is_err());
        
        // Test valid request
        let request = BulkUserOperationRequest {
            user_ids: vec!["user1".to_string(), "user2".to_string()],
            operation: "revoke_sessions".to_string(),
            parameters: None,
            requester: Some("admin@example.com".to_string()),
        };
        
        // This should pass validation
        assert!(request.validate().is_ok());
    }
}
