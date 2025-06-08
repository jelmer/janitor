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

use crate::auth::middleware::{AuthState, OptionalUser, UserContext};

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

    // Get OIDC client from auth state - if not configured, redirect to error page
    let oidc_client = match &_auth_state.oidc_client {
        Some(client) => client,
        None => {
            warn!("OIDC not configured, cannot initiate login");
            return Ok(Redirect::to("/login?error=not_configured").into_response());
        }
    };

    // Generate authorization URL with state and PKCE
    let (auth_url, auth_state_data) = oidc_client.get_authorization_url(query.redirect);

    // Store auth state in session storage for verification during callback
    let auth_state_key = format!("auth_state:{}", auth_state_data.state);
    if let Err(e) = _auth_state
        .session_manager
        .store_temporary_data(
            &auth_state_key,
            &auth_state_data,
            std::time::Duration::from_secs(600), // 10 minutes
        )
        .await
    {
        error!("Failed to store auth state: {}", e);
        return Ok(Redirect::to("/login?error=session_error").into_response());
    }

    info!("Redirecting to OIDC provider for authentication");
    Ok(Redirect::to(auth_url.as_str()).into_response())
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

    // Get OIDC client from auth state
    let oidc_client = match &_auth_state.oidc_client {
        Some(client) => client,
        None => {
            warn!("OIDC not configured, cannot handle callback");
            return Ok(Redirect::to("/login?error=not_configured").into_response());
        }
    };

    // Retrieve stored auth state for verification
    let auth_state_key = format!("auth_state:{}", state);
    let stored_auth_state = match _auth_state
        .session_manager
        .get_temporary_data::<crate::auth::oidc::AuthState>(&auth_state_key)
        .await
    {
        Ok(Some(state)) => state,
        Ok(None) => {
            warn!("Auth state not found for state: {}", state);
            return Ok(Redirect::to("/login?error=invalid_state").into_response());
        }
        Err(e) => {
            error!("Failed to retrieve auth state: {}", e);
            return Ok(Redirect::to("/login?error=session_error").into_response());
        }
    };

    // Exchange authorization code for user information
    let user = match oidc_client
        .handle_callback(&code, &state, &stored_auth_state)
        .await
    {
        Ok(user) => user,
        Err(e) => {
            error!("OIDC callback failed: {}", e);
            return Ok(Redirect::to("/login?error=auth_failed").into_response());
        }
    };

    // Create user session
    let user_email = user.email.clone(); // Clone email before moving user
    let session_id = match _auth_state.session_manager.create_session(user).await {
        Ok(id) => id,
        Err(e) => {
            error!("Failed to create session for user {}: {}", user_email, e);
            return Ok(Redirect::to("/login?error=session_error").into_response());
        }
    };

    // Clean up temporary auth state
    if let Err(e) = _auth_state
        .session_manager
        .delete_temporary_data(&auth_state_key)
        .await
    {
        warn!("Failed to clean up auth state: {}", e);
    }

    // Set session cookie and redirect
    let session_cookie = format!(
        "{}={}; Path={}; Max-Age={}; HttpOnly{}{}",
        _auth_state.cookie_config.name,
        session_id,
        _auth_state.cookie_config.path,
        _auth_state
            .cookie_config
            .max_age
            .map(|d| d.num_seconds())
            .unwrap_or(86400),
        if _auth_state.cookie_config.secure {
            "; Secure"
        } else {
            ""
        },
        match _auth_state.cookie_config.same_site {
            crate::auth::session::SameSite::Strict => "; SameSite=Strict",
            crate::auth::session::SameSite::Lax => "; SameSite=Lax",
            crate::auth::session::SameSite::None => "; SameSite=None",
        }
    );

    let redirect_url = stored_auth_state
        .redirect_url
        .unwrap_or_else(|| "/".to_string());
    let mut response = Redirect::to(&redirect_url).into_response();

    response.headers_mut().insert(
        axum::http::header::SET_COOKIE,
        session_cookie
            .parse()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );

    info!("User {} successfully authenticated", user_email);
    Ok(response)
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
