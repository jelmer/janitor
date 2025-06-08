//! Test utilities for the Janitor project.
//!
//! This module provides common testing utilities, including database setup,
//! mock implementations, and test configuration.

use crate::database::{Database, DatabaseConfig};
use sqlx::PgPool;
use std::sync::Once;
use std::time::Duration;
use uuid::Uuid;

static INIT: Once = Once::new();

/// Initialize test environment (logging, etc.)
pub fn init_test_env() {
    INIT.call_once(|| {
        // Initialize logging for tests if needed
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Debug)
            .is_test(true)
            .try_init();
    });
}

/// Test database configuration
#[derive(Debug, Clone)]
pub struct TestDatabaseConfig {
    /// Base database URL (without database name)
    pub base_url: String,
    /// Whether to create a unique test database per test
    pub unique_per_test: bool,
    /// Whether to run migrations
    pub run_migrations: bool,
}

impl Default for TestDatabaseConfig {
    fn default() -> Self {
        Self {
            base_url: std::env::var("TEST_DATABASE_URL")
                .unwrap_or_else(|_| "postgresql://localhost/postgres".to_string()),
            unique_per_test: true,
            run_migrations: false, // Default to false as we don't have migrations yet
        }
    }
}

/// Test database manager that handles creation and cleanup of test databases
pub struct TestDatabase {
    pub pool: PgPool,
    pub database_name: String,
    admin_pool: PgPool,
    should_cleanup: bool,
}

impl TestDatabase {
    /// Create a new test database
    pub async fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Self::with_config(TestDatabaseConfig::default()).await
    }

    /// Create a test database that will skip database setup if no database is available
    /// This is useful for CI environments where no database may be running
    pub async fn new_optional() -> Result<Option<Self>, Box<dyn std::error::Error + Send + Sync>> {
        match Self::with_config(TestDatabaseConfig::default()).await {
            Ok(db) => Ok(Some(db)),
            Err(_) => {
                eprintln!("Warning: Could not connect to test database, skipping database-dependent tests");
                Ok(None)
            }
        }
    }

    /// Connect to database with retries (useful for containers that need startup time)
    async fn connect_with_retries(
        url: &str,
        max_retries: u32,
    ) -> Result<PgPool, Box<dyn std::error::Error + Send + Sync>> {
        let mut retries = 0;
        loop {
            match PgPool::connect(url).await {
                Ok(pool) => return Ok(pool),
                Err(_e) if retries < max_retries => {
                    retries += 1;
                    eprintln!(
                        "Database connection attempt {} failed, retrying...",
                        retries
                    );
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
                Err(e) => return Err(Box::new(e)),
            }
        }
    }

    /// Create a new test database with custom configuration
    pub async fn with_config(
        config: TestDatabaseConfig,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        init_test_env();

        // Connect to the admin database (usually 'postgres')
        let admin_pool = PgPool::connect(&config.base_url).await?;

        // Generate unique database name if needed
        let database_name = if config.unique_per_test {
            format!("janitor_test_{}", Uuid::new_v4().simple())
        } else {
            "janitor_test".to_string()
        };

        // Create the test database
        let create_query = format!("CREATE DATABASE \"{}\"", database_name);
        sqlx::query(&create_query).execute(&admin_pool).await?;

        // Build the test database URL
        let test_db_url = if config.base_url.contains('/') {
            // Replace the database name in the URL
            let base = config.base_url.rsplit_once('/').unwrap().0;
            format!("{}/{}", base, database_name)
        } else {
            format!("{}/{}", config.base_url, database_name)
        };

        // Connect to the test database
        let pool = PgPool::connect(&test_db_url).await?;

        // Run migrations if requested
        if config.run_migrations {
            // TODO: Implement migration runner when we have migrations
            // sqlx::migrate!("./migrations").run(&pool).await?;
        }

        Ok(Self {
            pool,
            database_name,
            admin_pool,
            should_cleanup: config.unique_per_test,
        })
    }

    /// Get a reference to the database pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Create a janitor Database instance from this test database
    pub fn into_janitor_database(self) -> Database {
        let config = DatabaseConfig {
            url: format!("postgresql://localhost/{}", self.database_name),
            max_connections: 5,
            connect_timeout: Duration::from_secs(10),
            idle_timeout: Some(Duration::from_secs(600)),
            max_lifetime: Some(Duration::from_secs(3600)),
        };

        Database::from_pool_and_config(self.pool.clone(), config)
    }
}

impl Drop for TestDatabase {
    fn drop(&mut self) {
        if self.should_cleanup {
            // Note: We can't use async in Drop, so we spawn a task
            // In practice, test databases are often cleaned up by the test runner
            let admin_pool = self.admin_pool.clone();
            let database_name = self.database_name.clone();

            tokio::spawn(async move {
                let _ = sqlx::query(&format!("DROP DATABASE IF EXISTS \"{}\"", database_name))
                    .execute(&admin_pool)
                    .await;
            });
        }
    }
}

/// Mock implementations for testing

/// Mock artifact manager for testing
#[derive(Debug, Clone)]
pub struct MockArtifactManager;

#[async_trait::async_trait]
impl crate::artifacts::ArtifactManager for MockArtifactManager {
    async fn store_artifacts(
        &self,
        _run_id: &str,
        _local_path: &std::path::Path,
        _names: Option<&[String]>,
    ) -> Result<(), crate::artifacts::Error> {
        Ok(())
    }

    async fn get_artifact(
        &self,
        _run_id: &str,
        _filename: &str,
    ) -> Result<Box<dyn std::io::Read + Sync + Send>, crate::artifacts::Error> {
        Ok(Box::new(std::io::Cursor::new(b"mock artifact data")))
    }

    fn public_artifact_url(&self, run_id: &str, filename: &str) -> url::Url {
        format!("mock://artifacts/{}/{}", run_id, filename)
            .parse()
            .unwrap()
    }

    async fn retrieve_artifacts(
        &self,
        _run_id: &str,
        _local_path: &std::path::Path,
        _filter_fn: Option<&(dyn for<'a> Fn(&'a str) -> bool + Sync + Send)>,
    ) -> Result<(), crate::artifacts::Error> {
        Ok(())
    }

    async fn iter_ids(&self) -> Box<dyn Iterator<Item = String> + Send> {
        Box::new(vec!["test_run_1".to_string(), "test_run_2".to_string()].into_iter())
    }

    async fn delete_artifacts(&self, _run_id: &str) -> Result<(), crate::artifacts::Error> {
        Ok(())
    }
}

/// Mock log file manager for testing
#[derive(Debug, Clone)]
pub struct MockLogFileManager;

#[async_trait::async_trait]
impl crate::logs::LogFileManager for MockLogFileManager {
    async fn has_log(
        &self,
        _codebase: &str,
        _run_id: &str,
        _name: &str,
    ) -> Result<bool, crate::logs::Error> {
        Ok(true)
    }

    async fn get_log(
        &self,
        _codebase: &str,
        _run_id: &str,
        _name: &str,
    ) -> Result<Box<dyn std::io::Read + Send + Sync>, crate::logs::Error> {
        Ok(Box::new(std::io::Cursor::new(b"mock log content")))
    }

    async fn import_log(
        &self,
        _codebase: &str,
        _run_id: &str,
        _orig_path: &str,
        _mtime: Option<chrono::DateTime<chrono::Utc>>,
        _basename: Option<&str>,
    ) -> Result<(), crate::logs::Error> {
        Ok(())
    }

    async fn delete_log(
        &self,
        _codebase: &str,
        _run_id: &str,
        _name: &str,
    ) -> Result<(), crate::logs::Error> {
        Ok(())
    }

    async fn iter_logs(&self) -> Box<dyn Iterator<Item = (String, String, Vec<String>)>> {
        Box::new(
            vec![
                (
                    "codebase1".to_string(),
                    "run1".to_string(),
                    vec!["build.log".to_string()],
                ),
                (
                    "codebase2".to_string(),
                    "run2".to_string(),
                    vec!["test.log".to_string()],
                ),
            ]
            .into_iter(),
        )
    }

    async fn get_ctime(
        &self,
        _codebase: &str,
        _run_id: &str,
        _name: &str,
    ) -> Result<chrono::DateTime<chrono::Utc>, crate::logs::Error> {
        Ok(chrono::Utc::now())
    }

    async fn health_check(&self) -> Result<(), crate::logs::Error> {
        Ok(())
    }
}

/// Test configuration builder
pub struct TestConfigBuilder {
    database_url: Option<String>,
    enable_debug: bool,
    enable_logging: bool,
}

impl Default for TestConfigBuilder {
    fn default() -> Self {
        Self {
            database_url: None,
            enable_debug: true,
            enable_logging: false,
        }
    }
}

impl TestConfigBuilder {
    /// Create a new test configuration builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the database URL
    pub fn database_url(mut self, url: String) -> Self {
        self.database_url = Some(url);
        self
    }

    /// Enable debug mode
    pub fn debug(mut self, enable: bool) -> Self {
        self.enable_debug = enable;
        self
    }

    /// Enable logging
    pub fn logging(mut self, enable: bool) -> Self {
        self.enable_logging = enable;
        self
    }

    /// Build a janitor Config
    pub fn build_janitor_config(self) -> crate::config::Config {
        let mut config = crate::config::Config::new();

        // Set database location
        config.database_location = Some(self.database_url.unwrap_or_else(|| {
            std::env::var("TEST_DATABASE_URL")
                .unwrap_or_else(|_| "postgresql://localhost/janitor_test".to_string())
        }));

        // Set logs location to temp directory for tests
        config.logs_location = Some(
            std::env::temp_dir()
                .join("janitor_test_logs")
                .to_string_lossy()
                .to_string(),
        );

        // Set artifact location to temp directory for tests
        config.artifact_location = Some(
            std::env::temp_dir()
                .join("janitor_test_artifacts")
                .to_string_lossy()
                .to_string(),
        );

        // Set default committer
        config.committer = Some("Test Runner <test@example.com>".to_string());

        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifacts::ArtifactManager;
    use crate::logs::LogFileManager;

    #[tokio::test]
    async fn test_mock_artifact_manager() {
        let manager = MockArtifactManager;

        // Test store_artifacts
        manager
            .store_artifacts("test_run", std::path::Path::new("/tmp"), None)
            .await
            .unwrap();

        // Test get_artifact
        let mut artifact = manager.get_artifact("test_run", "test.log").await.unwrap();
        let mut content = Vec::new();
        std::io::Read::read_to_end(&mut artifact, &mut content).unwrap();
        assert_eq!(content, b"mock artifact data");

        // Test public_artifact_url
        let url = manager.public_artifact_url("test_run", "test.log");
        assert!(url.as_str().contains("test_run"));
        assert!(url.as_str().contains("test.log"));

        // Test retrieve_artifacts
        manager
            .retrieve_artifacts("test_run", std::path::Path::new("/tmp"), None)
            .await
            .unwrap();

        // Test iter_ids
        let ids: Vec<String> = manager.iter_ids().await.collect();
        assert!(!ids.is_empty());

        // Test delete_artifacts
        manager.delete_artifacts("test_run").await.unwrap();
    }

    #[tokio::test]
    async fn test_mock_log_manager() {
        let manager = MockLogFileManager;

        // Test has_log
        let has_log = manager
            .has_log("test_codebase", "test_run", "test.log")
            .await
            .unwrap();
        assert!(has_log);

        // Test get_log
        let mut log = manager
            .get_log("test_codebase", "test_run", "test.log")
            .await
            .unwrap();
        let mut content = String::new();
        std::io::Read::read_to_string(&mut log, &mut content).unwrap();
        assert_eq!(content, "mock log content");

        // Test import_log
        manager
            .import_log(
                "test_codebase",
                "test_run",
                "/tmp/test.log",
                None,
                Some("test.log"),
            )
            .await
            .unwrap();

        // Test delete_log
        manager
            .delete_log("test_codebase", "test_run", "test.log")
            .await
            .unwrap();

        // Test iter_logs
        let logs: Vec<_> = manager.iter_logs().await.collect();
        assert!(!logs.is_empty());

        // Test get_ctime
        let ctime = manager
            .get_ctime("test_codebase", "test_run", "test.log")
            .await
            .unwrap();
        assert!(ctime <= chrono::Utc::now());

        // Test health_check
        manager.health_check().await.unwrap();
    }
}
