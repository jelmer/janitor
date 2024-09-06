use crate::Mode;
use breezyshim::RevisionId;
use sqlx::PgPool;
use url::Url;

async fn store_publish(
    conn: &PgPool,
    change_set: &str,
    codebase: &str,
    branch_name: Option<&str>,
    target_branch_url: Option<&Url>,
    target_branch_web_url: Option<&str>,
    main_branch_revision: Option<&RevisionId>,
    revision: Option<&RevisionId>,
    role: &str,
    mode: Mode,
    result_code: &str,
    description: &str,
    merge_proposal_url: Option<&Url>,
    publish_id: Option<&str>,
    requester: Option<&str>,
    run_id: Option<&str>,
) -> Result<(), sqlx::Error> {
    let mut tx = conn.begin().await?;

    if result_code == "success" {
        if let Some(merge_proposal_url) = merge_proposal_url {
            assert_eq!(mode, Mode::Propose);
            sqlx::query(
                "INSERT INTO merge_proposal (url, status, revision, last_scanned,  target_branch_url, codebase) VALUES ($1, 'open', $2, NOW(), $3, $4) ON CONFLICT (url) DO UPDATE SET revision = EXCLUDED.revision, last_scanned = EXCLUDED.last_scanned, target_branch_url = EXCLUDED.target_branch_url, codebase = EXCLUDED.codebase")
            .bind(merge_proposal_url.to_string())
            .bind(revision.map(|r| r.to_string()))
            .bind(target_branch_url.map(|u| u.to_string()))
            .bind(codebase)
            .execute(&mut *tx)
            .await?;
        } else {
            assert!(revision.is_some());
            assert!([Mode::Push, Mode::PushDerived].contains(&mode));
            assert!(run_id.is_some());
            if mode == Mode::Push {
                sqlx::query(
                    "UPDATE new_result_branch SET absorbed = true WHERE run_id = $1 AND role = $2",
                )
                .bind(run_id)
                .bind(role)
                .execute(&mut *tx)
                .await?;
            }
        }
    }
    sqlx::query(
        "INSERT INTO publish (branch_name, main_branch_revision, revision, role, mode, result_code, description, merge_proposal_url, id, requester, change_set, run_id, target_branch_url, target_branch_web_url, codebase) values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15) ")
    .bind(branch_name)
    .bind(main_branch_revision.map(|r| r.to_string()))
    .bind(revision.map(|r| r.to_string()))
    .bind(role)
    .bind(mode.to_string())
    .bind(result_code)
    .bind(description)
    .bind(merge_proposal_url.map(|u| u.to_string()))
    .bind(publish_id)
    .bind(requester)
    .bind(change_set)
    .bind(run_id)
    .bind(target_branch_url.map(|u| u.to_string()))
    .bind(target_branch_web_url.map(|u| u.to_string()))
    .bind(codebase)
    .execute(&mut *tx)
    .await?;
    if result_code == "success" {
        sqlx::query("UPDATE change_set SET state = 'publishing' WHERE state = 'ready' AND id = $1")
            .bind(change_set)
            .execute(&mut *tx)
            .await?;
        // TODO(jelmer): if there is nothing left to publish, then mark this change_set as done
    }

    tx.commit().await
}

async fn already_published(
    conn: &PgPool,
    target_branch_url: &Url,
    branch_name: &str,
    revision: &RevisionId,
    modes: &[Mode],
) -> Result<bool, sqlx::Error> {
    let modes = modes.iter().map(|m| m.to_string()).collect::<Vec<_>>();
    let row = sqlx::query(
        "SELECT * FROM publish WHERE mode = ANY($1::publish_mode[]) AND revision = $2 AND target_branch_url = $3 AND branch_name = $4").bind(modes).bind(revision.to_string()).bind(target_branch_url.to_string()).bind(branch_name).fetch_optional(&*conn).await?;
    Ok(row.is_some())
}

async fn get_open_merge_proposal(
    conn: &PgPool,
    codebase: &str,
    branch_name: &str,
) -> Result<Option<(RevisionId, Url)>, sqlx::Error> {
    let row: Option<(String, String)> = sqlx::query_as(
        r###"
SELECT
    merge_proposal.revision,
    merge_proposal.url
FROM
    merge_proposal
INNER JOIN publish ON merge_proposal.url = publish.merge_proposal_url
WHERE
    merge_proposal.status = 'open' AND
    merge_proposal.codebase = $1 AND
    publish.branch_name = $2
ORDER BY timestamp DESC
"###,
    )
    .bind(codebase)
    .bind(branch_name)
    .fetch_optional(&*conn)
    .await?;

    Ok(row.map(|(revision, url)| {
        (
            RevisionId::from(revision.as_bytes().to_vec()),
            Url::parse(&url).unwrap(),
        )
    }))
}

async fn check_last_published(
    conn: &PgPool,
    campaign: &str,
    codebase: &str,
) -> Result<Option<chrono::DateTime<chrono::Utc>>, sqlx::Error> {
    let row: Option<(Option<chrono::DateTime<chrono::Utc>>,)> = sqlx::query_as(
        r###"
SELECT timestamp from publish left join run on run.revision = publish.revision
WHERE run.suite = $1 and run.codebase = $2 AND publish.result_code = 'success'
order by timestamp desc limit 1
"###,
    )
    .bind(campaign)
    .bind(codebase)
    .fetch_optional(conn)
    .await?;
    Ok(row.and_then(|(timestamp,)| timestamp))
}
