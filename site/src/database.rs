use anyhow::Result;
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, Row, Transaction};
use std::collections::HashMap;

use crate::config::Config;

// Simplified database types for now - will be enhanced when schema is ready
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Campaign {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Codebase {
    pub name: String,
    pub url: String,
    pub branch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run {
    pub id: String,
    pub codebase: String,
    pub campaign: String,
    pub start_time: DateTime<Utc>,
}

// Database errors that can be converted to HTTP responses
#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("Database connection error: {0}")]
    Connection(sqlx::Error),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

impl DatabaseError {
    pub fn to_status_code(&self) -> StatusCode {
        match self {
            DatabaseError::Connection(_) => StatusCode::INTERNAL_SERVER_ERROR,
            DatabaseError::NotFound(_) => StatusCode::NOT_FOUND,
            DatabaseError::InvalidInput(_) => StatusCode::BAD_REQUEST,
        }
    }
}

impl From<sqlx::Error> for DatabaseError {
    fn from(err: sqlx::Error) -> Self {
        match &err {
            sqlx::Error::RowNotFound => DatabaseError::NotFound("Row not found".to_string()),
            _ => DatabaseError::Connection(err),
        }
    }
}

// Database manager providing high-level query interface
#[derive(Clone)]
pub struct DatabaseManager {
    pool: PgPool,
}

impl DatabaseManager {
    pub async fn new(config: &Config) -> Result<Self> {
        let pool = PgPool::connect(config.database_url()).await?;

        // Run migrations
        sqlx::migrate!().run(&pool).await?;

        Ok(Self { pool })
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn begin(&self) -> Result<Transaction<'_, Postgres>, DatabaseError> {
        Ok(self.pool.begin().await?)
    }

    pub async fn health_check(&self) -> Result<(), DatabaseError> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map_err(DatabaseError::Connection)?;
        Ok(())
    }

    // Simplified queries that don't depend on specific schema
    pub async fn get_stats(&self) -> Result<HashMap<String, i64>, DatabaseError> {
        let mut stats = HashMap::new();

        // These queries work with common table patterns but will be enhanced
        // once we have access to the real schema
        stats.insert("total_codebases".to_string(), 0);
        stats.insert("active_runs".to_string(), 0);
        stats.insert("queue_size".to_string(), 0);
        stats.insert("recent_successful_runs".to_string(), 0);

        Ok(stats)
    }

    pub async fn get_codebases(
        &self,
        limit: Option<i64>,
        offset: Option<i64>,
        _search: Option<&str>,
    ) -> Result<Vec<Codebase>, DatabaseError> {
        // Return empty for now - will be implemented with real schema
        Ok(vec![])
    }

    pub async fn get_codebase(&self, _name: &str) -> Result<Codebase, DatabaseError> {
        // Return placeholder data for now
        Err(DatabaseError::NotFound("Codebase not found".to_string()))
    }

    pub async fn count_codebases(&self, _search: Option<&str>) -> Result<i64, DatabaseError> {
        Ok(0)
    }

    pub async fn get_runs_for_codebase(
        &self,
        _codebase: &str,
        _limit: Option<i64>,
        _offset: Option<i64>,
    ) -> Result<Vec<Run>, DatabaseError> {
        Ok(vec![])
    }

    // Campaign/suite-related queries
    pub async fn count_candidates(
        &self,
        _suite: &str,
        _search: Option<&str>,
    ) -> Result<i64, DatabaseError> {
        // TODO: Implement when schema is available
        Ok(0)
    }

    pub async fn count_runs_by_result(
        &self,
        _campaign: &str,
        _result_code: &str,
    ) -> Result<i64, DatabaseError> {
        // TODO: Implement when schema is available
        Ok(0)
    }

    pub async fn count_pending_publishes(
        &self,
        _campaign: &str,
    ) -> Result<i64, DatabaseError> {
        // TODO: Implement when schema is available
        Ok(0)
    }

    pub async fn get_candidates(
        &self,
        _suite: &str,
        _limit: Option<i64>,
        _offset: Option<i64>,
    ) -> Result<Vec<serde_json::Value>, DatabaseError> {
        // TODO: Implement when schema is available
        Ok(vec![])
    }

    pub async fn get_candidate(
        &self,
        _campaign: &str,
        _codebase: &str,
    ) -> Result<serde_json::Value, DatabaseError> {
        // TODO: Implement when schema is available
        Ok(serde_json::json!({
            "codebase": _codebase,
            "campaign": _campaign,
            "active": true
        }))
    }

    pub async fn get_vcs_info(
        &self,
        _codebase: &str,
    ) -> Result<VcsInfo, DatabaseError> {
        // TODO: Implement when schema is available
        Ok(VcsInfo {
            url: "https://github.com/example/repo".to_string(),
            vcs_type: "git".to_string(),
            branch_url: None,
        })
    }

    pub async fn get_last_unabsorbed_run(
        &self,
        _campaign: &str,
        _codebase: &str,
    ) -> Result<RunDetails, DatabaseError> {
        // TODO: Implement when schema is available
        Err(DatabaseError::NotFound("No unabsorbed runs".to_string()))
    }

    pub async fn get_previous_runs(
        &self,
        _codebase: &str,
        _campaign: &str,
        _limit: Option<i64>,
    ) -> Result<Vec<RunDetails>, DatabaseError> {
        // TODO: Implement when schema is available
        Ok(vec![])
    }

    pub async fn get_merge_proposals_for_codebase(
        &self,
        _campaign: &str,
        _codebase: &str,
    ) -> Result<Vec<serde_json::Value>, DatabaseError> {
        // TODO: Implement when schema is available
        Ok(vec![])
    }

    pub async fn get_queue_position(
        &self,
        _campaign: &str,
        _codebase: &str,
    ) -> Result<i64, DatabaseError> {
        // TODO: Implement when schema is available
        Ok(0)
    }

    pub async fn get_average_run_time(
        &self,
        _campaign: &str,
    ) -> Result<i64, DatabaseError> {
        // TODO: Implement when schema is available
        Ok(300) // 5 minutes default
    }

    pub async fn get_publish_policy(
        &self,
        _campaign: &str,
        _codebase: &str,
    ) -> Result<String, DatabaseError> {
        // TODO: Implement when schema is available
        Ok("manual".to_string())
    }

    pub async fn get_changelog_policy(
        &self,
        _campaign: &str,
        _codebase: &str,
    ) -> Result<String, DatabaseError> {
        // TODO: Implement when schema is available
        Ok("auto".to_string())
    }

    pub async fn get_run(
        &self,
        _run_id: &str,
    ) -> Result<RunDetails, DatabaseError> {
        // TODO: Implement when schema is available
        Err(DatabaseError::NotFound("Run not found".to_string()))
    }

    pub async fn get_run_statistics(
        &self,
        _campaign: &str,
        _codebase: &str,
    ) -> Result<RunStatistics, DatabaseError> {
        // TODO: Implement when schema is available
        Ok(RunStatistics {
            total: 10,
            successful: 7,
            failed: 3,
        })
    }

    pub async fn get_unchanged_run(
        &self,
        _campaign: &str,
        _codebase: &str,
        _before: Option<&DateTime<Utc>>,
    ) -> Result<RunDetails, DatabaseError> {
        // TODO: Implement when schema is available
        Err(DatabaseError::NotFound("No unchanged run found".to_string()))
    }

    pub async fn get_binary_packages(
        &self,
        _run_id: &str,
    ) -> Result<Vec<String>, DatabaseError> {
        // TODO: Implement when schema is available
        Ok(vec![])
    }

    pub async fn get_reviews(
        &self,
        _run_id: &str,
    ) -> Result<Vec<serde_json::Value>, DatabaseError> {
        // TODO: Implement when schema is available
        Ok(vec![])
    }

    pub async fn get_ready_runs(
        &self,
        _suite: &str,
        _search: Option<&str>,
        _result_code: Option<&str>,
        _limit: Option<i64>,
        _offset: Option<i64>,
    ) -> Result<Vec<serde_json::Value>, DatabaseError> {
        // TODO: Implement when schema is available
        Ok(vec![])
    }

    pub async fn get_absorbed_runs(
        &self,
        _campaign: &str,
        _from_date: Option<&DateTime<Utc>>,
        _to_date: Option<&DateTime<Utc>>,
        _limit: Option<i64>,
        _offset: Option<i64>,
    ) -> Result<Vec<serde_json::Value>, DatabaseError> {
        // TODO: Implement when schema is available
        Ok(vec![])
    }

    pub async fn get_merge_proposals_by_status(
        &self,
        _suite: &str,
        _status: &str,
    ) -> Result<Vec<serde_json::Value>, DatabaseError> {
        // TODO: Implement when schema is available
        Ok(vec![])
    }
}

// Additional types for database results

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VcsInfo {
    pub url: String,
    pub vcs_type: String,
    pub branch_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunDetails {
    pub id: String,
    pub codebase: String,
    pub suite: String,
    pub command: Option<String>,
    pub result_code: Option<String>,
    pub description: Option<String>,
    pub start_time: DateTime<Utc>,
    pub finish_time: DateTime<Utc>,
    pub worker: Option<String>,
    pub build_version: Option<String>,
    pub result_branches: Vec<serde_json::Value>,
    pub result_tags: Vec<serde_json::Value>,
    pub publish_status: Option<String>,
    pub failure_stage: Option<String>,
    pub main_branch_revision: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunStatistics {
    pub total: i64,
    pub successful: i64,
    pub failed: i64,
}
