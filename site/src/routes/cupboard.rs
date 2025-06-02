use axum::{
    routing::{get, post},
    Router,
};

use crate::{
    app::AppState,
    auth::{require_admin, require_qa_reviewer},
    handlers::cupboard,
};

/// Create cupboard (admin) routes
pub fn cupboard_routes() -> Router<AppState> {
    Router::new()
        // Main admin dashboard
        .route("/cupboard", get(cupboard::admin_dashboard))
        .route("/cupboard/", get(cupboard::admin_dashboard))
        .route("/cupboard/dashboard", get(cupboard::admin_dashboard))
        
        // Admin sidebar navigation
        .route("/cupboard/sidebar", get(cupboard::admin_sidebar))
        
        // System status and monitoring
        .route("/cupboard/status", get(cupboard::system_status))
        
        // Queue management
        .route("/cupboard/queue", get(cupboard::queue::queue_dashboard))
        .route("/cupboard/queue/:id", get(cupboard::queue::queue_item_details))
        .route("/cupboard/queue/stats", get(cupboard::queue::queue_statistics))
        .route("/cupboard/queue/bulk", post(cupboard::queue::bulk_queue_operation))
        .route("/cupboard/workers", get(cupboard::queue::worker_management))
        
        // Review administration  
        .route("/cupboard/reviews", get(cupboard::review::review_dashboard))
        .route("/cupboard/reviews/bulk", post(cupboard::review::bulk_review_action))
        
        // Publishing controls
        .route("/cupboard/publish", get(cupboard::publish::publish_dashboard))
        .route("/cupboard/publish/emergency-stop", post(cupboard::publish::emergency_publish_stop))
        
        // Merge proposal management
        .route("/cupboard/merge-proposals", get(cupboard::merge_proposals::mp_dashboard))
        .route("/cupboard/merge-proposals/bulk", post(cupboard::merge_proposals::bulk_mp_operation))
        
        // Admin API endpoints
        .route("/cupboard/api/status", get(cupboard::api::admin_system_status))
        .route("/cupboard/api/config", get(cupboard::api::admin_system_config))
        .route("/cupboard/api/metrics", get(cupboard::api::admin_system_metrics))
        .route("/cupboard/api/bulk", post(cupboard::api::admin_bulk_operation))
        .route("/cupboard/api/users", get(cupboard::api::admin_list_users).post(cupboard::api::admin_create_user))
        
        // Apply admin authentication middleware to all cupboard routes
        .route_layer(axum::middleware::from_fn(require_admin))
}

/// Create cupboard API routes (for external API access)
pub fn cupboard_api_routes() -> Router<AppState> {
    Router::new()
        // API-only endpoints for admin operations
        .route("/api/v1/admin/status", get(cupboard::api::admin_system_status))
        .route("/api/v1/admin/config", get(cupboard::api::admin_system_config))
        .route("/api/v1/admin/metrics", get(cupboard::api::admin_system_metrics))
        .route("/api/v1/admin/bulk", post(cupboard::api::admin_bulk_operation))
        .route("/api/v1/admin/users", get(cupboard::api::admin_list_users).post(cupboard::api::admin_create_user))
        
        // Queue management API
        .route("/api/v1/admin/queue", get(cupboard::queue::queue_dashboard))
        .route("/api/v1/admin/queue/stats", get(cupboard::queue::queue_statistics))
        .route("/api/v1/admin/queue/bulk", post(cupboard::queue::bulk_queue_operation))
        .route("/api/v1/admin/workers", get(cupboard::queue::worker_management))
        
        // Review API
        .route("/api/v1/admin/reviews", get(cupboard::review::review_dashboard))
        .route("/api/v1/admin/reviews/bulk", post(cupboard::review::bulk_review_action))
        
        // Publishing API
        .route("/api/v1/admin/publish", get(cupboard::publish::publish_dashboard))
        .route("/api/v1/admin/publish/emergency-stop", post(cupboard::publish::emergency_publish_stop))
        
        // Merge proposals API
        .route("/api/v1/admin/merge-proposals", get(cupboard::merge_proposals::mp_dashboard))
        .route("/api/v1/admin/merge-proposals/bulk", post(cupboard::merge_proposals::bulk_mp_operation))
        
        // Apply admin authentication middleware
        .route_layer(axum::middleware::from_fn(require_admin))
}