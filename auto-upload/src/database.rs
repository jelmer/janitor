//! Database operations for auto-upload service

use sqlx::{PgPool, Row};
use tracing::{debug, info};

use crate::error::{Result, UploadError};

/// Database client for backfill operations using shared infrastructure
pub struct DatabaseClient {
    /// Shared database connection
    shared_db: janitor::database::Database,
}

/// Debian build information from the database
#[derive(Debug, Clone)]
pub struct DebianBuild {
    /// Build distribution
    pub distribution: String,
    /// Source package name
    pub source: String,
    /// Run ID for artifact retrieval
    pub run_id: String,
}

impl DatabaseClient {
    /// Create a new database client
    pub async fn new(database_url: &str) -> Result<Self> {
        info!(
            "Connecting to database: {}",
            database_url.split('@').next_back().unwrap_or("***")
        );

        let config = janitor::database::DatabaseConfig::new(database_url).with_max_connections(5);

        let shared_db = janitor::database::Database::connect_with_config(config)
            .await
            .map_err(|e| UploadError::Database(e.to_string()))?;

        Ok(Self { shared_db })
    }

    /// Get a reference to the database pool for backward compatibility
    pub fn pool(&self) -> &PgPool {
        self.shared_db.pool()
    }

    /// Get distinct Debian builds for backfill
    pub async fn get_backfill_builds(
        &self,
        distributions: Option<&[String]>,
    ) -> Result<Vec<DebianBuild>> {
        info!("Querying backfill builds from database");

        let rows = if let Some(distributions) = distributions {
            debug!("Filtering by distributions: {:?}", distributions);

            let query = "
                SELECT DISTINCT ON (distribution, source) 
                       distribution, source, run_id 
                FROM debian_build 
                WHERE distribution = ANY($1::text[])
                ORDER BY distribution, source, version DESC
            ";

            sqlx::query(query)
                .bind(distributions)
                .fetch_all(self.pool())
                .await
                .map_err(|e| UploadError::Database(e.to_string()))?
        } else {
            debug!("No distribution filter applied");

            let query = "
                SELECT DISTINCT ON (distribution, source) 
                       distribution, source, run_id 
                FROM debian_build 
                ORDER BY distribution, source, version DESC
            ";

            sqlx::query(query)
                .fetch_all(self.pool())
                .await
                .map_err(|e| UploadError::Database(e.to_string()))?
        };

        let builds: Vec<DebianBuild> = rows
            .into_iter()
            .map(|row| DebianBuild {
                distribution: row.get("distribution"),
                source: row.get("source"),
                run_id: row.get("run_id"),
            })
            .collect();

        info!("Found {} builds for backfill", builds.len());
        Ok(builds)
    }

    /// Get builds for a specific distribution
    pub async fn get_builds_for_distribution(
        &self,
        distribution: &str,
    ) -> Result<Vec<DebianBuild>> {
        debug!("Getting builds for distribution: {}", distribution);

        let query = "
            SELECT DISTINCT ON (source) 
                   distribution, source, run_id 
            FROM debian_build 
            WHERE distribution = $1
            ORDER BY source, version DESC
        ";

        let rows = sqlx::query(query)
            .bind(distribution)
            .fetch_all(self.pool())
            .await
            .map_err(|e| UploadError::Database(e.to_string()))?;

        let builds: Vec<DebianBuild> = rows
            .into_iter()
            .map(|row| DebianBuild {
                distribution: row.get("distribution"),
                source: row.get("source"),
                run_id: row.get("run_id"),
            })
            .collect();

        debug!(
            "Found {} builds for distribution {}",
            builds.len(),
            distribution
        );
        Ok(builds)
    }

    /// Get builds for a specific source package
    pub async fn get_builds_for_source(
        &self,
        source: &str,
        distributions: Option<&[String]>,
    ) -> Result<Vec<DebianBuild>> {
        debug!("Getting builds for source package: {}", source);

        let rows = if let Some(distributions) = distributions {
            let query = "
                SELECT distribution, source, run_id 
                FROM debian_build 
                WHERE source = $1 AND distribution = ANY($2::text[])
                ORDER BY distribution, version DESC
            ";

            sqlx::query(query)
                .bind(source)
                .bind(distributions)
                .fetch_all(self.pool())
                .await
                .map_err(|e| UploadError::Database(e.to_string()))?
        } else {
            let query = "
                SELECT distribution, source, run_id 
                FROM debian_build 
                WHERE source = $1
                ORDER BY distribution, version DESC
            ";

            sqlx::query(query)
                .bind(source)
                .fetch_all(self.pool())
                .await
                .map_err(|e| UploadError::Database(e.to_string()))?
        };

        let builds: Vec<DebianBuild> = rows
            .into_iter()
            .map(|row| DebianBuild {
                distribution: row.get("distribution"),
                source: row.get("source"),
                run_id: row.get("run_id"),
            })
            .collect();

        debug!("Found {} builds for source {}", builds.len(), source);
        Ok(builds)
    }

    /// Test database connection
    pub async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .fetch_one(self.pool())
            .await
            .map_err(|e| UploadError::Database(e.to_string()))?;

        debug!("Database health check passed");
        Ok(())
    }

    /// Get connection pool statistics
    pub async fn get_pool_stats(&self) -> PoolStats {
        PoolStats {
            size: self.pool().size(),
            idle: self.pool().num_idle(),
        }
    }
}

/// Database connection pool statistics
#[derive(Debug)]
pub struct PoolStats {
    /// Total connections in pool
    pub size: u32,
    /// Idle connections
    pub idle: usize,
}

/// Represents a complete query with SQL and parameters
pub struct BackfillQuery {
    /// The SQL query string with parameter placeholders
    pub sql: String,
    /// The parameter values
    pub parameters: Vec<serde_json::Value>,
}

/// Backfill query builder for complex filtering
pub struct BackfillQueryBuilder {
    /// Base query
    base_query: String,
    /// WHERE conditions
    conditions: Vec<String>,
    /// Query parameters as dynamic values
    parameters: Vec<serde_json::Value>,
}

impl BackfillQueryBuilder {
    /// Create a new query builder
    pub fn new() -> Self {
        Self {
            base_query: "SELECT DISTINCT ON (distribution, source) distribution, source, run_id FROM debian_build".to_string(),
            conditions: Vec::new(),
            parameters: Vec::new(),
        }
    }

    /// Add distribution filter
    pub fn filter_distributions(mut self, distributions: Vec<String>) -> Self {
        if !distributions.is_empty() {
            let param_num = self.parameters.len() + 1;
            self.conditions
                .push(format!("distribution = ANY(${}::text[])", param_num));
            self.parameters.push(serde_json::Value::Array(
                distributions
                    .into_iter()
                    .map(serde_json::Value::String)
                    .collect(),
            ));
        }
        self
    }

    /// Add source package filter
    pub fn filter_source(mut self, source: String) -> Self {
        let param_num = self.parameters.len() + 1;
        self.conditions.push(format!("source = ${}", param_num));
        self.parameters.push(serde_json::Value::String(source));
        self
    }

    /// Add date range filter
    pub fn filter_date_range(
        mut self,
        start_date: Option<String>,
        end_date: Option<String>,
    ) -> Self {
        if let Some(start) = start_date {
            let param_num = self.parameters.len() + 1;
            self.conditions
                .push(format!("created_at >= ${}", param_num));
            self.parameters.push(serde_json::Value::String(start));
        }
        if let Some(end) = end_date {
            let param_num = self.parameters.len() + 1;
            self.conditions
                .push(format!("created_at <= ${}", param_num));
            self.parameters.push(serde_json::Value::String(end));
        }
        self
    }

    /// Build the final query with parameters
    pub fn build(self) -> BackfillQuery {
        let mut query = self.base_query;

        if !self.conditions.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&self.conditions.join(" AND "));
        }

        query.push_str(" ORDER BY distribution, source, version DESC");

        BackfillQuery {
            sql: query,
            parameters: self.parameters,
        }
    }
}

impl Default for BackfillQueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_builder_basic() {
        let query = BackfillQueryBuilder::new().build();
        assert!(query.sql.contains("SELECT DISTINCT ON"));
        assert!(query.sql.contains("ORDER BY"));
        assert!(query.parameters.is_empty());
    }

    #[test]
    fn test_query_builder_with_filters() {
        let query = BackfillQueryBuilder::new()
            .filter_distributions(vec!["unstable".to_string()])
            .filter_source("hello".to_string())
            .build();

        assert!(query.sql.contains("WHERE"));
        assert!(query.sql.contains("distribution = ANY($1::text[])"));
        assert!(query.sql.contains("source = $2"));
        assert_eq!(query.parameters.len(), 2);

        // Verify parameter values
        assert_eq!(
            query.parameters[0],
            serde_json::Value::Array(vec![serde_json::Value::String("unstable".to_string())])
        );
        assert_eq!(
            query.parameters[1],
            serde_json::Value::String("hello".to_string())
        );
    }

    #[test]
    fn test_query_builder_with_date_range() {
        let query = BackfillQueryBuilder::new()
            .filter_date_range(
                Some("2023-01-01".to_string()),
                Some("2023-12-31".to_string()),
            )
            .build();

        assert!(query.sql.contains("WHERE"));
        assert!(query.sql.contains("created_at >= $1"));
        assert!(query.sql.contains("created_at <= $2"));
        assert_eq!(query.parameters.len(), 2);

        // Verify parameter values
        assert_eq!(
            query.parameters[0],
            serde_json::Value::String("2023-01-01".to_string())
        );
        assert_eq!(
            query.parameters[1],
            serde_json::Value::String("2023-12-31".to_string())
        );
    }

    #[test]
    fn test_query_builder_parameter_numbering() {
        let query = BackfillQueryBuilder::new()
            .filter_distributions(vec!["unstable".to_string(), "testing".to_string()])
            .filter_source("hello".to_string())
            .filter_date_range(Some("2023-01-01".to_string()), None)
            .build();

        // Should have parameters numbered sequentially
        assert!(query.sql.contains("distribution = ANY($1::text[])"));
        assert!(query.sql.contains("source = $2"));
        assert!(query.sql.contains("created_at >= $3"));
        assert_eq!(query.parameters.len(), 3);
    }

    #[test]
    fn test_debian_build_creation() {
        let build = DebianBuild {
            distribution: "unstable".to_string(),
            source: "hello".to_string(),
            run_id: "test-123".to_string(),
        };

        assert_eq!(build.distribution, "unstable");
        assert_eq!(build.source, "hello");
        assert_eq!(build.run_id, "test-123");
    }

    #[test]
    fn test_pool_stats() {
        let stats = PoolStats { size: 5, idle: 3 };

        assert_eq!(stats.size, 5);
        assert_eq!(stats.idle, 3);
    }
}
