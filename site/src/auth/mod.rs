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

pub use cleanup::{spawn_cleanup_task, spawn_cleanup_task_with_interval, SessionCleanupTask};
pub use handlers::LoginStatus;
pub use middleware::{
    auth_middleware_layer, require_admin, require_admin_layer, require_login, require_login_layer,
    require_qa_reviewer, require_qa_reviewer_layer, session_middleware,
    session_middleware as auth_middleware, AuthMiddleware, AuthState, OptionalUser, UserContext,
};
pub use oidc::{AuthError, OidcClient, OidcConfig};
pub use routes::{api_auth_routes, auth_routes, with_auth_middleware};
pub use service::{AuthService, PendingAuth};
pub use session::{SessionCookieConfig, SessionManager};
pub use types::{SessionInfo, User, UserRole};
