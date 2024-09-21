use breezyshim::forge::MergeProposal;
use breezyshim::RevisionId;
use janitor::publish::{MergeProposalStatus, Mode};
use redis::AsyncCommands;
use sqlx::PgPool;
use url::Url;

#[derive(Debug, sqlx::FromRow)]
struct ProposalInfo {
    can_be_merged: Option<bool>,
    status: String,
    revision: RevisionId,
    target_branch_url: Option<String>,
    rate_limit_bucket: Option<String>,
    codebase: Option<String>,
}

struct ProposalInfoManager {
    conn: PgPool,
    redis: Option<redis::aio::ConnectionManager>,
}

impl ProposalInfoManager {
    pub async fn new(conn: PgPool, redis: Option<redis::aio::ConnectionManager>) -> Self {
        Self { conn, redis }
    }

    pub async fn iter_outdated_proposal_info_urls(
        &self,
        duration: chrono::Duration,
    ) -> Result<Vec<url::Url>, sqlx::Error> {
        let query = format!(
                "SELECT url FROM merge_proposal WHERE last_scanned is NULL OR now() - last_scanned > interval '{} days'", duration.num_days());
        let urls: Vec<String> = sqlx::query_scalar(&query).fetch_all(&self.conn).await?;

        Ok(urls.iter().map(|url| url.parse().unwrap()).collect())
    }

    async fn get_proposal_info(&self, url: &url::Url) -> Result<Option<ProposalInfo>, sqlx::Error> {
        let query = sqlx::query_as::<_, ProposalInfo>(
            r#"SELECT
                merge_proposal.rate_limit_bucket AS rate_limit_bucket,
                merge_proposal.revision,
                merge_proposal.status,
                merge_proposal.target_branch_url,
                merge_proposal.codebase,
                can_be_merged
            FROM
                merge_proposal
            WHERE
                merge_proposal.url = $1"#,
        );

        query.bind(url.to_string()).fetch_optional(&self.conn).await
    }

    async fn delete_proposal_info(&self, url: &url::Url) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM merge_proposal WHERE url = $1")
            .bind(url.to_string())
            .execute(&self.conn)
            .await?;
        Ok(())
    }

    async fn update_canonical_url(
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

    async fn update_proposal_info(
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
        let url = mp.url().unwrap();
        let (merged_by, merged_by_url, merged_at) = if status == MergeProposalStatus::Merged {
            let mp = mp.clone();
            tokio::task::spawn_blocking(move || {
                let merged_by = mp.get_merged_by().unwrap();
                let merged_by_url = if let Some(mb) = merged_by.clone().as_ref() {
                    crate::get_merged_by_user_url(&mp.url().unwrap(), &mb).unwrap()
                } else {
                    None
                };
                let merged_at = mp.get_merged_at().unwrap();
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
}
