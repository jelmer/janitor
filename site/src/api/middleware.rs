use axum::{
    extract::Request,
    http::{header, HeaderMap, Method, StatusCode},
    middleware::Next,
    response::Response,
};
use metrics::{counter, histogram};
use redis::AsyncCommands;
use serde_json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, error, warn, Span};
use uuid::Uuid;

use super::content_negotiation::{negotiate_content_type, ContentType};

/// Rate limiting configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per window
    pub max_requests: u32,
    /// Window duration in seconds
    pub window_seconds: u64,
    /// Key prefix for Redis storage
    pub key_prefix: String,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 100,      // 100 requests
            window_seconds: 60,     // per minute
            key_prefix: "rate_limit".to_string(),
        }
    }
}

/// Rate limiter implementation with Redis backend
#[derive(Debug)]
pub struct RateLimiter {
    redis_client: Option<redis::Client>,
    memory_store: Arc<Mutex<HashMap<String, (u32, Instant)>>>,
    config: RateLimitConfig,
}

impl RateLimiter {
    /// Create a new rate limiter with Redis backend
    pub fn new_with_redis(redis_url: &str, config: RateLimitConfig) -> Result<Self, redis::RedisError> {
        let redis_client = redis::Client::open(redis_url)?;
        Ok(Self {
            redis_client: Some(redis_client),
            memory_store: Arc::new(Mutex::new(HashMap::new())),
            config,
        })
    }

    /// Create a new rate limiter with in-memory backend (for testing/fallback)
    pub fn new_memory_only(config: RateLimitConfig) -> Self {
        Self {
            redis_client: None,
            memory_store: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }

    /// Check if request is allowed and increment counter
    pub async fn check_rate_limit(&self, client_id: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(ref redis_client) = self.redis_client {
            self.check_redis_rate_limit(redis_client, client_id).await
        } else {
            self.check_memory_rate_limit(client_id).await
        }
    }

    /// Redis-based rate limiting using sliding window
    async fn check_redis_rate_limit(
        &self,
        redis_client: &redis::Client,
        client_id: &str,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = redis_client.get_multiplexed_async_connection().await?;
        let key = format!("{}:{}", self.config.key_prefix, client_id);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();

        // Use Redis sliding window approach with sorted sets
        let window_start = now - self.config.window_seconds;

        // Remove old entries (use correct Redis command)
        let _: () = redis::cmd("ZREMRANGEBYSCORE")
            .arg(&key)
            .arg(0)
            .arg(window_start as f64)
            .query_async(&mut conn)
            .await?;

        // Count current entries
        let current_count: u32 = conn.zcard(&key).await?;

        if current_count >= self.config.max_requests {
            // Rate limit exceeded
            return Ok(false);
        }

        // Add current request
        let _: () = conn.zadd(&key, now, now).await?;
        
        // Set expiration for cleanup
        let _: () = conn.expire(&key, (self.config.window_seconds + 10) as i64).await?;

        Ok(true)
    }

    /// Memory-based rate limiting for fallback
    async fn check_memory_rate_limit(
        &self,
        client_id: &str,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let mut store = self.memory_store.lock().await;
        let now = Instant::now();
        let window_duration = Duration::from_secs(self.config.window_seconds);

        // Clean up old entries
        store.retain(|_, (_, timestamp)| now.duration_since(*timestamp) < window_duration);

        // Check current count for client
        if let Some((count, timestamp)) = store.get_mut(client_id) {
            if now.duration_since(*timestamp) < window_duration {
                if *count >= self.config.max_requests {
                    return Ok(false);
                }
                *count += 1;
            } else {
                // Reset window
                *count = 1;
                *timestamp = now;
            }
        } else {
            // First request for this client
            store.insert(client_id.to_string(), (1, now));
        }

        Ok(true)
    }

    /// Get current usage for a client (for monitoring)
    pub async fn get_usage(&self, client_id: &str) -> Result<u32, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(ref redis_client) = self.redis_client {
            let mut conn = redis_client.get_multiplexed_async_connection().await?;
            let key = format!("{}:{}", self.config.key_prefix, client_id);
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs();
            let window_start = now - self.config.window_seconds;

            // Remove old entries first
            let _: () = redis::cmd("ZREMRANGEBYSCORE")
                .arg(&key)
                .arg(0)
                .arg(window_start as f64)
                .query_async(&mut conn)
                .await?;
            
            // Get current count
            let count: u32 = conn.zcard(&key).await?;
            Ok(count)
        } else {
            let store = self.memory_store.lock().await;
            let now = Instant::now();
            let window_duration = Duration::from_secs(self.config.window_seconds);

            if let Some((count, timestamp)) = store.get(client_id) {
                if now.duration_since(*timestamp) < window_duration {
                    Ok(*count)
                } else {
                    Ok(0)
                }
            } else {
                Ok(0)
            }
        }
    }
}

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
pub async fn content_negotiation_middleware(mut req: Request, next: Next) -> Response {
    let headers = req.headers().clone();
    let path = req.uri().path();

    // Determine content type from headers and path
    let content_type = negotiate_content_type(&headers, path);

    // Add content type to request extensions for handlers to use
    req.extensions_mut().insert(content_type.clone());

    // Add Vary header to response to indicate content negotiation
    let mut response = next.run(req).await;
    response
        .headers_mut()
        .insert(header::VARY, header::HeaderValue::from_static("Accept"));

    response
}

/// Request logging middleware
pub async fn logging_middleware(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let headers = req.headers().clone();

    // Get content type from extension if set by content negotiation middleware
    let content_type = req
        .extensions()
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
pub async fn metrics_middleware(req: Request, next: Next) -> Response {
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
pub async fn cors_middleware(req: Request, next: Next) -> Response {
    let origin = req
        .headers()
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
            origin
                .parse()
                .unwrap_or_else(|_| header::HeaderValue::from_static("*")),
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
pub async fn security_headers_middleware(req: Request, next: Next) -> Response {
    let is_https = req.uri().scheme_str() == Some("https");

    let mut response = next.run(req).await;
    let headers = response.headers_mut();

    // Security headers
    headers.insert(
        "X-Content-Type-Options",
        header::HeaderValue::from_static("nosniff"),
    );

    headers.insert("X-Frame-Options", header::HeaderValue::from_static("DENY"));

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

/// Rate limiting middleware with functional implementation
pub async fn rate_limiting_middleware(req: Request, next: Next) -> Response {
    // Use a basic in-memory rate limiter as fallback
    static RATE_LIMITER: std::sync::OnceLock<Arc<RateLimiter>> = std::sync::OnceLock::new();
    let limiter = RATE_LIMITER.get_or_init(|| {
        Arc::new(RateLimiter::new_memory_only(RateLimitConfig::default()))
    });

    let client_id = get_client_identifier(&req);

    match limiter.check_rate_limit(&client_id).await {
        Ok(allowed) => {
            if allowed {
                debug!("Rate limit check passed for client: {}", client_id);
                next.run(req).await
            } else {
                warn!("Rate limit exceeded for client: {}", client_id);
                
                Response::builder()
                    .status(StatusCode::TOO_MANY_REQUESTS)
                    .header("content-type", "application/json")
                    .header("retry-after", "60")
                    .body(
                        serde_json::json!({
                            "error": "too_many_requests",
                            "message": "Rate limit exceeded. Please try again later.",
                            "retry_after": 60
                        })
                        .to_string()
                        .into(),
                    )
                    .unwrap()
            }
        }
        Err(e) => {
            error!("Rate limit check failed for client {}: {}", client_id, e);
            // On error, allow the request through but log the issue
            next.run(req).await
        }
    }
}

/// Rate limiting middleware with configurable limiter
pub fn rate_limiting_middleware_with_limiter(
    limiter: Arc<RateLimiter>,
) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>> + Clone {
    move |req: Request, next: Next| {
        let limiter = limiter.clone();
        Box::pin(async move {
            let client_id = get_client_identifier(&req);

            match limiter.check_rate_limit(&client_id).await {
                Ok(allowed) => {
                    if allowed {
                        debug!("Rate limit check passed for client: {}", client_id);
                        next.run(req).await
                    } else {
                        warn!("Rate limit exceeded for client: {}", client_id);
                        
                        Response::builder()
                            .status(StatusCode::TOO_MANY_REQUESTS)
                            .header("content-type", "application/json")
                            .header("retry-after", "60")
                            .body(
                                serde_json::json!({
                                    "error": "too_many_requests",
                                    "message": "Rate limit exceeded. Please try again later.",
                                    "retry_after": 60
                                })
                                .to_string()
                                .into(),
                            )
                            .unwrap()
                    }
                }
                Err(e) => {
                    error!("Rate limit check failed for client {}: {}", client_id, e);
                    next.run(req).await
                }
            }
        })
    }
}

/// Extract client identifier for rate limiting
fn get_client_identifier(req: &Request) -> String {
    // Priority order: API key, user session, forwarded IP, real IP
    
    // 1. Check for API key in headers
    if let Some(api_key) = req.headers().get("x-api-key").and_then(|v| v.to_str().ok()) {
        return format!("api:{}", api_key);
    }

    // 2. Check for authenticated user session
    if let Some(session_id) = req.headers().get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|auth| auth.strip_prefix("Bearer "))
    {
        return format!("session:{}", session_id);
    }

    // 3. Fall back to IP-based identification
    let ip = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|forwarded| forwarded.split(',').next())
        .or_else(|| req.headers().get("x-real-ip").and_then(|v| v.to_str().ok()))
        .unwrap_or("unknown");

    format!("ip:{}", ip.trim())
}

#[cfg(test)]
mod rate_limit_tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_memory_rate_limiter_basic() {
        let config = RateLimitConfig {
            max_requests: 5,
            window_seconds: 1,
            key_prefix: "test".to_string(),
        };
        let limiter = RateLimiter::new_memory_only(config);

        // First 5 requests should be allowed
        for _ in 0..5 {
            assert!(limiter.check_rate_limit("client1").await.unwrap());
        }

        // 6th request should be denied
        assert!(!limiter.check_rate_limit("client1").await.unwrap());

        // Different client should still be allowed
        assert!(limiter.check_rate_limit("client2").await.unwrap());
    }

    #[tokio::test]
    async fn test_memory_rate_limiter_window_reset() {
        let config = RateLimitConfig {
            max_requests: 2,
            window_seconds: 1,
            key_prefix: "test".to_string(),
        };
        let limiter = RateLimiter::new_memory_only(config);

        // Use up the limit
        assert!(limiter.check_rate_limit("client1").await.unwrap());
        assert!(limiter.check_rate_limit("client1").await.unwrap());
        assert!(!limiter.check_rate_limit("client1").await.unwrap());

        // Wait for window to reset
        sleep(Duration::from_secs(2)).await;

        // Should be allowed again
        assert!(limiter.check_rate_limit("client1").await.unwrap());
    }

    #[tokio::test]
    async fn test_rate_limiter_usage_tracking() {
        let config = RateLimitConfig {
            max_requests: 10,
            window_seconds: 60,
            key_prefix: "test".to_string(),
        };
        let limiter = RateLimiter::new_memory_only(config);

        // Make some requests
        for _ in 0..3 {
            limiter.check_rate_limit("client1").await.unwrap();
        }

        // Check usage
        let usage = limiter.get_usage("client1").await.unwrap();
        assert_eq!(usage, 3);

        // Different client should have 0 usage
        let usage = limiter.get_usage("client2").await.unwrap();
        assert_eq!(usage, 0);
    }

    #[test]
    fn test_get_client_identifier() {
        use axum::{body::Body, http::Request};

        // Test API key identification
        let req = Request::builder()
            .header("x-api-key", "test-api-key")
            .body(Body::empty())
            .unwrap();
        assert_eq!(get_client_identifier(&req), "api:test-api-key");

        // Test bearer token identification
        let req = Request::builder()
            .header("authorization", "Bearer test-session-token")
            .body(Body::empty())
            .unwrap();
        assert_eq!(get_client_identifier(&req), "session:test-session-token");

        // Test IP identification
        let req = Request::builder()
            .header("x-forwarded-for", "192.168.1.1, 10.0.0.1")
            .body(Body::empty())
            .unwrap();
        assert_eq!(get_client_identifier(&req), "ip:192.168.1.1");

        // Test real IP fallback
        let req = Request::builder()
            .header("x-real-ip", "203.0.113.1")
            .body(Body::empty())
            .unwrap();
        assert_eq!(get_client_identifier(&req), "ip:203.0.113.1");

        // Test unknown fallback
        let req = Request::builder().body(Body::empty()).unwrap();
        assert_eq!(get_client_identifier(&req), "ip:unknown");
    }
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

            // Additional middleware would be chained here
            security_headers_middleware(req, next).await
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

        let ctx = RequestContext::new(&Method::GET, "/api/test", &headers, ContentType::Json);

        assert_eq!(ctx.method, "GET");
        assert_eq!(ctx.path, "/api/test");
        assert_eq!(ctx.user_agent, Some("test-agent".to_string()));
        assert_eq!(ctx.content_type, ContentType::Json);
        assert!(!ctx.request_id.is_empty());
    }
}
