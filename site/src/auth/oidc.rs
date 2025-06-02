use anyhow::Result;
use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use oauth2::AccessToken;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge, RedirectUrl,
    Scope, TokenResponse, TokenUrl,
};
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

use crate::auth::types::User;
use crate::config::SiteConfig;

/// OpenID Connect discovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcConfig {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub userinfo_endpoint: String,
    pub jwks_uri: String,
    pub scopes_supported: Option<Vec<String>>,
    pub response_types_supported: Vec<String>,
    pub subject_types_supported: Vec<String>,
    pub id_token_signing_alg_values_supported: Vec<String>,
}

/// OIDC client for handling authentication flows
#[derive(Debug, Clone)]
pub struct OidcClient {
    oauth_client: BasicClient,
    oidc_config: OidcConfig,
    http_client: HttpClient,
    external_url: String,
    admin_group: Option<String>,
    qa_reviewer_group: Option<String>,
}

/// Authentication-related errors
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("OIDC discovery failed: {0}")]
    DiscoveryFailed(#[from] reqwest::Error),

    #[error("Invalid OIDC configuration: {0}")]
    InvalidConfig(String),

    #[error("OAuth2 error: {0}")]
    OAuth2Error(String),

    #[error("Token exchange failed: {0}")]
    TokenExchangeFailed(String),

    #[error("User info retrieval failed: {0}")]
    UserInfoFailed(String),

    #[error("Invalid state parameter")]
    InvalidState,

    #[error("Session not found")]
    SessionNotFound,

    #[error("Insufficient permissions")]
    InsufficientPermissions,
}

/// Authorization request state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthState {
    pub state: String,
    pub redirect_url: Option<String>,
    pub pkce_verifier: Option<String>,
}

impl OidcClient {
    /// Create a new OIDC client with discovery
    pub async fn new(config: &SiteConfig) -> Result<Self, AuthError> {
        let client_id = config
            .oidc_client_id
            .as_ref()
            .ok_or_else(|| AuthError::InvalidConfig("Missing OIDC client ID".to_string()))?;

        let client_secret = config
            .oidc_client_secret
            .as_ref()
            .ok_or_else(|| AuthError::InvalidConfig("Missing OIDC client secret".to_string()))?;

        let issuer_url = config
            .oidc_issuer_url
            .as_ref()
            .or(config.oidc_base_url.as_ref())
            .ok_or_else(|| AuthError::InvalidConfig("Missing OIDC issuer URL".to_string()))?;

        let external_url = config
            .external_url
            .as_ref()
            .ok_or_else(|| AuthError::InvalidConfig("Missing external URL".to_string()))?;

        let http_client = HttpClient::new();

        // Perform OIDC discovery
        let oidc_config = Self::discover_config(&http_client, issuer_url).await?;

        // Create OAuth2 client
        let oauth_client = BasicClient::new(
            ClientId::new(client_id.clone()),
            Some(ClientSecret::new(client_secret.clone())),
            AuthUrl::new(oidc_config.authorization_endpoint.clone())
                .map_err(|e| AuthError::InvalidConfig(format!("Invalid auth URL: {}", e)))?,
            Some(
                TokenUrl::new(oidc_config.token_endpoint.clone())
                    .map_err(|e| AuthError::InvalidConfig(format!("Invalid token URL: {}", e)))?,
            ),
        )
        .set_redirect_uri(
            RedirectUrl::new(format!("{}/auth/callback", external_url))
                .map_err(|e| AuthError::InvalidConfig(format!("Invalid redirect URL: {}", e)))?,
        );

        Ok(Self {
            oauth_client,
            oidc_config,
            http_client,
            external_url: external_url.clone(),
            admin_group: config.admin_group.clone(),
            qa_reviewer_group: config.qa_reviewer_group.clone(),
        })
    }

    /// Discover OIDC configuration from well-known endpoint
    async fn discover_config(
        http_client: &HttpClient,
        issuer_url: &str,
    ) -> Result<OidcConfig, AuthError> {
        let discovery_url = if issuer_url.ends_with('/') {
            format!("{}.well-known/openid-configuration", issuer_url)
        } else {
            format!("{}/.well-known/openid-configuration", issuer_url)
        };

        let response = http_client
            .get(&discovery_url)
            .send()
            .await?
            .error_for_status()?;

        let config: OidcConfig = response.json().await?;
        Ok(config)
    }

    /// Generate authorization URL for login flow
    pub fn get_authorization_url(&self, redirect_after_login: Option<String>) -> (Url, AuthState) {
        let state = Uuid::new_v4().to_string();

        // Generate PKCE challenge for security
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let (auth_url, _csrf_token) = self
            .oauth_client
            .authorize_url(|| CsrfToken::new(state.clone()))
            .add_scope(Scope::new("openid".to_string()))
            .add_scope(Scope::new("email".to_string()))
            .add_scope(Scope::new("profile".to_string()))
            .set_pkce_challenge(pkce_challenge)
            .url();

        let auth_state = AuthState {
            state: state.clone(),
            redirect_url: redirect_after_login,
            pkce_verifier: Some(pkce_verifier.secret().clone()),
        };

        (auth_url, auth_state)
    }

    /// Handle OAuth callback and exchange code for tokens
    pub async fn handle_callback(
        &self,
        code: &str,
        state: &str,
        stored_state: &AuthState,
    ) -> Result<User, AuthError> {
        // Verify state parameter
        if state != stored_state.state {
            return Err(AuthError::InvalidState);
        }

        // Exchange authorization code for tokens
        let token_result = self
            .oauth_client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .request_async(async_http_client)
            .await
            .map_err(|e| AuthError::TokenExchangeFailed(format!("{:?}", e)))?;

        let access_token = token_result.access_token();

        // Get user information
        let user = self.get_user_info(access_token).await?;

        Ok(user)
    }

    /// Retrieve user information from userinfo endpoint
    async fn get_user_info(&self, access_token: &AccessToken) -> Result<User, AuthError> {
        let response = self
            .http_client
            .get(&self.oidc_config.userinfo_endpoint)
            .bearer_auth(access_token.secret())
            .send()
            .await
            .map_err(|e| AuthError::UserInfoFailed(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(AuthError::UserInfoFailed(format!(
                "HTTP {}: {}",
                response.status(),
                response.status().canonical_reason().unwrap_or("Unknown")
            )));
        }

        let userinfo: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AuthError::UserInfoFailed(format!("JSON parse error: {}", e)))?;

        self.parse_user_info(userinfo)
    }

    /// Parse user information from OIDC userinfo response
    fn parse_user_info(&self, userinfo: serde_json::Value) -> Result<User, AuthError> {
        let email = userinfo
            .get("email")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AuthError::UserInfoFailed("Missing email claim".to_string()))?
            .to_string();

        let sub = userinfo
            .get("sub")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AuthError::UserInfoFailed("Missing sub claim".to_string()))?
            .to_string();

        let name = userinfo
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let preferred_username = userinfo
            .get("preferred_username")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Parse groups from various possible claim names
        let groups = self.extract_groups(&userinfo);

        // Extract additional claims
        let mut additional_claims = serde_json::Map::new();
        if let serde_json::Value::Object(obj) = userinfo {
            for (key, value) in obj {
                if !["email", "sub", "name", "preferred_username", "groups"].contains(&key.as_str())
                {
                    additional_claims.insert(key, value);
                }
            }
        }

        Ok(User {
            email,
            name,
            preferred_username,
            groups,
            sub,
            additional_claims,
        })
    }

    /// Extract groups from userinfo, trying various claim names
    fn extract_groups(&self, userinfo: &serde_json::Value) -> std::collections::HashSet<String> {
        let mut groups = std::collections::HashSet::new();

        // Try various group claim names commonly used
        for claim_name in ["groups", "memberOf", "roles", "authorities"] {
            if let Some(groups_value) = userinfo.get(claim_name) {
                match groups_value {
                    serde_json::Value::Array(arr) => {
                        for item in arr {
                            if let Some(group_str) = item.as_str() {
                                groups.insert(group_str.to_string());
                            }
                        }
                    }
                    serde_json::Value::String(s) => {
                        // Some providers return space or comma-separated strings
                        for group in s.split(|c| c == ' ' || c == ',') {
                            let group = group.trim();
                            if !group.is_empty() {
                                groups.insert(group.to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        groups
    }

    /// Get admin group configuration
    pub fn admin_group(&self) -> Option<&str> {
        self.admin_group.as_deref()
    }

    /// Get QA reviewer group configuration
    pub fn qa_reviewer_group(&self) -> Option<&str> {
        self.qa_reviewer_group.as_deref()
    }
}
