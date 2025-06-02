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
        let response: ApiResponse<()> = ApiResponse::error("Test error".to_string(), Some("reason".to_string()));
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