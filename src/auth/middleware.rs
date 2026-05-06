//! Authentication middleware for Axum

use axum::{
    extract::{Request, State},
    http::header,
    middleware::Next,
    response::{IntoResponse, Response},
};
use sqlx::{Pool, Postgres};
use std::sync::Arc;

use super::{
    basic::{check_worker_auth, require_worker_auth},
    AuthContext, AuthError, AuthService, UserContext, UserRole, WorkerAuth,
};

/// Middleware that requires worker authentication
///
/// This middleware checks for valid worker credentials and stores the
/// worker information in request extensions for use by handlers.
pub async fn require_worker_middleware(
    State(db): State<Pool<Postgres>>,
    mut request: Request,
    next: Next,
) -> Result<Response, Response> {
    // Authenticate worker
    let worker = require_worker_auth(&db, request.headers())
        .await
        .map_err(|e| {
            log::warn!("Worker authentication failed: {}", e);
            e.into_response()
        })?;

    // Store worker info in request extensions
    request.extensions_mut().insert(worker);

    log::debug!("Worker authenticated successfully");
    Ok(next.run(request).await)
}

/// Middleware that optionally checks for worker authentication
///
/// This middleware checks for worker credentials but doesn't require them.
/// If credentials are present and valid, the worker info is stored in extensions.
pub async fn optional_worker_middleware(
    State(db): State<Pool<Postgres>>,
    mut request: Request,
    next: Next,
) -> Result<Response, Response> {
    // Try to authenticate worker (optional)
    match check_worker_auth(&db, request.headers()).await {
        Ok(Some(worker)) => {
            log::debug!("Worker authenticated: {}", worker.name);
            request.extensions_mut().insert(worker);
        }
        Ok(None) => {
            log::debug!("No worker authentication provided");
        }
        Err(e) => {
            log::warn!("Worker authentication failed: {}", e);
            // Don't return error for optional auth, just continue without worker context
        }
    }

    Ok(next.run(request).await)
}

/// Helper function to check if user has required role
fn user_has_role(user: &UserContext, required_role: &UserRole) -> bool {
    match required_role {
        UserRole::Admin => user.is_admin(),
        UserRole::QaReviewer => user.is_qa_reviewer() || user.is_admin(),
        UserRole::User => true, // All authenticated users have User role
    }
}

/// Middleware that requires admin authentication
pub async fn require_admin_middleware(
    State(auth_service): State<Arc<dyn AuthService>>,
    mut request: Request,
    next: Next,
) -> Result<Response, Response> {
    require_user_role(&auth_service, &mut request, &UserRole::Admin).await?;
    Ok(next.run(request).await)
}

/// Middleware that requires QA reviewer or admin authentication
pub async fn require_qa_middleware(
    State(auth_service): State<Arc<dyn AuthService>>,
    mut request: Request,
    next: Next,
) -> Result<Response, Response> {
    require_user_role(&auth_service, &mut request, &UserRole::QaReviewer).await?;
    Ok(next.run(request).await)
}

/// Helper function to require a specific user role
async fn require_user_role(
    auth_service: &Arc<dyn AuthService>,
    request: &mut Request,
    required_role: &UserRole,
) -> Result<(), Response> {
    // Extract session token
    let token = extract_session_token(request).map_err(|e| e.into_response())?;

    // Validate session
    let auth_context = auth_service.validate_session(&token).await.map_err(|e| {
        log::warn!("Session validation failed: {}", e);
        e.into_response()
    })?;

    // Check if user has required role
    match auth_context {
        AuthContext::User(user) => {
            if !user_has_role(&user, required_role) {
                log::warn!(
                    "User {} lacks required role: {:?}",
                    user.email.as_deref().unwrap_or("unknown"),
                    required_role
                );
                return Err(AuthError::Unauthorized.into_response());
            }

            // Store user context in request extensions
            request.extensions_mut().insert(user);
            log::debug!("User authenticated with role: {:?}", required_role);
            Ok(())
        }
        _ => {
            log::warn!("Invalid authentication context for user role check");
            Err(AuthError::InvalidCredentials.into_response())
        }
    }
}

/// Generic authentication middleware that accepts multiple auth types
pub async fn auth_middleware(
    State(db): State<Pool<Postgres>>,
    State(auth_service): State<Arc<dyn AuthService>>,
    mut request: Request,
    next: Next,
) -> Result<Response, Response> {
    let mut auth_context = None;

    // Try worker authentication first
    match check_worker_auth(&db, request.headers()).await {
        Ok(Some(worker)) => {
            auth_context = Some(AuthContext::Worker(worker));
        }
        Ok(None) => {
            // No worker auth, try other methods
        }
        Err(e) => {
            log::debug!("Worker auth failed: {}", e);
        }
    }

    // If no worker auth, try session authentication
    if auth_context.is_none() {
        if let Ok(token) = extract_session_token(&request) {
            match auth_service.validate_session(&token).await {
                Ok(context) => {
                    auth_context = Some(context);
                }
                Err(e) => {
                    log::debug!("Session auth failed: {}", e);
                }
            }
        }
    }

    // If no session auth, try API key
    if auth_context.is_none() {
        if let Some(api_key) = extract_api_key(&request) {
            match auth_service.validate_api_key(&api_key).await {
                Ok(context) => {
                    auth_context = Some(context);
                }
                Err(e) => {
                    log::debug!("API key auth failed: {}", e);
                }
            }
        }
    }

    // Store auth context in request extensions
    if let Some(context) = auth_context {
        log::debug!("Request authenticated as: {}", context.identity());
        request.extensions_mut().insert(context);
    } else {
        log::debug!("Request not authenticated");
        request.extensions_mut().insert(AuthContext::Anonymous);
    }

    Ok(next.run(request).await)
}

/// Extract session token from request
fn extract_session_token(request: &Request) -> Result<String, AuthError> {
    // Try Authorization header first (Bearer token)
    if let Some(auth_header) = request.headers().get(header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                return Ok(token.to_string());
            }
        }
    }

    // Try session cookie
    if let Some(cookie_header) = request.headers().get(header::COOKIE) {
        if let Ok(cookie_str) = cookie_header.to_str() {
            for cookie in cookie_str.split(';') {
                let cookie = cookie.trim();
                if let Some(session_id) = cookie.strip_prefix("session_id=") {
                    return Ok(session_id.to_string());
                }
            }
        }
    }

    Err(AuthError::MissingAuth)
}

/// Extract API key from request
fn extract_api_key(request: &Request) -> Option<String> {
    // Try X-API-Key header
    if let Some(api_key_header) = request.headers().get("x-api-key") {
        if let Ok(api_key) = api_key_header.to_str() {
            return Some(api_key.to_string());
        }
    }

    // Try Authorization header with API key format
    if let Some(auth_header) = request.headers().get(header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(api_key) = auth_str.strip_prefix("ApiKey ") {
                return Some(api_key.to_string());
            }
        }
    }

    None
}

/// Helper function to get authenticated worker from request extensions
pub fn get_worker_from_request(request: &Request) -> Option<&WorkerAuth> {
    request.extensions().get::<WorkerAuth>()
}

/// Helper function to get auth context from request extensions
pub fn get_auth_context_from_request(request: &Request) -> Option<&AuthContext> {
    request.extensions().get::<AuthContext>()
}

/// Rate limiting configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per minute per identity
    pub max_requests_per_minute: u32,
    /// Enable audit logging
    pub enable_audit_logging: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests_per_minute: 60,
            enable_audit_logging: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;

    #[test]
    fn test_extract_session_token_bearer() {
        let request = Request::builder()
            .header(header::AUTHORIZATION, "Bearer token123")
            .body(Body::empty())
            .unwrap();

        let token = extract_session_token(&request).unwrap();
        assert_eq!(token, "token123");
    }

    #[test]
    fn test_extract_session_token_cookie() {
        let request = Request::builder()
            .header(header::COOKIE, "session_id=cookie123; other=value")
            .body(Body::empty())
            .unwrap();

        let token = extract_session_token(&request).unwrap();
        assert_eq!(token, "cookie123");
    }

    #[test]
    fn test_extract_api_key() {
        let request = Request::builder()
            .header("x-api-key", "api123")
            .body(Body::empty())
            .unwrap();

        let api_key = extract_api_key(&request).unwrap();
        assert_eq!(api_key, "api123");
    }

    #[test]
    fn test_rate_limit_config_default() {
        let config = RateLimitConfig::default();
        assert_eq!(config.max_requests_per_minute, 60);
        assert!(config.enable_audit_logging);
    }
}
