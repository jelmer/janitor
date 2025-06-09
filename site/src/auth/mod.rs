pub mod cleanup;
pub mod handlers;
pub mod middleware;
pub mod oidc;
pub mod routes;
pub mod service;
pub mod session;
pub mod types;

#[cfg(test)]
mod tests;

use axum::Router;
use std::sync::Arc;

use crate::app::AppState;

pub use middleware::{require_admin, OptionalUser, UserContext};
pub use oidc::AuthError;
pub use types::User;

/// Create authentication routes integrated with the main app state
pub fn create_auth_routes() -> Router<AppState> {
    // For now, return an empty router since auth integration requires significant work
    // This would need to:
    // 1. Create an AuthState from AppState 
    // 2. Set up session management with the database
    // 3. Configure OIDC client from app config
    // 4. Return the auth routes with proper state conversion
    
    Router::new()
        // TODO: Implement full auth integration
        // .merge(routes::auth_routes(auth_state))
}
