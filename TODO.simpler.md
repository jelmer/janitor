# TODO: Simplify and Deduplicate Janitor Rust Codebase

This document outlines opportunities to simplify the Janitor codebase by eliminating duplicate code and creating shared abstractions.

## Overview

The Janitor codebase has evolved with multiple services, each implementing similar patterns independently. While a `shared_config` module exists, it's underutilized, and services duplicate significant amounts of boilerplate code.

## Major Areas for Simplification

### 1. Configuration Management

**Current state:**
- Each service defines its own config struct with nested `DatabaseConfig`, `RedisConfig`, `WebConfig`
- Shared config modules exist in `src/shared_config/` but are rarely used
- Services parse environment variables and config files independently

**Proposed changes:**
- Make all services use `shared_config::DatabaseConfig`, `shared_config::RedisConfig`, `shared_config::WebConfig`
- Implement the `ConfigLoader` trait from shared_config for all service configs
- Remove duplicate environment variable parsing logic

**Files to modify:**
- `runner/src/config.rs`
- `site/src/config.rs`
- `archive/src/config.rs`
- `auto-upload/src/config.rs`
- `git-store/src/config.rs`
- `bzr-store/src/config.rs`

### 2. Web Server Setup

**Current state:**
- Every service has its own router creation with similar middleware setup
- Health check endpoints are duplicated with varying implementations
- CORS, logging, and tracing middleware configured separately in each service

**Proposed changes:**
- Create a `shared_web` module with:
  ```rust
  pub fn create_base_router<S>() -> Router<S> 
  where S: Clone + Send + Sync + 'static
  {
      Router::new()
          .route("/health", get(health_check))
          .route("/ready", get(ready_check))
          .layer(ServiceBuilder::new()
              .layer(TraceLayer::new_for_http())
              .layer(CorsLayer::permissive())
              .layer(CompressionLayer::new()))
  }
  ```
- Services extend the base router with their specific routes
- Standardize health check response format

**Files to modify:**
- Create `src/shared_web/mod.rs`
- Update all `main.rs` and `web.rs` files in services

### 3. Error Handling

**Current state:**
- Each service defines its own error enum with similar variants
- Duplicate From implementations for common errors (sqlx, io, redis)
- Similar HTTP status code mappings repeated everywhere

**Proposed changes:**
- Create a base error type in the root crate:
  ```rust
  #[derive(Error, Debug)]
  pub enum BaseError {
      #[error("Database error: {0}")]
      Database(#[from] sqlx::Error),
      #[error("Redis error: {0}")]
      Redis(#[from] redis::RedisError),
      #[error("I/O error: {0}")]
      Io(#[from] std::io::Error),
      #[error("Configuration error: {0}")]
      Config(String),
      #[error("Not found: {0}")]
      NotFound(String),
  }
  ```
- Services can wrap or extend this for service-specific errors
- Create shared IntoResponse implementation for consistent error responses

**Files to modify:**
- Create `src/error.rs` with base types
- Update all service-specific error modules

### 4. Database Connection Management

**Current state:**
- Some services use `src/state.rs::create_pool`, others implement their own
- Duplicate migration running logic
- Similar connection options but configured differently

**Proposed changes:**
- Enforce use of shared `create_pool` function everywhere
- Create a `DatabaseManager` trait that handles pool creation and migrations
- Remove duplicate pool creation code

**Files to modify:**
- Enhance `src/state.rs` or create `src/database/manager.rs`
- Update all database modules in services

### 5. Redis Client Management

**Current state:**
- Multiple implementations of Redis connection with retry logic
- Different pub/sub implementations
- Shared config exists but services implement their own clients

**Proposed changes:**
- Create a single `RedisManager` in the root crate:
  ```rust
  pub struct RedisManager {
      client: Client,
      connection_pool: Pool<RedisConnectionManager>,
  }
  
  impl RedisManager {
      pub async fn new(config: &RedisConfig) -> Result<Self, RedisError>;
      pub async fn get_connection(&self) -> Result<Connection, RedisError>;
      pub async fn subscribe(&self, channels: &[&str]) -> Result<PubSub, RedisError>;
  }
  ```

**Files to modify:**
- Create `src/redis/manager.rs`
- Remove service-specific Redis implementations

### 6. Authentication Middleware

**Current state:**
- Runner has worker authentication with Basic Auth
- Site has session-based authentication with OIDC
- No shared authentication primitives

**Proposed changes:**
- Create authentication traits and common extractors:
  ```rust
  pub trait AuthProvider: Send + Sync {
      async fn authenticate(&self, req: &Request) -> Result<AuthContext, AuthError>;
  }
  
  pub enum AuthContext {
      Worker(WorkerAuth),
      User(UserAuth),
      Service(ServiceAuth),
  }
  ```
- Implement providers for different auth methods
- Share bearer token extraction logic

**Files to create:**
- `src/auth/mod.rs`
- `src/auth/providers/basic.rs`
- `src/auth/providers/session.rs`
- `src/auth/middleware.rs`

### 7. Test Utilities

**Current state:**
- `src/test_utils.rs` has TestDatabase
- Each service has its own test utilities
- Duplicate mock implementations

**Proposed changes:**
- Create a `janitor-test-utils` crate with:
  - Common test database setup
  - Mock Redis implementation
  - Test fixture management
  - Common test assertions
- Services depend on this crate in dev-dependencies

**Files to create:**
- `test-utils/Cargo.toml`
- `test-utils/src/lib.rs`

### 8. HTTP Client Configuration

**Current state:**
- Services create `reqwest::Client` independently
- No consistent timeout or retry configuration
- No shared user agent or default headers

**Proposed changes:**
- Create an HTTP client factory:
  ```rust
  pub struct HttpClientConfig {
      timeout: Duration,
      connect_timeout: Duration,
      pool_idle_timeout: Duration,
      user_agent: String,
      retry_count: u32,
  }
  
  pub fn create_http_client(config: &HttpClientConfig) -> Client;
  ```

**Files to create:**
- `src/http/client.rs`

## Implementation Priority

1. **High Priority** (Most impact, least disruption):
   - Shared web server setup (health checks, middleware)
   - Base error types
   - Test utilities crate

2. **Medium Priority** (Good impact, moderate effort):
   - Configuration management consolidation
   - Database connection management
   - HTTP client factory

3. **Lower Priority** (Complex, may require careful migration):
   - Redis client unification
   - Authentication middleware abstraction

## Migration Strategy

1. Create new shared modules without breaking existing code
2. Migrate one service at a time to use shared modules
3. Remove old implementations after all services migrated
4. Add tests to ensure behavior remains consistent

## Expected Benefits

- **Reduced code size**: Estimate 20-30% reduction in total Rust LOC
- **Easier maintenance**: Bug fixes and improvements apply everywhere
- **Consistent behavior**: All services behave the same for common operations
- **Faster development**: New services can reuse existing patterns
- **Better testing**: Shared test utilities make tests more comprehensive

## Metrics for Success

- Number of duplicate implementations removed
- Lines of code reduced
- Time to implement new services decreased
- Number of service-specific bugs related to common functionality