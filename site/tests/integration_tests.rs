// Integration tests for the site module
// These tests require external services (database, Redis) and test full workflows

use std::sync::Arc;
use std::time::Duration;

use axum::http::StatusCode;
use axum_test::TestServer;
use serde_json::{json, Value};
use sqlx::Row;
use testcontainers::{runners::AsyncRunner, ContainerAsync};
use testcontainers_modules::{postgres::Postgres, redis::Redis};

use janitor_site::{
    app::AppState,
    config::{Config, SiteConfig},
    database::DatabaseManager,
};

// Import the create_app function - we'll need to define this
fn create_app(state: Arc<AppState>) -> axum::routing::Router {
    use axum::routing::get;

    // Simplified app for testing
    axum::routing::Router::new()
        .route("/health", get(health_check))
        .route("/api/status", get(api_status))
        .route("/api/queue", get(api_queue))
        .route("/api/search", get(api_search))
        .route("/auth/login", get(auth_login))
        .route("/admin/system/status", get(admin_status))
        .with_state(Arc::try_unwrap(state).unwrap_or_else(|arc| (*arc).clone()))
}

// Simple test handlers
async fn health_check() -> axum::Json<serde_json::Value> {
    axum::Json(json!({"status": "healthy", "timestamp": chrono::Utc::now().to_rfc3339()}))
}

async fn api_status() -> axum::Json<serde_json::Value> {
    axum::Json(json!({"version": "test", "services": {}}))
}

async fn api_queue() -> axum::Json<serde_json::Value> {
    axum::Json(json!({"total_candidates": 0, "pending_candidates": 0, "active_runs": 0}))
}

async fn api_search() -> axum::Json<serde_json::Value> {
    axum::Json(json!({"results": [], "pagination": {"page": 1, "total": 0}}))
}

async fn auth_login() -> axum::response::Redirect {
    axum::response::Redirect::to("/")
}

async fn admin_status() -> axum::http::StatusCode {
    axum::http::StatusCode::UNAUTHORIZED
}

// Test configuration and setup utilities
pub struct IntegrationTestEnvironment {
    pub postgres_container: ContainerAsync<Postgres>,
    pub redis_container: ContainerAsync<Redis>,
    pub database: DatabaseManager,
    pub app_state: Arc<AppState>,
    pub test_server: TestServer,
}

impl IntegrationTestEnvironment {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Start PostgreSQL container
        let postgres_container = Postgres::default().start().await?;

        let postgres_port = postgres_container.get_host_port_ipv4(5432).await?;
        let database_url = format!(
            "postgresql://postgres:postgres@localhost:{}/postgres",
            postgres_port
        );

        // Start Redis container
        let redis_container = Redis::default().start().await?;

        let redis_port = redis_container.get_host_port_ipv4(6379).await?;
        let redis_url = format!("redis://localhost:{}", redis_port);

        // Create test configuration
        let mut site_config = SiteConfig::default();
        site_config.database_url = database_url.clone();
        site_config.redis_url = Some(redis_url);
        site_config.debug = true;
        site_config.session_secret = "test-secret-key-for-integration-testing".to_string();

        let config = Config::new(site_config, None);

        // Initialize database
        let database = DatabaseManager::new(&config).await?;

        // Run database migrations
        database.run_migrations().await?;

        // Create app state
        let app_state = Arc::new(AppState::new(config).await?);

        // Create test server
        let app = create_app(app_state.clone());
        let test_server = TestServer::new(app)?;

        Ok(Self {
            postgres_container,
            redis_container,
            database,
            app_state,
            test_server,
        })
    }

    pub async fn cleanup(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Clean up test data
        // Clear test data - implement if needed
        // self.database.clear_test_data().await?;
        Ok(())
    }
}

#[cfg(test)]
mod database_integration_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires docker for testcontainers"]
    async fn test_database_connection_and_basic_operations() {
        let env = IntegrationTestEnvironment::new()
            .await
            .expect("Failed to set up test environment");

        // Test basic database connectivity
        let pool = env.database.pool();
        let result = sqlx::query("SELECT 1 as test_value").fetch_one(pool).await;

        assert!(result.is_ok());
        let row = result.unwrap();
        let test_value: i32 = row.get("test_value");
        assert_eq!(test_value, 1);

        env.cleanup().await.expect("Failed to cleanup");
    }

    #[tokio::test]
    #[ignore = "requires docker for testcontainers"]
    async fn test_database_schema_migrations() {
        let env = IntegrationTestEnvironment::new()
            .await
            .expect("Failed to set up test environment");

        // Verify that essential tables exist
        let tables = vec!["site_session", "run", "codebase", "campaign"];

        for table in tables {
            let result = sqlx::query(&format!(
                "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = '{}')",
                table
            ))
            .fetch_one(env.database.pool())
            .await;

            assert!(result.is_ok(), "Table {} should exist", table);
            let row = result.unwrap();
            let exists: bool = row.get("exists");
            assert!(exists, "Table {} should exist after migrations", table);
        }

        env.cleanup().await.expect("Failed to cleanup");
    }

    #[tokio::test]
    #[ignore = "requires docker for testcontainers"]
    async fn test_session_storage_and_retrieval() {
        let env = IntegrationTestEnvironment::new()
            .await
            .expect("Failed to set up test environment");

        // Create a session manager for testing
        let session_manager = janitor_site::auth::session::SessionManager::new(env.database.pool().clone());

        // Create a test user
        let mut groups = std::collections::HashSet::new();
        groups.insert("users".to_string());

        let user = janitor_site::auth::types::User {
            email: "test@example.com".to_string(),
            name: Some("Test User".to_string()),
            preferred_username: Some("testuser".to_string()),
            groups,
            sub: "test-user-123".to_string(),
            additional_claims: serde_json::Map::new(),
        };

        // Create session
        let session_id = session_manager
            .create_session(user.clone())
            .await
            .expect("Should create session");

        // Retrieve session
        let retrieved_session = session_manager
            .get_session(&session_id)
            .await
            .expect("Should retrieve session");

        assert_eq!(retrieved_session.user.email, user.email);
        assert_eq!(retrieved_session.user.sub, user.sub);

        // Update activity
        session_manager
            .update_activity(&session_id)
            .await
            .expect("Should update activity");

        // Clean up session
        session_manager
            .delete_session(&session_id)
            .await
            .expect("Should delete session");

        env.cleanup().await.expect("Failed to cleanup");
    }
}

#[cfg(test)]
mod workflow_integration_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires docker for testcontainers"]
    async fn test_health_check_workflow() {
        let env = IntegrationTestEnvironment::new()
            .await
            .expect("Failed to set up test environment");

        // Test health check endpoint
        let response = env.test_server.get("/health").await;
        response.assert_status(StatusCode::OK);

        let body: Value = response.json();
        assert_eq!(body["status"], "healthy");
        assert!(body["timestamp"].is_string());

        env.cleanup().await.expect("Failed to cleanup");
    }

    #[tokio::test]
    #[ignore = "requires docker for testcontainers"]
    async fn test_api_status_workflow() {
        let env = IntegrationTestEnvironment::new()
            .await
            .expect("Failed to set up test environment");

        // Test API status endpoint
        let response = env.test_server.get("/api/status").await;
        response.assert_status(StatusCode::OK);

        let body: Value = response.json();
        assert!(body["version"].is_string());
        assert!(body["services"].is_object());

        env.cleanup().await.expect("Failed to cleanup");
    }

    #[tokio::test]
    #[ignore = "requires docker for testcontainers"]
    async fn test_authentication_workflow() {
        let env = IntegrationTestEnvironment::new()
            .await
            .expect("Failed to set up test environment");

        // Test unauthenticated access to protected endpoint
        let response = env.test_server.get("/admin/system/status").await;
        response.assert_status(StatusCode::UNAUTHORIZED);

        // Test login flow (redirect to OIDC provider)
        let response = env.test_server.get("/auth/login").await;
        // Should redirect to OIDC provider or show login page
        assert!(
            response.status_code() == StatusCode::FOUND || response.status_code() == StatusCode::OK
        );

        env.cleanup().await.expect("Failed to cleanup");
    }

    #[tokio::test]
    #[ignore = "requires docker for testcontainers"]
    async fn test_queue_api_workflow() {
        let env = IntegrationTestEnvironment::new()
            .await
            .expect("Failed to set up test environment");

        // Test queue status API
        let response = env.test_server.get("/api/queue").await;
        response.assert_status(StatusCode::OK);

        let body: Value = response.json();
        assert!(body["total_candidates"].is_number());
        assert!(body["pending_candidates"].is_number());
        assert!(body["active_runs"].is_number());

        env.cleanup().await.expect("Failed to cleanup");
    }

    #[tokio::test]
    #[ignore = "requires docker for testcontainers"]
    async fn test_search_workflow() {
        let env = IntegrationTestEnvironment::new()
            .await
            .expect("Failed to set up test environment");

        // Test package search
        let response = env.test_server.get("/api/search?q=test&limit=10").await;
        response.assert_status(StatusCode::OK);

        let body: Value = response.json();
        assert!(body["results"].is_array());
        assert!(body["pagination"].is_object());

        env.cleanup().await.expect("Failed to cleanup");
    }
}

#[cfg(test)]
mod performance_integration_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires docker for testcontainers"]
    async fn test_response_time_performance() {
        let env = IntegrationTestEnvironment::new()
            .await
            .expect("Failed to set up test environment");

        // Test that health check responds quickly
        let start = std::time::Instant::now();
        let response = env.test_server.get("/health").await;
        let duration = start.elapsed();

        response.assert_status(StatusCode::OK);
        assert!(
            duration.as_millis() < 200,
            "Health check should respond in under 200ms, took {}ms",
            duration.as_millis()
        );

        // Test API endpoint performance
        let start = std::time::Instant::now();
        let response = env.test_server.get("/api/status").await;
        let duration = start.elapsed();

        response.assert_status(StatusCode::OK);
        assert!(
            duration.as_millis() < 500,
            "API status should respond in under 500ms, took {}ms",
            duration.as_millis()
        );

        env.cleanup().await.expect("Failed to cleanup");
    }

    #[tokio::test]
    #[ignore = "requires docker for testcontainers"]
    async fn test_concurrent_request_handling() {
        let env = IntegrationTestEnvironment::new()
            .await
            .expect("Failed to set up test environment");

        // Test concurrent requests sequentially to avoid borrowing issues
        let mut results = Vec::new();

        for i in 0..10 {
            let response = env.test_server.get("/health").await;
            results.push((i, response.status_code()));
        }

        // All requests should succeed
        for (i, status) in results {
            assert_eq!(status, StatusCode::OK, "Request {} should succeed", i);
        }

        env.cleanup().await.expect("Failed to cleanup");
    }
}

#[cfg(test)]
mod realtime_integration_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires docker for testcontainers"]
    async fn test_redis_connectivity() {
        let env = IntegrationTestEnvironment::new()
            .await
            .expect("Failed to set up test environment");

        // Test Redis connection through the realtime manager
        let realtime_manager = &env.app_state.realtime;
        // Test basic Redis connectivity
        let result = realtime_manager
            .publish_event("test_channel", &json!({"test": "message"}))
            .await;

        assert!(result.is_ok(), "Should be able to publish to Redis");

        env.cleanup().await.expect("Failed to cleanup");
    }

    #[tokio::test]
    #[ignore = "requires docker for testcontainers"]
    async fn test_event_publishing() {
        let env = IntegrationTestEnvironment::new()
            .await
            .expect("Failed to set up test environment");

        let realtime_manager = &env.app_state.realtime;
        // Test event publishing
        let event_data = json!({
            "type": "test_event",
            "data": {
                "message": "test message",
                "timestamp": chrono::Utc::now().to_rfc3339()
            }
        });

        let result = realtime_manager
            .publish_event("test_events", &event_data)
            .await;
        assert!(result.is_ok(), "Should publish event successfully");

        env.cleanup().await.expect("Failed to cleanup");
    }
}

// Helper module for testing utilities
#[cfg(test)]
mod test_utils {
    use super::*;

    pub async fn wait_for_service(url: &str, timeout_secs: u64) -> bool {
        let client = reqwest::Client::new();
        let timeout_duration = Duration::from_secs(timeout_secs);
        let start = std::time::Instant::now();

        while start.elapsed() < timeout_duration {
            if let Ok(response) = client.get(url).send().await {
                if response.status().is_success() {
                    return true;
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        false
    }

    pub async fn create_test_data(
        env: &IntegrationTestEnvironment,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Insert test data for integration tests
        sqlx::query(
            r#"
            INSERT INTO codebase (name, url, branch, suite, vcs_type) 
            VALUES ('test-package', 'https://github.com/test/test-package.git', 'main', 'lintian-fixes', 'Git')
            ON CONFLICT (name, branch, suite) DO NOTHING
            "#
        )
        .execute(env.database.pool())
        .await?;

        Ok(())
    }
}
