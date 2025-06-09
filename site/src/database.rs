use anyhow::Result;
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
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

    /// Run database migrations
    pub async fn run_migrations(&self) -> Result<(), DatabaseError> {
        sqlx::migrate!()
            .run(&self.pool)
            .await
            .map_err(|e| DatabaseError::Connection(sqlx::Error::Migrate(Box::new(e))))?;
        Ok(())
    }

    /// Clear test data (for integration tests)
    #[cfg(test)]
    pub async fn clear_test_data(&self) -> Result<(), DatabaseError> {
        // Delete test data in reverse dependency order
        sqlx::query("DELETE FROM run WHERE id LIKE 'test-%'")
            .execute(&self.pool)
            .await
            .map_err(DatabaseError::Connection)?;

        sqlx::query("DELETE FROM codebase WHERE name LIKE 'test-%'")
            .execute(&self.pool)
            .await
            .map_err(DatabaseError::Connection)?;

        Ok(())
    }

    // Get global statistics for homepage
    pub async fn get_stats(&self) -> Result<HashMap<String, i64>, DatabaseError> {
        let mut stats = HashMap::new();

        // Total codebases
        let total_codebases: i64 =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM codebase WHERE NOT inactive")
                .fetch_one(&self.pool)
                .await?;
        stats.insert("total_codebases".to_string(), total_codebases);

        // Active runs (currently running)
        let active_runs: i64 =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM run WHERE finish_time IS NULL")
                .fetch_one(&self.pool)
                .await?;
        stats.insert("active_runs".to_string(), active_runs);

        // Queue size
        let queue_size: i64 = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM queue")
            .fetch_one(&self.pool)
            .await?;
        stats.insert("queue_size".to_string(), queue_size);

        // Recent successful runs (last 24 hours)
        let recent_successful_runs: i64 = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM run WHERE result_code = 'success' AND finish_time > now() - interval '24 hours'"
        )
        .fetch_one(&self.pool)
        .await?;
        stats.insert("recent_successful_runs".to_string(), recent_successful_runs);

        Ok(stats)
    }

    pub async fn get_codebases(
        &self,
        limit: Option<i64>,
        offset: Option<i64>,
        search: Option<&str>,
    ) -> Result<Vec<Codebase>, DatabaseError> {
        let mut query = sqlx::query(
            "SELECT name, url, branch FROM codebase 
             WHERE NOT inactive 
             AND ($1::text IS NULL OR (name ILIKE '%' || $1 || '%' OR url ILIKE '%' || $1 || '%'))
             ORDER BY name
             LIMIT $2
             OFFSET $3",
        );

        query = query
            .bind(search)
            .bind(limit.unwrap_or(i64::MAX))
            .bind(offset.unwrap_or(0));

        let rows = query.fetch_all(&self.pool).await?;

        let mut codebases = Vec::new();
        for row in rows {
            codebases.push(Codebase {
                name: row.try_get("name")?,
                url: row.try_get("url")?,
                branch: row.try_get("branch")?,
            });
        }

        Ok(codebases)
    }

    pub async fn get_codebase(&self, name: &str) -> Result<Codebase, DatabaseError> {
        let row =
            sqlx::query("SELECT name, url, branch FROM codebase WHERE name = $1 AND NOT inactive")
                .bind(name)
                .fetch_one(&self.pool)
                .await?;

        Ok(Codebase {
            name: row.try_get("name")?,
            url: row.try_get("url")?,
            branch: row.try_get("branch")?,
        })
    }

    pub async fn get_repositories_by_vcs(
        &self,
        vcs_type: &str,
    ) -> Result<Vec<Codebase>, DatabaseError> {
        let rows = sqlx::query(
            "SELECT name, url, branch FROM codebase 
             WHERE url LIKE $1 AND NOT inactive 
             ORDER BY name LIMIT 1000",
        )
        .bind(format!("{}://%", vcs_type))
        .fetch_all(&self.pool)
        .await?;

        let repositories = rows
            .into_iter()
            .map(|row| Codebase {
                name: row.try_get("name").unwrap_or_default(),
                url: row.try_get("url").unwrap_or_default(),
                branch: row.try_get("branch").ok(),
            })
            .collect();

        Ok(repositories)
    }

    pub async fn count_codebases(&self, search: Option<&str>) -> Result<i64, DatabaseError> {
        let mut query = "SELECT COUNT(*) FROM codebase WHERE NOT inactive".to_string();

        if let Some(search_term) = search {
            query.push_str(&format!(
                " AND (name ILIKE '%{}%' OR url ILIKE '%{}%')",
                search_term.replace("%", "\\%").replace("_", "\\_"),
                search_term.replace("%", "\\%").replace("_", "\\_")
            ));
        }

        let count: i64 = sqlx::query_scalar::<_, i64>(&query)
            .fetch_one(&self.pool)
            .await?;

        Ok(count)
    }

    pub async fn get_runs_for_codebase(
        &self,
        codebase: &str,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<Run>, DatabaseError> {
        let mut query = "SELECT id, codebase, suite, start_time FROM run WHERE codebase = $1 ORDER BY start_time DESC".to_string();

        if let Some(limit) = limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }

        let rows = sqlx::query(&query)
            .bind(codebase)
            .fetch_all(&self.pool)
            .await?;

        let mut runs = Vec::new();
        for row in rows {
            runs.push(Run {
                id: row.try_get("id")?,
                codebase: row.try_get("codebase")?,
                campaign: row.try_get("suite")?,
                start_time: row.try_get("start_time")?,
            });
        }

        Ok(runs)
    }

    // Campaign/suite-related queries
    pub async fn count_candidates(
        &self,
        suite: &str,
        search: Option<&str>,
    ) -> Result<i64, DatabaseError> {
        let mut query = "SELECT COUNT(*) FROM candidate WHERE suite = $1".to_string();

        if let Some(search_term) = search {
            query.push_str(&format!(
                " AND codebase ILIKE '%{}%'",
                search_term.replace("%", "\\%").replace("_", "\\_")
            ));
        }

        let count: i64 = sqlx::query_scalar::<_, i64>(&query)
            .bind(suite)
            .fetch_one(&self.pool)
            .await?;

        Ok(count)
    }

    pub async fn count_runs_by_result(
        &self,
        campaign: &str,
        result_code: &str,
    ) -> Result<i64, DatabaseError> {
        let count: i64 = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM run WHERE suite = $1 AND result_code = $2",
        )
        .bind(campaign)
        .bind(result_code)
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }

    pub async fn count_pending_publishes(&self, campaign: &str) -> Result<i64, DatabaseError> {
        let count: i64 =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM publish_ready WHERE suite = $1")
                .bind(campaign)
                .fetch_one(&self.pool)
                .await?;

        Ok(count)
    }

    pub async fn get_candidates(
        &self,
        suite: &str,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<serde_json::Value>, DatabaseError> {
        let mut query = "SELECT candidate.codebase, candidate.suite, candidate.command, candidate.context, candidate.value, candidate.success_chance, candidate.publish_policy, codebase.url, codebase.branch FROM candidate LEFT JOIN codebase ON candidate.codebase = codebase.name WHERE candidate.suite = $1 ORDER BY candidate.value DESC, candidate.codebase".to_string();

        if let Some(limit) = limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }

        let rows = sqlx::query(&query)
            .bind(suite)
            .fetch_all(&self.pool)
            .await?;

        let mut candidates = Vec::new();
        for row in rows {
            let candidate = serde_json::json!({
                "codebase": row.try_get::<String, _>("codebase")?,
                "suite": row.try_get::<String, _>("suite")?,
                "command": row.try_get::<Option<String>, _>("command")?,
                "context": row.try_get::<Option<String>, _>("context")?,
                "value": row.try_get::<Option<i32>, _>("value")?,
                "success_chance": row.try_get::<Option<f64>, _>("success_chance")?,
                "publish_policy": row.try_get::<Option<String>, _>("publish_policy")?,
                "url": row.try_get::<Option<String>, _>("url")?,
                "branch": row.try_get::<Option<String>, _>("branch")?
            });
            candidates.push(candidate);
        }

        Ok(candidates)
    }

    pub async fn get_candidate(
        &self,
        campaign: &str,
        codebase: &str,
    ) -> Result<serde_json::Value, DatabaseError> {
        let row = sqlx::query(
            "SELECT candidate.codebase, candidate.suite, candidate.command, candidate.context, candidate.value, candidate.success_chance, candidate.publish_policy, codebase.url, codebase.branch, codebase.vcs_type FROM candidate LEFT JOIN codebase ON candidate.codebase = codebase.name WHERE candidate.suite = $1 AND candidate.codebase = $2"
        )
        .bind(campaign)
        .bind(codebase)
        .fetch_one(&self.pool)
        .await?;

        let candidate = serde_json::json!({
            "codebase": row.try_get::<String, _>("codebase")?,
            "suite": row.try_get::<String, _>("suite")?,
            "command": row.try_get::<Option<String>, _>("command")?,
            "context": row.try_get::<Option<String>, _>("context")?,
            "value": row.try_get::<Option<i32>, _>("value")?,
            "success_chance": row.try_get::<Option<f64>, _>("success_chance")?,
            "publish_policy": row.try_get::<Option<String>, _>("publish_policy")?,
            "url": row.try_get::<Option<String>, _>("url")?,
            "branch": row.try_get::<Option<String>, _>("branch")?,
            "vcs_type": row.try_get::<Option<String>, _>("vcs_type")?
        });

        Ok(candidate)
    }

    pub async fn get_vcs_info(&self, codebase: &str) -> Result<VcsInfo, DatabaseError> {
        let row =
            sqlx::query("SELECT url, branch_url, vcs_type, web_url FROM codebase WHERE name = $1")
                .bind(codebase)
                .fetch_one(&self.pool)
                .await?;

        Ok(VcsInfo {
            url: row.try_get::<Option<String>, _>("url")?.unwrap_or_default(),
            vcs_type: row
                .try_get::<Option<String>, _>("vcs_type")?
                .unwrap_or("unknown".to_string()),
            branch_url: row.try_get::<Option<String>, _>("web_url")?,
        })
    }

    pub async fn get_last_unabsorbed_run(
        &self,
        campaign: &str,
        codebase: &str,
    ) -> Result<RunDetails, DatabaseError> {
        let row = sqlx::query(
            "SELECT run.id, run.codebase, run.suite, run.command, run.result_code, run.description, 
             run.start_time, run.finish_time, run.worker, run.failure_stage, run.main_branch_revision,
             run.vcs_type, run.logfilenames, run.revision, run.publish_status, run.result_tags
             FROM last_unabsorbed_runs run WHERE run.suite = $1 AND run.codebase = $2"
        )
        .bind(campaign)
        .bind(codebase)
        .fetch_one(&self.pool)
        .await?;

        // Get result branches for this run
        let result_branches = self.get_result_branches(&row.try_get::<String, _>("id")?).await?;

        Ok(RunDetails {
            id: row.try_get("id")?,
            codebase: row.try_get("codebase")?,
            suite: row.try_get("suite")?,
            command: row.try_get("command")?,
            result_code: row.try_get("result_code")?,
            description: row.try_get("description")?,
            start_time: row.try_get("start_time")?,
            finish_time: row.try_get("finish_time")?,
            worker: row.try_get("worker")?,
            build_version: None, // Not in schema
            result_branches,
            result_tags: row.try_get::<Option<Vec<serde_json::Value>>, _>("result_tags")?
                .unwrap_or_default(),
            publish_status: row.try_get::<Option<String>, _>("publish_status")?,
            failure_stage: row.try_get("failure_stage")?,
            main_branch_revision: row.try_get("main_branch_revision")?,
            vcs_type: row.try_get::<Option<String>, _>("vcs_type")?,
            logfilenames: row.try_get::<Option<Vec<String>>, _>("logfilenames")?
                .unwrap_or_default(),
            revision: row.try_get("revision")?,
        })
    }

    /// Helper method to get result branches for a run
    async fn get_result_branches(&self, run_id: &str) -> Result<Vec<serde_json::Value>, DatabaseError> {
        let rows = sqlx::query(
            "SELECT role, remote_name, base_revision, revision, absorbed 
             FROM new_result_branch WHERE run_id = $1"
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;

        let mut result_branches = Vec::new();
        for row in rows {
            let branch = serde_json::json!({
                "role": row.try_get::<String, _>("role")?,
                "remote_name": row.try_get::<Option<String>, _>("remote_name")?,
                "base_revision": row.try_get::<Option<String>, _>("base_revision")?,
                "revision": row.try_get::<Option<String>, _>("revision")?,
                "absorbed": row.try_get::<Option<bool>, _>("absorbed")?.unwrap_or(false)
            });
            result_branches.push(branch);
        }

        Ok(result_branches)
    }

    pub async fn get_previous_runs(
        &self,
        codebase: &str,
        campaign: &str,
        limit: Option<i64>,
    ) -> Result<Vec<RunDetails>, DatabaseError> {
        let mut query = "SELECT id, codebase, suite, command, result_code, description, start_time, finish_time, worker, failure_stage, main_branch_revision, vcs_type, logfilenames, revision, publish_status, result_tags FROM run WHERE codebase = $1 AND suite = $2 ORDER BY start_time DESC".to_string();

        if let Some(limit) = limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        let rows = sqlx::query(&query)
            .bind(codebase)
            .bind(campaign)
            .fetch_all(&self.pool)
            .await?;

        let mut runs = Vec::new();
        for row in rows {
            let run_id = row.try_get::<String, _>("id")?;
            let result_branches = self.get_result_branches(&run_id).await?;

            runs.push(RunDetails {
                id: run_id,
                codebase: row.try_get("codebase")?,
                suite: row.try_get("suite")?,
                command: row.try_get("command")?,
                result_code: row.try_get("result_code")?,
                description: row.try_get("description")?,
                start_time: row.try_get("start_time")?,
                finish_time: row.try_get("finish_time")?,
                worker: row.try_get("worker")?,
                build_version: None, // Not in schema
                result_branches,
                result_tags: row.try_get::<Option<Vec<serde_json::Value>>, _>("result_tags")?
                    .unwrap_or_default(),
                publish_status: row.try_get::<Option<String>, _>("publish_status")?,
                failure_stage: row.try_get("failure_stage")?,
                main_branch_revision: row.try_get("main_branch_revision")?,
                vcs_type: row.try_get::<Option<String>, _>("vcs_type")?,
                logfilenames: row.try_get::<Option<Vec<String>>, _>("logfilenames")?
                    .unwrap_or_default(),
                revision: row.try_get("revision")?,
            });
        }

        Ok(runs)
    }

    pub async fn get_merge_proposals_for_codebase(
        &self,
        campaign: &str,
        codebase: &str,
    ) -> Result<Vec<serde_json::Value>, DatabaseError> {
        let rows = sqlx::query(
            "SELECT mp.url, mp.status, mp.revision, mp.merged_by, mp.merged_at, mp.can_be_merged FROM merge_proposal mp INNER JOIN run r ON mp.revision = r.revision WHERE r.suite = $1 AND r.codebase = $2 ORDER BY mp.merged_at DESC NULLS LAST"
        )
        .bind(campaign)
        .bind(codebase)
        .fetch_all(&self.pool)
        .await?;

        let mut proposals = Vec::new();
        for row in rows {
            let proposal = serde_json::json!({
                "url": row.try_get::<String, _>("url")?,
                "status": row.try_get::<Option<String>, _>("status")?,
                "revision": row.try_get::<Option<String>, _>("revision")?,
                "merged_by": row.try_get::<Option<String>, _>("merged_by")?,
                "merged_at": row.try_get::<Option<DateTime<Utc>>, _>("merged_at")?,
                "can_be_merged": row.try_get::<Option<bool>, _>("can_be_merged")?
            });
            proposals.push(proposal);
        }

        Ok(proposals)
    }

    pub async fn get_queue_position(
        &self,
        campaign: &str,
        codebase: &str,
    ) -> Result<i64, DatabaseError> {
        let position: Option<i64> = sqlx::query_scalar::<_, Option<i64>>(
            "SELECT position FROM queue_positions WHERE suite = $1 AND codebase = $2",
        )
        .bind(campaign)
        .bind(codebase)
        .fetch_optional(&self.pool)
        .await?
        .flatten();

        Ok(position.unwrap_or(0))
    }

    pub async fn get_average_run_time(&self, campaign: &str) -> Result<i64, DatabaseError> {
        let avg_seconds: Option<f64> = sqlx::query_scalar::<_, Option<f64>>(
            "SELECT EXTRACT(EPOCH FROM AVG(finish_time - start_time)) FROM run WHERE suite = $1 AND finish_time IS NOT NULL AND start_time IS NOT NULL"
        )
        .bind(campaign)
        .fetch_optional(&self.pool)
        .await?
        .flatten();

        Ok(avg_seconds.unwrap_or(300.0) as i64) // Default to 5 minutes
    }

    pub async fn get_publish_policy(
        &self,
        campaign: &str,
        codebase: &str,
    ) -> Result<String, DatabaseError> {
        let policy: Option<String> = sqlx::query_scalar::<_, Option<String>>(
            "SELECT publish_policy FROM candidate WHERE suite = $1 AND codebase = $2",
        )
        .bind(campaign)
        .bind(codebase)
        .fetch_optional(&self.pool)
        .await?
        .flatten();

        Ok(policy.unwrap_or("manual".to_string()))
    }

    pub async fn get_changelog_policy(
        &self,
        campaign: &str,
        codebase: &str,
    ) -> Result<String, DatabaseError> {
        // Changelog policy is not directly stored in the schema,
        // typically determined by publish policy or campaign configuration
        Ok("auto".to_string())
    }

    pub async fn get_run(&self, run_id: &str) -> Result<RunDetails, DatabaseError> {
        let row = sqlx::query(
            "SELECT id, codebase, suite, command, result_code, description, start_time, finish_time, worker, failure_stage, main_branch_revision, logfilenames, vcs_type, revision, publish_status, result_tags FROM run WHERE id = $1"
        )
        .bind(run_id)
        .fetch_one(&self.pool)
        .await?;

        // Get result branches for this run
        let result_branches = self.get_result_branches(run_id).await?;

        // Parse logfilenames array from the database
        let logfilenames: Vec<String> = row
            .try_get::<Option<Vec<String>>, _>("logfilenames")?
            .unwrap_or_default();

        Ok(RunDetails {
            id: row.try_get("id")?,
            codebase: row.try_get("codebase")?,
            suite: row.try_get("suite")?,
            command: row.try_get("command")?,
            result_code: row.try_get("result_code")?,
            description: row.try_get("description")?,
            start_time: row.try_get("start_time")?,
            finish_time: row.try_get("finish_time")?,
            worker: row.try_get("worker")?,
            build_version: None, // Not in schema
            result_branches,
            result_tags: row.try_get::<Option<Vec<serde_json::Value>>, _>("result_tags")?
                .unwrap_or_default(),
            publish_status: row.try_get::<Option<String>, _>("publish_status")?,
            failure_stage: row.try_get("failure_stage")?,
            main_branch_revision: row.try_get("main_branch_revision")?,
            vcs_type: row.try_get::<Option<String>, _>("vcs_type")?,
            logfilenames,
            revision: row.try_get("revision")?,
        })
    }

    pub async fn get_run_statistics(
        &self,
        campaign: &str,
        codebase: &str,
    ) -> Result<RunStatistics, DatabaseError> {
        let row = sqlx::query(
            "SELECT COUNT(*) as total, COUNT(CASE WHEN result_code = 'success' THEN 1 END) as successful, COUNT(CASE WHEN result_code != 'success' THEN 1 END) as failed FROM run WHERE suite = $1 AND codebase = $2"
        )
        .bind(campaign)
        .bind(codebase)
        .fetch_one(&self.pool)
        .await?;

        Ok(RunStatistics {
            total: row.try_get::<i64, _>("total")?,
            successful: row.try_get::<i64, _>("successful")?,
            failed: row.try_get::<i64, _>("failed")?,
        })
    }

    pub async fn get_unchanged_run(
        &self,
        campaign: &str,
        codebase: &str,
        before: Option<&DateTime<Utc>>,
    ) -> Result<RunDetails, DatabaseError> {
        let mut query = "SELECT id, codebase, suite, command, result_code, description, start_time, finish_time, worker, failure_stage, main_branch_revision, vcs_type, logfilenames, revision, publish_status, result_tags FROM run WHERE suite = 'unchanged' AND codebase = $1".to_string();

        let mut bind_count = 1;
        if let Some(before_time) = before {
            bind_count += 1;
            query.push_str(&format!(" AND start_time < ${}", bind_count));
        }

        query.push_str(" ORDER BY start_time DESC LIMIT 1");

        let mut sql_query = sqlx::query(&query).bind(codebase);

        if let Some(before_time) = before {
            sql_query = sql_query.bind(before_time);
        }

        let row = sql_query.fetch_one(&self.pool).await?;

        // Get result branches for this run
        let run_id = row.try_get::<String, _>("id")?;
        let result_branches = self.get_result_branches(&run_id).await?;

        Ok(RunDetails {
            id: run_id,
            codebase: row.try_get("codebase")?,
            suite: row.try_get("suite")?,
            command: row.try_get("command")?,
            result_code: row.try_get("result_code")?,
            description: row.try_get("description")?,
            start_time: row.try_get("start_time")?,
            finish_time: row.try_get("finish_time")?,
            worker: row.try_get("worker")?,
            build_version: None, // Not in schema
            result_branches,
            result_tags: row.try_get::<Option<Vec<serde_json::Value>>, _>("result_tags")?
                .unwrap_or_default(),
            publish_status: row.try_get::<Option<String>, _>("publish_status")?,
            failure_stage: row.try_get("failure_stage")?,
            main_branch_revision: row.try_get("main_branch_revision")?,
            vcs_type: row.try_get::<Option<String>, _>("vcs_type")?,
            logfilenames: row.try_get::<Option<Vec<String>>, _>("logfilenames")?
                .unwrap_or_default(),
            revision: row.try_get("revision")?,
        })
    }

    pub async fn get_binary_packages(&self, run_id: &str) -> Result<Vec<String>, DatabaseError> {
        let packages: Option<Vec<String>> = sqlx::query_scalar::<_, Option<Vec<String>>>(
            "SELECT binary_packages FROM debian_build WHERE run_id = $1",
        )
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await?
        .flatten();

        Ok(packages.unwrap_or_default())
    }

    pub async fn get_reviews(&self, run_id: &str) -> Result<Vec<serde_json::Value>, DatabaseError> {
        let rows = sqlx::query(
            "SELECT comment, reviewer, verdict, reviewed_at FROM review WHERE run_id = $1 ORDER BY reviewed_at DESC"
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;

        let mut reviews = Vec::new();
        for row in rows {
            let review = serde_json::json!({
                "comment": row.try_get::<Option<String>, _>("comment")?,
                "reviewer": row.try_get::<String, _>("reviewer")?,
                "verdict": row.try_get::<String, _>("verdict")?,
                "reviewed_at": row.try_get::<DateTime<Utc>, _>("reviewed_at")?
            });
            reviews.push(review);
        }

        Ok(reviews)
    }

    pub async fn get_ready_runs(
        &self,
        suite: &str,
        search: Option<&str>,
        result_code: Option<&str>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<serde_json::Value>, DatabaseError> {
        let mut query = "SELECT id, codebase, command, description, start_time, finish_time, result_code, value, publish_policy_name, publish_status FROM publish_ready WHERE suite = $1".to_string();
        let mut bind_count = 1;

        if let Some(search_term) = search {
            bind_count += 1;
            query.push_str(&format!(" AND codebase ILIKE ${}", bind_count));
        }

        if let Some(code) = result_code {
            bind_count += 1;
            query.push_str(&format!(" AND result_code = ${}", bind_count));
        }

        query.push_str(" ORDER BY value DESC, start_time DESC");

        if let Some(limit) = limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }

        let mut sql_query = sqlx::query(&query).bind(suite);

        if let Some(search_term) = search {
            sql_query = sql_query.bind(format!("%{}%", search_term));
        }

        if let Some(code) = result_code {
            sql_query = sql_query.bind(code);
        }

        let rows = sql_query.fetch_all(&self.pool).await?;

        let mut runs = Vec::new();
        for row in rows {
            let run = serde_json::json!({
                "id": row.try_get::<String, _>("id")?,
                "codebase": row.try_get::<String, _>("codebase")?,
                "command": row.try_get::<Option<String>, _>("command")?,
                "description": row.try_get::<Option<String>, _>("description")?,
                "start_time": row.try_get::<Option<DateTime<Utc>>, _>("start_time")?,
                "finish_time": row.try_get::<Option<DateTime<Utc>>, _>("finish_time")?,
                "result_code": row.try_get::<String, _>("result_code")?,
                "value": row.try_get::<Option<i32>, _>("value")?,
                "publish_policy": row.try_get::<Option<String>, _>("publish_policy_name")?,
                "publish_status": row.try_get::<Option<String>, _>("publish_status")?
            });
            runs.push(run);
        }

        Ok(runs)
    }

    pub async fn get_absorbed_runs(
        &self,
        campaign: &str,
        from_date: Option<&DateTime<Utc>>,
        to_date: Option<&DateTime<Utc>>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<serde_json::Value>, DatabaseError> {
        let mut query = "SELECT id, codebase, campaign, result, absorbed_at, merged_by, merge_proposal_url, revision, delay FROM absorbed_runs WHERE campaign = $1".to_string();
        let mut bind_count = 1;

        if let Some(from) = from_date {
            bind_count += 1;
            query.push_str(&format!(" AND absorbed_at >= ${}", bind_count));
        }

        if let Some(to) = to_date {
            bind_count += 1;
            query.push_str(&format!(" AND absorbed_at <= ${}", bind_count));
        }

        query.push_str(" ORDER BY absorbed_at DESC");

        if let Some(limit) = limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }

        let mut sql_query = sqlx::query(&query).bind(campaign);

        if let Some(from) = from_date {
            sql_query = sql_query.bind(from);
        }

        if let Some(to) = to_date {
            sql_query = sql_query.bind(to);
        }

        let rows = sql_query.fetch_all(&self.pool).await?;

        let mut runs = Vec::new();
        for row in rows {
            let run = serde_json::json!({
                "id": row.try_get::<String, _>("id")?,
                "codebase": row.try_get::<String, _>("codebase")?,
                "campaign": row.try_get::<String, _>("campaign")?,
                "result": row.try_get::<Option<serde_json::Value>, _>("result")?,
                "absorbed_at": row.try_get::<Option<DateTime<Utc>>, _>("absorbed_at")?,
                "merged_by": row.try_get::<Option<String>, _>("merged_by")?,
                "merge_proposal_url": row.try_get::<Option<String>, _>("merge_proposal_url")?,
                "revision": row.try_get::<Option<String>, _>("revision")?
            });
            runs.push(run);
        }

        Ok(runs)
    }

    pub async fn get_merge_proposals_by_status(
        &self,
        suite: &str,
        status: &str,
    ) -> Result<Vec<serde_json::Value>, DatabaseError> {
        let rows = sqlx::query(
            "SELECT mp.url, mp.status, mp.revision, mp.merged_by, mp.merged_at, mp.can_be_merged, r.codebase FROM merge_proposal mp INNER JOIN run r ON mp.revision = r.revision WHERE r.suite = $1 AND mp.status = $2 ORDER BY mp.merged_at DESC NULLS LAST"
        )
        .bind(suite)
        .bind(status)
        .fetch_all(&self.pool)
        .await?;

        let mut proposals = Vec::new();
        for row in rows {
            let proposal = serde_json::json!({
                "url": row.try_get::<String, _>("url")?,
                "status": row.try_get::<Option<String>, _>("status")?,
                "revision": row.try_get::<Option<String>, _>("revision")?,
                "merged_by": row.try_get::<Option<String>, _>("merged_by")?,
                "merged_at": row.try_get::<Option<DateTime<Utc>>, _>("merged_at")?,
                "can_be_merged": row.try_get::<Option<bool>, _>("can_be_merged")?,
                "codebase": row.try_get::<String, _>("codebase")?
            });
            proposals.push(proposal);
        }

        Ok(proposals)
    }

    /// Optimized batch method to fetch merge proposals for multiple statuses in one query
    /// Eliminates N+1 query pattern when fetching proposals for all statuses
    pub async fn get_merge_proposals_by_statuses(
        &self,
        suite: &str,
        statuses: &[&str],
    ) -> Result<HashMap<String, Vec<serde_json::Value>>, DatabaseError> {
        let rows = sqlx::query(
            "SELECT mp.url, mp.status, mp.revision, mp.merged_by, mp.merged_at, mp.can_be_merged, r.codebase 
             FROM merge_proposal mp 
             INNER JOIN run r ON mp.revision = r.revision 
             WHERE r.suite = $1 AND mp.status = ANY($2::text[]) 
             ORDER BY mp.status, mp.merged_at DESC NULLS LAST"
        )
        .bind(suite)
        .bind(statuses)
        .fetch_all(&self.pool)
        .await?;

        let mut grouped_proposals: HashMap<String, Vec<serde_json::Value>> = HashMap::new();

        // Initialize empty vectors for all requested statuses
        for status in statuses {
            grouped_proposals.insert(status.to_string(), Vec::new());
        }

        // Group results by status
        for row in rows {
            let status = row
                .try_get::<Option<String>, _>("status")?
                .unwrap_or_else(|| "unknown".to_string());

            let proposal = serde_json::json!({
                "url": row.try_get::<String, _>("url")?,
                "status": Some(status.clone()),
                "revision": row.try_get::<Option<String>, _>("revision")?,
                "merged_by": row.try_get::<Option<String>, _>("merged_by")?,
                "merged_at": row.try_get::<Option<DateTime<Utc>>, _>("merged_at")?,
                "can_be_merged": row.try_get::<Option<bool>, _>("can_be_merged")?,
                "codebase": row.try_get::<String, _>("codebase")?
            });

            grouped_proposals.entry(status).or_default().push(proposal);
        }

        Ok(grouped_proposals)
    }

    /// Search codebase names for typeahead functionality
    pub async fn search_codebase_names(
        &self,
        search_term: Option<&str>,
        limit: Option<i64>,
    ) -> Result<Vec<String>, DatabaseError> {
        let limit = limit.unwrap_or(20);

        let query = if let Some(term) = search_term {
            // Search with prefix matching and relevance scoring
            sqlx::query_scalar::<_, String>(
                "SELECT name FROM codebase 
                 WHERE NOT inactive 
                 AND (name ILIKE $1 OR name ILIKE $2)
                 ORDER BY 
                   CASE WHEN name ILIKE $1 THEN 1 ELSE 2 END,
                   name ASC
                 LIMIT $3",
            )
            .bind(format!("{}%", term)) // Prefix match
            .bind(format!("%{}%", term)) // Contains match
            .bind(limit)
        } else {
            // Return most recently active codebases
            sqlx::query_scalar::<_, String>(
                "SELECT c.name FROM codebase c
                 LEFT JOIN run r ON c.name = r.codebase
                 WHERE NOT c.inactive
                 GROUP BY c.name
                 ORDER BY MAX(r.finish_time) DESC NULLS LAST, c.name ASC
                 LIMIT $1",
            )
            .bind(limit)
        };

        let names = query.fetch_all(&self.pool).await?;
        Ok(names)
    }

    /// Advanced package search with filtering and ranking
    pub async fn search_packages_advanced(
        &self,
        search_term: Option<&str>,
        campaign: Option<&str>,
        result_code: Option<&str>,
        publishable_only: Option<bool>,
        limit: Option<i64>,
    ) -> Result<Vec<serde_json::Value>, DatabaseError> {
        let limit = limit.unwrap_or(50);
        let publishable_only = publishable_only.unwrap_or(false);

        // Build dynamic query with relevance scoring
        let mut query_parts = vec!["SELECT DISTINCT
                c.name as codebase,
                c.summary,
                c.vcs_url,
                r.suite as campaign,
                r.result_code,
                r.finish_time,
                r.id as last_run_id,
                CASE 
                    WHEN c.name ILIKE $1 THEN 100
                    WHEN c.summary ILIKE $2 THEN 50  
                    WHEN c.name ILIKE $2 THEN 25
                    ELSE 10
                END as relevance_score
             FROM codebase c
             LEFT JOIN last_unabsorbed_runs r ON c.name = r.codebase
             WHERE NOT c.inactive"
            .to_string()];

        let mut param_count = 2; // $1 and $2 for search terms
        let mut bind_values: Vec<Box<dyn sqlx::Encode<'_, sqlx::Postgres> + Send + Sync>> = vec![];

        // Add search term filters
        if let Some(term) = search_term {
            bind_values.push(Box::new(format!("{}%", term))); // $1: prefix match
            bind_values.push(Box::new(format!("%{}%", term))); // $2: contains match
            query_parts
                .push("AND (c.name ILIKE $1 OR c.name ILIKE $2 OR c.summary ILIKE $2)".to_string());
        } else {
            bind_values.push(Box::new("".to_string())); // $1: empty for no search
            bind_values.push(Box::new("".to_string())); // $2: empty for no search
        }

        // Add campaign filter
        if let Some(campaign_filter) = campaign {
            param_count += 1;
            query_parts.push(format!("AND r.suite = ${}", param_count));
            bind_values.push(Box::new(campaign_filter.to_string()));
        }

        // Add result code filter
        if let Some(code) = result_code {
            param_count += 1;
            query_parts.push(format!("AND r.result_code = ${}", param_count));
            bind_values.push(Box::new(code.to_string()));
        }

        // Add publishable filter
        if publishable_only {
            query_parts.push("AND r.result_code = 'success'".to_string());
        }

        // Add ordering and limit
        query_parts.push("ORDER BY relevance_score DESC, c.name ASC".to_string());
        param_count += 1;
        query_parts.push(format!("LIMIT ${}", param_count));
        bind_values.push(Box::new(limit));

        let query_str = query_parts.join(" ");

        // For now, use a simpler query that we can actually execute
        // TODO: Implement proper dynamic query building
        let simplified_query = if let Some(term) = search_term {
            sqlx::query(
                "SELECT 
                    c.name as codebase,
                    c.summary,
                    c.vcs_url,
                    r.suite as campaign,
                    r.result_code,
                    r.finish_time,
                    r.id as last_run_id
                 FROM codebase c
                 LEFT JOIN last_unabsorbed_runs r ON c.name = r.codebase
                 WHERE NOT c.inactive
                 AND (c.name ILIKE $1 OR c.summary ILIKE $2)
                 ORDER BY 
                   CASE WHEN c.name ILIKE $1 THEN 1 ELSE 2 END,
                   c.name ASC
                 LIMIT $3",
            )
            .bind(format!("{}%", term))
            .bind(format!("%{}%", term))
            .bind(limit)
        } else {
            sqlx::query(
                "SELECT 
                    c.name as codebase,
                    c.summary,
                    c.vcs_url,
                    r.suite as campaign,
                    r.result_code,
                    r.finish_time,
                    r.id as last_run_id
                 FROM codebase c
                 LEFT JOIN last_unabsorbed_runs r ON c.name = r.codebase
                 WHERE NOT c.inactive
                 ORDER BY r.finish_time DESC NULLS LAST, c.name ASC
                 LIMIT $1",
            )
            .bind(limit)
        };

        let rows = simplified_query.fetch_all(&self.pool).await?;

        let mut results = Vec::new();
        for row in rows {
            let result = serde_json::json!({
                "codebase": row.try_get::<String, _>("codebase")?,
                "summary": row.try_get::<Option<String>, _>("summary")?,
                "vcs_url": row.try_get::<Option<String>, _>("vcs_url")?,
                "campaign": row.try_get::<Option<String>, _>("campaign")?,
                "result_code": row.try_get::<Option<String>, _>("result_code")?,
                "finish_time": row.try_get::<Option<DateTime<Utc>>, _>("finish_time")?,
                "last_run_id": row.try_get::<Option<String>, _>("last_run_id")?
            });
            results.push(result);
        }

        Ok(results)
    }

    // Queue management methods for admin interface

    /// Get queue items with filtering and statistics
    pub async fn get_queue_items_with_stats(
        &self,
        suite: Option<&str>,
        status: Option<&str>,
        priority: Option<&str>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<(Vec<serde_json::Value>, serde_json::Value), DatabaseError> {
        // TODO: Implement proper filtering - for now using basic query without filters

        // For now, use a simple query without complex filtering
        let rows = sqlx::query(
            "SELECT 
                q.id,
                q.codebase,
                q.suite,
                q.command,
                q.context,
                q.value as priority_value,
                q.success_chance,
                q.publish_policy,
                q.status,
                q.created_time,
                q.assigned_time,
                q.worker,
                c.url as vcs_url,
                c.branch,
                c.vcs_type
             FROM queue q
             LEFT JOIN codebase c ON q.codebase = c.name
             ORDER BY q.value DESC, q.created_time ASC
             LIMIT $1",
        )
        .bind(limit.unwrap_or(50))
        .fetch_all(&self.pool)
        .await?;

        let mut items = Vec::new();
        for row in rows {
            let item = serde_json::json!({
                "id": row.try_get::<Option<String>, _>("id")?,
                "codebase": row.try_get::<String, _>("codebase")?,
                "suite": row.try_get::<String, _>("suite")?,
                "command": row.try_get::<Option<String>, _>("command")?,
                "context": row.try_get::<Option<String>, _>("context")?,
                "priority_value": row.try_get::<Option<i32>, _>("priority_value")?,
                "success_chance": row.try_get::<Option<f64>, _>("success_chance")?,
                "publish_policy": row.try_get::<Option<String>, _>("publish_policy")?,
                "status": row.try_get::<Option<String>, _>("status")?,
                "created_time": row.try_get::<Option<DateTime<Utc>>, _>("created_time")?,
                "assigned_time": row.try_get::<Option<DateTime<Utc>>, _>("assigned_time")?,
                "worker": row.try_get::<Option<String>, _>("worker")?,
                "vcs_url": row.try_get::<Option<String>, _>("vcs_url")?,
                "branch": row.try_get::<Option<String>, _>("branch")?,
                "vcs_type": row.try_get::<Option<String>, _>("vcs_type")?
            });
            items.push(item);
        }

        // Get queue statistics
        let stats = self.get_queue_statistics().await?;

        Ok((items, stats))
    }

    /// Get comprehensive queue statistics for admin dashboard
    pub async fn get_queue_statistics(&self) -> Result<serde_json::Value, DatabaseError> {
        // Total items in queue
        let total_items: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM queue")
            .fetch_one(&self.pool)
            .await?;

        // Items by status
        let pending_items: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM queue WHERE status = 'pending' OR status IS NULL",
        )
        .fetch_one(&self.pool)
        .await?;

        let assigned_items: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM queue WHERE status = 'assigned'")
                .fetch_one(&self.pool)
                .await?;

        let running_items: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM queue WHERE status = 'running'")
                .fetch_one(&self.pool)
                .await?;

        // Worker statistics
        let active_workers: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT worker) FROM queue WHERE status = 'running' AND worker IS NOT NULL"
        )
        .fetch_one(&self.pool)
        .await?;

        // Priority distribution
        let high_priority: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM queue WHERE value >= 75")
            .fetch_one(&self.pool)
            .await?;

        let medium_priority: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM queue WHERE value >= 25 AND value < 75")
                .fetch_one(&self.pool)
                .await?;

        let low_priority: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM queue WHERE value < 25")
            .fetch_one(&self.pool)
            .await?;

        // Average wait time for pending items
        let avg_wait_time: Option<f64> = sqlx::query_scalar(
            "SELECT EXTRACT(EPOCH FROM AVG(NOW() - created_time))/3600 
             FROM queue 
             WHERE status = 'pending' OR status IS NULL",
        )
        .fetch_optional(&self.pool)
        .await?
        .flatten();

        Ok(serde_json::json!({
            "total_items": total_items,
            "pending_items": pending_items,
            "assigned_items": assigned_items,
            "running_items": running_items,
            "active_workers": active_workers,
            "priority_distribution": {
                "high": high_priority,
                "medium": medium_priority,
                "low": low_priority
            },
            "average_wait_time_hours": avg_wait_time.unwrap_or(0.0)
        }))
    }

    /// Reschedule queue items (bulk operation)
    pub async fn bulk_reschedule_queue_items(
        &self,
        item_ids: &[String],
        admin_user: &str,
    ) -> Result<i64, DatabaseError> {
        let mut tx = self.pool.begin().await?;

        let affected_rows = sqlx::query(
            "UPDATE queue 
             SET status = 'pending', 
                 assigned_time = NULL, 
                 worker = NULL, 
                 updated_time = NOW()
             WHERE id = ANY($1)",
        )
        .bind(item_ids)
        .execute(&mut *tx)
        .await?
        .rows_affected();

        // Log the bulk operation
        sqlx::query(
            "INSERT INTO admin_audit_log (timestamp, admin_user, action, target, details)
             VALUES (NOW(), $1, 'bulk_reschedule', 'queue', $2)",
        )
        .bind(admin_user)
        .bind(serde_json::json!({
            "item_ids": item_ids,
            "affected_rows": affected_rows
        }))
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(affected_rows as i64)
    }

    /// Cancel queue items (bulk operation)
    pub async fn bulk_cancel_queue_items(
        &self,
        item_ids: &[String],
        admin_user: &str,
    ) -> Result<i64, DatabaseError> {
        let mut tx = self.pool.begin().await?;

        let affected_rows = sqlx::query("DELETE FROM queue WHERE id = ANY($1)")
            .bind(item_ids)
            .execute(&mut *tx)
            .await?
            .rows_affected();

        // Log the bulk operation
        sqlx::query(
            "INSERT INTO admin_audit_log (timestamp, admin_user, action, target, details)
             VALUES (NOW(), $1, 'bulk_cancel', 'queue', $2)",
        )
        .bind(admin_user)
        .bind(serde_json::json!({
            "item_ids": item_ids,
            "affected_rows": affected_rows
        }))
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(affected_rows as i64)
    }

    /// Adjust priority of queue items (bulk operation)
    pub async fn bulk_adjust_priority(
        &self,
        item_ids: &[String],
        priority_adjustment: i32,
        admin_user: &str,
    ) -> Result<i64, DatabaseError> {
        let mut tx = self.pool.begin().await?;

        let affected_rows = sqlx::query(
            "UPDATE queue 
             SET value = GREATEST(0, LEAST(100, value + $1)),
                 updated_time = NOW()
             WHERE id = ANY($2)",
        )
        .bind(priority_adjustment)
        .bind(item_ids)
        .execute(&mut *tx)
        .await?
        .rows_affected();

        // Log the bulk operation
        sqlx::query(
            "INSERT INTO admin_audit_log (timestamp, admin_user, action, target, details)
             VALUES (NOW(), $1, 'bulk_priority_adjust', 'queue', $2)",
        )
        .bind(admin_user)
        .bind(serde_json::json!({
            "item_ids": item_ids,
            "priority_adjustment": priority_adjustment,
            "affected_rows": affected_rows
        }))
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(affected_rows as i64)
    }

    /// Get worker information for admin monitoring
    pub async fn get_worker_information(&self) -> Result<serde_json::Value, DatabaseError> {
        let workers = sqlx::query(
            "SELECT 
                worker,
                COUNT(*) as assigned_tasks,
                MIN(assigned_time) as earliest_assignment,
                MAX(assigned_time) as latest_assignment,
                COUNT(DISTINCT suite) as suite_count
             FROM queue 
             WHERE worker IS NOT NULL AND status IN ('assigned', 'running')
             GROUP BY worker
             ORDER BY assigned_tasks DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut worker_list = Vec::new();
        for row in workers {
            let worker_info = serde_json::json!({
                "worker": row.try_get::<String, _>("worker")?,
                "assigned_tasks": row.try_get::<i64, _>("assigned_tasks")?,
                "earliest_assignment": row.try_get::<Option<DateTime<Utc>>, _>("earliest_assignment")?,
                "latest_assignment": row.try_get::<Option<DateTime<Utc>>, _>("latest_assignment")?,
                "suite_count": row.try_get::<i64, _>("suite_count")?
            });
            worker_list.push(worker_info);
        }

        // Get overall worker statistics
        let total_workers: i64 =
            sqlx::query_scalar("SELECT COUNT(DISTINCT worker) FROM queue WHERE worker IS NOT NULL")
                .fetch_one(&self.pool)
                .await?;

        let idle_workers: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT w.name) 
             FROM workers w 
             LEFT JOIN queue q ON w.name = q.worker AND q.status IN ('assigned', 'running')
             WHERE q.worker IS NULL",
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0); // This query might fail if workers table doesn't exist

        Ok(serde_json::json!({
            "workers": worker_list,
            "total_workers": total_workers,
            "active_workers": worker_list.len(),
            "idle_workers": idle_workers
        }))
    }

    /// Get detailed information about a specific worker
    pub async fn get_worker_details(
        &self,
        worker_id: &str,
    ) -> Result<serde_json::Value, DatabaseError> {
        // Get worker basic info from worker table
        let worker_info = sqlx::query("SELECT name, password, link FROM worker WHERE name = $1")
            .bind(worker_id)
            .fetch_optional(&self.pool)
            .await?;

        if worker_info.is_none() {
            return Err(DatabaseError::NotFound(format!(
                "Worker '{}' not found",
                worker_id
            )));
        }

        let worker_row = worker_info.unwrap();

        // Get current assignments
        let current_assignments = sqlx::query(
            "SELECT id, codebase, suite, command, priority, estimated_duration, bucket, assigned_time
             FROM queue 
             WHERE worker = $1 AND status IN ('assigned', 'running')
             ORDER BY assigned_time DESC"
        )
        .bind(worker_id)
        .fetch_all(&self.pool)
        .await?;

        let mut assignments = Vec::new();
        for row in current_assignments {
            assignments.push(serde_json::json!({
                "queue_id": row.try_get::<i32, _>("id")?,
                "codebase": row.try_get::<String, _>("codebase")?,
                "suite": row.try_get::<String, _>("suite")?,
                "command": row.try_get::<Option<String>, _>("command")?,
                "priority": row.try_get::<i64, _>("priority")?,
                "estimated_duration": row.try_get::<Option<String>, _>("estimated_duration")?,
                "bucket": row.try_get::<String, _>("bucket")?,
                "assigned_time": row.try_get::<Option<DateTime<Utc>>, _>("assigned_time")?
            }));
        }

        // Get recent run history
        let recent_runs = sqlx::query(
            "SELECT id, codebase, suite, result_code, start_time, finish_time, duration
             FROM run 
             WHERE worker = $1 
             ORDER BY start_time DESC 
             LIMIT 10",
        )
        .bind(worker_id)
        .fetch_all(&self.pool)
        .await?;

        let mut runs = Vec::new();
        for row in recent_runs {
            runs.push(serde_json::json!({
                "run_id": row.try_get::<String, _>("id")?,
                "codebase": row.try_get::<String, _>("codebase")?,
                "suite": row.try_get::<String, _>("suite")?,
                "result_code": row.try_get::<String, _>("result_code")?,
                "start_time": row.try_get::<Option<DateTime<Utc>>, _>("start_time")?,
                "finish_time": row.try_get::<Option<DateTime<Utc>>, _>("finish_time")?,
                "duration": row.try_get::<Option<String>, _>("duration")?
            }));
        }

        // Get performance statistics
        let performance_stats = sqlx::query(
            "SELECT 
                COUNT(*) as total_runs,
                COUNT(CASE WHEN result_code = 'success' THEN 1 END) as successful_runs,
                COUNT(CASE WHEN result_code != 'success' THEN 1 END) as failed_runs,
                AVG(EXTRACT(EPOCH FROM duration)) as avg_duration_seconds,
                MIN(start_time) as first_run_time,
                MAX(finish_time) as last_run_time
             FROM run 
             WHERE worker = $1 
             AND start_time >= NOW() - INTERVAL '30 days'",
        )
        .bind(worker_id)
        .fetch_one(&self.pool)
        .await?;

        let total_runs: i64 = performance_stats.try_get("total_runs")?;
        let success_rate = if total_runs > 0 {
            let successful_runs: i64 = performance_stats.try_get("successful_runs")?;
            (successful_runs as f64 / total_runs as f64) * 100.0
        } else {
            0.0
        };

        Ok(serde_json::json!({
            "worker_id": worker_row.try_get::<String, _>("name")?,
            "link": worker_row.try_get::<Option<String>, _>("link")?,
            "password_configured": !worker_row.try_get::<String, _>("password")?.is_empty(),
            "status": if assignments.is_empty() { "idle" } else { "active" },
            "current_assignments": assignments,
            "assignment_count": assignments.len(),
            "recent_runs": runs,
            "performance": {
                "total_runs_30d": total_runs,
                "successful_runs_30d": performance_stats.try_get::<i64, _>("successful_runs")?,
                "failed_runs_30d": performance_stats.try_get::<i64, _>("failed_runs")?,
                "success_rate_30d": format!("{:.1}%", success_rate),
                "avg_duration_seconds": performance_stats.try_get::<Option<f64>, _>("avg_duration_seconds")?,
                "first_run_time": performance_stats.try_get::<Option<DateTime<Utc>>, _>("first_run_time")?,
                "last_run_time": performance_stats.try_get::<Option<DateTime<Utc>>, _>("last_run_time")?
            },
            "timestamp": chrono::Utc::now()
        }))
    }

    /// Get current tasks assigned to a specific worker
    pub async fn get_worker_tasks(
        &self,
        worker_id: &str,
    ) -> Result<serde_json::Value, DatabaseError> {
        let tasks = sqlx::query(
            "SELECT 
                id,
                codebase,
                suite,
                command,
                priority,
                estimated_duration,
                bucket,
                context,
                assigned_time,
                status
             FROM queue 
             WHERE worker = $1 
             ORDER BY assigned_time ASC",
        )
        .bind(worker_id)
        .fetch_all(&self.pool)
        .await?;

        let mut task_list = Vec::new();
        for row in tasks {
            task_list.push(serde_json::json!({
                "queue_id": row.try_get::<i32, _>("id")?,
                "codebase": row.try_get::<String, _>("codebase")?,
                "suite": row.try_get::<String, _>("suite")?,
                "command": row.try_get::<Option<String>, _>("command")?,
                "priority": row.try_get::<i64, _>("priority")?,
                "estimated_duration": row.try_get::<Option<String>, _>("estimated_duration")?,
                "bucket": row.try_get::<String, _>("bucket")?,
                "context": row.try_get::<Option<String>, _>("context")?,
                "assigned_time": row.try_get::<Option<DateTime<Utc>>, _>("assigned_time")?,
                "status": row.try_get::<Option<String>, _>("status")?
            }));
        }

        Ok(serde_json::json!({
            "worker_id": worker_id,
            "tasks": task_list,
            "task_count": task_list.len(),
            "timestamp": chrono::Utc::now()
        }))
    }

    /// Cancel a specific task assigned to a worker
    pub async fn cancel_worker_task(
        &self,
        worker_id: &str,
        queue_id: i32,
    ) -> Result<bool, DatabaseError> {
        let result = sqlx::query(
            "DELETE FROM queue 
             WHERE id = $1 AND worker = $2 AND status IN ('assigned', 'pending')",
        )
        .bind(queue_id)
        .bind(worker_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get basic run information for log reprocessing
    pub async fn get_run_for_reprocessing(
        &self,
        run_id: &str,
    ) -> Result<Option<serde_json::Value>, DatabaseError> {
        let row = sqlx::query(
            "SELECT id, codebase, suite, command, result_code, description, 
                    start_time, finish_time, failure_stage, failure_details, 
                    main_branch_revision, revision, worker
             FROM run 
             WHERE id = $1",
        )
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(serde_json::json!({
                "id": row.try_get::<String, _>("id")?,
                "codebase": row.try_get::<String, _>("codebase")?,
                "suite": row.try_get::<String, _>("suite")?,
                "command": row.try_get::<Option<String>, _>("command")?,
                "result_code": row.try_get::<String, _>("result_code")?,
                "description": row.try_get::<Option<String>, _>("description")?,
                "start_time": row.try_get::<Option<DateTime<Utc>>, _>("start_time")?,
                "finish_time": row.try_get::<Option<DateTime<Utc>>, _>("finish_time")?,
                "failure_stage": row.try_get::<Option<String>, _>("failure_stage")?,
                "failure_details": row.try_get::<Option<serde_json::Value>, _>("failure_details")?,
                "main_branch_revision": row.try_get::<Option<String>, _>("main_branch_revision")?,
                "revision": row.try_get::<Option<String>, _>("revision")?,
                "worker": row.try_get::<Option<String>, _>("worker")?
            })))
        } else {
            Ok(None)
        }
    }

    /// Update run failure details from log analysis
    pub async fn update_run_failure_details(
        &self,
        run_id: &str,
        failure_details: &serde_json::Value,
    ) -> Result<bool, DatabaseError> {
        let result = sqlx::query("UPDATE run SET failure_details = $1 WHERE id = $2")
            .bind(failure_details)
            .bind(run_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Update run description from log analysis
    pub async fn update_run_description(
        &self,
        run_id: &str,
        description: &str,
    ) -> Result<bool, DatabaseError> {
        let result = sqlx::query("UPDATE run SET description = $1 WHERE id = $2")
            .bind(description)
            .bind(run_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get comprehensive run context data in a single optimized query
    /// Combines run details, statistics, reviews, binary packages in one call
    pub async fn get_run_context(
        &self,
        run_id: &str,
        campaign: &str,
        codebase: &str,
    ) -> Result<RunContext, DatabaseError> {
        let row = sqlx::query(
            r#"
            SELECT 
                -- Run statistics
                (SELECT COUNT(*) FROM run WHERE suite = $2 AND codebase = $3) as total_runs,
                (SELECT COUNT(*) FROM run WHERE suite = $2 AND codebase = $3 AND result_code = 'success') as successful_runs,
                -- Binary packages
                (SELECT binary_packages FROM debian_build WHERE run_id = $1) as binary_packages,
                -- Review count
                (SELECT COUNT(*) FROM review WHERE run_id = $1) as review_count,
                -- Queue position for this codebase/campaign
                (SELECT COUNT(*) + 1 FROM queue q2 WHERE q2.suite = $2 
                 AND q2.priority > COALESCE((SELECT priority FROM queue WHERE suite = $2 AND codebase = $3), 0)) as queue_position
            "#,
        )
        .bind(run_id)
        .bind(campaign)
        .bind(codebase)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(RunContext {
                total_runs: row.get("total_runs"),
                successful_runs: row.get("successful_runs"),
                binary_packages: row
                    .get::<Option<Vec<String>>, _>("binary_packages")
                    .unwrap_or_default(),
                review_count: row.get("review_count"),
                queue_position: row.get::<i64, _>("queue_position") as i32,
            }),
            None => Ok(RunContext::default()),
        }
    }

    /// Get a list of all campaigns with their status information
    pub async fn get_campaign_status_list(
        &self,
    ) -> Result<Vec<super::api::schemas::CampaignStatus>, DatabaseError> {
        let rows = sqlx::query(
            r#"
            SELECT 
                suite as campaign_name,
                COUNT(*) as total_candidates,
                COUNT(CASE WHEN status = 'pending' THEN 1 END) as pending_candidates,
                COUNT(CASE WHEN status = 'in_progress' THEN 1 END) as active_runs,
                MAX(last_update) as last_updated
            FROM candidate 
            GROUP BY suite
            ORDER BY campaign_name
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut campaigns = Vec::new();
        for row in rows {
            let campaign_name: String = row.try_get("campaign_name")?;

            // Get success rate for this campaign
            let success_stats = sqlx::query(
                r#"
                SELECT 
                    COUNT(*) as total_runs,
                    COUNT(CASE WHEN result_code = 'success' THEN 1 END) as successful_runs
                FROM run 
                WHERE suite = $1
                "#,
            )
            .bind(&campaign_name)
            .fetch_one(&self.pool)
            .await?;

            let total_runs: i64 = success_stats.try_get("total_runs")?;
            let successful_runs: i64 = success_stats.try_get("successful_runs")?;

            let success_rate = if total_runs > 0 {
                Some(successful_runs as f64 / total_runs as f64)
            } else {
                None
            };

            campaigns.push(super::api::schemas::CampaignStatus {
                name: campaign_name,
                total_candidates: row.try_get("total_candidates")?,
                pending_candidates: row.try_get("pending_candidates")?,
                active_runs: row.try_get("active_runs")?,
                success_rate,
                last_updated: row.try_get("last_updated")?,
                description: None, // TODO: Add campaign descriptions if available
            });
        }

        Ok(campaigns)
    }

    /// Get comprehensive campaign statistics in a single optimized query
    /// Eliminates N+1 pattern from multiple separate count queries
    pub async fn get_campaign_statistics(
        &self,
        campaign: &str,
    ) -> Result<CampaignStatistics, DatabaseError> {
        let row = sqlx::query(
            r#"
            SELECT 
                -- Total candidates for this campaign
                (SELECT COUNT(*) FROM candidate WHERE suite = $1) as total_candidates,
                -- Successful runs for this campaign
                (SELECT COUNT(*) FROM run WHERE suite = $1 AND result_code = 'success') as successful_runs,
                -- Failed runs for this campaign
                (SELECT COUNT(*) FROM run WHERE suite = $1 AND result_code != 'success') as failed_runs,
                -- Pending publishes for this campaign
                (SELECT COUNT(*) FROM publish_ready WHERE suite = $1) as pending_publishes,
                -- Total runs for this campaign
                (SELECT COUNT(*) FROM run WHERE suite = $1) as total_runs,
                -- Active queue items for this campaign
                (SELECT COUNT(*) FROM queue WHERE suite = $1) as queued_items,
                -- Average run time in seconds
                (SELECT EXTRACT(EPOCH FROM AVG(finish_time - start_time)) 
                 FROM run WHERE suite = $1 AND finish_time IS NOT NULL AND start_time IS NOT NULL) as avg_run_time_seconds
            "#,
        )
        .bind(campaign)
        .fetch_one(&self.pool)
        .await?;

        Ok(CampaignStatistics {
            total_candidates: row.get("total_candidates"),
            successful_runs: row.get("successful_runs"),
            failed_runs: row.get("failed_runs"),
            pending_publishes: row.get("pending_publishes"),
            total_runs: row.get("total_runs"),
            queued_items: row.get("queued_items"),
            avg_run_time_seconds: row
                .get::<Option<f64>, _>("avg_run_time_seconds")
                .unwrap_or(0.0),
        })
    }

    /// Get merge proposals for a specific campaign with filtering and pagination
    pub async fn get_campaign_merge_proposals(
        &self,
        campaign: &str,
        query: &std::collections::HashMap<String, String>,
    ) -> Result<Vec<serde_json::Value>, DatabaseError> {
        let mut sql_query = "SELECT mp.url, mp.status, mp.revision, mp.merged_by, mp.merged_at, mp.can_be_merged, r.codebase FROM merge_proposal mp INNER JOIN run r ON mp.revision = r.revision WHERE r.suite = $1".to_string();
        let mut bind_count = 1;
        let mut bind_values: Vec<String> = vec![campaign.to_string()];

        // Add status filter if specified
        if let Some(status) = query.get("status") {
            bind_count += 1;
            sql_query.push_str(&format!(" AND mp.status = ${}", bind_count));
            bind_values.push(status.clone());
        }

        // Add codebase filter if specified
        if let Some(codebase) = query.get("codebase") {
            bind_count += 1;
            sql_query.push_str(&format!(" AND r.codebase = ${}", bind_count));
            bind_values.push(codebase.clone());
        }

        // Add ordering
        sql_query.push_str(" ORDER BY mp.merged_at DESC NULLS LAST");

        // Add pagination
        let limit = query
            .get("limit")
            .and_then(|l| l.parse::<i64>().ok())
            .unwrap_or(50);
        let offset = query
            .get("offset")
            .and_then(|o| o.parse::<i64>().ok())
            .unwrap_or(0);

        bind_count += 1;
        sql_query.push_str(&format!(" LIMIT ${}", bind_count));
        bind_values.push(limit.to_string());

        bind_count += 1;
        sql_query.push_str(&format!(" OFFSET ${}", bind_count));
        bind_values.push(offset.to_string());

        // Execute the query
        let mut query_builder = sqlx::query(&sql_query);
        for value in &bind_values {
            query_builder = query_builder.bind(value);
        }

        let rows = query_builder.fetch_all(&self.pool).await?;

        let mut proposals = Vec::new();
        for row in rows {
            let proposal = serde_json::json!({
                "url": row.try_get::<String, _>("url")?,
                "status": row.try_get::<Option<String>, _>("status")?,
                "revision": row.try_get::<Option<String>, _>("revision")?,
                "merged_by": row.try_get::<Option<String>, _>("merged_by")?,
                "merged_at": row.try_get::<Option<DateTime<Utc>>, _>("merged_at")?,
                "can_be_merged": row.try_get::<Option<bool>, _>("can_be_merged")?,
                "codebase": row.try_get::<String, _>("codebase")?
            });
            proposals.push(proposal);
        }

        Ok(proposals)
    }

    /// Fetch comprehensive codebase context in a single optimized query
    /// This eliminates N+1 query patterns by combining multiple related queries
    pub async fn get_codebase_context(
        &self,
        campaign: &str,
        codebase: &str,
    ) -> Result<CodebaseContext, DatabaseError> {
        let row = sqlx::query(
            r#"
            SELECT 
                -- Candidate info
                c.codebase,
                c.suite,
                c.command,
                c.publish_policy,
                c.priority,
                c.value,
                -- VCS info
                cb.url as vcs_url,
                cb.vcs_type,
                cb.branch_url,
                -- Last run info
                lr.id as last_run_id,
                lr.result_code as last_result_code,
                lr.description as last_description,
                lr.start_time as last_start_time,
                lr.finish_time as last_finish_time,
                lr.worker as last_worker,
                -- Queue position (subquery)
                (
                    SELECT COUNT(*) + 1 
                    FROM queue q2 
                    WHERE q2.suite = $1 
                    AND q2.priority > COALESCE(
                        (SELECT priority FROM queue WHERE suite = $1 AND codebase = $2), 0
                    )
                ) as queue_position
            FROM candidate c
            LEFT JOIN codebase cb ON c.codebase = cb.name
            LEFT JOIN LATERAL (
                SELECT * FROM run 
                WHERE codebase = $2 AND suite = $1 
                ORDER BY start_time DESC 
                LIMIT 1
            ) lr ON true
            WHERE c.suite = $1 AND c.codebase = $2
            "#,
        )
        .bind(campaign)
        .bind(codebase)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                Ok(CodebaseContext {
                    // Candidate info
                    codebase: row.get("codebase"),
                    suite: row.get("suite"),
                    command: row.get("command"),
                    publish_policy: row.get("publish_policy"),
                    priority: row.get("priority"),
                    value: row.get("value"),
                    // VCS info
                    vcs_url: row.get("vcs_url"),
                    vcs_type: row.get("vcs_type"),
                    branch_url: row.get("branch_url"),
                    // Last run info
                    last_run_id: row.get("last_run_id"),
                    last_result_code: row.get("last_result_code"),
                    last_description: row.get("last_description"),
                    last_start_time: row.get("last_start_time"),
                    last_finish_time: row.get("last_finish_time"),
                    last_worker: row.get("last_worker"),
                    // Queue info
                    queue_position: row.get::<i64, _>("queue_position") as i32,
                })
            }
            None => Err(DatabaseError::NotFound(format!(
                "Codebase {} not found in campaign {}",
                codebase, campaign
            ))),
        }
    }

    /// Get merge proposals with filtering and pagination
    pub async fn get_merge_proposals(
        &self,
        offset: Option<i64>,
        limit: Option<i64>,
        status_filter: Option<&str>,
        codebase_filter: Option<&str>,
    ) -> Result<Vec<super::api::schemas::MergeProposal>, DatabaseError> {
        let mut query = String::from("SELECT url, status, revision, merged_by, merged_at, can_be_merged, codebase FROM merge_proposal");
        let mut conditions = Vec::new();
        let mut param_count = 0;

        if let Some(_status) = status_filter {
            param_count += 1;
            conditions.push(format!("status = ${}", param_count));
        }

        if let Some(_codebase) = codebase_filter {
            param_count += 1;
            conditions.push(format!("codebase = ${}", param_count));
        }

        if !conditions.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&conditions.join(" AND "));
        }

        query.push_str(" ORDER BY merged_at DESC NULLS LAST, url");

        if let Some(limit) = limit {
            param_count += 1;
            query.push_str(&format!(" LIMIT ${}", param_count));
        }

        if let Some(offset) = offset {
            param_count += 1;
            query.push_str(&format!(" OFFSET ${}", param_count));
        }

        let mut query_builder = sqlx::query_as::<_, super::api::schemas::MergeProposal>(&query);

        if let Some(status) = status_filter {
            query_builder = query_builder.bind(status);
        }

        if let Some(codebase) = codebase_filter {
            query_builder = query_builder.bind(codebase);
        }

        if let Some(limit) = limit {
            query_builder = query_builder.bind(limit);
        }

        if let Some(offset) = offset {
            query_builder = query_builder.bind(offset);
        }

        let merge_proposals = query_builder.fetch_all(&self.pool).await?;

        Ok(merge_proposals)
    }

    /// Count merge proposals with filtering
    pub async fn count_merge_proposals(
        &self,
        status_filter: Option<&str>,
        codebase_filter: Option<&str>,
    ) -> Result<i64, DatabaseError> {
        let mut query = String::from("SELECT COUNT(*) FROM merge_proposal");
        let mut conditions = Vec::new();
        let mut param_count = 0;

        if let Some(_status) = status_filter {
            param_count += 1;
            conditions.push(format!("status = ${}", param_count));
        }

        if let Some(_codebase) = codebase_filter {
            param_count += 1;
            conditions.push(format!("codebase = ${}", param_count));
        }

        if !conditions.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&conditions.join(" AND "));
        }

        let mut query_builder = sqlx::query_scalar::<_, i64>(&query);

        if let Some(status) = status_filter {
            query_builder = query_builder.bind(status);
        }

        if let Some(codebase) = codebase_filter {
            query_builder = query_builder.bind(codebase);
        }

        let count = query_builder.fetch_one(&self.pool).await?;

        Ok(count)
    }

    /// Get detailed information for a specific run
    pub async fn get_run_details(&self, run_id: &str) -> Result<Option<RunDetails>, DatabaseError> {
        let query = r#"
            SELECT r.id, r.codebase, r.suite, r.command, r.result_code, r.description,
                   r.start_time, r.finish_time, r.worker, r.result_branches, r.result_tags,
                   r.publish_status, r.failure_stage, r.main_branch_revision, r.vcs_type,
                   r.logfilenames, r.revision
            FROM run r
            WHERE r.id = $1
        "#;

        let row = sqlx::query(query)
            .bind(run_id)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            let run_details = RunDetails {
                id: row.try_get("id")?,
                codebase: row.try_get("codebase")?,
                suite: row.try_get("suite")?,
                command: row.try_get("command")?,
                result_code: row.try_get("result_code")?,
                description: row.try_get("description")?,
                start_time: row.try_get("start_time")?,
                finish_time: row.try_get("finish_time")?,
                worker: row.try_get("worker")?,
                build_version: None, // Not in the current schema
                result_branches: row
                    .try_get::<Option<serde_json::Value>, _>("result_branches")?
                    .map(|v| vec![v])
                    .unwrap_or_default(),
                result_tags: row
                    .try_get::<Option<serde_json::Value>, _>("result_tags")?
                    .map(|v| vec![v])
                    .unwrap_or_default(),
                publish_status: row.try_get("publish_status")?,
                failure_stage: row.try_get("failure_stage")?,
                main_branch_revision: row.try_get("main_branch_revision")?,
                vcs_type: row.try_get("vcs_type")?,
                logfilenames: row
                    .try_get::<Vec<String>, _>("logfilenames")
                    .unwrap_or_default(),
                revision: row.try_get("revision")?,
            };

            Ok(Some(run_details))
        } else {
            Ok(None)
        }
    }

    // ============================================================================
    // User Management Methods
    // ============================================================================

    /// Get all active sessions with user information
    pub async fn get_active_sessions(&self) -> Result<Vec<serde_json::Value>, DatabaseError> {
        let query = r#"
            SELECT 
                id, user_id, username, role, created_at, last_active,
                expires_at, ip_address
            FROM site_session 
            WHERE expires_at > NOW()
            ORDER BY last_active DESC
        "#;

        let rows = sqlx::query(query).fetch_all(&self.pool).await?;

        let sessions: Vec<serde_json::Value> = rows.into_iter().map(|row| {
            json!({
                "session_id": row.get::<Option<String>, _>("id").unwrap_or_default(),
                "user_id": row.get::<Option<String>, _>("user_id").unwrap_or_default(),
                "username": row.get::<Option<String>, _>("username").unwrap_or_default(),
                "role": row.get::<Option<String>, _>("role").unwrap_or("User".to_string()),
                "created_at": row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("created_at")
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default(),
                "last_active": row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("last_active")
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default(),
                "expires_at": row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("expires_at")
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default(),
                "ip_address": row.get::<Option<String>, _>("ip_address").unwrap_or_default()
            })
        }).collect();

        Ok(sessions)
    }

    /// Get sessions for a specific user
    pub async fn get_user_sessions(
        &self,
        user_id: &str,
    ) -> Result<Vec<serde_json::Value>, DatabaseError> {
        let query = r#"
            SELECT 
                id, user_id, username, role, created_at, last_active,
                expires_at, ip_address
            FROM site_session 
            WHERE user_id = $1 AND expires_at > NOW()
            ORDER BY last_active DESC
        "#;

        let rows = sqlx::query(query)
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;

        let sessions: Vec<serde_json::Value> = rows.into_iter().map(|row| {
            json!({
                "session_id": row.get::<Option<String>, _>("id").unwrap_or_default(),
                "user_id": row.get::<Option<String>, _>("user_id").unwrap_or_default(),
                "username": row.get::<Option<String>, _>("username").unwrap_or_default(),
                "role": row.get::<Option<String>, _>("role").unwrap_or("User".to_string()),
                "created_at": row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("created_at")
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default(),
                "last_active": row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("last_active")
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default(),
                "expires_at": row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("expires_at")
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default(),
                "ip_address": row.get::<Option<String>, _>("ip_address").unwrap_or_default()
            })
        }).collect();

        Ok(sessions)
    }

    /// Update a user's role in all their active sessions
    pub async fn update_user_role(
        &self,
        user_id: &str,
        new_role: &str,
    ) -> Result<bool, DatabaseError> {
        let query = r#"
            UPDATE site_session 
            SET role = $2, last_active = NOW()
            WHERE user_id = $1 AND expires_at > NOW()
        "#;

        let result = sqlx::query(query)
            .bind(user_id)
            .bind(new_role)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Revoke all sessions for a specific user
    pub async fn revoke_user_sessions(&self, user_id: &str) -> Result<u64, DatabaseError> {
        let query = r#"
            UPDATE site_session 
            SET expires_at = NOW() - INTERVAL '1 second'
            WHERE user_id = $1 AND expires_at > NOW()
        "#;

        let result = sqlx::query(query).bind(user_id).execute(&self.pool).await?;

        Ok(result.rows_affected())
    }

    /// Revoke a specific session
    pub async fn revoke_session(&self, session_id: &str) -> Result<bool, DatabaseError> {
        let query = r#"
            UPDATE site_session 
            SET expires_at = NOW() - INTERVAL '1 second'
            WHERE id = $1 AND expires_at > NOW()
        "#;

        let result = sqlx::query(query)
            .bind(session_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get list of available campaigns from the database
    pub async fn get_campaigns(&self) -> Result<Vec<String>, DatabaseError> {
        let query = r#"
            SELECT name FROM campaigns ORDER BY name
        "#;

        let rows = sqlx::query(query)
            .fetch_all(&self.pool)
            .await?;

        let campaigns: Vec<String> = rows
            .into_iter()
            .map(|row| row.try_get("name"))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(campaigns)
    }

    /// Get list of available suites from the database  
    pub async fn get_suites(&self) -> Result<Vec<String>, DatabaseError> {
        let query = r#"
            SELECT DISTINCT suite as name FROM run 
            WHERE suite IS NOT NULL 
            ORDER BY suite
        "#;

        let rows = sqlx::query(query)
            .fetch_all(&self.pool)
            .await?;

        let suites: Vec<String> = rows
            .into_iter()
            .map(|row| row.try_get("name"))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(suites)
    }

    /// Get both campaigns and suites for template context
    pub async fn get_campaigns_and_suites(&self) -> Result<(Vec<String>, Vec<String>), DatabaseError> {
        // Run both queries concurrently
        let (campaigns_result, suites_result) = tokio::try_join!(
            self.get_campaigns(),
            self.get_suites()
        )?;

        Ok((campaigns_result, suites_result))
    }
}

// Additional types for database results

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodebaseContext {
    // Candidate info
    pub codebase: String,
    pub suite: String,
    pub command: Option<String>,
    pub publish_policy: Option<String>,
    pub priority: Option<i32>,
    pub value: Option<i64>,
    // VCS info
    pub vcs_url: Option<String>,
    pub vcs_type: Option<String>,
    pub branch_url: Option<String>,
    // Last run info
    pub last_run_id: Option<String>,
    pub last_result_code: Option<String>,
    pub last_description: Option<String>,
    pub last_start_time: Option<DateTime<Utc>>,
    pub last_finish_time: Option<DateTime<Utc>>,
    pub last_worker: Option<String>,
    // Queue info
    pub queue_position: i32,
}

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
    pub vcs_type: Option<String>,
    pub logfilenames: Vec<String>,
    pub revision: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunStatistics {
    pub total: i64,
    pub successful: i64,
    pub failed: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignStatistics {
    pub total_candidates: i64,
    pub successful_runs: i64,
    pub failed_runs: i64,
    pub pending_publishes: i64,
    pub total_runs: i64,
    pub queued_items: i64,
    pub avg_run_time_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RunContext {
    pub total_runs: i64,
    pub successful_runs: i64,
    pub binary_packages: Vec<String>,
    pub review_count: i64,
    pub queue_position: i32,
}
