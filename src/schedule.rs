use crate::publish::Mode;
use crate::queue::Queue;
use breezyshim::RevisionId;
use chrono::Duration;
use debian_control::lossless::relations::{Relation, Relations};
use debian_control::relations::VersionConstraint;
use sqlx::postgres::types::PgInterval;
use sqlx::PgPool;
use std::collections::HashMap;

pub const FIRST_RUN_BONUS: f64 = 100.0;

// Default estimation if there is no median for the campaign or the codebase.
pub const DEFAULT_ESTIMATED_DURATION: i64 = 15;
pub const DEFAULT_SCHEDULE_OFFSET: f64 = -1.0;

fn publish_mode_value(mode: &Mode) -> usize {
    match mode {
        Mode::Skip => 0,
        Mode::BuildOnly => 0,
        Mode::Push => 500,
        Mode::Propose => 400,
        Mode::AttemptPush => 450,
        Mode::Bts => 100,
        Mode::PushDerived => 200,
    }
}

#[derive(sqlx::FromRow)]
pub struct ScheduleRequest {
    pub codebase: String,
    pub branch_url: String,
    pub campaign: String,
    pub context: String,
    pub value: i64,
    pub success_chance: f64,
    pub command: String,
    pub change_set: Option<String>,
}

pub async fn iter_schedule_requests_from_candidates(
    conn: &PgPool,
    codebases: Option<Vec<&str>>,
    campaign: Option<&str>,
) -> Result<impl Iterator<Item = ScheduleRequest>, sqlx::Error> {
    let mut query = sqlx::QueryBuilder::new(
        r###"
SELECT
  codebase.name AS codebase,
  codebase.branch_url AS branch_url,
  candidate.suite AS campaign,
  candidate.context AS context,
  candidate.value AS value,
  candidate.success_chance AS success_chance,
  array_agg(named_publish_policy.per_branch_policy.mode) AS publish_modes,
  candidate.command AS command,
  candidate.change_set AS change_set
FROM candidate
INNER JOIN codebase on codebase.name = candidate.codebase
INNER JOIN named_publish_policy ON
    named_publish_policy.name = candidate.publish_policy
INNER JOIN branch_publish_policy ON branch_publish_policy.role = ANY(named_publish_policy.per_branch_policy)
"###,
    );
    if let Some(codebases) = codebases {
        query.push(" AND codebase.name = ANY(");
        query.push_bind(codebases);
        query.push("::text[])");
    }
    if let Some(campaign) = campaign {
        query.push(" AND candidate.suite = ");
        query.push_bind(campaign);
    }

    let query = query.build();

    let rows = query.fetch_all(conn).await?;

    Ok(rows.into_iter().map(|row| {
        use sqlx::FromRow;
        use sqlx::Row;
        let mut req = ScheduleRequest::from_row(&row).unwrap();

        let pm = row.get::<Vec<String>, _>("publish_modes");

        req.value += pm
            .iter()
            .map(|m| publish_mode_value(&m.parse().unwrap()))
            .sum::<usize>() as i64;

        req
    }))
}

async fn estimate_duration_campaign_codebase(
    conn: &PgPool,
    codebase: Option<&str>,
    campaign: Option<&str>,
) -> Result<Option<Duration>, sqlx::Error> {
    let mut query = sqlx::QueryBuilder::new(
        r###"
SELECT AVG(finish_time - start_time) FROM run
WHERE failure_transient is not True
"###,
    );
    if let Some(codebase) = codebase {
        query.push(" AND codebase = ");
        query.push_bind(codebase);
    }
    if let Some(campaign) = campaign {
        query.push(" AND suite = ");
        query.push_bind(campaign);
    }
    let query = query.build_query_scalar::<PgInterval>();
    let duration: Option<PgInterval> = query.fetch_optional(conn).await?;
    Ok(duration.map(|d| chrono::Duration::microseconds(d.microseconds)))
}

/// Estimate the duration of a codebase build for a certain campaign.
async fn estimate_duration(
    conn: &PgPool,
    codebase: &str,
    campaign: &str,
) -> Result<Duration, sqlx::Error> {
    if let Some(estimated_duration) =
        estimate_duration_campaign_codebase(conn, Some(codebase), Some(campaign)).await?
    {
        Ok(estimated_duration)
    } else if let Some(estimated_duration) =
        estimate_duration_campaign_codebase(conn, Some(codebase), None).await?
    {
        Ok(estimated_duration)
    } else if let Some(estimated_duration) =
        estimate_duration_campaign_codebase(conn, None, Some(campaign)).await?
    {
        Ok(estimated_duration)
    } else {
        Ok(Duration::seconds(DEFAULT_ESTIMATED_DURATION))
    }
}

async fn estimate_success_probability_and_duration(
    conn: &PgPool,
    codebase: &str,
    campaign: &str,
    context: Option<&str>,
) -> Result<(f64, chrono::Duration, usize), sqlx::Error> {
    // TODO(jelmer): Bias this towards recent runs?
    let mut total = 0;
    let mut success = 0;
    let mut same_context_multiplier = if context.is_none() { 0.5 } else { 1.0 };
    let mut durations = vec![];
    #[derive(sqlx::FromRow)]
    struct Run {
        result_code: String,
        instigated_context: Option<String>,
        context: Option<String>,
        failure_details: Option<serde_json::Value>,
        duration: PgInterval,
        start_time: chrono::DateTime<chrono::Utc>,
    }

    // In some cases, we want to ignore certain results when guessing whether a future run is going to
    // be successful.  For example, some results are transient, or sometimes new runs will give a
    // clearer error message.
    fn ignore_result_code(run: &Run) -> bool {
        match run.result_code.as_str() {
            "worker-failure" => (chrono::Utc::now() - run.start_time).num_days() > 0,
            _ => false,
        }
    }

    let query = sqlx::query_as::<_, Run>(
        r#"""
SELECT
  result_code, instigated_context, context, failure_details,
  finish_time - start_time AS duration,
  start_time
FROM run
WHERE codebase = $1 AND suite = $2 AND failure_transient IS NOT True
ORDER BY start_time DESC
"""#,
    );
    for run in query
        .bind(codebase)
        .bind(campaign)
        .fetch_all(conn)
        .await?
        .iter()
    {
        if ignore_result_code(run) {
            continue;
        }

        durations.push(run.duration.microseconds / (1000 * 1000));
        total += 1;
        if run.result_code == "success" {
            success += 1;
        }
        let mut same_context = context != Some("")
            && context.is_some()
            && [run.instigated_context.as_deref(), run.context.as_deref()].contains(&context);
        if run.result_code == "install-deps-unsatisfied-dependencies"
            && run
                .failure_details
                .as_ref()
                .is_some_and(|d| d.get("relations").is_some())
        {
            let relations: Relations = run.failure_details.as_ref().unwrap()["relations"]
                .as_str()
                .unwrap()
                .parse()
                .unwrap();
            if deps_satisfied(conn, campaign, &relations).await? {
                success += 1;
                same_context = false;
            }
        }
        if same_context {
            same_context_multiplier = 0.1;
        }
    }

    let estimated_duration = if total == 0 {
        // If there were no previous runs, then it doesn't really matter that we don't know the context.
        same_context_multiplier = 1.0;

        estimate_duration(conn, codebase, campaign).await?
    } else {
        chrono::Duration::seconds(durations.iter().sum::<i64>() / durations.len() as i64)
    };

    Ok((
        (((success * 10 + 1) / (total * 10 + 1)) as f64 * same_context_multiplier),
        estimated_duration,
        total,
    ))
}

// Overhead of doing a run; estimated to be roughly 20s
pub const MINIMUM_COST: f64 = 20000.0;
pub const MINIMUM_NORMALIZED_CODEBASE_VALUE: f64 = 0.1;
pub const DEFAULT_NORMALIZED_CODEBASE_VALUE: f64 = 0.5;

fn calculate_offset(
    estimated_duration: chrono::Duration,
    normalized_codebase_value: Option<f64>,
    estimated_probability_of_success: f64,
    candidate_value: Option<f64>,
    total_previous_runs: usize,
    mut success_chance: Option<f64>,
) -> f64 {
    let normalized_codebase_value =
        normalized_codebase_value.unwrap_or(DEFAULT_NORMALIZED_CODEBASE_VALUE);

    let normalized_codebase_value =
        f64::max(MINIMUM_NORMALIZED_CODEBASE_VALUE, normalized_codebase_value);
    assert!(
        normalized_codebase_value > 0.0,
        "normalized codebase value is {}",
        normalized_codebase_value
    );

    let candidate_value = candidate_value.map_or(1.0, |v| {
        if total_previous_runs == 0 {
            v + FIRST_RUN_BONUS
        } else {
            v
        }
    });

    assert!(
        candidate_value > 0.0,
        "candidate value is {}",
        candidate_value
    );

    assert!(
        (0.0..=1.0).contains(&estimated_probability_of_success),
        "Probability of success: {}",
        estimated_probability_of_success
    );

    if let Some(success_chance) = success_chance.as_mut() {
        *success_chance *= estimated_probability_of_success;
    }

    // Estimated cost of doing the run, in milliseconds
    let estimated_cost = MINIMUM_COST
        + (1000.0 * (estimated_duration.num_seconds() as f64)
            + ((estimated_duration.num_microseconds().unwrap_or(0) as f64) / 1000.0));
    assert!(estimated_cost > 0.0, "Estimated cost: {}", estimated_cost);

    let estimated_value =
        normalized_codebase_value * estimated_probability_of_success * candidate_value;
    assert!(estimated_value > 0.0, "Estimated value: normalized_codebase_value({}) * estimated_probability_of_success({}) * candidate_value({})", normalized_codebase_value, estimated_probability_of_success, candidate_value);

    log::debug!(
        "normalized_codebase_value({}) * probability_of_success({}) * candidate_value({}) = estimated_value({}), estimated cost ({})", normalized_codebase_value,
        estimated_probability_of_success,
        candidate_value,
        estimated_value,
        estimated_cost,
    );

    estimated_cost / estimated_value
}

async fn do_schedule_regular(
    conn: &PgPool,
    codebase: &str,
    campaign: &str,
    command: Option<&str>,
    candidate_value: Option<f64>,
    success_chance: Option<f64>,
    mut normalized_codebase_value: Option<f64>,
    requester: Option<&str>,
    default_offset: f64,
    context: Option<&str>,
    change_set: Option<&str>,
    dry_run: bool,
    refresh: bool,
    bucket: Option<&str>,
) -> Result<(f64, chrono::Duration, i32, String), Error> {
    let (candidate_value, success_chance, command, context) = if candidate_value.is_none()
        || success_chance.is_none()
        || command.is_none()
    {
        let candidate = sqlx::query_as::<_, (f64, f64, String, Option<String>)>(
            "SELECT value, success_chance, command, context FROM candidate WHERE codebase = $1 and suite = $2 and coalesce(change_set, '') = $3").bind(codebase).bind(campaign).bind(change_set.unwrap_or("")).fetch_optional(conn).await?;
        let candidate: (f64, f64, String, Option<String>) = if let Some(candidate) = candidate {
            candidate
        } else {
            return Err(Error::CandidateUnavailable {
                campaign: campaign.to_string(),
                codebase: codebase.to_string(),
            });
        };
        (
            candidate_value.unwrap_or(candidate.0),
            success_chance.unwrap_or(candidate.1),
            command.unwrap_or(&candidate.2).to_owned(),
            if let Some(context) = context {
                Some(context.to_string())
            } else {
                candidate.3.map(|s| s.to_owned())
            },
        )
    } else {
        (
            candidate_value.unwrap(),
            success_chance.unwrap(),
            command.unwrap().to_string(),
            context.map(|s| s.to_owned()),
        )
    };

    let (estimated_probability_of_success, estimated_duration, total_previous_runs) =
        estimate_success_probability_and_duration(conn, codebase, campaign, context.as_deref())
            .await?;

    assert!(
        estimated_duration >= chrono::Duration::seconds(0),
        "{}: estimated duration < 0.0: {}",
        codebase,
        estimated_duration
    );

    if normalized_codebase_value.is_none() {
        normalized_codebase_value = sqlx::query_scalar::<_, f64>(
            "select coalesce(least(1.0 * value / (select max(value) from codebase), 1.0), 1.0) from codebase WHERE name = $1").bind(codebase).fetch_optional(conn).await?
    }

    let offset = calculate_offset(
        estimated_duration,
        normalized_codebase_value,
        estimated_probability_of_success,
        Some(candidate_value),
        total_previous_runs,
        Some(success_chance),
    );
    assert!(offset > 0.0);
    let offset = default_offset + offset;
    let bucket = bucket.unwrap_or("default");

    let requester = requester.unwrap_or("scheduler");

    assert!(!command.is_empty());
    let (queue_id, bucket): (i32, String) = if !dry_run {
        let queue = Queue::new(conn);
        let (queue_id, actual_bucket) = queue
            .add(
                codebase,
                &command,
                campaign,
                change_set,
                offset,
                bucket,
                context.as_deref(),
                Some(estimated_duration),
                refresh,
                Some(requester),
            )
            .await?;
        (queue_id, actual_bucket)
    } else {
        (-1, bucket.to_owned())
    };
    log::debug!(
        "Scheduled {} ({}) with offset {}",
        codebase,
        campaign,
        offset
    );
    Ok((offset, estimated_duration, queue_id, bucket))
}

pub async fn bulk_add_to_queue(
    conn: &PgPool,
    todo: &[ScheduleRequest],
    dry_run: bool,
    default_offset: f64,
    bucket: Option<&str>,
    requester: Option<&str>,
    refresh: bool,
) -> Result<(), Error> {
    let bucket = bucket.unwrap_or("default");
    let mut codebase_values = sqlx::query_as::<_, (String, f64)>(
        "SELECT name, coalesce(value, 0) FROM codebase WHERE name IS NOT NULL",
    )
    .fetch_all(conn)
    .await?
    .into_iter()
    .collect::<HashMap<_, _>>();
    let max_codebase_value = if !codebase_values.is_empty() {
        codebase_values
            .clone()
            .into_values()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
    } else {
        None
    };
    if let Some(max_codebase_value) = max_codebase_value.filter(|&v| v > 0.0) {
        log::info!("Maximum value: {}", max_codebase_value);
    }
    for req in todo {
        let normalized_codebase_value = if let Some(max_codebase_value) = max_codebase_value {
            std::cmp::min_by(
                codebase_values.remove(&req.codebase).unwrap_or(0.0) / max_codebase_value,
                1.0,
                |a, b| a.partial_cmp(b).unwrap(),
            )
        } else {
            1.0
        };
        do_schedule_regular(
            conn,
            &req.codebase,
            &req.campaign,
            Some(&req.command),
            Some(req.value as f64),
            Some(req.success_chance),
            Some(normalized_codebase_value),
            requester,
            default_offset,
            Some(&req.context),
            req.change_set.as_deref(),
            dry_run,
            refresh,
            Some(bucket),
        )
        .await?;
    }

    Ok(())
}

async fn dep_available(conn: &PgPool, rel: &Relation) -> Result<bool, sqlx::Error> {
    let mut query = sqlx::QueryBuilder::new(
        r###"
SELECT
  1
FROM
  all_debian_versions
WHERE
  source = "###,
    );
    query.push_bind(rel.name());

    if let Some(version) = rel.version() {
        query.push(" AND version ");
        query.push(match version.0 {
            VersionConstraint::Equal => "=",
            VersionConstraint::GreaterThan => ">",
            VersionConstraint::GreaterThanEqual => ">=",
            VersionConstraint::LessThan => "<",
            VersionConstraint::LessThanEqual => "<=",
        });
        query.push_bind(version.1);
    }

    let query = query.build_query_scalar::<bool>();

    Ok(query.fetch_optional(conn).await?.is_some())
}

async fn deps_satisfied(
    conn: &PgPool,
    _campaign: &str,
    dependencies: &Relations,
) -> Result<bool, sqlx::Error> {
    for dep in dependencies.entries() {
        // TODO: This is a bit inefficient, we should be able to do this in a single query.
        let mut found = false;

        for subdep in dep.relations() {
            if dep_available(conn, &subdep).await? {
                found = true;
                break;
            }
        }
        if !found {
            return Ok(false);
        }
    }
    Ok(true)
}

pub async fn do_schedule_control(
    conn: &PgPool,
    codebase: &str,
    change_set: Option<&str>,
    main_branch_revision: Option<&RevisionId>,
    offset: Option<f64>,
    refresh: bool,
    bucket: Option<&str>,
    requester: Option<&str>,
    estimated_duration: Option<chrono::Duration>,
) -> Result<(f64, chrono::Duration, i32, String), Error> {
    let mut command = vec!["brz".to_owned(), "up".to_owned()];
    if let Some(main_branch_revision) = main_branch_revision {
        command.push(format!("--revision={}", main_branch_revision));
    }
    let bucket = bucket.unwrap_or("control");
    do_schedule(
        conn,
        "control",
        codebase,
        bucket,
        change_set,
        offset,
        refresh,
        requester,
        estimated_duration,
        Some(&shlex::try_join(command.iter().map(|x| x.as_str()).collect::<Vec<_>>()).unwrap()),
    )
    .await
}

#[derive(Debug)]
pub enum Error {
    CandidateUnavailable { campaign: String, codebase: String },
    SqlError(sqlx::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::CandidateUnavailable { campaign, codebase } => {
                write!(f, "No candidate available for {} in {}", campaign, codebase)
            }
            Error::SqlError(e) => write!(f, "SQL error: {}", e),
        }
    }
}

impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        Error::SqlError(e)
    }
}

pub async fn do_schedule(
    conn: &PgPool,
    campaign: &str,
    codebase: &str,
    bucket: &str,
    change_set: Option<&str>,
    offset: Option<f64>,
    refresh: bool,
    requester: Option<&str>,
    estimated_duration: Option<chrono::Duration>,
    command: Option<&str>,
) -> Result<(f64, chrono::Duration, i32, String), Error> {
    let offset = offset.unwrap_or(DEFAULT_SCHEDULE_OFFSET);
    let command = if let Some(command) = command {
        command.to_string()
    } else {
        let candidate: Option<(String,)> =
            sqlx::query_as("SELECT command FROM candidate WHERE codebase = $1 AND suite = $2")
                .bind(codebase)
                .bind(campaign)
                .fetch_optional(conn)
                .await?;
        if candidate.is_none() {
            return Err(Error::CandidateUnavailable {
                campaign: campaign.to_owned(),
                codebase: codebase.to_owned(),
            });
        }
        candidate.unwrap().0
    };
    let estimated_duration = if let Some(estimated_duration) = estimated_duration {
        estimated_duration
    } else {
        estimate_duration(conn, codebase, campaign).await?
    };
    let queue = Queue::new(conn);
    let (queue_id, bucket) = queue
        .add(
            codebase,
            &command,
            campaign,
            change_set,
            offset,
            bucket,
            None,
            Some(estimated_duration),
            refresh,
            requester,
        )
        .await?;
    Ok((offset, estimated_duration, queue_id, bucket))
}
