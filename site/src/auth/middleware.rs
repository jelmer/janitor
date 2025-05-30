use axum::{
    extract::{FromRequestParts, Request, State},
    http::{request::Parts, StatusCode},
    middleware::Next,
    response::Response,
};
use axum_extra::extract::CookieJar;
use std::sync::Arc;
use crate::auth::session::{SessionCookieConfig, SessionManager};
use crate::auth::types::{SessionInfo, User, UserRole};
use crate::config::SiteConfig;

/// User context extracted from session
#[derive(Debug, Clone)]
pub struct UserContext {
    pub session_info: SessionInfo,
    pub roles: Vec<UserRole>,
    pub highest_role: UserRole,
}

impl UserContext {
    pub fn new(session_info: SessionInfo, admin_group: Option<&str>, qa_reviewer_group: Option<&str>) -> Self {
        let user = &session_info.user;
        
        let mut roles = vec![UserRole::User]; // All authenticated users have User role
        
        if user.has_role(UserRole::QaReviewer, admin_group, qa_reviewer_group) {
            roles.push(UserRole::QaReviewer);
        }
        
        if user.has_role(UserRole::Admin, admin_group, qa_reviewer_group) {
            roles.push(UserRole::Admin);
        }
        
        let highest_role = user.get_highest_role(admin_group, qa_reviewer_group);
        
        Self {
            session_info,
            roles,
            highest_role,
        }
    }
    
    pub fn user(&self) -> &User {
        &self.session_info.user
    }
    
    pub fn has_role(&self, role: UserRole) -> bool {
        self.roles.contains(&role)
    }
    
    pub fn is_admin(&self) -> bool {
        self.has_role(UserRole::Admin)
    }
    
    pub fn is_qa_reviewer(&self) -> bool {
        self.has_role(UserRole::QaReviewer)
    }
}

/// Extractor for getting the current user from request
impl<S> FromRequestParts<S> for UserContext
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<UserContext>()
            .cloned()
            .ok_or(StatusCode::UNAUTHORIZED)
    }
}

/// Extractor for optional user context (doesn't fail if not authenticated)
#[derive(Debug, Clone)]
pub struct OptionalUser(pub Option<UserContext>);

impl<S> FromRequestParts<S> for OptionalUser
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let user = parts
            .extensions
            .get::<UserContext>()
            .cloned();
        Ok(OptionalUser(user))
    }
}

/// Middleware function to require login
pub async fn require_login(
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let (parts, body) = req.into_parts();
    
    // Check if user is authenticated
    if parts.extensions.get::<UserContext>().is_none() {
        return Err(StatusCode::UNAUTHORIZED);
    }
    
    let req = Request::from_parts(parts, body);
    Ok(next.run(req).await)
}

/// Middleware function to require admin role
pub async fn require_admin(
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let (parts, body) = req.into_parts();
    
    // Check if user is authenticated and has admin role
    let is_admin = parts
        .extensions
        .get::<UserContext>()
        .map(|ctx| ctx.is_admin())
        .unwrap_or(false);
    
    if !is_admin {
        return Err(StatusCode::FORBIDDEN);
    }
    
    let req = Request::from_parts(parts, body);
    Ok(next.run(req).await)
}

/// Middleware function to require QA reviewer role
pub async fn require_qa_reviewer(
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let (parts, body) = req.into_parts();
    
    // Check if user is authenticated and has QA reviewer role (or admin)
    let has_qa_access = parts
        .extensions
        .get::<UserContext>()
        .map(|ctx| ctx.is_qa_reviewer() || ctx.is_admin())
        .unwrap_or(false);
    
    if !has_qa_access {
        return Err(StatusCode::FORBIDDEN);
    }
    
    let req = Request::from_parts(parts, body);
    Ok(next.run(req).await)
}

/// Application state for authentication
#[derive(Clone)]
pub struct AuthState {
    pub session_manager: SessionManager,
    pub cookie_config: SessionCookieConfig,
    pub admin_group: Option<String>,
    pub qa_reviewer_group: Option<String>,
}

impl AuthState {
    pub fn new(
        session_manager: SessionManager,
        config: &SiteConfig,
    ) -> Self {
        let cookie_config = if config.debug {
            SessionCookieConfig::for_development()
        } else {
            SessionCookieConfig::default()
        };
        
        Self {
            session_manager,
            cookie_config,
            admin_group: config.admin_group.clone(),
            qa_reviewer_group: config.qa_reviewer_group.clone(),
        }
    }
}

/// Session middleware that extracts user context from session cookies
pub async fn session_middleware(
    State(auth_state): State<Arc<AuthState>>,
    jar: CookieJar,
    mut req: Request,
    next: Next,
) -> Response {
    // Try to extract session from cookie
    if let Some(session_cookie) = jar.get(&auth_state.cookie_config.name) {
        let session_id = session_cookie.value();
        
        // Try to get session from database
        match auth_state.session_manager.get_session(session_id).await {
            Ok(session_info) => {
                // Update session activity
                if let Err(e) = auth_state.session_manager.update_activity(session_id).await {
                    tracing::warn!("Failed to update session activity: {}", e);
                }
                
                // Create user context with role information
                let user_context = UserContext::new(
                    session_info,
                    auth_state.admin_group.as_deref(),
                    auth_state.qa_reviewer_group.as_deref(),
                );
                
                // Insert user context into request extensions
                req.extensions_mut().insert(user_context);
            }
            Err(e) => {
                tracing::debug!("Session validation failed: {}", e);
                // Continue without authentication - let individual routes decide
            }
        }
    }
    
    next.run(req).await
}

/// Create middleware layer for session management  
pub fn auth_middleware_layer(auth_state: Arc<AuthState>) -> axum::middleware::FromFnLayer<impl Clone, Arc<AuthState>, ()> {
    axum::middleware::from_fn_with_state(auth_state, session_middleware)
}

/// Middleware layer for session management  
#[derive(Clone)]
pub struct AuthMiddleware;

pub use self::UserContext as AuthContext;