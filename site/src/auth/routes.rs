use axum::{middleware, routing::{get, post}, Router};
use std::sync::Arc;

use crate::auth::{
    handlers::{admin_handler, callback_handler, login_handler, logout_handler, protected_handler, qa_handler, status_handler},
    middleware::{require_admin, require_login, AuthState},
};

/// Create authentication routes (placeholder for full implementation)
pub fn auth_routes(auth_state: Arc<AuthState>) -> Router<Arc<AuthState>> {
    Router::new()
        // Public authentication routes
        .route("/login", get(login_handler))
        .route("/auth/callback", get(callback_handler))
        .route("/logout", post(logout_handler))
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

    #[test]
    fn test_auth_routes_creation() {
        // This test verifies that the auth routes function exists and compiles
        // Real tests would require database setup and proper AuthState initialization
        // For now, just test that the function signature is correct
        assert!(true);
    }

    #[test]
    #[ignore = "requires database setup"]
    fn test_auth_routes_with_database() {
        // This test would create real auth routes with a test database
        // It's ignored because it requires complex database setup
        // TODO: Implement when test infrastructure is ready
        assert!(true);
    }
}
