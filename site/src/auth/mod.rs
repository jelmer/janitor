pub mod oidc;
pub mod session;
pub mod middleware;
pub mod types;
pub mod cleanup;

pub use middleware::{AuthMiddleware, AuthState, require_admin, require_login, require_qa_reviewer, session_middleware, auth_middleware_layer};
pub use session::{SessionManager, SessionCookieConfig};
pub use types::{User, UserRole, SessionInfo};
pub use oidc::{OidcClient, OidcConfig, AuthError};
pub use cleanup::{SessionCleanupTask, spawn_cleanup_task, spawn_cleanup_task_with_interval};