pub mod oidc;
pub mod session;
pub mod middleware;
pub mod types;
pub mod cleanup;
pub mod handlers;
pub mod routes;
pub mod service;

#[cfg(test)]
mod tests;

pub use middleware::{AuthMiddleware, AuthState, require_admin, require_login, require_qa_reviewer, session_middleware, session_middleware as auth_middleware, auth_middleware_layer, OptionalUser, UserContext};
pub use session::{SessionManager, SessionCookieConfig};
pub use types::{User, UserRole, SessionInfo};
pub use oidc::{OidcClient, OidcConfig, AuthError};
pub use cleanup::{SessionCleanupTask, spawn_cleanup_task, spawn_cleanup_task_with_interval};
pub use service::{AuthService, PendingAuth};
pub use routes::{auth_routes, api_auth_routes, with_auth_middleware};
pub use handlers::{LoginStatus};