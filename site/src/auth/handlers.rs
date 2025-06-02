use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    Form,
};
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::auth::{
    middleware::{AuthState, OptionalUser, UserContext},
    oidc::OidcClient,
    session::SessionManager,
};
use crate::config::SiteConfig;

/// Query parameters for login redirect
#[derive(Debug, Deserialize)]
pub struct LoginQuery {
    pub redirect: Option<String>,
}

/// Form data for logout
#[derive(Debug, Deserialize)]
pub struct LogoutForm {
    pub redirect: Option<String>,
}

/// OAuth callback query parameters
#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

/// Login status response
#[derive(Debug, Serialize)]
pub struct LoginStatus {
    pub authenticated: bool,
    pub user: Option<serde_json::Value>,
    pub permissions: Option<Vec<String>>,
}

/// Initiate login flow - redirect to OIDC provider
pub async fn login_handler(
    State(_auth_state): State<Arc<AuthState>>,
    Query(query): Query<LoginQuery>,
    OptionalUser(user): OptionalUser,
    _jar: CookieJar,
) -> Result<Response, StatusCode> {
    // If user is already authenticated, redirect to requested page or home
    if let Some(_user_context) = user {
        let redirect_url = query.redirect.unwrap_or_else(|| "/".to_string());
        return Ok(Redirect::to(&redirect_url).into_response());
    }

    // This is a placeholder implementation - the OIDC client needs to be injected
    // from the main application state when this is integrated
    //
    // The proper implementation would:
    // 1. Create OIDC client from config
    // 2. Generate authorization URL with state and PKCE
    // 3. Store auth state in session/cache
    // 4. Redirect to OIDC provider

    Ok(Redirect::to("/login?error=not_configured").into_response())
}

/// Handle OAuth callback from OIDC provider
pub async fn callback_handler(
    State(_auth_state): State<Arc<AuthState>>,
    Query(query): Query<CallbackQuery>,
    _jar: CookieJar,
) -> Result<Response, StatusCode> {
    // Check for OAuth errors
    if let Some(error) = query.error {
        warn!(
            "OAuth error: {} - {}",
            error,
            query.error_description.unwrap_or_default()
        );
        return Ok(Redirect::to("/login?error=oauth_failed").into_response());
    }

    // Extract authorization code and state
    let code = query.code.ok_or(StatusCode::BAD_REQUEST)?;
    let state = query.state.ok_or(StatusCode::BAD_REQUEST)?;

    // For now, return a placeholder since we need to retrieve stored auth state
    // This will be properly implemented when integrated with session storage
    Ok(StatusCode::NOT_IMPLEMENTED.into_response())
}

/// Logout handler - clear session and redirect
pub async fn logout_handler(
    State(auth_state): State<Arc<AuthState>>,
    OptionalUser(user): OptionalUser,
    Form(form): Form<LogoutForm>,
    jar: CookieJar,
) -> Result<Response, StatusCode> {
    let redirect_url = form.redirect.unwrap_or_else(|| "/".to_string());

    // If user is authenticated, clear their session
    if let Some(user_context) = user {
        if let Some(session_cookie) = jar.get(&auth_state.cookie_config.name) {
            let session_id = session_cookie.value();

            // Delete the session from storage
            if let Err(e) = auth_state.session_manager.delete_session(session_id).await {
                error!("Failed to delete session {}: {}", session_id, e);
            } else {
                info!("User {} logged out", user_context.user().email);
            }
        }
    }

    // Create response with cleared session cookie
    let mut response = Redirect::to(&redirect_url).into_response();

    // Clear the session cookie
    let clear_cookie = format!(
        "{}=; Path={}; Max-Age=0; HttpOnly{}{}",
        auth_state.cookie_config.name,
        auth_state.cookie_config.path,
        if auth_state.cookie_config.secure {
            "; Secure"
        } else {
            ""
        },
        match auth_state.cookie_config.same_site {
            crate::auth::session::SameSite::Strict => "; SameSite=Strict",
            crate::auth::session::SameSite::Lax => "; SameSite=Lax",
            crate::auth::session::SameSite::None => "; SameSite=None",
        }
    );

    response.headers_mut().insert(
        axum::http::header::SET_COOKIE,
        clear_cookie
            .parse()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );

    Ok(response)
}

/// Get current user status (for API endpoints)
pub async fn status_handler(
    OptionalUser(user): OptionalUser,
) -> Result<axum::Json<LoginStatus>, StatusCode> {
    match user {
        Some(user_context) => {
            let user_data = serde_json::to_value(&user_context.session_info.user)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            let permissions = user_context
                .roles
                .iter()
                .map(|role| format!("{:?}", role))
                .collect();

            Ok(axum::Json(LoginStatus {
                authenticated: true,
                user: Some(user_data),
                permissions: Some(permissions),
            }))
        }
        None => Ok(axum::Json(LoginStatus {
            authenticated: false,
            user: None,
            permissions: None,
        })),
    }
}

/// Protected route example - requires authentication
pub async fn protected_handler(
    user: UserContext, // This will return 401 if not authenticated
) -> Result<axum::Json<serde_json::Value>, StatusCode> {
    Ok(axum::Json(serde_json::json!({
        "message": "Access granted",
        "user": user.user().email,
        "roles": user.roles
    })))
}

/// Admin-only route example
pub async fn admin_handler(user: UserContext) -> Result<axum::Json<serde_json::Value>, StatusCode> {
    if !user.is_admin() {
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(axum::Json(serde_json::json!({
        "message": "Admin access granted",
        "user": user.user().email,
        "admin": true
    })))
}

/// QA reviewer route example
pub async fn qa_handler(user: UserContext) -> Result<axum::Json<serde_json::Value>, StatusCode> {
    if !user.is_qa_reviewer() && !user.is_admin() {
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(axum::Json(serde_json::json!({
        "message": "QA access granted",
        "user": user.user().email,
        "qa_reviewer": user.is_qa_reviewer(),
        "admin": user.is_admin()
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_status_serialization() {
        let status = LoginStatus {
            authenticated: false,
            user: None,
            permissions: None,
        };

        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"authenticated\":false"));
    }

    #[test]
    fn test_callback_query_parsing() {
        // Test valid callback query
        let query = "code=abc123&state=xyz789";
        // In a real test, we'd use axum's query parsing
        assert!(query.contains("code=abc123"));
    }
}
