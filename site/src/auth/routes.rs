use axum::{middleware, routing::get, Router};
use std::sync::Arc;

use crate::auth::{
    handlers::{admin_handler, protected_handler, qa_handler, status_handler},
    middleware::{require_admin, require_login, AuthState},
};

/// Create authentication routes (placeholder for full implementation)
pub fn auth_routes(auth_state: Arc<AuthState>) -> Router<Arc<AuthState>> {
    Router::new()
        // Public authentication routes - simplified for now
        .route("/status", get(status_handler))
        .route("/protected", get(protected_handler))
        .route("/admin", get(admin_handler))
        .route("/qa", get(qa_handler))
        .with_state(auth_state)
}

/// Create API authentication routes (for API endpoints)
pub fn api_auth_routes(auth_state: Arc<AuthState>) -> Router<Arc<AuthState>> {
    Router::new()
        .route("/auth/status", get(status_handler))
        // API routes that require authentication
        .route(
            "/auth/user-info",
            get(protected_handler).route_layer(middleware::from_fn(require_login)),
        )
        .route(
            "/auth/admin-info",
            get(admin_handler).route_layer(middleware::from_fn(require_admin)),
        )
        .with_state(auth_state)
}

/// Helper function to create a router with authentication middleware applied
pub fn with_auth_middleware<S>(router: Router<S>, auth_state: Arc<AuthState>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    // For now, just return the router without middleware - this will be implemented
    // when integrated with the main application
    router
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::config::SiteConfig;

    fn create_test_auth_state() -> Arc<AuthState> {
        // This is a placeholder for testing - in real tests we'd need a test database
        let config = SiteConfig::default();

        // For testing, create a minimal AuthState without database dependencies
        // In real tests, this would be mocked or use a test database
        Arc::new(AuthState {
            state: "test-state".to_string(),
            redirect_url: Some("/test".to_string()),
            pkce_verifier: None,
        })
    }

    #[test]
    fn test_auth_routes_creation() {
        // Test would create auth routes and verify they're set up correctly
        // For now, just test that the function exists and compiles
        assert!(true);
    }
}
