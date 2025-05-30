use anyhow::Result;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use cookie::{Cookie, SameSite};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::auth::{
    middleware::OptionalUser,
    oidc::{AuthError, AuthState as OidcAuthState, OidcClient},
    session::{SessionCookieConfig, SessionManager},
    types::User,
};
use crate::config::SiteConfig;

/// Temporary storage for OAuth state during authentication flow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingAuth {
    pub state: String,
    pub redirect_url: Option<String>,
    pub pkce_verifier: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Authentication service that coordinates OIDC and session management
#[derive(Clone)]
pub struct AuthService {
    oidc_client: Option<OidcClient>,
    session_manager: SessionManager,
    cookie_config: SessionCookieConfig,
    pending_auths: Arc<RwLock<HashMap<String, PendingAuth>>>,
    admin_group: Option<String>,
    qa_reviewer_group: Option<String>,
}

impl AuthService {
    /// Create a new authentication service
    pub async fn new(config: &SiteConfig, session_manager: SessionManager) -> Result<Self> {
        let oidc_client = if config.oidc_client_id.is_some() && config.oidc_issuer_url.is_some() {
            match OidcClient::new(config).await {
                Ok(client) => Some(client),
                Err(e) => {
                    warn!("Failed to initialize OIDC client: {}", e);
                    None
                }
            }
        } else {
            info!("OIDC not configured - authentication will be disabled");
            None
        };

        let cookie_config = if config.debug {
            SessionCookieConfig::for_development()
        } else {
            SessionCookieConfig::default()
        };

        Ok(Self {
            oidc_client,
            session_manager,
            cookie_config,
            pending_auths: Arc::new(RwLock::new(HashMap::new())),
            admin_group: config.admin_group.clone(),
            qa_reviewer_group: config.qa_reviewer_group.clone(),
        })
    }

    /// Check if authentication is enabled
    pub fn is_enabled(&self) -> bool {
        self.oidc_client.is_some()
    }

    /// Start the login flow
    pub async fn start_login(&self, redirect_after_login: Option<String>) -> Result<Response> {
        let oidc_client = self.oidc_client.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Authentication not configured"))?;

        // Generate authorization URL and state
        let (auth_url, auth_state) = oidc_client.get_authorization_url(redirect_after_login.clone());

        // Store pending auth state
        let pending_auth = PendingAuth {
            state: auth_state.state.clone(),
            redirect_url: redirect_after_login,
            pkce_verifier: auth_state.pkce_verifier,
            created_at: chrono::Utc::now(),
        };

        {
            let mut pending_auths = self.pending_auths.write().await;
            pending_auths.insert(auth_state.state.clone(), pending_auth);
        }

        // Cleanup old pending auths (older than 10 minutes)
        self.cleanup_expired_pending_auths().await;

        Ok(Redirect::to(auth_url.as_str()).into_response())
    }

    /// Handle OAuth callback
    pub async fn handle_callback(
        &self,
        code: &str,
        state: &str,
    ) -> Result<(User, String), AuthError> {
        let oidc_client = self.oidc_client.as_ref()
            .ok_or(AuthError::InvalidConfig("Authentication not configured".to_string()))?;

        // Retrieve and remove pending auth state
        let pending_auth = {
            let mut pending_auths = self.pending_auths.write().await;
            pending_auths.remove(state)
                .ok_or(AuthError::InvalidState)?
        };

        // Verify state matches
        if state != pending_auth.state {
            return Err(AuthError::InvalidState);
        }

        // Check if auth state hasn't expired (10 minutes)
        let age = chrono::Utc::now().signed_duration_since(pending_auth.created_at);
        if age > chrono::Duration::minutes(10) {
            return Err(AuthError::InvalidState);
        }

        // Create stored auth state for OIDC client
        let stored_state = OidcAuthState {
            state: pending_auth.state,
            redirect_url: pending_auth.redirect_url.clone(),
            pkce_verifier: pending_auth.pkce_verifier,
        };

        // Exchange code for user info
        let user = oidc_client.handle_callback(code, state, &stored_state).await?;

        // Create session
        let session_id = self.session_manager.create_session(user.clone()).await
            .map_err(|e| AuthError::TokenExchangeFailed(format!("Session creation failed: {}", e)))?;

        let redirect_url = pending_auth.redirect_url.unwrap_or_else(|| "/".to_string());

        Ok((user, session_id))
    }

    /// Create a session cookie
    pub fn create_session_cookie(&self, session_id: &str) -> Cookie<'static> {
        let mut cookie = Cookie::new(self.cookie_config.name.clone(), session_id.to_string());
        
        if let Some(domain) = &self.cookie_config.domain {
            cookie.set_domain(domain.clone());
        }
        
        cookie.set_path(self.cookie_config.path.clone());
        cookie.set_secure(self.cookie_config.secure);
        cookie.set_http_only(self.cookie_config.http_only);
        
        cookie.set_same_site(match self.cookie_config.same_site {
            crate::auth::session::SameSite::Strict => SameSite::Strict,
            crate::auth::session::SameSite::Lax => SameSite::Lax,
            crate::auth::session::SameSite::None => SameSite::None,
        });

        if let Some(max_age) = self.cookie_config.max_age {
            cookie.set_max_age(time::Duration::seconds(max_age.num_seconds()));
        }

        cookie
    }

    /// Clear session cookie
    pub fn clear_session_cookie(&self) -> Cookie<'static> {
        let mut cookie = Cookie::new(self.cookie_config.name.clone(), "");
        cookie.set_path(self.cookie_config.path.clone());
        cookie.set_max_age(time::Duration::seconds(0));
        cookie
    }

    /// Logout user and clear session
    pub async fn logout(&self, session_id: &str) -> Result<()> {
        if let Err(e) = self.session_manager.delete_session(session_id).await {
            error!("Failed to delete session {}: {}", session_id, e);
        }
        Ok(())
    }

    /// Cleanup expired pending auth states
    async fn cleanup_expired_pending_auths(&self) {
        let cutoff = chrono::Utc::now() - chrono::Duration::minutes(10);
        
        let mut pending_auths = self.pending_auths.write().await;
        pending_auths.retain(|_, auth| auth.created_at > cutoff);
    }

    /// Get session manager reference
    pub fn session_manager(&self) -> &SessionManager {
        &self.session_manager
    }
}

/// Authentication service handlers with integrated OIDC flow
impl AuthService {
    /// Login handler
    pub async fn login_handler(
        &self,
        redirect: Option<String>,
        current_user: OptionalUser,
    ) -> Result<Response, StatusCode> {
        // If user is already authenticated, redirect to requested page
        if let Some(_user) = current_user.0 {
            let redirect_url = redirect.unwrap_or_else(|| "/".to_string());
            return Ok(Redirect::to(&redirect_url).into_response());
        }

        // Start login flow
        match self.start_login(redirect).await {
            Ok(response) => Ok(response),
            Err(e) => {
                error!("Login failed: {}", e);
                Ok(Redirect::to("/login?error=config_error").into_response())
            }
        }
    }

    /// OAuth callback handler
    pub async fn callback_handler(
        &self,
        code: Option<String>,
        state: Option<String>,
        error: Option<String>,
    ) -> Result<Response, StatusCode> {
        // Check for OAuth errors
        if let Some(error) = error {
            warn!("OAuth error: {}", error);
            return Ok(Redirect::to("/login?error=oauth_failed").into_response());
        }

        let code = code.ok_or(StatusCode::BAD_REQUEST)?;
        let state = state.ok_or(StatusCode::BAD_REQUEST)?;

        // Handle the callback
        match self.handle_callback(&code, &state).await {
            Ok((user, session_id)) => {
                info!("User {} logged in successfully", user.email);
                
                // Create session cookie
                let session_cookie = self.create_session_cookie(&session_id);
                
                // Create response with session cookie
                let mut response = Redirect::to("/").into_response();
                response.headers_mut().insert(
                    axum::http::header::SET_COOKIE,
                    session_cookie.to_string().parse()
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
                );
                
                Ok(response)
            }
            Err(e) => {
                error!("Callback handling failed: {}", e);
                Ok(Redirect::to("/login?error=auth_failed").into_response())
            }
        }
    }

    /// Logout handler
    pub async fn logout_handler(
        &self,
        current_user: OptionalUser,
        session_id: Option<String>,
        redirect: Option<String>,
    ) -> Result<Response, StatusCode> {
        let redirect_url = redirect.unwrap_or_else(|| "/".to_string());

        // If user is authenticated, clear their session
        if let (Some(user_context), Some(session_id)) = (current_user.0, session_id) {
            if let Err(e) = self.logout(&session_id).await {
                error!("Logout failed: {}", e);
            } else {
                info!("User {} logged out", user_context.user().email);
            }
        }

        // Create response with cleared session cookie
        let clear_cookie = self.clear_session_cookie();
        let mut response = Redirect::to(&redirect_url).into_response();
        
        response.headers_mut().insert(
            axum::http::header::SET_COOKIE,
            clear_cookie.to_string().parse()
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        );

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pending_auth_serialization() {
        let pending_auth = PendingAuth {
            state: "test_state".to_string(),
            redirect_url: Some("/dashboard".to_string()),
            pkce_verifier: Some("verifier".to_string()),
            created_at: chrono::Utc::now(),
        };
        
        let json = serde_json::to_string(&pending_auth).unwrap();
        let deserialized: PendingAuth = serde_json::from_str(&json).unwrap();
        
        assert_eq!(pending_auth.state, deserialized.state);
        assert_eq!(pending_auth.redirect_url, deserialized.redirect_url);
    }

    #[test]
    fn test_cookie_configuration() {
        let config = SessionCookieConfig::default();
        assert_eq!(config.name, "session_id");
        assert!(config.secure);
        assert!(config.http_only);
        
        let dev_config = SessionCookieConfig::for_development();
        assert!(!dev_config.secure);
    }
}