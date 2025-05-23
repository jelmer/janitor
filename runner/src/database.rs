//! Database operations for the runner.

use crate::{ActiveRun, BuilderResult, JanitorResult};
use breezyshim::RevisionId;
use sqlx::{PgPool, Row};
use std::collections::HashMap;

// Re-export from main crate
pub use janitor::state::{create_pool, Run};
use crate::{QueueItem, QueueAssignment};

/// Database manager for runner operations.
pub struct RunnerDatabase {
    pool: PgPool,
}

impl RunnerDatabase {
    /// Create a new database manager.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
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
            resume: None, // TODO: Implement resume logic
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
