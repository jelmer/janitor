use crate::Mode;
use breezyshim::transport::Transport;
use breezyshim::RevisionId;
use sqlx::{FromRow, PgPool, Row};
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

async fn guess_codebase_from_branch_url(
    conn: &PgPool,
    url: &url::Url,
    possible_transports: Option<&mut Vec<Transport>>,
) -> Result<Option<String>, sqlx::Error> {
    let url = url
        .to_string()
        .trim_end_matches('/')
        .parse::<Url>()
        .unwrap();
    // TODO(jelmer): use codebase table
    let query = sqlx::query_as::<_, (String, String)>(
        r#"""
SELECT
  name, branch_url
FROM
  codebase
WHERE
  TRIM(trailing '/' from branch_url) = ANY($1::text[])
ORDER BY length(branch_url) DESC
"""#,
    );
    let (repo_url, params) = breezyshim::urlutils::split_segment_parameters(&url);
    let branch = params
        .get("branch")
        .map(|b| breezyshim::urlutils::unescape_utf8(b));
    let result = query
        .bind(url.to_string())
        .bind(repo_url.to_string().trim_end_matches('/'))
        .fetch_optional(&*conn)
        .await?;

    if result.is_none() {
        return Ok(None);
    }

    let result = result.unwrap();

    if &url.to_string() == result.1.trim_end_matches('/') {
        return Ok(Some(result.0));
    }

    let source_branch = tokio::task::spawn_blocking(move || {
        silver_platter::vcs::open_branch(&result.1.parse().unwrap(), Some(&mut vec![]), None, None)
    })
    .await
    .unwrap()
    .unwrap();
    if source_branch
        .get_user_url()
        .to_string()
        .trim_end_matches("/")
        != url.to_string().trim_end_matches("/")
        && source_branch.name() != branch
    {
        log::info!(
            "Did not resolve branch URL to codebase: {} ({}) != {} ({})",
            source_branch.get_user_url(),
            source_branch.name().unwrap_or("".to_string()),
            url,
            branch.unwrap_or("".to_string()),
        );
        return Ok(None);
    }
    Ok(Some(result.0))
}

#[derive(sqlx::FromRow)]
/// Information about a run that resulted in a merge proposal.
pub struct MergeProposalRun {
    id: String,
    campaign: String,
    branch_url: String,
    command: String,
    value: i64,
    role: String,
    remote_branch_name: String,
    revision: RevisionId,
    codebase: String,
    change_set: String,
}

async fn get_merge_proposal_run(
    conn: &PgPool,
    mp_url: &url::Url,
) -> Result<Option<MergeProposalRun>, sqlx::Error> {
    sqlx::query_as::<_, MergeProposalRun>(
        r#"
SELECT
    run.id AS id,
    run.suite AS campaign,
    run.branch_url AS branch_url,
    run.command AS command,
    run.value AS value,
    rb.role AS role,
    rb.remote_name AS remote_branch_name,
    rb.revision AS revision,
    run.codebase AS codebase,
    run.change_set AS change_set
FROM new_result_branch rb
RIGHT JOIN run ON rb.run_id = run.id
WHERE rb.revision IN (
    SELECT revision from merge_proposal WHERE merge_proposal.url = $1)
ORDER BY run.finish_time DESC
LIMIT 1
"#,
    )
    .bind(mp_url.to_string())
    .fetch_optional(conn)
    .await
}

async fn get_last_effective_run(
    conn: &PgPool,
    codebase: &str,
    campaign: &str,
) -> Result<Option<janitor::state::Run>, sqlx::Error> {
    sqlx::query_as(
        r#"""
SELECT
    id, command, start_time, finish_time, description,
    result_code,
    value, main_branch_revision, revision, context, result, suite,
    instigated_context, vcs_type, branch_url, logfilenames,
    worker,
    array(SELECT row(role, remote_name, base_revision,
     revision) FROM new_result_branch WHERE run_id = id) AS result_branches,
    result_tags, target_branch_url, change_set AS change_set,
    failure_transient, failure_stage, codebase
FROM
    last_effective_runs
WHERE codebase = $1 AND suite = $2
LIMIT 1
"""#,
    )
    .bind(codebase)
    .bind(campaign)
    .fetch_optional(&*conn)
    .await
}

/// Count the number of publish attempts for a specific revision, excluding those with transient result codes.
pub async fn get_publish_attempt_count(
    conn: &PgPool,
    revision: &RevisionId,
    transient_result_codes: &[&str],
) -> Result<usize, sqlx::Error> {
    Ok(sqlx::query_scalar::<_, i64>(
        "select count(*) from publish where revision = $1 and result_code != ALL($2::text[])",
    )
    .bind(revision)
    .bind(transient_result_codes)
    .fetch_one(&*conn)
    .await? as usize)
}

/// Get the status of previous merge proposals for a codebase and campaign.
pub async fn get_previous_mp_status(
    conn: &PgPool,
    codebase: &str,
    campaign: &str,
) -> Result<Vec<(String, String)>, sqlx::Error> {
    sqlx::query_as(
        r#"""
WITH per_run_mps AS (
    SELECT run.id AS run_id, run.finish_time,
    merge_proposal.url AS mp_url, merge_proposal.status AS mp_status
    FROM run
    LEFT JOIN merge_proposal ON run.revision = merge_proposal.revision
    WHERE run.codebase = $1
    AND run.suite = $2
    AND run.result_code = 'success'
    AND merge_proposal.status NOT IN ('open', 'abandoned')
    GROUP BY run.id, merge_proposal.url
)
SELECT mp_url, mp_status FROM per_run_mps
WHERE run_id = (
    SELECT run_id FROM per_run_mps ORDER BY finish_time DESC LIMIT 1)
"""#,
    )
    .bind(codebase)
    .bind(campaign)
    .fetch_all(conn)
    .await
}

#[derive(Debug, sqlx::FromRow)]
/// Information about a branch that hasn't been published yet.
pub struct UnpublishedBranch {
    /// Role of the branch.
    pub role: String,
    /// Name of the remote branch.
    pub remote_name: String,
    /// Base revision ID.
    pub base_revision: RevisionId,
    /// Current revision ID.
    pub revision: RevisionId,
    /// Mode to use for publishing.
    pub publish_mode: Mode,
    /// Maximum frequency in days between publish attempts.
    pub max_frequency_days: Option<i32>,
}

/// Iterate through runs that are ready to be published.
pub async fn iter_publish_ready(
    conn: &PgPool,
    run_id: Option<&str>,
) -> Result<Vec<(janitor::state::Run, String, String, Vec<UnpublishedBranch>)>, sqlx::Error> {
    let mut query = sqlx::QueryBuilder::new("SELECT * FROM publish_ready WHERE ");
    if let Some(run_id) = run_id {
        query.push("id = ");
        query.push_bind(run_id);
    } else {
        query.push("True");
    }
    query.push(" AND publish_status = 'approved'");
    query.push(" AND change_set_state IN ('ready', 'publishing')");
    query.push("and exists (select from unnest(unpublished_branches) where mode in ('propose', 'attempt-push', 'push-derived', 'push'))");
    query.push(
        " ORDER BY change_set_state = 'publishing' DESC, value DESC NULLS LAST, finish_time DESC",
    );

    let query = query.build();

    let rows = query.fetch_all(conn).await?;

    let mut result = vec![];
    for row in rows {
        result.push((
            janitor::state::Run::from_row(&row).unwrap(),
            row.get("rate_limit_bucket"),
            row.get("policy_command"),
            row.get("unpublished_branches"),
        ));
    }
    Ok(result)
}
