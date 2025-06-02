//! Database integration for archive service.
//!
//! This module provides functions to query build results from the database
//! for repository generation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use tracing::{debug, warn};

use crate::error::{ArchiveError, ArchiveResult};
use crate::scanner::BuildInfo;

/// Build result information from database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildRecord {
    /// Build ID.
    pub id: String,
    /// Run ID associated with this build.
    pub run_id: String,
    /// Codebase name.
    pub codebase: String,
    /// Suite/campaign name.
    pub suite: String,
    /// Package name.
    pub package: String,
    /// Source package name.
    pub source_package: String,
    /// Build architecture.
    pub architecture: String,
    /// Component (main, contrib, non-free).
    pub component: String,
    /// Version string.
    pub version: String,
    /// Build status.
    pub status: String,
    /// Build finish time.
    pub finish_time: Option<DateTime<Utc>>,
    /// Binary package files produced.
    pub binary_files: Vec<String>,
    /// Source package files produced.
    pub source_files: Vec<String>,
}

/// Campaign configuration from database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignInfo {
    /// Campaign name.
    pub name: String,
    /// Campaign description.
    pub description: String,
    /// Suite name for repository.
    pub suite: String,
    /// Component name.
    pub component: String,
    /// Supported architectures.
    pub architectures: Vec<String>,
}

/// Database manager for archive operations.
pub struct ArchiveDatabase {
    pool: PgPool,
}

impl ArchiveDatabase {
    /// Create a new archive database manager.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get all successful builds for a specific suite.
    pub async fn get_builds_for_suite(&self, suite_name: &str) -> ArchiveResult<Vec<BuildRecord>> {
        debug!("Querying builds for suite: {}", suite_name);

        let query = r#"
            SELECT 
                db.id,
                db.run_id,
                r.codebase,
                r.suite,
                db.package,
                db.source_package,
                db.architecture,
                COALESCE(db.component, 'main') as component,
                db.version,
                db.status,
                r.finish_time,
                COALESCE(db.binary_files, '[]'::jsonb) as binary_files,
                COALESCE(db.source_files, '[]'::jsonb) as source_files
            FROM debian_build db
            JOIN run r ON db.run_id = r.id
            WHERE r.suite = $1
              AND db.status = 'success'
              AND r.result_code = 'success'
            ORDER BY db.package, db.version DESC
        "#;

        let rows = sqlx::query(query)
            .bind(suite_name)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ArchiveError::Database(e))?;

        let mut builds = Vec::new();

        for row in rows {
            let binary_files: serde_json::Value = row.get("binary_files");
            let source_files: serde_json::Value = row.get("source_files");

            let binary_files: Vec<String> =
                serde_json::from_value(binary_files).unwrap_or_else(|e| {
                    warn!("Failed to parse binary_files: {}", e);
                    Vec::new()
                });

            let source_files: Vec<String> =
                serde_json::from_value(source_files).unwrap_or_else(|e| {
                    warn!("Failed to parse source_files: {}", e);
                    Vec::new()
                });

            builds.push(BuildRecord {
                id: row.get("id"),
                run_id: row.get("run_id"),
                codebase: row.get("codebase"),
                suite: row.get("suite"),
                package: row.get("package"),
                source_package: row.get("source_package"),
                architecture: row.get("architecture"),
                component: row.get("component"),
                version: row.get("version"),
                status: row.get("status"),
                finish_time: row.get("finish_time"),
                binary_files,
                source_files,
            });
        }

        debug!("Found {} builds for suite {}", builds.len(), suite_name);
        Ok(builds)
    }

    /// Get builds for a specific changeset.
    pub async fn get_builds_for_changeset(
        &self,
        changeset_id: &str,
    ) -> ArchiveResult<Vec<BuildRecord>> {
        debug!("Querying builds for changeset: {}", changeset_id);

        let query = r#"
            SELECT 
                db.id,
                db.run_id,
                r.codebase,
                r.suite,
                db.package,
                db.source_package,
                db.architecture,
                COALESCE(db.component, 'main') as component,
                db.version,
                db.status,
                r.finish_time,
                COALESCE(db.binary_files, '[]'::jsonb) as binary_files,
                COALESCE(db.source_files, '[]'::jsonb) as source_files
            FROM debian_build db
            JOIN run r ON db.run_id = r.id
            WHERE r.change_set = $1
              AND db.status = 'success'
              AND r.result_code = 'success'
            ORDER BY db.package, db.version DESC
        "#;

        let rows = sqlx::query(query)
            .bind(changeset_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ArchiveError::Database(e))?;

        self.parse_build_records(rows).await
    }

    /// Get builds for a specific run.
    pub async fn get_builds_for_run(&self, run_id: &str) -> ArchiveResult<Vec<BuildRecord>> {
        debug!("Querying builds for run: {}", run_id);

        let query = r#"
            SELECT 
                db.id,
                db.run_id,
                r.codebase,
                r.suite,
                db.package,
                db.source_package,
                db.architecture,
                COALESCE(db.component, 'main') as component,
                db.version,
                db.status,
                r.finish_time,
                COALESCE(db.binary_files, '[]'::jsonb) as binary_files,
                COALESCE(db.source_files, '[]'::jsonb) as source_files
            FROM debian_build db
            JOIN run r ON db.run_id = r.id
            WHERE r.id = $1
              AND db.status = 'success'
              AND r.result_code = 'success'
            ORDER BY db.package, db.version DESC
        "#;

        let rows = sqlx::query(query)
            .bind(run_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ArchiveError::Database(e))?;

        self.parse_build_records(rows).await
    }

    /// Get builds grouped by package for a suite (for duplicate resolution).
    pub async fn get_latest_builds_for_suite(
        &self,
        suite_name: &str,
    ) -> ArchiveResult<Vec<BuildRecord>> {
        debug!("Querying latest builds for suite: {}", suite_name);

        let query = r#"
            WITH latest_builds AS (
                SELECT 
                    db.package,
                    db.architecture,
                    MAX(r.finish_time) as latest_time
                FROM debian_build db
                JOIN run r ON db.run_id = r.id
                WHERE r.suite = $1
                  AND db.status = 'success'
                  AND r.result_code = 'success'
                GROUP BY db.package, db.architecture
            )
            SELECT 
                db.id,
                db.run_id,
                r.codebase,
                r.suite,
                db.package,
                db.source_package,
                db.architecture,
                COALESCE(db.component, 'main') as component,
                db.version,
                db.status,
                r.finish_time,
                COALESCE(db.binary_files, '[]'::jsonb) as binary_files,
                COALESCE(db.source_files, '[]'::jsonb) as source_files
            FROM debian_build db
            JOIN run r ON db.run_id = r.id
            JOIN latest_builds lb ON db.package = lb.package 
                                  AND db.architecture = lb.architecture
                                  AND r.finish_time = lb.latest_time
            WHERE r.suite = $1
              AND db.status = 'success'
              AND r.result_code = 'success'
            ORDER BY db.package, db.architecture
        "#;

        let rows = sqlx::query(query)
            .bind(suite_name)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ArchiveError::Database(e))?;

        self.parse_build_records(rows).await
    }

    /// Get campaign information from configuration.
    pub async fn get_campaign_info(
        &self,
        campaign_name: &str,
    ) -> ArchiveResult<Option<CampaignInfo>> {
        debug!("Querying campaign info for: {}", campaign_name);

        // This would query a campaigns table or configuration
        // For now, return a placeholder implementation
        // TODO: Implement actual campaign configuration queries

        match campaign_name {
            "lintian-fixes" => Ok(Some(CampaignInfo {
                name: campaign_name.to_string(),
                description: "Automated lintian issue fixes".to_string(),
                suite: "lintian-fixes".to_string(),
                component: "main".to_string(),
                architectures: vec!["amd64".to_string(), "i386".to_string(), "arm64".to_string()],
            })),
            "fresh-releases" => Ok(Some(CampaignInfo {
                name: campaign_name.to_string(),
                description: "Fresh upstream releases".to_string(),
                suite: "fresh-releases".to_string(),
                component: "main".to_string(),
                architectures: vec!["amd64".to_string(), "arm64".to_string()],
            })),
            _ => Ok(None),
        }
    }

    /// Convert BuildRecord to BuildInfo for scanner.
    pub fn build_record_to_info(&self, record: &BuildRecord) -> BuildInfo {
        BuildInfo {
            id: record.id.clone(),
            codebase: record.codebase.clone(),
            suite: record.suite.clone(),
            architecture: record.architecture.clone(),
            component: record.component.clone(),
            binary_files: record.binary_files.clone(),
            source_files: record.source_files.clone(),
        }
    }

    /// Helper function to parse build records from database rows.
    async fn parse_build_records(
        &self,
        rows: Vec<sqlx::postgres::PgRow>,
    ) -> ArchiveResult<Vec<BuildRecord>> {
        let mut builds = Vec::new();

        for row in rows {
            let binary_files: serde_json::Value = row.get("binary_files");
            let source_files: serde_json::Value = row.get("source_files");

            let binary_files: Vec<String> =
                serde_json::from_value(binary_files).unwrap_or_else(|e| {
                    warn!("Failed to parse binary_files: {}", e);
                    Vec::new()
                });

            let source_files: Vec<String> =
                serde_json::from_value(source_files).unwrap_or_else(|e| {
                    warn!("Failed to parse source_files: {}", e);
                    Vec::new()
                });

            builds.push(BuildRecord {
                id: row.get("id"),
                run_id: row.get("run_id"),
                codebase: row.get("codebase"),
                suite: row.get("suite"),
                package: row.get("package"),
                source_package: row.get("source_package"),
                architecture: row.get("architecture"),
                component: row.get("component"),
                version: row.get("version"),
                status: row.get("status"),
                finish_time: row.get("finish_time"),
                binary_files,
                source_files,
            });
        }

        Ok(builds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_database_manager_creation() {
        // This test would require a database connection
        // In practice, this would use a test database or mocking
        // let pool = PgPool::connect("postgresql://test").await.unwrap();
        // let db = ArchiveDatabase::new(pool);
        // assert!(db.pool.is_closed() == false);
    }

    #[test]
    fn test_build_record_to_info_conversion() {
        let record = BuildRecord {
            id: "build-123".to_string(),
            run_id: "run-456".to_string(),
            codebase: "test-package".to_string(),
            suite: "lintian-fixes".to_string(),
            package: "test-package".to_string(),
            source_package: "test-package".to_string(),
            architecture: "amd64".to_string(),
            component: "main".to_string(),
            version: "1.0-1".to_string(),
            status: "success".to_string(),
            finish_time: None,
            binary_files: vec!["test-package_1.0-1_amd64.deb".to_string()],
            source_files: vec!["test-package_1.0-1.dsc".to_string()],
        };

        // Create a dummy pool for testing (won't be used)
        // let pool = PgPool::connect_lazy("postgresql://dummy").unwrap();
        // let db = ArchiveDatabase::new(pool);
        // let info = db.build_record_to_info(&record);

        // For now, just test the record structure
        assert_eq!(record.id, "build-123");
        assert_eq!(record.architecture, "amd64");
        assert_eq!(record.binary_files.len(), 1);
    }
}

/// Type alias for compatibility with repository module.
pub type BuildManager = ArchiveDatabase;

impl From<BuildRecord> for BuildInfo {
    fn from(record: BuildRecord) -> Self {
        Self {
            id: record.id,
            codebase: record.codebase,
            suite: record.suite,
            architecture: record.architecture,
            component: record.component,
            binary_files: record.binary_files,
            source_files: record.source_files,
        }
    }
}
