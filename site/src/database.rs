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

    // Get global statistics for homepage
    pub async fn get_stats(&self) -> Result<HashMap<String, i64>, DatabaseError> {
        let mut stats = HashMap::new();

        // Total codebases
        let total_codebases: i64 = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM codebase WHERE NOT inactive"
        )
        .fetch_one(&self.pool)
        .await?;
        stats.insert("total_codebases".to_string(), total_codebases);

        // Active runs (currently running)
        let active_runs: i64 = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM run WHERE finish_time IS NULL"
        )
        .fetch_one(&self.pool)
        .await?;
        stats.insert("active_runs".to_string(), active_runs);

        // Queue size
        let queue_size: i64 = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM queue"
        )
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
        let mut query = "SELECT name, url, branch FROM codebase WHERE NOT inactive".to_string();
        
        if let Some(search_term) = search {
            query.push_str(&format!(" AND (name ILIKE '%{}%' OR url ILIKE '%{}%')", 
                search_term.replace("%", "\\%").replace("_", "\\_"),
                search_term.replace("%", "\\%").replace("_", "\\_")));
        }
        
        query.push_str(" ORDER BY name");
        
        if let Some(limit) = limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }
        
        if let Some(offset) = offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }
        
        let rows = sqlx::query(&query)
            .fetch_all(&self.pool)
            .await?;
            
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
        let row = sqlx::query(
            "SELECT name, url, branch FROM codebase WHERE name = $1 AND NOT inactive"
        )
        .bind(name)
        .fetch_one(&self.pool)
        .await?;
        
        Ok(Codebase {
            name: row.try_get("name")?,
            url: row.try_get("url")?,
            branch: row.try_get("branch")?,
        })
    }

    pub async fn count_codebases(&self, search: Option<&str>) -> Result<i64, DatabaseError> {
        let mut query = "SELECT COUNT(*) FROM codebase WHERE NOT inactive".to_string();
        
        if let Some(search_term) = search {
            query.push_str(&format!(" AND (name ILIKE '%{}%' OR url ILIKE '%{}%')", 
                search_term.replace("%", "\\%").replace("_", "\\_"),
                search_term.replace("%", "\\%").replace("_", "\\_")));
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
            query.push_str(&format!(" AND codebase ILIKE '%{}%'", 
                search_term.replace("%", "\\%").replace("_", "\\_")));
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
            "SELECT COUNT(*) FROM run WHERE suite = $1 AND result_code = $2"
        )
        .bind(campaign)
        .bind(result_code)
        .fetch_one(&self.pool)
        .await?;
            
        Ok(count)
    }

    pub async fn count_pending_publishes(
        &self,
        campaign: &str,
    ) -> Result<i64, DatabaseError> {
        let count: i64 = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM publish_ready WHERE suite = $1"
        )
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

    pub async fn get_vcs_info(
        &self,
        codebase: &str,
    ) -> Result<VcsInfo, DatabaseError> {
        let row = sqlx::query(
            "SELECT url, branch_url, vcs_type, web_url FROM codebase WHERE name = $1"
        )
        .bind(codebase)
        .fetch_one(&self.pool)
        .await?;
        
        Ok(VcsInfo {
            url: row.try_get::<Option<String>, _>("url")?.unwrap_or_default(),
            vcs_type: row.try_get::<Option<String>, _>("vcs_type")?.unwrap_or("unknown".to_string()),
            branch_url: row.try_get::<Option<String>, _>("web_url")?,
        })
    }

    pub async fn get_last_unabsorbed_run(
        &self,
        campaign: &str,
        codebase: &str,
    ) -> Result<RunDetails, DatabaseError> {
        let row = sqlx::query(
            "SELECT run.id, run.codebase, run.suite, run.command, run.result_code, run.description, run.start_time, run.finish_time, run.worker, run.failure_stage, run.main_branch_revision FROM last_unabsorbed_runs run WHERE run.suite = $1 AND run.codebase = $2"
        )
        .bind(campaign)
        .bind(codebase)
        .fetch_one(&self.pool)
        .await?;
        
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
            result_branches: vec![], // TODO: Join with new_result_branch
            result_tags: vec![], // TODO: Handle result_tags
            publish_status: None, // TODO: Add to query
            failure_stage: row.try_get("failure_stage")?,
            main_branch_revision: row.try_get("main_branch_revision")?,
            vcs_type: None, // TODO: Add to query
        })
    }

    pub async fn get_previous_runs(
        &self,
        codebase: &str,
        campaign: &str,
        limit: Option<i64>,
    ) -> Result<Vec<RunDetails>, DatabaseError> {
        let mut query = "SELECT id, codebase, suite, command, result_code, description, start_time, finish_time, worker, failure_stage, main_branch_revision FROM run WHERE codebase = $1 AND suite = $2 ORDER BY start_time DESC".to_string();
        
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
            runs.push(RunDetails {
                id: row.try_get("id")?,
                codebase: row.try_get("codebase")?,
                suite: row.try_get("suite")?,
                command: row.try_get("command")?,
                result_code: row.try_get("result_code")?,
                description: row.try_get("description")?,
                start_time: row.try_get("start_time")?,
                finish_time: row.try_get("finish_time")?,
                worker: row.try_get("worker")?,
                build_version: None,
                result_branches: vec![],
                result_tags: vec![],
                publish_status: None,
                failure_stage: row.try_get("failure_stage")?,
                main_branch_revision: row.try_get("main_branch_revision")?,
                vcs_type: None, // TODO: Add to query
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
            "SELECT position FROM queue_positions WHERE suite = $1 AND codebase = $2"
        )
        .bind(campaign)
        .bind(codebase)
        .fetch_optional(&self.pool)
        .await?
        .flatten();
            
        Ok(position.unwrap_or(0))
    }

    pub async fn get_average_run_time(
        &self,
        campaign: &str,
    ) -> Result<i64, DatabaseError> {
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
            "SELECT publish_policy FROM candidate WHERE suite = $1 AND codebase = $2"
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

    pub async fn get_run(
        &self,
        run_id: &str,
    ) -> Result<RunDetails, DatabaseError> {
        let row = sqlx::query(
            "SELECT id, codebase, suite, command, result_code, description, start_time, finish_time, worker, failure_stage, main_branch_revision FROM run WHERE id = $1"
        )
        .bind(run_id)
        .fetch_one(&self.pool)
        .await?;
        
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
            build_version: None,
            result_branches: vec![], // TODO: Join with new_result_branch
            result_tags: vec![], // TODO: Handle result_tags array
            publish_status: None, // TODO: Add publish_status to query
            failure_stage: row.try_get("failure_stage")?,
            main_branch_revision: row.try_get("main_branch_revision")?,
            vcs_type: None, // TODO: Add to query
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
        let mut query = "SELECT id, codebase, suite, command, result_code, description, start_time, finish_time, worker, failure_stage, main_branch_revision FROM run WHERE suite = 'unchanged' AND codebase = $1".to_string();
        
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
            build_version: None,
            result_branches: vec![],
            result_tags: vec![],
            publish_status: None,
            failure_stage: row.try_get("failure_stage")?,
            main_branch_revision: row.try_get("main_branch_revision")?,
            vcs_type: None, // TODO: Add to query
        })
    }

    pub async fn get_binary_packages(
        &self,
        run_id: &str,
    ) -> Result<Vec<String>, DatabaseError> {
        let packages: Option<Vec<String>> = sqlx::query_scalar::<_, Option<Vec<String>>>(
            "SELECT binary_packages FROM debian_build WHERE run_id = $1"
        )
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await?
        .flatten();
            
        Ok(packages.unwrap_or_default())
    }

    pub async fn get_reviews(
        &self,
        run_id: &str,
    ) -> Result<Vec<serde_json::Value>, DatabaseError> {
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
    pub vcs_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunStatistics {
    pub total: i64,
    pub successful: i64,
    pub failed: i64,
}
