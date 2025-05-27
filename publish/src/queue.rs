//! Queue processing functionality for the publish service.
//!
//! This module handles the main queue processing loop and related functionality,
//! ported from the Python implementation.

use crate::{consider_publish_run, AppState, PublishError};
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use std::sync::Arc;

/// Represents a publish-ready run with its associated metadata.
#[derive(Debug, Clone)]
pub struct PublishReadyRun {
    /// The run information.
    pub run: janitor::state::Run,
    /// The rate limit bucket for this run.
    pub rate_limit_bucket: String,
    /// The command that was executed.
    pub command: String,
    /// List of unpublished branches for this run.
    pub unpublished_branches: Vec<crate::state::UnpublishedBranch>,
}

/// Iterator over publish-ready runs from the database.
pub struct PublishReadyIterator {
    /// Database connection pool.
    conn: PgPool,
    /// Optional specific run ID to filter by.
    run_id: Option<String>,
}

impl PublishReadyIterator {
    /// Create a new iterator over publish-ready runs.
    ///
    /// # Arguments
    /// * `conn` - Database connection pool
    /// * `run_id` - Optional specific run ID to filter by
    ///
    /// # Returns
    /// A new iterator instance
    pub fn new(conn: PgPool, run_id: Option<String>) -> Self {
        Self { conn, run_id }
    }

    /// Get the next publish-ready run.
    ///
    /// # Returns
    /// The next publish-ready run, or None if no more runs are available
    pub async fn next(&mut self) -> Result<Option<PublishReadyRun>, sqlx::Error> {
        // Query for runs that are ready to publish
        let mut query = r#"
            SELECT DISTINCT ON (run.id)
                run.id,
                run.codebase,
                run.suite,
                run.main_branch_revision,
                run.revision,
                run.result_code,
                run.target_branch_url,
                COALESCE(run.rate_limit_bucket, 'default') as rate_limit_bucket,
                run.command,
                run.start_time,
                run.finish_time,
                run.description,
                run.value,
                run.worker_name,
                run.worker_link,
                run.log_id,
                run.worker_result,
                new_result_branch.role,
                new_result_branch.revision as branch_revision,
                new_result_branch.name as branch_name
            FROM run
            LEFT JOIN new_result_branch ON run.id = new_result_branch.run_id
            LEFT JOIN publish ON run.id = publish.run_id 
                AND new_result_branch.role = publish.branch_name
                AND new_result_branch.revision = publish.revision
            WHERE run.result_code = 'success'
                AND run.suite IS NOT NULL
                AND new_result_branch.role IS NOT NULL
                AND publish.id IS NULL  -- Not already published
        "#
        .to_string();

        if let Some(ref run_id) = self.run_id {
            query.push_str(" AND run.id = $1");
        }

        query.push_str(" ORDER BY run.id, run.start_time DESC");

        let rows = if let Some(ref run_id) = self.run_id {
            sqlx::query(&query)
                .bind(run_id)
                .fetch_all(&self.conn)
                .await?
        } else {
            sqlx::query(&query).fetch_all(&self.conn).await?
        };

        if rows.is_empty() {
            return Ok(None);
        }

        // Group by run ID to handle multiple branches per run
        let mut runs_map: HashMap<
            String,
            (
                janitor::state::Run,
                Vec<crate::state::UnpublishedBranch>,
                String,
            ),
        > = HashMap::new();

        for row in rows {
            let run_id: String = row.get("id");
            let rate_limit_bucket: String = row.get("rate_limit_bucket");
            let command: String = row.get("command");

            let run = janitor::state::Run {
                id: run_id.clone(),
                codebase: row.get("codebase"),
                suite: row.get("suite"),
                main_branch_revision: row.get("main_branch_revision"),
                revision: row.get("revision"),
                result_code: row.get("result_code"),
                target_branch_url: row.get("target_branch_url"),
                command: command.clone(),
                start_time: row.get("start_time"),
                finish_time: row.get("finish_time"),
                description: row.get("description"),
                value: row.get("value"),
                worker_name: row.get("worker_name"),
                vcs_type: row.get("vcs_type"),
                branch_url: row.get("branch_url"),
                change_set: row.get("change_set"),
                failure_details: row.get("failure_details"),
                failure_transient: row.get("failure_transient"),
                failure_stage: row.get("failure_stage"),
                context: row.get("context"),
                result: row.get("result"),
                instigated_context: row.get("instigated_context"),
                logfilenames: row.get("logfilenames"),
                result_branches: row.get("result_branches"),
                result_tags: row.get("result_tags"),
            };

            let branch = crate::state::UnpublishedBranch {
                role: row.get("role"),
                revision: row.get("branch_revision"),
                remote_name: row.get("branch_name"),
                base_revision: None, // TODO: Get from query if available
                publish_mode: Some("propose".to_string()), // Default mode
                max_frequency_days: None, // TODO: Get from config if needed
                name: row.get("branch_name"), // Use remote_name as name
            };

            match runs_map.get_mut(&run_id) {
                Some((_, branches, _)) => {
                    branches.push(branch);
                }
                None => {
                    runs_map.insert(run_id, (run, vec![branch], rate_limit_bucket));
                }
            }
        }

        // Return the first run (if any)
        if let Some((_, (run, branches, rate_limit_bucket))) = runs_map.into_iter().next() {
            Ok(Some(PublishReadyRun {
                rate_limit_bucket,
                command: run.command.clone(),
                unpublished_branches: branches,
                run,
            }))
        } else {
            Ok(None)
        }
    }
}

/// Process the publish queue in a loop.
///
/// This is the main queue processing function that periodically checks for
/// existing merge proposals and publishes pending ready changes.
///
/// # Arguments
/// * `state` - The application state
/// * `interval` - The interval at which to process the queue
/// * `auto_publish` - Whether to automatically publish changes
/// * `push_limit` - Optional limit on the number of pushes
/// * `modify_mp_limit` - Optional limit on the number of merge proposals to modify
/// * `require_binary_diff` - Whether to require binary diffs
pub async fn process_queue_loop(
    state: Arc<AppState>,
    interval: chrono::Duration,
    auto_publish: bool,
    push_limit: Option<usize>,
    modify_mp_limit: Option<i32>,
    require_binary_diff: bool,
) {
    log::info!(
        "Starting publish queue processing loop (auto_publish: {}, interval: {:?})",
        auto_publish,
        interval
    );

    loop {
        let cycle_start = Utc::now();
        log::debug!("Starting publish queue cycle at {}", cycle_start);

        // Check existing merge proposals
        log::debug!("Checking existing merge proposals");
        crate::check_existing(
            state.conn.clone(),
            state.redis.clone(),
            state.config,
            &state.publish_worker,
            &state.bucket_rate_limiter,
            state.forge_rate_limiter.clone(),
            &state.vcs_managers,
            modify_mp_limit,
            state.unexpected_mp_limit,
        )
        .await;

        // Check for straggler merge proposals
        log::debug!("Checking straggler merge proposals");
        if let Err(e) = check_stragglers(&state.conn, state.redis.clone()).await {
            log::warn!("Error checking stragglers: {}", e);
        }

        // Publish pending ready changes if auto-publishing is enabled
        if auto_publish {
            log::debug!("Publishing pending ready changes");
            if let Err(e) =
                publish_pending_ready(state.clone(), push_limit, require_binary_diff).await
            {
                log::error!("Error publishing pending ready changes: {}", e);
            }
        } else {
            log::debug!("Auto-publish disabled, skipping publish phase");
        }

        // Calculate how long this cycle took and sleep for the remaining interval
        let cycle_duration = Utc::now() - cycle_start;
        let sleep_duration = interval - cycle_duration;

        if sleep_duration > chrono::Duration::zero() {
            log::debug!(
                "Cycle completed in {:?}, sleeping for {:?}",
                cycle_duration,
                sleep_duration
            );
            tokio::time::sleep(std::time::Duration::from_millis(
                sleep_duration.num_milliseconds().max(0) as u64,
            ))
            .await;
        } else {
            log::warn!(
                "Cycle took {:?}, longer than interval {:?}",
                cycle_duration,
                interval
            );
        }
    }
}

/// Publish all pending ready changes.
///
/// This function identifies runs that are ready to be published and initiates
/// the publishing process for them.
///
/// # Arguments
/// * `state` - The application state
/// * `push_limit` - Optional limit on the number of pushes
/// * `require_binary_diff` - Whether to require binary diffs
///
/// # Returns
/// Ok(()) if successful, or a PublishError
pub async fn publish_pending_ready(
    state: Arc<AppState>,
    push_limit: Option<usize>,
    require_binary_diff: bool,
) -> Result<(), PublishError> {
    let start_time = std::time::Instant::now();
    let mut actions: HashMap<Option<String>, usize> = HashMap::new();
    let mut published_count = 0;
    let mut error_count = 0;

    log::info!(
        "Starting publish_pending_ready (push_limit: {:?}, require_binary_diff: {})",
        push_limit,
        require_binary_diff
    );

    // Create iterator for publish-ready runs
    let mut iterator = PublishReadyIterator::new(state.conn.clone(), None);

    // Process each publish-ready run
    while let Some(ready_run) = iterator.next().await.map_err(|e| {
        log::error!("Database error iterating publish-ready runs: {}", e);
        PublishError::Failure {
            code: "database-error".to_string(),
            description: format!("Failed to iterate publish-ready runs: {}", e),
        }
    })? {
        log::info!(
            "Processing publish-ready run: {} (campaign: {}, codebase: {})",
            ready_run.run.id,
            ready_run.run.suite,
            ready_run.run.codebase
        );

        // Check push limit
        if let Some(limit) = push_limit {
            if published_count >= limit {
                log::info!("Reached push limit of {}, stopping", limit);
                break;
            }
        }

        // Consider publishing this run
        match consider_publish_run(
            &state.conn,
            state.redis.clone(),
            state.config,
            &state.publish_worker,
            &state.vcs_managers,
            &state.bucket_rate_limiter,
            &ready_run.run,
            &ready_run.rate_limit_bucket,
            &ready_run.unpublished_branches,
            &ready_run.command,
            push_limit.map(|limit| limit - published_count),
            require_binary_diff,
        )
        .await
        {
            Ok(results) => {
                // Count different types of actions
                for (key, value) in results {
                    if key == "status" {
                        if let Some(status) = value.as_ref() {
                            *actions.entry(Some(status.clone())).or_insert(0) += 1;

                            if status == "processing" {
                                published_count += 1;
                            }
                        }
                    }
                }

                log::debug!(
                    "Successfully considered run {} for publishing",
                    ready_run.run.id
                );
            }
            Err(e) => {
                error_count += 1;
                log::error!(
                    "Error considering run {} for publishing: {}",
                    ready_run.run.id,
                    e
                );
                *actions.entry(Some("error".to_string())).or_insert(0) += 1;
            }
        }
    }

    let duration = start_time.elapsed();

    log::info!(
        "Completed publish_pending_ready in {:?}: {} published, {} errors, actions: {:?}",
        duration,
        published_count,
        error_count,
        actions
    );

    // Update metrics if available
    if published_count > 0 {
        log::info!("Published {} changes this cycle", published_count);
    }

    Ok(())
}

/// Check for straggler merge proposals that need attention.
///
/// This function identifies merge proposals that have been open for a long time
/// and may need to be updated or closed.
///
/// # Arguments
/// * `conn` - Database connection pool
/// * `redis` - Optional Redis connection
///
/// # Returns
/// Ok(()) if successful, or a sqlx::Error
pub async fn check_stragglers(
    conn: &PgPool,
    redis: Option<redis::aio::ConnectionManager>,
) -> Result<(), sqlx::Error> {
    log::debug!("Checking for straggler merge proposals");

    // Query for merge proposals that have been open for more than a week
    let straggler_threshold = Utc::now() - chrono::Duration::days(7);

    let stragglers = sqlx::query_as::<_, (String, String, DateTime<Utc>)>(
        r#"
        SELECT url, status, created_time
        FROM merge_proposal
        WHERE status = 'open'
            AND created_time < $1
        ORDER BY created_time ASC
        LIMIT 100
        "#,
    )
    .bind(straggler_threshold)
    .fetch_all(conn)
    .await?;

    if !stragglers.is_empty() {
        log::info!(
            "Found {} straggler merge proposals older than 7 days",
            stragglers.len()
        );

        for (url, status, created_time) in stragglers {
            log::debug!(
                "Straggler MP: {} (status: {}, age: {} days)",
                url,
                status,
                (Utc::now() - created_time).num_days()
            );

            // In a full implementation, this would:
            // 1. Check if the merge proposal still exists
            // 2. Update its status if it has been merged/closed
            // 3. Consider abandoning very old proposals
            // 4. Notify relevant parties about stale proposals
        }
    } else {
        log::debug!("No straggler merge proposals found");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires database connection"]
    async fn test_publish_ready_iterator_creation() {
        let conn = sqlx::PgPool::connect("postgresql://localhost/test")
            .await
            .unwrap();
        let iterator = PublishReadyIterator::new(conn.clone(), None);
        assert!(iterator.run_id.is_none());

        let iterator_with_id = PublishReadyIterator::new(conn, Some("test-run".to_string()));
        assert_eq!(iterator_with_id.run_id, Some("test-run".to_string()));
    }
}
