//! Database operations for the runner.

use crate::{ActiveRun, BuilderResult, JanitorResult};
use breezyshim::RevisionId;
use sqlx::{PgPool, Row};
use std::collections::HashMap;

// Re-export from main crate
pub use janitor::state::{create_pool, Run};

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
