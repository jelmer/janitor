use axum::{
    extract::{Request, State},
    http::{header, HeaderMap, Method, StatusCode},
    middleware::Next,
    response::Response,
};
use metrics::{counter, histogram};
use std::{sync::Arc, time::Instant};
use tracing::{debug, info, warn, Span};
use uuid::Uuid;

use crate::config::SiteConfig;
use super::content_negotiation::{negotiate_content_type, ContentType};

/// Request context for logging and metrics
#[derive(Debug, Clone)]
pub struct RequestContext {
    pub request_id: String,
    pub method: String,
    pub path: String,
    pub user_agent: Option<String>,
    pub remote_addr: Option<String>,
    pub content_type: ContentType,
    pub start_time: Instant,
}

impl RequestContext {
    pub fn new(
        method: &Method,
        path: &str,
        headers: &HeaderMap,
        content_type: ContentType,
    ) -> Self {
        Self {
            request_id: Uuid::new_v4().to_string(),
            method: method.to_string(),
            path: path.to_string(),
            user_agent: headers
                .get(header::USER_AGENT)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string()),
            remote_addr: headers
                .get("x-forwarded-for")
                .or_else(|| headers.get("x-real-ip"))
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string()),
            content_type,
            start_time: Instant::now(),
        }
    }
}

/// Content negotiation middleware
pub async fn content_negotiation_middleware(
    mut req: Request,
    next: Next,
) -> Response {
    let headers = req.headers().clone();
    let path = req.uri().path();
    
    // Determine content type from headers and path
    let content_type = negotiate_content_type(&headers, path);
    
    // Add content type to request extensions for handlers to use
    req.extensions_mut().insert(content_type.clone());
    
    // Add Vary header to response to indicate content negotiation
    let mut response = next.run(req).await;
    response.headers_mut().insert(
        header::VARY,
        header::HeaderValue::from_static("Accept"),
    );
    
    response
}

/// Request logging middleware
pub async fn logging_middleware(
    req: Request,
    next: Next,
) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let headers = req.headers().clone();
    
    // Get content type from extension if set by content negotiation middleware
    let content_type = req.extensions()
        .get::<ContentType>()
        .cloned()
        .unwrap_or_else(|| negotiate_content_type(&headers, &path));
    
    let request_ctx = RequestContext::new(&method, &path, &headers, content_type);
    let request_id = request_ctx.request_id.clone();
    
    // Add request context to span
    let span = Span::current();
    span.record("request_id", &request_id);
    span.record("method", &request_ctx.method);
    span.record("path", &request_ctx.path);
    
    debug!(
        request_id = %request_id,
        method = %method,
        path = %path,
        user_agent = ?request_ctx.user_agent,
        remote_addr = ?request_ctx.remote_addr,
        "Request started"
    );
    
    // Add request context to request for handlers to use
    let mut req = req;
    req.extensions_mut().insert(request_ctx.clone());
    
    let response = next.run(req).await;
    let status = response.status();
    let duration = request_ctx.start_time.elapsed();
    
    // Log request completion
    if status.is_server_error() {
        tracing::error!(
            request_id = %request_id,
            method = %method,
            path = %path,
            status = %status.as_u16(),
            duration_ms = %duration.as_millis(),
            "Request completed"
        );
    } else if status.is_client_error() {
        tracing::warn!(
            request_id = %request_id,
            method = %method,
            path = %path,
            status = %status.as_u16(),
            duration_ms = %duration.as_millis(),
            "Request completed"
        );
    } else {
        tracing::info!(
            request_id = %request_id,
            method = %method,
            path = %path,
            status = %status.as_u16(),
            duration_ms = %duration.as_millis(),
            "Request completed"
        );
    }
    
    response
}

/// Metrics middleware for collecting request statistics
pub async fn metrics_middleware(
    req: Request,
    next: Next,
) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let start_time = Instant::now();
    
    // Increment request counter
    counter!("http_requests_total", "method" => method.to_string(), "path" => sanitize_path_for_metrics(&path)).increment(1);
    
    let response = next.run(req).await;
    let status = response.status();
    let duration = start_time.elapsed();
    
    // Record metrics
    let labels = &[
        ("method", method.to_string()),
        ("path", sanitize_path_for_metrics(&path)),
        ("status", status.as_u16().to_string()),
    ];
    
    counter!("http_requests_total", labels);
    histogram!("http_request_duration_seconds", labels).record(duration.as_secs_f64());
    
    // Record status code specific metrics
    match status.as_u16() {
        200..=299 => counter!("http_requests_success_total").increment(1),
        400..=499 => counter!("http_requests_client_error_total").increment(1),
        500..=599 => counter!("http_requests_server_error_total").increment(1),
        _ => {}
    }
    
    response
}

/// CORS middleware for API endpoints
pub async fn cors_middleware(
    req: Request,
    next: Next,
) -> Response {
    let origin = req.headers()
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    
    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    
    // Set CORS headers
    if let Some(origin) = origin {
        // In production, validate origin against allowed origins
        headers.insert(
            header::ACCESS_CONTROL_ALLOW_ORIGIN,
            origin.parse().unwrap_or_else(|_| header::HeaderValue::from_static("*")),
        );
    } else {
        headers.insert(
            header::ACCESS_CONTROL_ALLOW_ORIGIN,
            header::HeaderValue::from_static("*"),
        );
    }
    
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        header::HeaderValue::from_static("GET, POST, PUT, DELETE, OPTIONS"),
    );
    
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        header::HeaderValue::from_static("Content-Type, Authorization, Accept"),
    );
    
    headers.insert(
        header::ACCESS_CONTROL_MAX_AGE,
        header::HeaderValue::from_static("86400"),
    );
    
    response
}

/// Security headers middleware
pub async fn security_headers_middleware(
    req: Request,
    next: Next,
) -> Response {
    let is_https = req.uri().scheme_str() == Some("https");
    
    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    
    // Security headers
    headers.insert(
        "X-Content-Type-Options",
        header::HeaderValue::from_static("nosniff"),
    );
    
    headers.insert(
        "X-Frame-Options",
        header::HeaderValue::from_static("DENY"),
    );
    
    headers.insert(
        "X-XSS-Protection",
        header::HeaderValue::from_static("1; mode=block"),
    );
    
    headers.insert(
        "Referrer-Policy",
        header::HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    
    // Only add HSTS in production with HTTPS
    // This should be configurable based on environment
    if is_https {
        headers.insert(
            "Strict-Transport-Security",
            header::HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        );
    }
    
    response
}

/// Rate limiting middleware (basic implementation)
pub async fn rate_limiting_middleware(
    req: Request,
    next: Next,
) -> Response {
    // Get client identifier (IP address or user ID)
    let client_id = req.headers()
        .get("x-forwarded-for")
        .or_else(|| req.headers().get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");
    
    // TODO: Implement actual rate limiting logic with Redis or in-memory store
    // For now, just log and continue
    debug!("Rate limit check for client: {}", client_id);
    
    // In a real implementation, you would:
    // 1. Check current request count for client
    // 2. If over limit, return 429 Too Many Requests
    // 3. Otherwise, increment counter and continue
    
    next.run(req).await
}

/// Helper function to sanitize paths for metrics labels
fn sanitize_path_for_metrics(path: &str) -> String {
    // Replace dynamic segments with placeholders for better metric aggregation
    let mut sanitized = path.to_string();
    
    // Replace UUIDs and numeric IDs with placeholders
    sanitized = regex::Regex::new(r"/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}")
        .unwrap()
        .replace_all(&sanitized, "/{id}")
        .to_string();
    
    sanitized = regex::Regex::new(r"/\d+")
        .unwrap()
        .replace_all(&sanitized, "/{id}")
        .to_string();
    
    // Limit length to prevent metric explosion
    if sanitized.len() > 100 {
        sanitized.truncate(97);
        sanitized.push_str("...");
    }
    
    sanitized
}

/// Middleware stack builder for API routes
pub fn api_middleware_stack() -> axum::middleware::FromFnLayer<
    fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>>,
    (),
    (),
> {
    // Create a composed middleware stack
    axum::middleware::from_fn(|req, next| {
        Box::pin(async move {
            // Apply middleware in order
            let req = security_headers_middleware(req, next).await;
            // Additional middleware would be chained here
            req
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_path_for_metrics() {
        assert_eq!(
            sanitize_path_for_metrics("/api/run/12345/status"),
            "/api/run/{id}/status"
        );
        
        assert_eq!(
            sanitize_path_for_metrics("/api/user/550e8400-e29b-41d4-a716-446655440000"),
            "/api/user/{id}"
        );
        
        assert_eq!(
            sanitize_path_for_metrics("/api/static/path"),
            "/api/static/path"
        );
    }

    #[test]
    fn test_request_context_creation() {
        let mut headers = HeaderMap::new();
        headers.insert(header::USER_AGENT, "test-agent".parse().unwrap());
        
        let ctx = RequestContext::new(
            &Method::GET,
            "/api/test",
            &headers,
            ContentType::Json,
        );
        
        assert_eq!(ctx.method, "GET");
        assert_eq!(ctx.path, "/api/test");
        assert_eq!(ctx.user_agent, Some("test-agent".to_string()));
        assert_eq!(ctx.content_type, ContentType::Json);
        assert!(!ctx.request_id.is_empty());
    }
}