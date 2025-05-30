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
}
