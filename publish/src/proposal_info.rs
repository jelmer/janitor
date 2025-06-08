use breezyshim::forge::MergeProposal;
use breezyshim::RevisionId;
use janitor::publish::MergeProposalStatus;
use redis::AsyncCommands;
use sqlx::{PgPool, Row};

/// Information about a merge proposal stored in the database.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ProposalInfo {
    /// Whether the proposal can be merged.
    pub can_be_merged: Option<bool>,
    /// Current status of the proposal.
    pub status: String,
    /// Source revision ID.
    pub revision: Option<String>,
    /// Target branch URL.
    pub target_branch_url: Option<String>,
    /// Rate limit bucket for this proposal.
    pub rate_limit_bucket: Option<String>,
    /// Codebase this proposal belongs to.
    pub codebase: String,
}

/// Manager for handling merge proposal information.
pub struct ProposalInfoManager {
    conn: PgPool,
    redis: Option<redis::aio::ConnectionManager>,
}

impl ProposalInfoManager {
    /// Create a new proposal info manager.
    ///
    /// # Arguments
    /// * `conn` - Database connection pool
    /// * `redis` - Optional Redis connection manager
    ///
    /// # Returns
    /// A new ProposalInfoManager instance
    pub async fn new(conn: PgPool, redis: Option<redis::aio::ConnectionManager>) -> Self {
        Self { conn, redis }
    }

    /// Retrieve a list of proposal info URLs that haven't been scanned in a given duration.
    pub async fn iter_outdated_proposal_info_urls(
        &self,
        duration: chrono::Duration,
    ) -> Result<Vec<url::Url>, sqlx::Error> {
        let query = format!(
                "SELECT url FROM merge_proposal WHERE last_scanned is NULL OR now() - last_scanned > interval '{} days'", duration.num_days());
        let urls: Vec<String> = sqlx::query_scalar(&query).fetch_all(&self.conn).await?;

        Ok(urls.iter().map(|url| url.parse().unwrap()).collect())
    }

    /// Retrieve proposal information for a given URL.
    ///
    /// # Arguments
    /// * `url` - The URL of the merge proposal
    ///
    /// # Returns
    /// Proposal info if found, or None
    pub async fn get_proposal_info(
        &self,
        url: &url::Url,
    ) -> Result<Option<ProposalInfo>, sqlx::Error> {
        let row = sqlx::query(
            r#"SELECT
                merge_proposal.rate_limit_bucket,
                merge_proposal.revision,
                merge_proposal.status,
                merge_proposal.target_branch_url,
                merge_proposal.codebase,
                merge_proposal.can_be_merged
            FROM merge_proposal
            WHERE merge_proposal.url = $1"#,
        )
        .bind(url.to_string())
        .fetch_optional(&self.conn)
        .await?;

        if let Some(row) = row {
            Ok(Some(ProposalInfo {
                rate_limit_bucket: row.try_get("rate_limit_bucket").ok(),
                revision: row.try_get("revision").ok(),
                status: row.get("status"),
                target_branch_url: row.try_get("target_branch_url").ok(),
                can_be_merged: row.try_get("can_be_merged").ok(),
                codebase: row.get("codebase"),
            }))
        } else {
            Ok(None)
        }
    }

    /// Delete proposal information for a given URL.
    pub async fn delete_proposal_info(&self, url: &url::Url) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM merge_proposal WHERE url = $1")
            .bind(url.to_string())
            .execute(&self.conn)
            .await?;
        Ok(())
    }

    /// Mark a proposal as not found (tombstone) instead of deleting it.
    /// This preserves the record while indicating the proposal no longer exists.
    pub async fn mark_proposal_as_not_found(&self, url: &url::Url) -> Result<(), sqlx::Error> {
        // Set status to 'closed' and update last_scanned to indicate we checked it
        sqlx::query(
            "UPDATE merge_proposal SET status = 'closed', last_scanned = NOW() WHERE url = $1",
        )
        .bind(url.to_string())
        .execute(&self.conn)
        .await?;
        Ok(())
    }

    /// Update the canonical URL for a proposal.
    pub async fn update_canonical_url(
        &self,
        old_url: &url::Url,
        canonical_url: &url::Url,
    ) -> Result<(), sqlx::Error> {
        let old_url: Option<String> = sqlx::query_scalar(
            "UPDATE merge_proposal canonical SET codebase = COALESCE(canonical.codebase, old.codebase), rate_limit_bucket = COALESCE(canonical.rate_limit_bucket, old.rate_limit_bucket) FROM merge_proposal old WHERE old.url = $1 AND canonical.url = $2 RETURNING old.url").bind(old_url.to_string()).bind(canonical_url.to_string()).fetch_optional(&self.conn).await?;
        sqlx::query("UPDATE publish SET merge_proposal_url = $1 WHERE merge_proposal_url = $2")
            .bind(canonical_url.to_string())
            .bind(old_url.as_ref().unwrap().to_string())
            .execute(&self.conn)
            .await?;

        if let Some(old_url) = old_url.as_ref() {
            sqlx::query("DELETE FROM merge_proposal WHERE url = $1")
                .bind(old_url)
                .execute(&self.conn)
                .await?;
        } else {
            sqlx::query("UPDATE merge_proposal SET url = $1 WHERE url = $2")
                .bind(canonical_url.to_string())
                .bind(old_url)
                .execute(&self.conn)
                .await?;
        }
        Ok(())
    }

    /// Update proposal information in the database.
    ///
    /// # Arguments
    /// * `mp` - The merge proposal
    /// * `status` - Current status of the proposal
    /// * `revision` - Source revision ID
    /// * `codebase` - Codebase name
    /// * `target_branch_url` - Target branch URL
    /// * `campaign` - Campaign name
    /// * `can_be_merged` - Whether the proposal can be merged
    /// * `rate_limit_bucket` - Rate limit bucket
    ///
    /// # Returns
    /// Ok(()) if successful, or a sqlx::Error
    pub async fn update_proposal_info(
        &mut self,
        mp: &MergeProposal,
        status: MergeProposalStatus,
        revision: Option<&RevisionId>,
        codebase: &str,
        target_branch_url: &url::Url,
        campaign: &str,
        can_be_merged: Option<bool>,
        rate_limit_bucket: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        if status == MergeProposalStatus::Closed {
            // TODO(jelmer): Check if changes were applied manually and mark as applied rather than closed?
        }
        let url = match mp.url() {
            Ok(url) => url,
            Err(e) => {
                log::error!("Failed to get merge proposal URL: {}", e);
                return Err(sqlx::Error::RowNotFound);
            }
        };
        let (merged_by, merged_by_url, merged_at) = if status == MergeProposalStatus::Merged {
            let mp = mp.clone();
            tokio::task::spawn_blocking(move || {
                let merged_by = match mp.get_merged_by() {
                    Ok(merged_by) => merged_by,
                    Err(e) => {
                        log::error!("Failed to get merged_by from merge proposal: {}", e);
                        None
                    }
                };
                let merged_by_url = if let Some(mb) = merged_by.clone().as_ref() {
                    match mp.url() {
                        Ok(mp_url) => match crate::get_merged_by_user_url(&mp_url, mb) {
                            Ok(url) => url,
                            Err(e) => {
                                log::error!("Failed to get merged_by user URL: {}", e);
                                None
                            }
                        },
                        Err(e) => {
                            log::error!("Failed to get merge proposal URL for merged_by: {}", e);
                            None
                        }
                    }
                } else {
                    None
                };
                let merged_at = match mp.get_merged_at() {
                    Ok(merged_at) => merged_at,
                    Err(e) => {
                        log::error!("Failed to get merged_at from merge proposal: {}", e);
                        None
                    }
                };
                (merged_by, merged_by_url, merged_at)
            })
            .await
            .unwrap()
        } else {
            (None, None, None)
        };
        let mut tx = self.conn.begin().await?;

        sqlx::query(
            r###"INSERT INTO merge_proposal (
                    url, status, revision, merged_by, merged_at,
                    target_branch_url, last_scanned, can_be_merged, rate_limit_bucket,
                    codebase)
                VALUES ($1, $2, $3, $4, $5, $6, NOW(), $7, $8, $9)
                ON CONFLICT (url)
                DO UPDATE SET
                  status = EXCLUDED.status,
                  revision = EXCLUDED.revision,
                  merged_by = EXCLUDED.merged_by,
                  merged_at = EXCLUDED.merged_at,
                  target_branch_url = EXCLUDED.target_branch_url,
                  last_scanned = EXCLUDED.last_scanned,
                  can_be_merged = EXCLUDED.can_be_merged,
                  rate_limit_bucket = EXCLUDED.rate_limit_bucket,
                  codebase = EXCLUDED.codebase
                "###,
        )
        .bind(url.to_string())
        .bind(status.to_string())
        .bind(revision)
        .bind(merged_by.clone())
        .bind(merged_at)
        .bind(target_branch_url.to_string())
        .bind(can_be_merged)
        .bind(rate_limit_bucket)
        .bind(codebase)
        .execute(&mut *tx)
        .await?;
        if let Some(revision) = revision.as_ref() {
            sqlx::query(r#"UPDATE new_result_branch SET absorbed = $1 WHERE revision = $2"#)
                .bind(status == MergeProposalStatus::Merged)
                .bind(revision)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;

        // TODO(jelmer): Check if the change_set should be marked as published

        if let Some(redis) = self.redis.as_mut() {
            redis
                .publish::<_, _, i32>(
                    "merge-proposal",
                    serde_json::to_string(&serde_json::json!({
                        "url": url,
                        "target_branch_url": target_branch_url,
                        "rate_limit_bucket": rate_limit_bucket,
                        "status": status,
                        "codebase": codebase,
                        "merged_by": merged_by,
                        "merged_by_url": merged_by_url,
                        "merged_at": merged_at,
                        "campaign": campaign,
                    }))
                    .unwrap(),
                )
                .await
                .unwrap();
        }
        Ok(())
    }

    /// Check if a proposal exists in the database.
    ///
    /// # Arguments
    /// * `url` - The URL of the merge proposal
    ///
    /// # Returns
    /// True if the proposal exists, false otherwise
    pub async fn proposal_exists(&self, url: &url::Url) -> Result<bool, sqlx::Error> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM merge_proposal WHERE url = $1")
            .bind(url.to_string())
            .fetch_one(&self.conn)
            .await?;

        Ok(count > 0)
    }

    /// Get all proposals for a specific codebase.
    ///
    /// # Arguments
    /// * `codebase` - The codebase name
    /// * `status_filter` - Optional status filter
    ///
    /// # Returns
    /// List of proposal info for the codebase
    pub async fn get_proposals_for_codebase(
        &self,
        codebase: &str,
        status_filter: Option<&str>,
    ) -> Result<Vec<ProposalInfo>, sqlx::Error> {
        let mut query = "SELECT rate_limit_bucket, revision, status, target_branch_url, codebase, can_be_merged FROM merge_proposal WHERE codebase = $1".to_string();

        if let Some(status) = status_filter {
            query.push_str(" AND status = $2");
            let rows = sqlx::query(&query)
                .bind(codebase)
                .bind(status)
                .fetch_all(&self.conn)
                .await?;

            Ok(rows
                .into_iter()
                .map(|row| ProposalInfo {
                    rate_limit_bucket: row.try_get("rate_limit_bucket").ok(),
                    revision: row.try_get("revision").ok(),
                    status: row.get("status"),
                    target_branch_url: row.try_get("target_branch_url").ok(),
                    can_be_merged: row.try_get("can_be_merged").ok(),
                    codebase: row.get("codebase"),
                })
                .collect())
        } else {
            let rows = sqlx::query(&query)
                .bind(codebase)
                .fetch_all(&self.conn)
                .await?;

            Ok(rows
                .into_iter()
                .map(|row| ProposalInfo {
                    rate_limit_bucket: row.try_get("rate_limit_bucket").ok(),
                    revision: row.try_get("revision").ok(),
                    status: row.get("status"),
                    target_branch_url: row.try_get("target_branch_url").ok(),
                    can_be_merged: row.try_get("can_be_merged").ok(),
                    codebase: row.get("codebase"),
                })
                .collect())
        }
    }

    /// Update the last scanned timestamp for a proposal.
    ///
    /// # Arguments
    /// * `url` - The URL of the merge proposal
    ///
    /// # Returns
    /// Ok(()) if successful, or a sqlx::Error
    pub async fn touch_proposal(&self, url: &url::Url) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE merge_proposal SET last_scanned = NOW() WHERE url = $1")
            .bind(url.to_string())
            .execute(&self.conn)
            .await?;
        Ok(())
    }

    /// Get statistics about proposals in the database.
    ///
    /// # Returns
    /// A map of status -> count
    pub async fn get_proposal_statistics(
        &self,
    ) -> Result<std::collections::HashMap<String, i64>, sqlx::Error> {
        let rows =
            sqlx::query("SELECT status, COUNT(*) as count FROM merge_proposal GROUP BY status")
                .fetch_all(&self.conn)
                .await?;

        let mut stats = std::collections::HashMap::new();
        for row in rows {
            let status: String = row.get("status");
            let count: i64 = row.get("count");
            stats.insert(status, count);
        }

        Ok(stats)
    }

    /// Clean up old closed proposals.
    ///
    /// # Arguments
    /// * `days_old` - Remove proposals closed more than this many days ago
    ///
    /// # Returns
    /// Number of proposals removed
    pub async fn cleanup_old_proposals(&self, days_old: i32) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "DELETE FROM merge_proposal WHERE status IN ('closed', 'merged') AND last_scanned < NOW() - INTERVAL '$1 days'"
        )
        .bind(days_old)
        .execute(&self.conn)
        .await?;

        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_proposal_info_manager_creation() {
        // This would need a test database connection
        // let manager = ProposalInfoManager::new(pool, None).await;
        // assert!(manager.redis.is_none());
    }

    #[test]
    fn test_proposal_info_serialization() {
        let info = ProposalInfo {
            can_be_merged: Some(true),
            status: "open".to_string(),
            revision: Some("abc123".to_string()),
            target_branch_url: Some("https://github.com/test/repo".to_string()),
            rate_limit_bucket: Some("default".to_string()),
            codebase: "test-codebase".to_string(),
        };

        assert_eq!(info.status, "open");
        assert_eq!(info.codebase, "test-codebase");
        assert!(info.can_be_merged.unwrap());
    }
}
