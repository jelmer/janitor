use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use tower_http::trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer};

use crate::{
    app::AppState,
    auth::{
        session::{SessionError, SessionManager},
        types::SessionInfo,
    },
    templates::{FlashMessage, FLASH_MESSAGES_KEY},
};

// Request logging middleware
pub async fn request_logging_middleware(
    State(_state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let start = std::time::Instant::now();

    let response = next.run(request).await;

    let duration = start.elapsed();
    let status = response.status();

    tracing::info!(
        method = %method,
        uri = %uri,
        status = %status,
        duration_ms = duration.as_millis(),
        "Request completed"
    );

    response
}

// Health check middleware for database connectivity
pub async fn health_check_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Skip health check for static assets and health endpoint itself
    let path = request.uri().path();
    if path.starts_with("/static/") || path == "/health" {
        return Ok(next.run(request).await);
    }

    // Quick database health check
    match state.database.health_check().await {
        Ok(_) => Ok(next.run(request).await),
        Err(e) => {
            tracing::error!("Database health check failed: {}", e);
            Err(StatusCode::SERVICE_UNAVAILABLE)
        }
    }
}

// Create trace layer with custom configuration
pub fn create_trace_layer() -> TraceLayer<
    tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>,
    DefaultMakeSpan,
    DefaultOnRequest,
    DefaultOnResponse,
> {
    TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::new().include_headers(false))
        .on_request(DefaultOnRequest::new().level(tracing::Level::INFO))
        .on_response(DefaultOnResponse::new().level(tracing::Level::INFO))
}

/// Middleware to inject flash messages into request extensions
pub async fn flash_middleware(
    State(state): State<AppState>, 
    mut req: Request, 
    next: Next
) -> Response {
    let flash_messages = match extract_flash_messages(&state, &req).await {
        Ok(messages) => messages,
        Err(e) => {
            tracing::warn!("Failed to extract flash messages: {}", e);
            Vec::new()
        }
    };

    // Add flash messages to request extensions so handlers can access them
    req.extensions_mut().insert(flash_messages);

    next.run(req).await
}

/// Enhanced middleware that extracts both flash messages and session information
pub async fn session_middleware(
    State(state): State<AppState>, 
    mut req: Request, 
    next: Next
) -> Response {
    let session_id = extract_session_id(&req);
    
    // Extract flash messages
    let flash_messages = match extract_flash_messages(&state, &req).await {
        Ok(messages) => messages,
        Err(e) => {
            tracing::warn!("Failed to extract flash messages: {}", e);
            Vec::new()
        }
    };

    // Extract session information if session ID is available
    let session_info = if let Some(ref session_id) = session_id {
        match extract_session_info(&state, session_id).await {
            Ok(info) => info,
            Err(e) => {
                tracing::debug!("Failed to extract session info for {}: {}", session_id, e);
                None
            }
        }
    } else {
        None
    };

    // Add to request extensions
    req.extensions_mut().insert(flash_messages);
    if let Some(session_info) = session_info {
        req.extensions_mut().insert(session_info);
    }
    if let Some(session_id) = session_id {
        req.extensions_mut().insert(SessionId(session_id));
    }

    next.run(req).await
}

/// Wrapper for session ID in request extensions
#[derive(Debug, Clone)]
pub struct SessionId(pub String);

/// Extract flash messages from session and remove them (consume once)
async fn extract_flash_messages(
    state: &AppState,
    req: &Request,
) -> Result<Vec<FlashMessage>, SessionError> {
    let session_id = match extract_session_id(req) {
        Some(session_id) => session_id,
        None => return Ok(Vec::new()),
    };

    let session_manager = SessionManager::new(state.database.pool().clone());
    
    // Try to get flash messages from session temporary data
    let messages: Vec<FlashMessage> = session_manager
        .get_temporary_data(&format!("{}:{}", FLASH_MESSAGES_KEY, session_id))
        .await?
        .unwrap_or_default();

    // Delete the flash messages after retrieving them (consume once)
    if !messages.is_empty() {
        session_manager
            .delete_temporary_data(&format!("{}:{}", FLASH_MESSAGES_KEY, session_id))
            .await?;
    }

    Ok(messages)
}

/// Extract session information from database
async fn extract_session_info(
    state: &AppState,
    session_id: &str,
) -> Result<Option<SessionInfo>, SessionError> {
    let session_manager = SessionManager::new(state.database.pool().clone());
    
    match session_manager.get_session(session_id).await {
        Ok(session_info) => Ok(Some(session_info)),
        Err(SessionError::NotFound) | Err(SessionError::Expired) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Extract session ID from request headers or cookies
fn extract_session_id(req: &Request) -> Option<String> {
    // Try to get session ID from Authorization header first
    if let Some(auth_header) = req.headers().get(header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                return Some(token.to_string());
            }
        }
    }

    // Try to get session ID from Cookie header
    if let Some(cookie_header) = req.headers().get(header::COOKIE) {
        if let Ok(cookie_str) = cookie_header.to_str() {
            return parse_session_from_cookies(cookie_str);
        }
    }

    None
}

/// Parse session ID from cookie string
fn parse_session_from_cookies(cookie_str: &str) -> Option<String> {
    for cookie in cookie_str.split(';') {
        let cookie = cookie.trim();
        if let Some((name, value)) = cookie.split_once('=') {
            if name.trim() == "session_id" {
                return Some(value.trim().to_string());
            }
        }
    }
    None
}

/// Helper function to add flash messages to a session
pub async fn add_flash_message(
    session_manager: &SessionManager,
    session_id: &str,
    message: FlashMessage,
) -> Result<(), SessionError> {
    let key = format!("{}:{}", FLASH_MESSAGES_KEY, session_id);
    
    // Get existing messages
    let mut messages: Vec<FlashMessage> = session_manager
        .get_temporary_data(&key)
        .await?
        .unwrap_or_default();

    // Add new message
    messages.push(message);

    // Store updated messages with 1 hour expiration
    session_manager
        .store_temporary_data(&key, &messages, std::time::Duration::from_secs(3600))
        .await?;

    Ok(())
}

/// Helper function to add multiple flash messages to a session
pub async fn add_flash_messages(
    session_manager: &SessionManager,
    session_id: &str,
    new_messages: Vec<FlashMessage>,
) -> Result<(), SessionError> {
    if new_messages.is_empty() {
        return Ok(());
    }

    let key = format!("{}:{}", FLASH_MESSAGES_KEY, session_id);
    
    // Get existing messages
    let mut messages: Vec<FlashMessage> = session_manager
        .get_temporary_data(&key)
        .await?
        .unwrap_or_default();

    // Add new messages
    messages.extend(new_messages);

    // Store updated messages with 1 hour expiration
    session_manager
        .store_temporary_data(&key, &messages, std::time::Duration::from_secs(3600))
        .await?;

    Ok(())
}

/// Extension trait for easy flash message access in handlers
pub trait FlashMessageExtension {
    fn flash_messages(&self) -> Vec<FlashMessage>;
}

impl FlashMessageExtension for axum::http::Extensions {
    fn flash_messages(&self) -> Vec<FlashMessage> {
        self.get::<Vec<FlashMessage>>()
            .cloned()
            .unwrap_or_default()
    }
}

/// Extension trait for easy session access in handlers
pub trait SessionExtension {
    fn session_info(&self) -> Option<&SessionInfo>;
    fn session_id(&self) -> Option<&str>;
    fn is_authenticated(&self) -> bool;
}

impl SessionExtension for axum::http::Extensions {
    fn session_info(&self) -> Option<&SessionInfo> {
        self.get::<SessionInfo>()
    }

    fn session_id(&self) -> Option<&str> {
        self.get::<SessionId>().map(|s| s.0.as_str())
    }

    fn is_authenticated(&self) -> bool {
        self.session_info().is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderValue, Method};
    use crate::templates::{FlashCategory, FlashMessage};

    #[test]
    fn test_parse_session_from_cookies() {
        assert_eq!(
            parse_session_from_cookies("session_id=test123; other=value"),
            Some("test123".to_string())
        );

        assert_eq!(
            parse_session_from_cookies("other=value; session_id=test456"),
            Some("test456".to_string())
        );

        assert_eq!(
            parse_session_from_cookies("session_id=test789"),
            Some("test789".to_string())
        );

        assert_eq!(
            parse_session_from_cookies("other=value"),
            None
        );

        assert_eq!(
            parse_session_from_cookies(""),
            None
        );
    }

    #[test]
    fn test_extract_session_id_from_auth_header() {
        let req = Request::builder()
            .method(Method::GET)
            .uri("/")
            .header(header::AUTHORIZATION, "Bearer test-session-123")
            .body(axum::body::Body::empty())
            .unwrap();

        assert_eq!(extract_session_id(&req), Some("test-session-123".to_string()));
    }

    #[test]
    fn test_extract_session_id_from_cookie() {
        let req = Request::builder()
            .method(Method::GET)
            .uri("/")
            .header(header::COOKIE, "session_id=cookie-session-456; other=value")
            .body(axum::body::Body::empty())
            .unwrap();

        assert_eq!(extract_session_id(&req), Some("cookie-session-456".to_string()));
    }

    #[test]
    fn test_extract_session_id_none() {
        let req = Request::builder()
            .method(Method::GET)
            .uri("/")
            .body(axum::body::Body::empty())
            .unwrap();

        assert_eq!(extract_session_id(&req), None);
    }

    #[test]
    fn test_flash_message_extension() {
        let mut extensions = axum::http::Extensions::new();
        
        // Test empty case
        assert_eq!(extensions.flash_messages(), Vec::<FlashMessage>::new());
        
        // Test with messages
        let messages = vec![
            FlashMessage::success("Success message".to_string()),
            FlashMessage::error("Error message".to_string()),
        ];
        
        extensions.insert(messages.clone());
        let retrieved = extensions.flash_messages();
        
        assert_eq!(retrieved.len(), 2);
        assert_eq!(retrieved[0].category, FlashCategory::Success);
        assert_eq!(retrieved[0].message, "Success message");
        assert_eq!(retrieved[1].category, FlashCategory::Error);
        assert_eq!(retrieved[1].message, "Error message");
    }

    #[test]
    fn test_flash_messages_key_constant() {
        assert_eq!(FLASH_MESSAGES_KEY, "flash_messages");
    }

    #[test]
    fn test_session_id_wrapper() {
        let session_id = SessionId("test-session-123".to_string());
        assert_eq!(session_id.0, "test-session-123");
    }

    #[test]
    fn test_session_extension() {
        use crate::auth::types::{SessionInfo, User};
        use std::collections::HashSet;

        let mut extensions = axum::http::Extensions::new();
        
        // Test when no session is present
        assert!(extensions.session_info().is_none());
        assert!(extensions.session_id().is_none());
        assert!(!extensions.is_authenticated());
        
        // Add session info
        let user = User {
            email: "test@example.com".to_string(),
            name: Some("Test User".to_string()),
            preferred_username: Some("testuser".to_string()),
            groups: HashSet::new(),
            sub: "test123".to_string(),
            additional_claims: serde_json::Map::new(),
        };
        
        let session_info = SessionInfo::new(user);
        let session_id = SessionId("session-456".to_string());
        
        extensions.insert(session_info.clone());
        extensions.insert(session_id);
        
        // Test when session is present
        assert!(extensions.session_info().is_some());
        assert_eq!(extensions.session_info().unwrap().user.email, "test@example.com");
        assert_eq!(extensions.session_id().unwrap(), "session-456");
        assert!(extensions.is_authenticated());
    }
}
