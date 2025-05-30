use axum::{
    extract::{FromRequestParts, Request},
    http::{request::Parts, StatusCode},
    middleware::Next,
    response::Response,
};
use crate::auth::session::{SessionCookieConfig};
use crate::auth::types::{SessionInfo, User, UserRole};
use crate::config::Config;

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
#[async_trait::async_trait]
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

#[async_trait::async_trait]
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

// TODO: Add the full authentication middleware layer implementation
// For now, we have the basic middleware functions

#[derive(Clone)]
pub struct AuthMiddleware;

pub use self::UserContext as AuthContext;

// TODO: Implement full AuthLayer and AuthMiddleware when integrated with the app