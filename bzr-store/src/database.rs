//! Database operations for the BZR Store service

use std::sync::Arc;

use bcrypt::{hash, verify, DEFAULT_COST};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use tracing::{debug, info};

use crate::config::Config;
use crate::error::{BzrError, Result};

/// Database manager for BZR Store operations
#[derive(Debug, Clone)]
pub struct DatabaseManager {
    /// PostgreSQL connection pool for database operations
    pool: Arc<PgPool>,
}

/// Worker permissions
#[derive(Debug, Clone)]
pub struct WorkerPermissions {
    /// Whether the worker has read access to repositories
    pub can_read: bool,
    /// Whether the worker has write access to repositories
    pub can_write: bool,
    /// List of campaigns the worker is authorized to access
    pub campaigns: Vec<String>,
}

impl DatabaseManager {
    /// Create a new database manager
    pub async fn new(config: &Config) -> Result<Self> {
        info!("Connecting to database: {}", config.database_url);

        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .connect(&config.database_url)
            .await?;

        // Test the connection
        let row: (i64,) = sqlx::query_as("SELECT 1").fetch_one(&pool).await?;
        debug!("Database connection test successful: {}", row.0);

        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    /// Get the database pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Validate that a codebase exists in the database
    pub async fn validate_codebase(&self, codebase: &str) -> Result<bool> {
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM codebase WHERE name = $1)")
                .bind(codebase)
                .fetch_one(&*self.pool)
                .await?;

        Ok(exists)
    }

    /// Authenticate a worker using username and password
    pub async fn authenticate_worker(&self, username: &str, password: &str) -> Result<bool> {
        let row = sqlx::query("SELECT password_hash FROM worker WHERE name = $1")
            .bind(username)
            .fetch_optional(&*self.pool)
            .await?;

        if let Some(row) = row {
            let stored_hash: String = row.get("password_hash");
            Ok(verify(password, &stored_hash).unwrap_or(false))
        } else {
            Ok(false)
        }
    }

    /// Get worker permissions
    pub async fn get_worker_permissions(&self, username: &str) -> Result<WorkerPermissions> {
        let row = sqlx::query("SELECT can_read, can_write FROM worker WHERE name = $1")
            .bind(username)
            .fetch_optional(&*self.pool)
            .await?;

        if let Some(row) = row {
            let can_read: bool = row.get("can_read");
            let can_write: bool = row.get("can_write");

            // Get campaigns this worker can access
            let campaign_rows =
                sqlx::query("SELECT campaign FROM worker_campaign WHERE worker_name = $1")
                    .bind(username)
                    .fetch_all(&*self.pool)
                    .await?;

            let campaigns: Vec<String> = campaign_rows
                .into_iter()
                .map(|row| row.get("campaign"))
                .collect();

            Ok(WorkerPermissions {
                can_read,
                can_write,
                campaigns,
            })
        } else {
            Err(BzrError::AuthenticationFailed)
        }
    }

    /// Create a new worker (for testing/admin purposes)
    pub async fn create_worker(
        &self,
        username: &str,
        password: &str,
        can_read: bool,
        can_write: bool,
    ) -> Result<()> {
        let password_hash = hash(password, DEFAULT_COST)
            .map_err(|e| BzrError::internal(format!("Failed to hash password: {}", e)))?;

        sqlx::query(
            "INSERT INTO worker (name, password_hash, can_read, can_write) VALUES ($1, $2, $3, $4)",
        )
        .bind(username)
        .bind(&password_hash)
        .bind(can_read)
        .bind(can_write)
        .execute(&*self.pool)
        .await?;

        info!("Created worker: {}", username);
        Ok(())
    }

    /// Check database health
    pub async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1").execute(&*self.pool).await?;
        Ok(())
    }
}
