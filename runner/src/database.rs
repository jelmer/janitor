//! Database operations for the runner.

use crate::{ActiveRun, BuilderResult, JanitorResult};
use breezyshim::RevisionId;
use chrono::{DateTime, Utc};
use redis::AsyncCommands;
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use std::time::Duration;

// Re-export from main crate
pub use janitor::state::{create_pool, Run};
use crate::{QueueItem, QueueAssignment};

/// Database manager for runner operations.
#[derive(Clone)]
pub struct RunnerDatabase {
    pool: PgPool,
    redis: Option<redis::Client>,
}

impl RunnerDatabase {
    /// Create a new database manager.
    pub fn new(pool: PgPool) -> Self {
        Self { pool, redis: None }
    }

    /// Create a new database manager with Redis support.
    pub fn new_with_redis(pool: PgPool, redis: redis::Client) -> Self {
        Self {
            pool,
            redis: Some(redis),
        }
    }

    /// Create a new database instance with optional Redis connection from URL.
    pub async fn new_with_redis_url(
        pool: PgPool,
        redis_url: Option<String>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let redis = if let Some(url) = redis_url {
            Some(redis::Client::open(url)?)
        } else {
            None
        };

        Ok(Self { pool, redis })
    }

    /// Perform a health check on the database connection.
    pub async fn health_check(&self) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        Ok(())
    }

    /// Get a reference to the database pool.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Convert a database Run to JanitorResult.
    pub fn run_to_janitor_result(&self, run: Run) -> JanitorResult {
        JanitorResult {
            log_id: run.id,
            branch_url: run.branch_url,
            subpath: None, // Not stored in run table currently
            code: run.result_code,
            transient: run.failure_transient,
            codebase: run.codebase,
            campaign: run.suite,
            description: run.description,
            codemod: run.result,
            value: run.value.map(|v| v as u64),
            logfilenames: run.logfilenames.unwrap_or_default(),
            start_time: run.start_time,
            finish_time: run.finish_time,
            revision: run.revision,
            main_branch_revision: run.main_branch_revision,
            change_set: Some(run.change_set),
            tags: run.result_tags.map(|tags| {
                tags.into_iter()
                    .map(|(name, rev)| (name, Some(RevisionId::from(rev.as_bytes()))))
                    .collect()
            }),
            remotes: None, // TODO: Get from separate table if needed
            branches: run.result_branches.map(|branches| {
                branches
                    .into_iter()
                    .map(|(fn_name, name, br, r)| (Some(fn_name), Some(name), br, r))
                    .collect()
            }),
            failure_details: run.failure_details,
            failure_stage: run.failure_stage.map(|s| vec![s]),
            resume: self.get_resume_info(&run.id, &run.codebase, &run.suite).await.ok().flatten(),
            target: None, // TODO: Get from builder result
            worker_name: run.worker_name,
            vcs_type: Some(run.vcs_type),
            target_branch_url: run.target_branch_url,
            context: run.context.and_then(|s| serde_json::from_str(&s).ok()),
            builder_result: None, // TODO: Load from debian_build table if needed
        }
    }

    /// Get a run by ID.
    pub async fn get_run(&self, run_id: &str) -> Result<Option<JanitorResult>, sqlx::Error> {
        let run: Option<Run> = sqlx::query_as(
            r#"
            SELECT id, command, description, result_code, main_branch_revision, revision,
                   context, result, suite, instigated_context, vcs_type, branch_url,
                   logfilenames, worker_name, result_branches, result_tags, target_branch_url,
                   change_set, failure_details, failure_transient, failure_stage, codebase,
                   start_time, finish_time, value
            FROM run WHERE id = $1
            "#,
        )
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(run.map(|r| self.run_to_janitor_result(r)))
    }

    /// Store an active run.
    pub async fn store_active_run(&self, active_run: &ActiveRun) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO active_runs (
                id, queue_id, worker_name, worker_link, start_time, estimated_duration,
                campaign, change_set, command, backchannel, vcs_info, codebase,
                instigated_context, resume_from
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            ON CONFLICT (id) DO UPDATE SET
                worker_name = EXCLUDED.worker_name,
                worker_link = EXCLUDED.worker_link,
                start_time = EXCLUDED.start_time,
                estimated_duration = EXCLUDED.estimated_duration,
                campaign = EXCLUDED.campaign,
                change_set = EXCLUDED.change_set,
                command = EXCLUDED.command,
                backchannel = EXCLUDED.backchannel,
                vcs_info = EXCLUDED.vcs_info,
                codebase = EXCLUDED.codebase,
                instigated_context = EXCLUDED.instigated_context,
                resume_from = EXCLUDED.resume_from
            "#,
        )
        .bind(&active_run.log_id)
        .bind(&active_run.queue_id)
        .bind(&active_run.worker_name)
        .bind(&active_run.worker_link)
        .bind(&active_run.start_time)
        .bind(active_run.estimated_duration.map(|d| d.as_secs() as i64))
        .bind(&active_run.campaign)
        .bind(&active_run.change_set)
        .bind(&active_run.command)
        .bind(serde_json::to_value(&active_run.backchannel).unwrap())
        .bind(serde_json::to_value(&active_run.vcs_info).unwrap())
        .bind(&active_run.codebase)
        .bind(&active_run.instigated_context)
        .bind(&active_run.resume_from)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get an active run by ID.
    pub async fn get_active_run(&self, run_id: &str) -> Result<Option<ActiveRun>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT id, queue_id, worker_name, worker_link, start_time, estimated_duration,
                   campaign, change_set, command, backchannel, vcs_info, codebase,
                   instigated_context, resume_from
            FROM active_runs WHERE id = $1
            "#,
        )
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let active_run = ActiveRun {
                worker_name: row.get("worker_name"),
                worker_link: row.get("worker_link"),
                queue_id: row.get("queue_id"),
                log_id: row.get("id"),
                start_time: row.get("start_time"),
                estimated_duration: row
                    .get::<Option<i64>, _>("estimated_duration")
                    .map(|d| std::time::Duration::from_secs(d as u64)),
                campaign: row.get("campaign"),
                change_set: row.get("change_set"),
                command: row.get("command"),
                backchannel: serde_json::from_value(row.get("backchannel")).unwrap_or_default(),
                vcs_info: serde_json::from_value(row.get("vcs_info")).unwrap_or_default(),
                codebase: row.get("codebase"),
                instigated_context: row.get("instigated_context"),
                resume_from: row.get("resume_from"),
            };
            Ok(Some(active_run))
        } else {
            Ok(None)
        }
    }

    /// Get all active runs.
    pub async fn get_active_runs(&self) -> Result<Vec<ActiveRun>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id, queue_id, worker_name, worker_link, start_time, estimated_duration,
                   campaign, change_set, command, backchannel, vcs_info, codebase,
                   instigated_context, resume_from
            FROM active_runs ORDER BY start_time ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut active_runs = Vec::new();
        for row in rows {
            let active_run = ActiveRun {
                worker_name: row.get("worker_name"),
                worker_link: row.get("worker_link"),
                queue_id: row.get("queue_id"),
                log_id: row.get("id"),
                start_time: row.get("start_time"),
                estimated_duration: row
                    .get::<Option<i64>, _>("estimated_duration")
                    .map(|d| std::time::Duration::from_secs(d as u64)),
                campaign: row.get("campaign"),
                change_set: row.get("change_set"),
                command: row.get("command"),
                backchannel: serde_json::from_value(row.get("backchannel")).unwrap_or_default(),
                vcs_info: serde_json::from_value(row.get("vcs_info")).unwrap_or_default(),
                codebase: row.get("codebase"),
                instigated_context: row.get("instigated_context"),
                resume_from: row.get("resume_from"),
            };
            active_runs.push(active_run);
        }

        Ok(active_runs)
    }

    /// Remove an active run.
    pub async fn remove_active_run(&self, run_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM active_runs WHERE id = $1")
            .bind(run_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Get basic queue statistics.
    pub async fn get_queue_stats(&self) -> Result<HashMap<String, i64>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT 
                'total' as key, COUNT(*) as value FROM queue
            UNION ALL
            SELECT 
                'active' as key, COUNT(*) as value FROM active_runs
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut stats = HashMap::new();
        for row in rows {
            let key: String = row.get("key");
            let value: i64 = row.get("value");
            stats.insert(key, value);
        }

        Ok(stats)
    }

    /// Check if a run exists.
    pub async fn run_exists(&self, run_id: &str) -> Result<bool, sqlx::Error> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM run WHERE id = $1")
            .bind(run_id)
            .fetch_one(&self.pool)
            .await?;

        Ok(count > 0)
    }

    /// Update a run result.
    pub async fn update_run_result(
        &self,
        run_id: &str,
        result_code: &str,
        description: Option<&str>,
        failure_details: Option<&serde_json::Value>,
        failure_transient: Option<bool>,
        finish_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE run SET
                result_code = $2,
                description = $3,
                failure_details = $4,
                failure_transient = $5,
                finish_time = $6
            WHERE id = $1
            "#,
        )
        .bind(run_id)
        .bind(result_code)
        .bind(description)
        .bind(failure_details)
        .bind(failure_transient)
        .bind(finish_time)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Store builder result data.
    pub async fn store_builder_result(
        &self,
        run_id: &str,
        builder_result: &BuilderResult,
    ) -> Result<(), sqlx::Error> {
        match builder_result {
            BuilderResult::Generic => {
                // No additional data to store for generic builds
            }
            BuilderResult::Debian {
                source,
                build_version,
                build_distribution,
                changes_filenames: _,
                lintian_result,
                binary_packages,
            } => {
                sqlx::query(
                    r#"
                    INSERT INTO debian_build (
                        run_id, source, version, distribution, lintian_result, binary_packages
                    ) VALUES ($1, $2, $3, $4, $5, $6)
                    ON CONFLICT (run_id) DO UPDATE SET
                        source = EXCLUDED.source,
                        version = EXCLUDED.version,
                        distribution = EXCLUDED.distribution,
                        lintian_result = EXCLUDED.lintian_result,
                        binary_packages = EXCLUDED.binary_packages
                    "#,
                )
                .bind(run_id)
                .bind(source)
                .bind(build_version)
                .bind(build_distribution)
                .bind(lintian_result)
                .bind(binary_packages)
                .execute(&self.pool)
                .await?;
            }
        }
        Ok(())
    }

    /// Get the next available queue item for assignment.
    pub async fn next_queue_item(
        &self,
        codebase: Option<&str>,
        campaign: Option<&str>,
        exclude_hosts: &[String],
        assigned_queue_items: &[i64],
    ) -> Result<Option<QueueAssignment>, sqlx::Error> {
        let mut query = r#"
            SELECT
                queue.command,
                queue.context,
                queue.id,
                queue.estimated_duration,
                queue.suite AS campaign,
                queue.refresh,
                queue.requester,
                queue.change_set,
                codebase.vcs_type,
                codebase.branch_url,
                codebase.subpath,
                queue.codebase
            FROM
                queue
            LEFT JOIN codebase ON codebase.name = queue.codebase
        "#.to_string();

        let mut conditions = Vec::new();
        let mut bind_count = 0;

        // Exclude already assigned queue items
        if !assigned_queue_items.is_empty() {
            bind_count += 1;
            conditions.push(format!("NOT (queue.id = ANY(${}::int[]))", bind_count));
        }

        // Filter by codebase if specified
        if codebase.is_some() {
            bind_count += 1;
            conditions.push(format!("queue.codebase = ${}", bind_count));
        }

        // Filter by campaign if specified
        if campaign.is_some() {
            bind_count += 1;
            conditions.push(format!("queue.suite = ${}", bind_count));
        }

        // Exclude hosts
        if !exclude_hosts.is_empty() {
            bind_count += 1;
            conditions.push(format!(
                "NOT (codebase.branch_url IS NOT NULL AND SUBSTRING(codebase.branch_url from '.*://(?:[^/@]*@)?([^/]*)') = ANY(${}::text[]))",
                bind_count
            ));
        }

        if !conditions.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&conditions.join(" AND "));
        }

        query.push_str(r#"
            ORDER BY
            queue.bucket ASC,
            queue.priority ASC,
            queue.id ASC
            LIMIT 1
        "#);

        let mut sqlx_query = sqlx::query(&query);

        // Bind parameters in order
        let mut _bind_idx = 1;
        
        if !assigned_queue_items.is_empty() {
            sqlx_query = sqlx_query.bind(assigned_queue_items);
            _bind_idx += 1;
        }
        
        if let Some(cb) = codebase {
            sqlx_query = sqlx_query.bind(cb);
            _bind_idx += 1;
        }
        
        if let Some(camp) = campaign {
            sqlx_query = sqlx_query.bind(camp);
            _bind_idx += 1;
        }
        
        if !exclude_hosts.is_empty() {
            sqlx_query = sqlx_query.bind(exclude_hosts);
        }

        let row = sqlx_query.fetch_optional(&self.pool).await?;

        if let Some(row) = row {
            let queue_item = QueueItem {
                id: row.get("id"),
                context: row.get("context"),
                command: row.get("command"),
                estimated_duration: row
                    .get::<Option<i64>, _>("estimated_duration")
                    .map(|d| std::time::Duration::from_secs(d as u64)),
                campaign: row.get("campaign"),
                refresh: row.get("refresh"),
                requester: row.get("requester"),
                change_set: row.get("change_set"),
                codebase: row.get("codebase"),
            };

            let mut vcs_info = janitor::queue::VcsInfo {
                vcs_type: None,
                branch_url: None,
                subpath: None,
            };

            if let Some(branch_url) = row.get::<Option<String>, _>("branch_url") {
                vcs_info.branch_url = Some(branch_url);
            }
            if let Some(subpath) = row.get::<Option<String>, _>("subpath") {
                vcs_info.subpath = Some(subpath);
            }
            if let Some(vcs_type) = row.get::<Option<String>, _>("vcs_type") {
                vcs_info.vcs_type = Some(vcs_type);
            }

            Ok(Some(QueueAssignment {
                queue_item,
                vcs_info,
            }))
        } else {
            Ok(None)
        }
    }

    /// Get a queue item by ID.
    pub async fn get_queue_item(&self, queue_id: i64) -> Result<Option<QueueItem>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT
                queue.command,
                queue.context,
                queue.id,
                queue.estimated_duration,
                queue.suite AS campaign,
                queue.refresh,
                queue.requester,
                queue.change_set,
                queue.codebase
            FROM queue
            WHERE queue.id = $1
            "#,
        )
        .bind(queue_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(QueueItem {
                id: row.get("id"),
                context: row.get("context"),
                command: row.get("command"),
                estimated_duration: row
                    .get::<Option<i64>, _>("estimated_duration")
                    .map(|d| std::time::Duration::from_secs(d as u64)),
                campaign: row.get("campaign"),
                refresh: row.get("refresh"),
                requester: row.get("requester"),
                change_set: row.get("change_set"),
                codebase: row.get("codebase"),
            }))
        } else {
            Ok(None)
        }
    }

    /// Get queue position for a specific codebase and campaign.
    pub async fn get_queue_position(
        &self,
        codebase: &str,
        campaign: &str,
    ) -> Result<Option<(i32, std::time::Duration)>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT position, wait_time FROM queue_positions WHERE codebase = $1 AND suite = $2",
        )
        .bind(codebase)
        .bind(campaign)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            // Convert postgres interval to Duration
            let wait_time_secs: i64 = row.get("wait_time");
            Ok(Some((
                row.get("position"),
                std::time::Duration::from_secs(wait_time_secs as u64),
            )))
        } else {
            Ok(None)
        }
    }

    /// Update run publish status.
    pub async fn update_run_publish_status(
        &self,
        run_id: &str,
        publish_status: &str,
    ) -> Result<Option<(String, String, String)>, sqlx::Error> {
        let row = sqlx::query(
            "UPDATE run SET publish_status = $2 WHERE id = $1 RETURNING id, codebase, suite",
        )
        .bind(run_id)
        .bind(publish_status)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some((
                row.get("id"),
                row.get("codebase"),
                row.get("suite"),
            )))
        } else {
            Ok(None)
        }
    }

    /// Add a host to the rate limit list.
    pub async fn rate_limit_host(
        &self,
        host: &str,
        retry_after: DateTime<Utc>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(redis_client) = &self.redis {
            let mut conn = redis_client.get_async_connection().await?;
            let _: () = conn.hset("rate-limit-hosts", host, retry_after.to_rfc3339())
                .await?;
        }
        Ok(())
    }

    /// Get all rate limited hosts that are still active.
    pub async fn get_rate_limited_hosts(
        &self,
    ) -> Result<HashMap<String, DateTime<Utc>>, Box<dyn std::error::Error + Send + Sync>> {
        let mut result = HashMap::new();
        
        if let Some(redis_client) = &self.redis {
            let mut conn = redis_client.get_async_connection().await?;
            let hosts: HashMap<String, String> = conn.hgetall("rate-limit-hosts").await?;
            
            let now = Utc::now();
            for (host, time_str) in hosts {
                if let Ok(retry_time) = DateTime::parse_from_rfc3339(&time_str) {
                    let retry_time = retry_time.with_timezone(&Utc);
                    if retry_time > now {
                        result.insert(host, retry_time);
                    }
                }
            }
        }
        
        Ok(result)
    }

    /// Get assigned queue items from Redis.
    pub async fn get_assigned_queue_items(
        &self,
    ) -> Result<Vec<i64>, Box<dyn std::error::Error + Send + Sync>> {
        let mut result = Vec::new();
        
        if let Some(redis_client) = &self.redis {
            let mut conn = redis_client.get_async_connection().await?;
            let items: Vec<String> = conn.hkeys("assigned-queue-items").await?;
            
            for item in items {
                if let Ok(id) = item.parse::<i64>() {
                    result.push(id);
                }
            }
        }
        
        Ok(result)
    }

    /// Assign a queue item to a worker in Redis.
    pub async fn assign_queue_item(
        &self,
        queue_id: i64,
        worker_name: &str,
        log_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(redis_client) = &self.redis {
            let mut conn = redis_client.get_async_connection().await?;
            
            // Check if already assigned to prevent double assignment
            let existing: Option<String> = conn.hget("assigned-queue-items", queue_id.to_string()).await?;
            if existing.is_some() {
                return Err(format!("Queue item {} already assigned", queue_id).into());
            }
            
            // Store assignment with worker info and timestamp
            let assignment_info = serde_json::json!({
                "worker_name": worker_name,
                "log_id": log_id,
                "assigned_at": Utc::now().to_rfc3339(),
            });
            
            let _: () = conn.hset("assigned-queue-items", queue_id.to_string(), assignment_info.to_string()).await?;
            let _: () = conn.sadd(format!("worker-queue-items:{}", worker_name), queue_id).await?;
            
            // Set expiration for assignments (cleanup stale assignments)
            let _: () = conn.expire("assigned-queue-items", 3600).await?; // 1 hour
        }
        
        Ok(())
    }

    /// Get detailed assigned queue items from Redis with worker info.
    pub async fn get_assigned_queue_items_detailed(
        &self,
    ) -> Result<Vec<(i64, String, String, String)>, Box<dyn std::error::Error + Send + Sync>> {
        let mut result = Vec::new();
        
        if let Some(redis_client) = &self.redis {
            let mut conn = redis_client.get_async_connection().await?;
            let assignments: HashMap<String, String> = conn.hgetall("assigned-queue-items").await?;
            
            for (queue_id_str, assignment_info_str) in assignments {
                if let (Ok(queue_id), Ok(assignment_info)) = (
                    queue_id_str.parse::<i64>(),
                    serde_json::from_str::<serde_json::Value>(&assignment_info_str)
                ) {
                    if let (Some(worker_name), Some(log_id), Some(assigned_at)) = (
                        assignment_info.get("worker_name").and_then(|v| v.as_str()),
                        assignment_info.get("log_id").and_then(|v| v.as_str()),
                        assignment_info.get("assigned_at").and_then(|v| v.as_str())
                    ) {
                        result.push((queue_id, worker_name.to_string(), log_id.to_string(), assigned_at.to_string()));
                    }
                }
            }
        }
        
        Ok(result)
    }

    /// Get queue items assigned to a specific worker.
    pub async fn get_worker_queue_items(
        &self,
        worker_name: &str,
    ) -> Result<Vec<i64>, Box<dyn std::error::Error + Send + Sync>> {
        let mut result = Vec::new();
        
        if let Some(redis_client) = &self.redis {
            let mut conn = redis_client.get_async_connection().await?;
            let queue_ids: Vec<String> = conn.smembers(format!("worker-queue-items:{}", worker_name)).await?;
            
            for queue_id_str in queue_ids {
                if let Ok(queue_id) = queue_id_str.parse::<i64>() {
                    result.push(queue_id);
                }
            }
        }
        
        Ok(result)
    }

    /// Check if a queue item is currently assigned.
    pub async fn is_queue_item_assigned(
        &self,
        queue_id: i64,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(redis_client) = &self.redis {
            let mut conn = redis_client.get_async_connection().await?;
            let exists: bool = conn.hexists("assigned-queue-items", queue_id.to_string()).await?;
            Ok(exists)
        } else {
            Ok(false)
        }
    }

    /// Get the worker assigned to a queue item.
    pub async fn get_queue_item_assignment(
        &self,
        queue_id: i64,
    ) -> Result<Option<(String, String)>, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(redis_client) = &self.redis {
            let mut conn = redis_client.get_async_connection().await?;
            
            if let Ok(assignment_info_str) = conn.hget::<&str, String, String>("assigned-queue-items", queue_id.to_string()).await {
                if let Ok(assignment_info) = serde_json::from_str::<serde_json::Value>(&assignment_info_str) {
                    if let (Some(worker_name), Some(log_id)) = (
                        assignment_info.get("worker_name").and_then(|v| v.as_str()),
                        assignment_info.get("log_id").and_then(|v| v.as_str())
                    ) {
                        return Ok(Some((worker_name.to_string(), log_id.to_string())));
                    }
                }
            }
        }
        
        Ok(None)
    }

    /// Remove queue item assignment from Redis.
    pub async fn unassign_queue_item(
        &self,
        queue_id: i64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(redis_client) = &self.redis {
            let mut conn = redis_client.get_async_connection().await?;
            
            // Get worker name before removing assignment
            if let Ok(assignment_info_str) = conn.hget::<&str, String, String>("assigned-queue-items", queue_id.to_string()).await {
                if let Ok(assignment_info) = serde_json::from_str::<serde_json::Value>(&assignment_info_str) {
                    if let Some(worker_name) = assignment_info.get("worker_name").and_then(|v| v.as_str()) {
                        // Remove from worker's set
                        let _: () = conn.srem(format!("worker-queue-items:{}", worker_name), queue_id).await?;
                    }
                }
            }
            
            // Remove from assignments hash
            let _: () = conn.hdel("assigned-queue-items", queue_id.to_string()).await?;
        }
        
        Ok(())
    }

    /// Coordinate worker health status via Redis.
    pub async fn update_worker_health(
        &self,
        worker_name: &str,
        status: &str,
        current_run: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(redis_client) = &self.redis {
            let mut conn = redis_client.get_async_connection().await?;
            
            let health_info = serde_json::json!({
                "status": status,
                "current_run": current_run,
                "last_heartbeat": Utc::now().to_rfc3339(),
            });
            
            let _: () = conn.hset("worker-health", worker_name, health_info.to_string()).await?;
            
            // Set expiration for worker health (cleanup stale workers)
            let _: () = conn.expire("worker-health", 1800).await?; // 30 minutes
        }
        
        Ok(())
    }

    /// Get worker health status from Redis.
    pub async fn get_worker_health(
        &self,
        worker_name: &str,
    ) -> Result<Option<serde_json::Value>, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(redis_client) = &self.redis {
            let mut conn = redis_client.get_async_connection().await?;
            
            match conn.hget::<&str, &str, String>("worker-health", worker_name).await {
                Ok(health_str) => {
                    let health_info: serde_json::Value = serde_json::from_str(&health_str)?;
                    return Ok(Some(health_info));
                }
                Err(_) => {
                    // Worker health not found
                }
            }
        }
        
        Ok(None)
    }

    /// Get all worker health statuses.
    pub async fn get_all_worker_health(
        &self,
    ) -> Result<HashMap<String, serde_json::Value>, Box<dyn std::error::Error + Send + Sync>> {
        let mut result = HashMap::new();
        
        if let Some(redis_client) = &self.redis {
            let mut conn = redis_client.get_async_connection().await?;
            let workers: HashMap<String, String> = conn.hgetall("worker-health").await?;
            
            for (worker_name, health_str) in workers {
                if let Ok(health_info) = serde_json::from_str::<serde_json::Value>(&health_str) {
                    result.insert(worker_name, health_info);
                }
            }
        }
        
        Ok(result)
    }

    /// Store coordination lock in Redis.
    pub async fn acquire_lock(
        &self,
        lock_name: &str,
        holder: &str,
        ttl_seconds: u64,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(redis_client) = &self.redis {
            let mut conn = redis_client.get_async_connection().await?;
            
            // Use SET with NX (only if not exists) and EX (expiration)
            let result: Option<String> = conn.set_options(
                format!("lock:{}", lock_name),
                holder,
                redis::SetOptions::default()
                    .conditional_set(redis::ExistenceCheck::NX)
                    .get(true)
                    .with_expiration(redis::SetExpiry::EX(ttl_seconds as u64))
            ).await?;
            
            Ok(result.is_some())
        } else {
            // No Redis, assume lock acquired
            Ok(true)
        }
    }

    /// Release coordination lock in Redis.
    pub async fn release_lock(
        &self,
        lock_name: &str,
        holder: &str,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(redis_client) = &self.redis {
            let mut conn = redis_client.get_async_connection().await?;
            
            // Lua script to atomically check holder and delete
            let script = r#"
                if redis.call("GET", KEYS[1]) == ARGV[1] then
                    return redis.call("DEL", KEYS[1])
                else
                    return 0
                end
            "#;
            
            let result: i32 = redis::Script::new(script)
                .key(format!("lock:{}", lock_name))
                .arg(holder)
                .invoke_async(&mut conn)
                .await?;
            
            Ok(result == 1)
        } else {
            // No Redis, assume lock released
            Ok(true)
        }
    }

    /// Enhanced queue assignment with rate limiting support.
    pub async fn next_queue_item_with_rate_limiting(
        &self,
        codebase: Option<&str>,
        campaign: Option<&str>,
        avoid_hosts: &[String],
    ) -> Result<Option<QueueAssignment>, sqlx::Error> {
        // Get rate limited hosts
        let rate_limited_hosts = self.get_rate_limited_hosts().await.unwrap_or_default();
        let mut exclude_hosts = avoid_hosts.to_vec();
        exclude_hosts.extend(rate_limited_hosts.keys().cloned());

        // Get assigned queue items
        let assigned_items = self.get_assigned_queue_items().await.unwrap_or_default();

        // Use advanced queue assignment logic with scoring
        self.next_queue_item_with_scoring(codebase, campaign, &exclude_hosts, &assigned_items)
            .await
    }

    /// Advanced queue assignment with priority scoring and success rates.
    pub async fn next_queue_item_with_scoring(
        &self,
        codebase: Option<&str>,
        campaign: Option<&str>,
        exclude_hosts: &[String],
        assigned_queue_items: &[i64],
    ) -> Result<Option<QueueAssignment>, sqlx::Error> {
        let mut query = r#"
            SELECT
                queue.command,
                queue.context,
                queue.id,
                queue.estimated_duration,
                queue.suite AS campaign,
                queue.refresh,
                queue.requester,
                queue.change_set,
                codebase.vcs_type,
                codebase.branch_url,
                codebase.subpath,
                queue.codebase,
                queue.success_chance,
                queue.priority,
                queue.bucket,
                CASE 
                    WHEN queue.success_chance IS NOT NULL THEN 
                        (queue.success_chance * 100) + (100 - queue.priority)
                    ELSE 
                        (100 - queue.priority)
                END as computed_score
            FROM
                queue
            LEFT JOIN codebase ON codebase.name = queue.codebase
        "#.to_string();

        let mut conditions = Vec::new();
        let mut bind_count = 0;

        // Only select items that should be scheduled
        conditions.push("queue.schedule_time IS NOT NULL".to_string());
        conditions.push("queue.schedule_time <= NOW()".to_string());

        // Exclude already assigned queue items
        if !assigned_queue_items.is_empty() {
            bind_count += 1;
            conditions.push(format!("NOT (queue.id = ANY(${}::int[]))", bind_count));
        }

        // Filter by codebase if specified
        if codebase.is_some() {
            bind_count += 1;
            conditions.push(format!("queue.codebase = ${}", bind_count));
        }

        // Filter by campaign if specified
        if campaign.is_some() {
            bind_count += 1;
            conditions.push(format!("queue.suite = ${}", bind_count));
        }

        // Exclude hosts
        if !exclude_hosts.is_empty() {
            bind_count += 1;
            conditions.push(format!(
                "NOT (codebase.branch_url IS NOT NULL AND SUBSTRING(codebase.branch_url from '.*://(?:[^/@]*@)?([^/]*)') = ANY(${}::text[]))",
                bind_count
            ));
        }

        if !conditions.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&conditions.join(" AND "));
        }

        // Advanced ordering by computed score (higher is better), then bucket, then priority
        query.push_str(r#"
            ORDER BY
                queue.bucket ASC,
                computed_score DESC,
                queue.priority ASC,
                queue.id ASC
            LIMIT 1
        "#);

        let mut sqlx_query = sqlx::query(&query);

        // Bind parameters in order
        let mut _bind_idx = 1;
        
        if !assigned_queue_items.is_empty() {
            sqlx_query = sqlx_query.bind(assigned_queue_items);
            _bind_idx += 1;
        }
        
        if let Some(cb) = codebase {
            sqlx_query = sqlx_query.bind(cb);
            _bind_idx += 1;
        }
        
        if let Some(camp) = campaign {
            sqlx_query = sqlx_query.bind(camp);
            _bind_idx += 1;
        }
        
        if !exclude_hosts.is_empty() {
            sqlx_query = sqlx_query.bind(exclude_hosts);
        }

        let row = sqlx_query.fetch_optional(&self.pool).await?;

        if let Some(row) = row {
            let queue_item = QueueItem {
                id: row.get("id"),
                context: row.get("context"),
                command: row.get("command"),
                estimated_duration: row
                    .get::<Option<i64>, _>("estimated_duration")
                    .map(|d| std::time::Duration::from_secs(d as u64)),
                campaign: row.get("campaign"),
                refresh: row.get("refresh"),
                requester: row.get("requester"),
                change_set: row.get("change_set"),
                codebase: row.get("codebase"),
            };

            let mut vcs_info = janitor::queue::VcsInfo {
                vcs_type: None,
                branch_url: None,
                subpath: None,
            };

            if let Some(branch_url) = row.get::<Option<String>, _>("branch_url") {
                vcs_info.branch_url = Some(branch_url);
            }
            if let Some(subpath) = row.get::<Option<String>, _>("subpath") {
                vcs_info.subpath = Some(subpath);
            }
            if let Some(vcs_type) = row.get::<Option<String>, _>("vcs_type") {
                vcs_info.vcs_type = Some(vcs_type);
            }

            Ok(Some(QueueAssignment {
                queue_item,
                vcs_info,
            }))
        } else {
            Ok(None)
        }
    }

    /// Reschedule failed candidates with minimum success chance.
    pub async fn reschedule_failed_candidates(
        &self,
        campaign: &str,
        suite: Option<&str>,
        min_success_chance: f64,
    ) -> Result<i64, sqlx::Error> {
        let mut query = sqlx::QueryBuilder::new(
            "UPDATE queue SET schedule_time = NOW() WHERE campaign = "
        );
        query.push_bind(campaign);
        query.push(" AND failure_stage IS NOT NULL AND success_chance >= ");
        query.push_bind(min_success_chance);
        
        if let Some(suite) = suite {
            query.push(" AND suite = ");
            query.push_bind(suite);
        }
        
        let result = query.build().execute(&self.pool).await?;
        Ok(result.rows_affected() as i64)
    }

    /// Deschedule candidates matching criteria.
    pub async fn deschedule_candidates(
        &self,
        campaign: &str,
        suite: Option<&str>,
        result_code: Option<&str>,
    ) -> Result<i64, sqlx::Error> {
        let mut query = sqlx::QueryBuilder::new(
            "UPDATE queue SET schedule_time = NULL WHERE campaign = "
        );
        query.push_bind(campaign);
        
        if let Some(suite) = suite {
            query.push(" AND suite = ");
            query.push_bind(suite);
        }
        
        if let Some(result_code) = result_code {
            query.push(" AND result_code = ");
            query.push_bind(result_code);
        }
        
        let result = query.build().execute(&self.pool).await?;
        Ok(result.rows_affected() as i64)
    }

    /// Reset candidates to unprocessed state.
    pub async fn reset_candidates(
        &self,
        campaign: &str,
        suite: Option<&str>,
    ) -> Result<i64, sqlx::Error> {
        let mut query = sqlx::QueryBuilder::new(
            "UPDATE queue SET schedule_time = NOW(), failure_stage = NULL, result_code = NULL, finish_time = NULL WHERE campaign = "
        );
        query.push_bind(campaign);
        
        if let Some(suite) = suite {
            query.push(" AND suite = ");
            query.push_bind(suite);
        }
        
        let result = query.build().execute(&self.pool).await?;
        Ok(result.rows_affected() as i64)
    }

    /// Reschedule some candidates in a bucket.
    pub async fn reschedule_some(
        &self,
        campaign: &str,
        suite: Option<&str>,
        bucket: &str,
        refresh: bool,
        estimated_duration: Option<&Duration>,
        offset: i64,
        limit: i64,
    ) -> Result<i64, sqlx::Error> {
        let mut query = sqlx::QueryBuilder::new(
            "UPDATE queue SET schedule_time = NOW()"
        );
        
        if let Some(duration) = estimated_duration {
            query.push(", estimated_duration = ");
            query.push_bind(duration.as_secs() as i64);
        }
        
        if refresh {
            query.push(", failure_stage = NULL, result_code = NULL, finish_time = NULL");
        }
        
        query.push(" WHERE campaign = ");
        query.push_bind(campaign);
        query.push(" AND bucket = ");
        query.push_bind(bucket);
        
        if let Some(suite) = suite {
            query.push(" AND suite = ");
            query.push_bind(suite);
        }
        
        query.push(" OFFSET ");
        query.push_bind(offset);
        query.push(" LIMIT ");
        query.push_bind(limit);
        
        let result = query.build().execute(&self.pool).await?;
        Ok(result.rows_affected() as i64)
    }

    /// Reschedule all candidates.
    pub async fn reschedule_all(
        &self,
        campaign: &str,
        suite: Option<&str>,
        refresh: bool,
        estimated_duration: Option<&Duration>,
        offset: i64,
        limit: i64,
    ) -> Result<i64, sqlx::Error> {
        let mut query = sqlx::QueryBuilder::new(
            "UPDATE queue SET schedule_time = NOW()"
        );
        
        if let Some(duration) = estimated_duration {
            query.push(", estimated_duration = ");
            query.push_bind(duration.as_secs() as i64);
        }
        
        if refresh {
            query.push(", failure_stage = NULL, result_code = NULL, finish_time = NULL");
        }
        
        query.push(" WHERE campaign = ");
        query.push_bind(campaign);
        
        if let Some(suite) = suite {
            query.push(" AND suite = ");
            query.push_bind(suite);
        }
        
        query.push(" OFFSET ");
        query.push_bind(offset);
        query.push(" LIMIT ");
        query.push_bind(limit);
        
        let result = query.build().execute(&self.pool).await?;
        Ok(result.rows_affected() as i64)
    }

    /// Calculate queue position for a specific queue item or codebase/campaign combination.
    pub async fn calculate_queue_position(
        &self,
        codebase: Option<&str>,
        campaign: Option<&str>,
        queue_id: Option<i64>,
    ) -> Result<Option<i64>, sqlx::Error> {
        if let Some(id) = queue_id {
            // Calculate position for a specific queue item
            let position: Option<i64> = sqlx::query_scalar(
                r#"
                WITH ranked_queue AS (
                    SELECT 
                        queue.id,
                        ROW_NUMBER() OVER (
                            ORDER BY 
                                queue.bucket ASC,
                                CASE 
                                    WHEN queue.success_chance IS NOT NULL THEN 
                                        (queue.success_chance * 100) + (100 - queue.priority)
                                    ELSE 
                                        (100 - queue.priority)
                                END DESC,
                                queue.priority ASC,
                                queue.id ASC
                        ) as position
                    FROM queue
                    WHERE queue.schedule_time IS NOT NULL 
                      AND queue.schedule_time <= NOW()
                )
                SELECT position FROM ranked_queue WHERE id = $1
                "#
            )
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
            
            Ok(position)
        } else if let (Some(cb), Some(camp)) = (codebase, campaign) {
            // Calculate position for next item matching codebase/campaign
            let position: Option<i64> = sqlx::query_scalar(
                r#"
                WITH ranked_queue AS (
                    SELECT 
                        queue.id,
                        queue.codebase,
                        queue.suite,
                        ROW_NUMBER() OVER (
                            ORDER BY 
                                queue.bucket ASC,
                                CASE 
                                    WHEN queue.success_chance IS NOT NULL THEN 
                                        (queue.success_chance * 100) + (100 - queue.priority)
                                    ELSE 
                                        (100 - queue.priority)
                                END DESC,
                                queue.priority ASC,
                                queue.id ASC
                        ) as position
                    FROM queue
                    WHERE queue.schedule_time IS NOT NULL 
                      AND queue.schedule_time <= NOW()
                )
                SELECT MIN(position) FROM ranked_queue 
                WHERE codebase = $1 AND suite = $2
                "#
            )
            .bind(cb)
            .bind(camp)
            .fetch_optional(&self.pool)
            .await?;
            
            Ok(position)
        } else {
            // Return total queue length if no specific filters
            let count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM queue WHERE schedule_time IS NOT NULL AND schedule_time <= NOW()"
            )
            .fetch_one(&self.pool)
            .await?;
            
            Ok(Some(count))
        }
    }

    /// Get queue positions for multiple items.
    pub async fn get_queue_positions(&self) -> Result<HashMap<i64, i64>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT 
                queue.id,
                ROW_NUMBER() OVER (
                    ORDER BY 
                        queue.bucket ASC,
                        CASE 
                            WHEN queue.success_chance IS NOT NULL THEN 
                                (queue.success_chance * 100) + (100 - queue.priority)
                            ELSE 
                                (100 - queue.priority)
                        END DESC,
                        queue.priority ASC,
                        queue.id ASC
                ) as position
            FROM queue
            WHERE queue.schedule_time IS NOT NULL 
              AND queue.schedule_time <= NOW()
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let mut positions = HashMap::new();
        for row in rows {
            let id: i64 = row.get("id");
            let position: i64 = row.get("position");
            positions.insert(id, position);
        }

        Ok(positions)
    }

    /// Clean up stale active runs that have been running too long.
    pub async fn cleanup_stale_runs(
        &self,
        max_age_hours: i64,
    ) -> Result<i64, sqlx::Error> {
        let cutoff_time = Utc::now() - chrono::Duration::hours(max_age_hours);
        
        let stale_runs = sqlx::query(
            "SELECT log_id FROM active_run WHERE start_time < $1"
        )
        .bind(cutoff_time)
        .fetch_all(&self.pool)
        .await?;
        
        let mut cleaned_count = 0;
        
        for row in stale_runs {
            let log_id: String = row.get("log_id");
            
            // Create failure result for stale run
            if let Err(e) = self.update_run_result(
                &log_id,
                "worker-timeout",
                Some("Run exceeded maximum allowed time and was cleaned up"),
                None,
                Some(true), // Transient failure
                Utc::now(),
            ).await {
                log::error!("Failed to update result for stale run {}: {}", log_id, e);
                continue;
            }
            
            // Remove from active runs
            if let Err(e) = self.remove_active_run(&log_id).await {
                log::error!("Failed to remove stale active run {}: {}", log_id, e);
                continue;
            }
            
            cleaned_count += 1;
        }
        
        Ok(cleaned_count)
    }

    /// Mark failed runs as ready for retry if they meet retry criteria.
    pub async fn mark_runs_for_retry(
        &self,
        max_retries: i32,
        min_retry_delay_hours: i64,
    ) -> Result<i64, sqlx::Error> {
        let retry_cutoff = Utc::now() - chrono::Duration::hours(min_retry_delay_hours);
        
        let result = sqlx::query(
            r#"
            UPDATE queue SET 
                schedule_time = NOW(),
                retry_count = COALESCE(retry_count, 0) + 1
            WHERE id IN (
                SELECT DISTINCT queue.id 
                FROM queue 
                JOIN run ON run.suite = queue.suite 
                    AND run.codebase = queue.codebase
                    AND run.command = queue.command
                WHERE run.result_code IN ('worker-timeout', 'worker-failure', 'worker-disappeared')
                    AND run.failure_transient = true
                    AND run.finish_time < $1
                    AND COALESCE(queue.retry_count, 0) < $2
                    AND queue.schedule_time IS NULL
            )
            "#
        )
        .bind(retry_cutoff)
        .bind(max_retries)
        .execute(&self.pool)
        .await?;
        
        Ok(result.rows_affected() as i64)
    }

    /// Clean up orphaned data and maintain database consistency.
    pub async fn maintenance_cleanup(&self) -> Result<(), sqlx::Error> {
        // Remove active runs that don't have corresponding run records
        sqlx::query(
            "DELETE FROM active_run WHERE log_id NOT IN (SELECT id FROM run)"
        )
        .execute(&self.pool)
        .await?;
        
        // Clean up old rate limit entries from Redis
        if let Some(redis_client) = &self.redis {
            if let Ok(mut conn) = redis_client.get_async_connection().await {
                let hosts: HashMap<String, String> = conn.hgetall("rate-limit-hosts").await.unwrap_or_default();
                let now = Utc::now();
                
                for (host, time_str) in hosts {
                    if let Ok(retry_time) = DateTime::parse_from_rfc3339(&time_str) {
                        let retry_time = retry_time.with_timezone(&Utc);
                        if retry_time <= now {
                            let _: () = conn.hdel("rate-limit-hosts", &host).await.unwrap_or(());
                        }
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Get statistics about failed runs and retries.
    pub async fn get_failure_stats(&self) -> Result<HashMap<String, i64>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT 
                'total_failed' as stat_name, 
                COUNT(*) as count
            FROM run 
            WHERE result_code != 'success' AND finish_time > NOW() - INTERVAL '24 hours'
            UNION ALL
            SELECT 
                'transient_failures' as stat_name, 
                COUNT(*) as count
            FROM run 
            WHERE failure_transient = true AND finish_time > NOW() - INTERVAL '24 hours'
            UNION ALL
            SELECT 
                'retry_eligible' as stat_name, 
                COUNT(*) as count
            FROM queue 
            WHERE COALESCE(retry_count, 0) > 0
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let mut stats = HashMap::new();
        for row in rows {
            let stat_name: String = row.get("stat_name");
            let count: i64 = row.get("count");
            stats.insert(stat_name, count);
        }

        Ok(stats)
    }

    /// Get all codebases from the database.
    pub async fn get_codebases(&self) -> Result<Vec<serde_json::Value>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT name, branch_url, url, branch, subpath, vcs_type, web_url, vcs_last_revision, value FROM codebase"
        )
        .fetch_all(&self.pool)
        .await?;

        let mut codebases = Vec::new();
        for row in rows {
            let codebase = serde_json::json!({
                "name": row.get::<Option<String>, _>("name"),
                "branch_url": row.get::<Option<String>, _>("branch_url"),
                "url": row.get::<Option<String>, _>("url"),
                "branch": row.get::<Option<String>, _>("branch"),
                "subpath": row.get::<Option<String>, _>("subpath"),
                "vcs_type": row.get::<Option<String>, _>("vcs_type"),
                "web_url": row.get::<Option<String>, _>("web_url"),
                "vcs_last_revision": row.get::<Option<String>, _>("vcs_last_revision"),
                "value": row.get::<Option<i64>, _>("value")
            });
            codebases.push(codebase);
        }

        Ok(codebases)
    }

    /// Upload/update codebases in the database.
    pub async fn upload_codebases(&self, codebases: &[serde_json::Value]) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        for codebase in codebases {
            // Parse URL parameters if branch_url is provided
            let (url, branch_url, branch) = if let Some(branch_url_str) = codebase.get("branch_url").and_then(|v| v.as_str()) {
                // For now, simple handling - in the real implementation this would parse URL parameters
                (Some(branch_url_str.to_string()), Some(branch_url_str.to_string()), codebase.get("branch").and_then(|v| v.as_str()).map(String::from))
            } else if let Some(url_str) = codebase.get("url").and_then(|v| v.as_str()) {
                let branch = codebase.get("branch").and_then(|v| v.as_str()).map(String::from);
                (Some(url_str.to_string()), Some(url_str.to_string()), branch)
            } else {
                (None, None, None)
            };

            sqlx::query(
                r#"
                INSERT INTO codebase 
                (name, branch_url, url, branch, subpath, vcs_type, vcs_last_revision, value, web_url) 
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                ON CONFLICT (name) DO UPDATE SET 
                    branch_url = EXCLUDED.branch_url, 
                    subpath = EXCLUDED.subpath, 
                    vcs_type = EXCLUDED.vcs_type, 
                    vcs_last_revision = EXCLUDED.vcs_last_revision, 
                    value = EXCLUDED.value, 
                    url = EXCLUDED.url, 
                    branch = EXCLUDED.branch, 
                    web_url = EXCLUDED.web_url
                "#
            )
            .bind(codebase.get("name").and_then(|v| v.as_str()))
            .bind(branch_url)
            .bind(url)
            .bind(branch)
            .bind(codebase.get("subpath").and_then(|v| v.as_str()))
            .bind(codebase.get("vcs_type").and_then(|v| v.as_str()))
            .bind(codebase.get("vcs_last_revision").and_then(|v| v.as_str()))
            .bind(codebase.get("value").and_then(|v| v.as_i64()))
            .bind(codebase.get("web_url").and_then(|v| v.as_str()))
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Get all candidates from the database.
    pub async fn get_candidates(&self) -> Result<Vec<serde_json::Value>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, codebase, suite, command, publish_policy, change_set, context, value, success_chance FROM candidate"
        )
        .fetch_all(&self.pool)
        .await?;

        let mut candidates = Vec::new();
        for row in rows {
            let candidate = serde_json::json!({
                "id": row.get::<i64, _>("id"),
                "codebase": row.get::<String, _>("codebase"),
                "campaign": row.get::<String, _>("suite"), // Note: "suite" maps to "campaign" in API
                "command": row.get::<Option<String>, _>("command"),
                "publish-policy": row.get::<Option<String>, _>("publish_policy"),
                "change_set": row.get::<Option<String>, _>("change_set"),
                "context": row.get::<Option<serde_json::Value>, _>("context"),
                "value": row.get::<Option<i64>, _>("value"),
                "success_chance": row.get::<Option<f64>, _>("success_chance")
            });
            candidates.push(candidate);
        }

        Ok(candidates)
    }

    /// Upload/update candidates in the database.
    pub async fn upload_candidates(&self, candidates: &[serde_json::Value]) -> Result<Vec<String>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let mut errors = Vec::new();

        for candidate in candidates {
            // Validate required fields
            let codebase = match candidate.get("codebase").and_then(|v| v.as_str()) {
                Some(cb) => cb,
                None => {
                    errors.push("Missing or invalid codebase field".to_string());
                    continue;
                }
            };

            let campaign = match candidate.get("campaign").and_then(|v| v.as_str()) {
                Some(c) => c,
                None => {
                    errors.push("Missing or invalid campaign field".to_string());
                    continue;
                }
            };

            let command = candidate.get("command").and_then(|v| v.as_str());
            let publish_policy = candidate.get("publish-policy").and_then(|v| v.as_str());
            let change_set = candidate.get("change_set").and_then(|v| v.as_str());
            let context = candidate.get("context");
            let value = candidate.get("value").and_then(|v| v.as_i64());
            let success_chance = candidate.get("success_chance").and_then(|v| v.as_f64());

            // Insert/update candidate
            let result = sqlx::query(
                r#"
                INSERT INTO candidate 
                (suite, command, change_set, context, value, success_chance, publish_policy, codebase) 
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8) 
                ON CONFLICT (codebase, suite, coalesce(change_set, ''::text)) 
                DO UPDATE SET 
                    context = EXCLUDED.context, 
                    value = EXCLUDED.value, 
                    success_chance = EXCLUDED.success_chance, 
                    command = EXCLUDED.command, 
                    publish_policy = EXCLUDED.publish_policy, 
                    codebase = EXCLUDED.codebase
                RETURNING id
                "#
            )
            .bind(campaign)
            .bind(command)
            .bind(change_set)
            .bind(context)
            .bind(value)
            .bind(success_chance)
            .bind(publish_policy)
            .bind(codebase)
            .fetch_one(&mut *tx)
            .await;

            if let Err(e) = result {
                errors.push(format!("Failed to insert candidate for {}/{}: {}", codebase, campaign, e));
            }
        }

        if errors.is_empty() {
            tx.commit().await?;
        } else {
            tx.rollback().await?;
        }

        Ok(errors)
    }

    /// Delete a candidate by ID.
    pub async fn delete_candidate(&self, candidate_id: i64) -> Result<bool, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        // Delete followups first
        sqlx::query("DELETE FROM followup WHERE candidate = $1")
            .bind(candidate_id)
            .execute(&mut *tx)
            .await?;

        // Get candidate info before deletion
        let candidate_info = sqlx::query(
            "DELETE FROM candidate WHERE id = $1 RETURNING suite, codebase"
        )
        .bind(candidate_id)
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(row) = candidate_info {
            let suite: String = row.get("suite");
            let codebase: String = row.get("codebase");

            // Delete associated queue items
            sqlx::query("DELETE FROM queue WHERE suite = $1 AND codebase = $2")
                .bind(&suite)
                .bind(&codebase)
                .execute(&mut *tx)
                .await?;

            tx.commit().await?;
            Ok(true)
        } else {
            tx.rollback().await?;
            Ok(false)
        }
    }

    /// Get resume information for a run, looking for related runs that can be resumed.
    pub async fn get_resume_info(
        &self,
        run_id: &str,
        codebase: &str,
        campaign: &str,
    ) -> Result<Option<crate::ResultResume>, sqlx::Error> {
        // Look for a previous run on the same codebase and campaign that might be resumable
        let resume_run = sqlx::query(
            r#"
            SELECT id FROM run 
            WHERE codebase = $1 AND suite = $2 AND id != $3
            AND result_code IN ('interrupted', 'worker-failure', 'timeout')
            AND finish_time > NOW() - INTERVAL '7 days'
            ORDER BY finish_time DESC 
            LIMIT 1
            "#,
        )
        .bind(codebase)
        .bind(campaign)
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = resume_run {
            let resume_run_id: String = row.get("id");
            Ok(Some(crate::ResultResume {
                run_id: resume_run_id,
            }))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_module_compiles() {
        // Basic compilation test - the module exists and can be imported
        assert!(true);
    }
}
