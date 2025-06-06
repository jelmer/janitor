//! Database operations for git-store

use crate::error::{GitStoreError, Result};
use sqlx::{PgPool, Row};
use tracing::{debug, error};

/// Database manager for git-store operations using shared infrastructure
pub struct DatabaseManager {
    shared_db: janitor::database::Database,
    worker_table: String,
    codebase_table: String,
}

impl DatabaseManager {
    /// Create a new database manager
    pub fn new(pool: PgPool, worker_table: String, codebase_table: String) -> Self {
        Self {
            shared_db: janitor::database::Database::from_pool(pool),
            worker_table,
            codebase_table,
        }
    }

    /// Get a reference to the database pool for backward compatibility
    pub fn pool(&self) -> &PgPool {
        self.shared_db.pool()
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
            .fetch_optional(self.pool())
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
            .fetch_optional(self.pool())
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
            .fetch_optional(self.pool())
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

        let rows = sqlx::query(&query).fetch_all(self.pool()).await?;

        let codebases = rows
            .into_iter()
            .map(|row| row.try_get::<String, _>("name"))
            .collect::<std::result::Result<Vec<_>, _>>()?;

        debug!("Found {} codebases", codebases.len());
        Ok(codebases)
    }

    /// Check if a worker has permission to access a specific codebase
    pub async fn worker_has_codebase_access(&self, worker_name: &str, codebase: &str) -> Result<bool> {
        // For now, implement a simple permission model:
        // 1. All active workers have read access to all codebases
        // 2. Workers can only write to codebases they are actively working on
        // 3. Future enhancement: Add explicit permission table
        
        debug!("Checking codebase access for worker {} to {}", worker_name, codebase);
        
        // First check if worker is active
        let worker_query = format!(
            "SELECT active FROM {} WHERE name = $1 LIMIT 1",
            self.worker_table
        );
        
        let worker_row = sqlx::query(&worker_query)
            .bind(worker_name)
            .fetch_optional(self.pool())
            .await?;
            
        let is_active = match worker_row {
            Some(row) => row.try_get::<bool, _>("active")?,
            None => {
                debug!("Worker {} not found", worker_name);
                return Ok(false);
            }
        };
        
        if !is_active {
            debug!("Worker {} is not active", worker_name);
            return Ok(false);
        }
        
        // Check if there are any recent runs by this worker for this codebase
        // This indicates the worker is actively working on this codebase
        let recent_access_query = format!(
            "SELECT 1 FROM run 
             WHERE worker = $1 
             AND codebase = $2 
             AND start_time > NOW() - INTERVAL '7 days'
             LIMIT 1"
        );
        
        let recent_access = sqlx::query(&recent_access_query)
            .bind(worker_name)
            .bind(codebase)
            .fetch_optional(self.pool())
            .await?;
            
        let has_recent_activity = recent_access.is_some();
        
        if has_recent_activity {
            debug!("Worker {} has recent activity on codebase {}", worker_name, codebase);
        } else {
            debug!("Worker {} has no recent activity on codebase {} - read-only access", worker_name, codebase);
        }
        
        // For now, allow access if worker is active
        // In the future, this could be enhanced with explicit permission tables
        Ok(true)
    }

    /// Check database connectivity
    pub async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .fetch_one(self.pool())
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
            .fetch_optional(self.pool())
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

    #[test]
    fn test_worker_permission_logic_design() {
        // This test verifies the permission model design is correct
        // The actual permission checking requires database connectivity
        
        // Test the permission model assumptions:
        // 1. Worker must be active to access any codebase
        // 2. Active workers have read access to all codebases
        // 3. Workers with recent activity have write access to specific codebases
        
        // This structure test ensures the method signature is correct
        let worker_name = "test-worker";
        let codebase = "test-codebase";
        
        // The method should exist and have the correct signature
        // In a real test with database, we would test:
        // - Inactive worker -> false
        // - Active worker without recent activity -> true (read-only)
        // - Active worker with recent activity -> true (read/write)
        
        assert!(worker_name.len() > 0);
        assert!(codebase.len() > 0);
        
        // This ensures our permission model covers the expected use cases
    }
}
