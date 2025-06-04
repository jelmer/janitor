//! Database operations for git-store

use crate::error::{GitStoreError, Result};
use sqlx::{PgPool, Row};
use tracing::{debug, error};

/// Database manager for git-store operations
pub struct DatabaseManager {
    pool: PgPool,
    worker_table: String,
    codebase_table: String,
}

impl DatabaseManager {
    /// Create a new database manager
    pub fn new(pool: PgPool, worker_table: String, codebase_table: String) -> Self {
        Self {
            pool,
            worker_table,
            codebase_table,
        }
    }

    /// Check if a codebase exists in the database
    pub async fn codebase_exists(&self, codebase: &str) -> Result<bool> {
        let query = format!(
            "SELECT 1 FROM {} WHERE name = $1 LIMIT 1",
            self.codebase_table
        );

        debug!("Checking if codebase exists: {}", codebase);

        let result = sqlx::query(&query)
            .bind(codebase)
            .fetch_optional(&self.pool)
            .await?;

        Ok(result.is_some())
    }

    /// Authenticate a worker using credentials
    pub async fn authenticate_worker(&self, username: &str, password: &str) -> Result<bool> {
        let query = format!(
            "SELECT password_hash FROM {} WHERE name = $1 AND active = true LIMIT 1",
            self.worker_table
        );

        debug!("Authenticating worker: {}", username);

        let row = sqlx::query(&query)
            .bind(username)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            let stored_hash: String = row.try_get("password_hash")?;

            // Use bcrypt to verify password
            match bcrypt::verify(password, &stored_hash) {
                Ok(valid) => {
                    if valid {
                        debug!("Worker authentication successful: {}", username);
                    } else {
                        debug!(
                            "Worker authentication failed - invalid password: {}",
                            username
                        );
                    }
                    Ok(valid)
                }
                Err(e) => {
                    error!("Error verifying password for worker {}: {}", username, e);
                    Err(GitStoreError::AuthenticationFailed)
                }
            }
        } else {
            debug!("Worker not found or inactive: {}", username);
            Ok(false)
        }
    }

    /// Get worker information
    pub async fn get_worker_info(&self, username: &str) -> Result<Option<WorkerInfo>> {
        let query = format!(
            "SELECT name, active, created_at, last_seen FROM {} WHERE name = $1 LIMIT 1",
            self.worker_table
        );

        debug!("Getting worker info: {}", username);

        let row = sqlx::query(&query)
            .bind(username)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            let info = WorkerInfo {
                name: row.try_get("name")?,
                active: row.try_get("active")?,
                created_at: row.try_get("created_at")?,
                last_seen: row.try_get("last_seen").ok(),
            };
            Ok(Some(info))
        } else {
            Ok(None)
        }
    }

    /// List all codebases
    pub async fn list_codebases(&self, limit: Option<i64>) -> Result<Vec<String>> {
        let query = if let Some(limit) = limit {
            format!(
                "SELECT name FROM {} ORDER BY name LIMIT {}",
                self.codebase_table, limit
            )
        } else {
            format!("SELECT name FROM {} ORDER BY name", self.codebase_table)
        };

        debug!("Listing codebases with limit: {:?}", limit);

        let rows = sqlx::query(&query).fetch_all(&self.pool).await?;

        let codebases = rows
            .into_iter()
            .map(|row| row.try_get::<String, _>("name"))
            .collect::<std::result::Result<Vec<_>, _>>()?;

        debug!("Found {} codebases", codebases.len());
        Ok(codebases)
    }

    /// Check database connectivity
    pub async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .map_err(GitStoreError::from)?;
        Ok(())
    }

    /// Get codebase details
    pub async fn get_codebase_info(&self, codebase: &str) -> Result<Option<CodebaseInfo>> {
        let query = format!(
            "SELECT name, url, vcs_type, created_at, updated_at FROM {} WHERE name = $1 LIMIT 1",
            self.codebase_table
        );

        debug!("Getting codebase info: {}", codebase);

        let row = sqlx::query(&query)
            .bind(codebase)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            let info = CodebaseInfo {
                name: row.try_get("name")?,
                url: row.try_get("url").ok(),
                vcs_type: row.try_get("vcs_type").ok(),
                created_at: row.try_get("created_at").ok(),
                updated_at: row.try_get("updated_at").ok(),
            };
            Ok(Some(info))
        } else {
            Ok(None)
        }
    }
}

/// Worker information
#[derive(Debug, Clone, serde::Serialize)]
pub struct WorkerInfo {
    /// Worker name
    pub name: String,
    /// Whether the worker is active
    pub active: bool,
    /// When the worker was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the worker was last seen
    pub last_seen: Option<chrono::DateTime<chrono::Utc>>,
}

/// Codebase information
#[derive(Debug, Clone, serde::Serialize)]
pub struct CodebaseInfo {
    /// Codebase name
    pub name: String,
    /// Repository URL
    pub url: Option<String>,
    /// VCS type
    pub vcs_type: Option<String>,
    /// When created
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    /// When last updated
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests would require a test database setup
    // For now, we'll just test the structure compilation

    #[test]
    fn test_worker_info_serialization() {
        let info = WorkerInfo {
            name: "test-worker".to_string(),
            active: true,
            created_at: chrono::Utc::now(),
            last_seen: Some(chrono::Utc::now()),
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("test-worker"));
    }

    #[test]
    fn test_codebase_info_serialization() {
        let info = CodebaseInfo {
            name: "test-codebase".to_string(),
            url: Some("https://github.com/example/repo".to_string()),
            vcs_type: Some("git".to_string()),
            created_at: Some(chrono::Utc::now()),
            updated_at: Some(chrono::Utc::now()),
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("test-codebase"));
    }
}
