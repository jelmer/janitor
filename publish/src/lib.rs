//! Publish crate for the Janitor project.
//!
//! This crate provides functionality for publishing changes and managing merge proposals.

#![deny(missing_docs)]

use breezyshim::error::Error as BrzError;
use breezyshim::forge::Forge;
use breezyshim::RevisionId;
use chrono::{DateTime, Utc};
use janitor::config::Campaign;
use janitor::publish::{MergeProposalStatus, Mode};
use janitor::vcs::{VcsManager, VcsType};
use reqwest::header::HeaderMap;
use serde::ser::SerializeStruct;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

/// Module for managing merge proposal information.
pub mod proposal_info;
/// Module for publishing a single change.
pub mod publish_one;
/// Module for rate limiting publish operations.
pub mod rate_limiter;
/// Module for managing publish state.
pub mod state;
/// Module for web interface to publish functionality.
pub mod web;

use rate_limiter::RateLimiter;

/// Calculate the next time to try publishing based on previous attempts.
///
/// This implements an exponential backoff strategy with a maximum delay.
///
/// # Arguments
/// * `finish_time` - The time of the last attempt
/// * `attempt_count` - The number of previous attempts
///
/// # Returns
/// The next time to try publishing
pub fn calculate_next_try_time(finish_time: DateTime<Utc>, attempt_count: usize) -> DateTime<Utc> {
    if attempt_count == 0 {
        finish_time
    } else {
        let delta = chrono::Duration::hours(2usize.pow(attempt_count as u32).min(7 * 24) as i64);

        finish_time + delta
    }
}

/// Errors that can occur when retrieving a debdiff.
#[derive(Debug)]
pub enum DebdiffError {
    /// An HTTP error occurred.
    Http(reqwest::Error),
    /// The run ID was missing.
    MissingRun(String),
    /// The debdiff is unavailable.
    Unavailable(String),
}

impl From<reqwest::Error> for DebdiffError {
    fn from(e: reqwest::Error) -> Self {
        DebdiffError::Http(e)
    }
}

impl std::fmt::Display for DebdiffError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DebdiffError::Http(e) => write!(f, "HTTP error: {}", e),
            DebdiffError::MissingRun(e) => write!(f, "Missing run: {}", e),
            DebdiffError::Unavailable(e) => write!(f, "Unavailable: {}", e),
        }
    }
}

impl std::error::Error for DebdiffError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DebdiffError::Http(e) => Some(e),
            _ => None,
        }
    }
}

/// Get a debdiff between two runs.
///
/// # Arguments
/// * `differ_url` - The URL of the differ service
/// * `unchanged_id` - The ID of the unchanged run
/// * `log_id` - The ID of the changed run
///
/// # Returns
/// The debdiff as a byte vector, or an error
pub fn get_debdiff(
    differ_url: &url::Url,
    unchanged_id: &str,
    log_id: &str,
) -> Result<Vec<u8>, DebdiffError> {
    let debdiff_url = differ_url
        .join(&format!(
            "/debdiff/{}/{}?filter_boring=1",
            unchanged_id, log_id
        ))
        .unwrap();

    let mut headers = HeaderMap::new();
    headers.insert("Accept", "text/plain".parse().unwrap());

    let client = reqwest::blocking::Client::new();
    let response = client.get(debdiff_url).headers(headers).send()?;

    match response.status() {
        reqwest::StatusCode::OK => Ok(response.bytes()?.to_vec()),
        reqwest::StatusCode::NOT_FOUND => {
            let run_id = response
                .headers()
                .get("unavailable_run_id")
                .unwrap()
                .to_str()
                .unwrap();
            Err(DebdiffError::MissingRun(run_id.to_string()))
        }
        reqwest::StatusCode::BAD_REQUEST
        | reqwest::StatusCode::INTERNAL_SERVER_ERROR
        | reqwest::StatusCode::BAD_GATEWAY
        | reqwest::StatusCode::SERVICE_UNAVAILABLE
        | reqwest::StatusCode::GATEWAY_TIMEOUT => {
            Err(DebdiffError::Unavailable(response.text().unwrap()))
        }
        _e => Err(DebdiffError::Http(response.error_for_status().unwrap_err())),
    }
}

/// Request to publish a single run.
#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct PublishOneRequest {
    /// The campaign name.
    pub campaign: String,
    /// The URL of the target branch.
    pub target_branch_url: url::Url,
    /// The role of the publisher.
    pub role: String,
    /// The ID of the log.
    pub log_id: String,
    /// Optional list of reviewers.
    pub reviewers: Option<Vec<String>>,
    /// The revision ID of the change.
    pub revision_id: RevisionId,
    /// The ID of the unchanged run.
    pub unchanged_id: String,
    /// Whether to require a binary diff.
    #[serde(rename = "require-binary-diff")]
    pub require_binary_diff: bool,
    /// The URL of the differ service.
    pub differ_url: url::Url,
    /// The name of the derived branch.
    pub derived_branch_name: String,
    /// Optional map of tags to revision IDs.
    pub tags: Option<HashMap<String, RevisionId>>,
    /// Whether to allow creating a new proposal.
    pub allow_create_proposal: bool,
    /// The URL of the source branch.
    pub source_branch_url: url::Url,
    /// The result of the codemod.
    pub codemod_result: serde_json::Value,
    /// Optional template for the commit message.
    pub commit_message_template: Option<String>,
    /// Optional template for the title.
    pub title_template: Option<String>,
    /// Optional URL of an existing merge proposal.
    pub existing_mp_url: Option<url::Url>,
    /// Optional extra context for the templates.
    pub extra_context: Option<serde_json::Value>,
    /// The mode of the publish operation.
    pub mode: Mode,
    /// The command that was run.
    pub command: String,
    /// Optional external URL for the publish operation.
    pub external_url: Option<url::Url>,
    /// Optional owner of the derived branch.
    pub derived_owner: Option<String>,
    /// Optional flag to automatically merge the proposal.
    pub auto_merge: Option<bool>,
}

/// Errors that can occur during publishing.
#[derive(Debug)]
pub enum PublishError {
    /// A failure occurred with a specific code and description.
    Failure {
        /// Error code that indicates the type of failure.
        code: String,
        /// Detailed description of the failure.
        description: String,
    },
    /// Nothing to do, with a reason.
    NothingToDo(String),
    /// The branch is already being used.
    BranchBusy(url::Url),
}

impl PublishError {
    /// Get the error code.
    ///
    /// # Returns
    /// The error code as a string
    pub fn code(&self) -> &str {
        match self {
            PublishError::Failure { code, .. } => code,
            PublishError::NothingToDo(_) => "nothing-to-do",
            PublishError::BranchBusy(_) => "branch-busy",
        }
    }

    /// Get the error description.
    ///
    /// # Returns
    /// The error description as a string
    pub fn description(&self) -> &str {
        match self {
            PublishError::Failure { description, .. } => description,
            PublishError::NothingToDo(description) => description,
            PublishError::BranchBusy(_) => "Branch is busy",
        }
    }
}

impl serde::Serialize for PublishError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            PublishError::Failure { code, description } => {
                let mut state = serializer.serialize_struct("PublishError", 2)?;
                state.serialize_field("code", code)?;
                state.serialize_field("description", description)?;
                state.end()
            }
            PublishError::NothingToDo(description) => {
                let mut state = serializer.serialize_struct("PublishError", 2)?;
                state.serialize_field("code", "nothing-to-do")?;
                state.serialize_field("description", description)?;
                state.end()
            }
            PublishError::BranchBusy(url) => {
                let mut state = serializer.serialize_struct("PublishError", 2)?;
                state.serialize_field("code", "branch-busy")?;
                state.serialize_field("description", &format!("Branch is busy: {}", url))?;
                state.end()
            }
        }
    }
}

impl std::fmt::Display for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PublishError::Failure { code, description } => {
                write!(f, "PublishError::Failure: {}: {}", code, description)
            }
            PublishError::NothingToDo(description) => {
                write!(f, "PublishError::PublishNothingToDo: {}", description)
            }
            PublishError::BranchBusy(url) => {
                write!(f, "PublishError::BranchBusy: Branch is busy: {}", url)
            }
        }
    }
}

impl std::error::Error for PublishError {}

/// Result of a publish operation.
#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct PublishOneResult {
    /// The URL of the created merge proposal, if any.
    proposal_url: Option<url::Url>,
    /// The web URL of the created merge proposal, if any.
    proposal_web_url: Option<url::Url>,
    /// Whether the merge proposal is new.
    is_new: Option<bool>,
    /// The name of the branch.
    branch_name: String,
    /// The URL of the target branch.
    target_branch_url: url::Url,
    /// The web URL of the target branch, if any.
    target_branch_web_url: Option<url::Url>,
    /// The mode of the publish operation.
    mode: Mode,
}

/// Error returned by the publish_one operation.
#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct PublishOneError {
    /// The error code.
    code: String,
    /// A description of the error.
    description: String,
}

/// Worker for publishing changes.
pub struct PublishWorker {
    /// Optional path to the template environment.
    pub template_env_path: Option<PathBuf>,
    /// Optional external URL for the publish operation.
    pub external_url: Option<url::Url>,
    /// URL of the differ service.
    pub differ_url: url::Url,
    /// Optional Redis connection manager.
    pub redis: Option<redis::aio::ConnectionManager>,
    /// Optional lock manager for coordinating publish operations.
    pub lock_manager: Option<rslock::LockManager>,
}

/// Errors that can occur when interacting with a worker process.
#[derive(Debug)]
pub enum WorkerInvalidResponse {
    /// An I/O error occurred.
    Io(std::io::Error),
    /// An error occurred during serialization or deserialization.
    Serde(serde_json::Error),
    /// An error returned by the worker process.
    WorkerError(String),
}

impl From<std::io::Error> for WorkerInvalidResponse {
    fn from(e: std::io::Error) -> Self {
        WorkerInvalidResponse::Io(e)
    }
}

impl From<serde_json::Error> for WorkerInvalidResponse {
    fn from(e: serde_json::Error) -> Self {
        WorkerInvalidResponse::Serde(e)
    }
}

impl From<String> for WorkerInvalidResponse {
    fn from(e: String) -> Self {
        WorkerInvalidResponse::WorkerError(e)
    }
}

impl std::fmt::Display for WorkerInvalidResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            WorkerInvalidResponse::Io(e) => write!(f, "IO error: {}", e),
            WorkerInvalidResponse::Serde(e) => write!(f, "Serde error: {}", e),
            WorkerInvalidResponse::WorkerError(e) => write!(f, "Worker error: {}", e),
        }
    }
}

impl std::error::Error for WorkerInvalidResponse {}

/// Run a worker process with the given arguments and request.
///
/// # Arguments
/// * `args` - The command line arguments for the worker process
/// * `request` - The publish request to send to the worker
///
/// # Returns
/// A tuple of the exit code and the response value, or an error
async fn run_worker_process(
    args: Vec<String>,
    request: PublishOneRequest,
) -> Result<(i32, serde_json::Value), WorkerInvalidResponse> {
    let mut p = tokio::process::Command::new(args[0].clone())
        .args(&args[1..])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    use tokio::io::AsyncWriteExt;
    p.stdin
        .as_mut()
        .unwrap()
        .write_all(serde_json::to_string(&request).unwrap().as_bytes())
        .await?;

    let status = p.wait().await?;

    if status.success() {
        let mut stdout = p.stdout.take().unwrap();
        let _stderr = p.stderr.take().unwrap();
        let mut output = Vec::new();
        tokio::io::AsyncReadExt::read_to_end(&mut stdout, &mut output).await?;

        let response =
            serde_json::from_reader(&mut output.as_slice()).map_err(WorkerInvalidResponse::from)?;
        Ok((status.code().unwrap(), response))
    } else if status.code() == Some(1) {
        let mut stdout = p.stdout.take().unwrap();
        let mut stderr = p.stderr.take().unwrap();
        let mut output = Vec::new();
        tokio::io::AsyncReadExt::read_to_end(&mut stdout, &mut output).await?;
        let response =
            serde_json::from_reader(&mut output.as_slice()).map_err(WorkerInvalidResponse::from)?;
        let mut error = Vec::new();
        tokio::io::AsyncReadExt::read_to_end(&mut stderr, &mut error).await?;
        use std::io::Write;
        std::io::stderr().write_all(&error)?;
        return Ok((status.code().unwrap(), response));
    } else {
        let mut stderr = p.stderr.take().unwrap();
        let mut error = Vec::new();
        tokio::io::AsyncReadExt::read_to_end(&mut stderr, &mut error).await?;
        return Err(WorkerInvalidResponse::from(
            String::from_utf8(error).unwrap(),
        ));
    }
}

impl PublishWorker {
    /// Create a new publish worker.
    ///
    /// # Arguments
    /// * `template_env_path` - Optional path to the template environment
    /// * `external_url` - Optional external URL for the publish operation
    /// * `differ_url` - URL of the differ service
    /// * `redis` - Optional Redis connection manager
    /// * `lock_manager` - Optional lock manager for coordinating publish operations
    ///
    /// # Returns
    /// A new PublishWorker instance
    pub async fn new(
        template_env_path: Option<PathBuf>,
        external_url: Option<url::Url>,
        differ_url: url::Url,
        redis: Option<redis::aio::ConnectionManager>,
        lock_manager: Option<rslock::LockManager>,
    ) -> Self {
        Self {
            template_env_path,
            external_url,
            differ_url,
            redis,
            lock_manager,
        }
    }

    /// Publish a single run in some form.
    ///
    /// # Arguments
    /// * `campaign` - The campaign name
    /// * `command` - Command that was run
    async fn publish_one(
        &mut self,
        campaign: &str,
        codebase: &str,
        command: &str,
        target_branch_url: &url::Url,
        mode: Mode,
        role: &str,
        revision: &RevisionId,
        log_id: &str,
        unchanged_id: &str,
        derived_branch_name: &str,
        rate_limit_bucket: Option<&str>,
        vcs_manager: &dyn VcsManager,
        mut bucket_rate_limiter: Option<&mut dyn RateLimiter>,
        require_binary_diff: bool,
        allow_create_proposal: bool,
        reviewers: Option<Vec<&str>>,
        tags: Option<Vec<(String, RevisionId)>>,
        commit_message_template: Option<&str>,
        title_template: Option<&str>,
        codemod_result: &serde_json::Value,
        existing_mp_url: Option<&url::Url>,
        extra_context: Option<&serde_json::Value>,
        derived_owner: Option<&str>,
        auto_merge: Option<bool>,
    ) -> Result<PublishOneResult, PublishError> {
        let local_branch_url =
            vcs_manager.get_branch_url(codebase, &format!("{}/{}", campaign, role));

        let request = PublishOneRequest {
            campaign: campaign.to_owned(),
            command: command.to_owned(),
            codemod_result: codemod_result.clone(),
            target_branch_url: target_branch_url.clone(),
            source_branch_url: local_branch_url,
            existing_mp_url: existing_mp_url.cloned(),
            derived_branch_name: derived_branch_name.to_owned(),
            mode,
            role: role.to_owned(),
            log_id: log_id.to_owned(),
            unchanged_id: unchanged_id.to_owned(),
            require_binary_diff,
            allow_create_proposal,
            external_url: self.external_url.clone(),
            differ_url: self.differ_url.clone(),
            revision_id: revision.clone(),
            reviewers: reviewers.map(|r| r.iter().map(|s| s.to_string()).collect()),
            commit_message_template: commit_message_template.map(|s| s.to_string()),
            title_template: title_template.map(|s| s.to_string()),
            extra_context: extra_context.cloned(),
            tags: tags.map(|t| t.into_iter().collect()),
            derived_owner: derived_owner.map(|s| s.to_string()),
            auto_merge,
        };

        let mut args = vec!["janitor-publish-one".to_string()];

        if let Some(template_env_path) = self.template_env_path.as_ref() {
            args.push(format!(
                "--template-env-path={}",
                template_env_path.display()
            ));
        }

        let (returncode, response) = if let Some(lock_manager) = &self.lock_manager {
            match lock_manager
                .lock(
                    format!("publish:{}", target_branch_url).as_bytes(),
                    std::time::Duration::from_secs(60),
                )
                .await
            {
                Ok(rl) => {
                    let (returncode, response) = match run_worker_process(args, request).await {
                        Ok((returncode, response)) => (returncode, response),
                        Err(e) => {
                            return Err(PublishError::Failure {
                                code: "publisher-invalid-response".to_string(),
                                description: e.to_string(),
                            });
                        }
                    };
                    lock_manager.unlock(&rl).await;
                    (returncode, response)
                }
                Err(_) => {
                    return Err(PublishError::BranchBusy(target_branch_url.clone()));
                }
            }
        } else {
            match run_worker_process(args, request).await {
                Ok((returncode, response)) => (returncode, response),
                Err(e) => {
                    return Err(PublishError::Failure {
                        code: "publisher-invalid-response".to_string(),
                        description: e.to_string(),
                    });
                }
            }
        };

        if returncode == 1 {
            let error: PublishOneError =
                serde_json::from_value(response).map_err(|e| PublishError::Failure {
                    code: "publisher-invalid-response".to_string(),
                    description: e.to_string(),
                })?;
            return Err(PublishError::Failure {
                code: error.code,
                description: error.description,
            });
        }

        if returncode == 0 {
            let result: PublishOneResult =
                serde_json::from_value(response).map_err(|e| PublishError::Failure {
                    code: "publisher-invalid-response".to_string(),
                    description: e.to_string(),
                })?;

            if result.proposal_url.is_some() && result.is_new.unwrap() {
                use redis::AsyncCommands;
                if let Some(redis) = self.redis.as_mut() {
                    let _: () = redis
                        .publish(
                            "merge-proposal".to_string(),
                            serde_json::to_string(&serde_json::json!({
                                "url": result.proposal_url,
                                "web_url": result.proposal_web_url,
                                "status": "open",
                                "codebase": codebase,
                                "campaign": campaign,
                                "target_branch_url": result.target_branch_url,
                                "target_branch_web_url": result.target_branch_web_url,
                            }))
                            .unwrap(),
                        )
                        .await
                        .unwrap();
                }

                if let Some(bucket) = rate_limit_bucket {
                    if let Some(rate_limiter) = bucket_rate_limiter.as_mut() {
                        rate_limiter.inc(bucket);
                    }
                }
            }

            return Ok(result);
        }

        unreachable!();
    }
}

/// Check if a run is sufficient to create a merge proposal based on its value.
///
/// # Arguments
/// * `campaign_config` - The campaign configuration
/// * `run_value` - The value associated with the run
///
/// # Returns
/// `true` if the run is sufficient to create a merge proposal, `false` otherwise
pub fn run_sufficient_for_proposal(campaign_config: &Campaign, run_value: Option<i32>) -> bool {
    if let (Some(run_value), Some(threshold)) =
        (run_value, &campaign_config.merge_proposal.value_threshold)
    {
        run_value >= *threshold
    } else {
        // Assume yes, if the run doesn't have an associated value or if there is no threshold configured.
        true
    }
}

/// Get the URL for a role branch.
///
/// # Arguments
/// * `url` - The base URL
/// * `remote_branch_name` - Optional name of the remote branch
///
/// # Returns
/// The URL for the role branch
pub fn role_branch_url(url: &url::Url, remote_branch_name: Option<&str>) -> url::Url {
    if let Some(remote_branch_name) = remote_branch_name {
        let (base_url, mut params) = breezyshim::urlutils::split_segment_parameters(
            &url.to_string().trim_end_matches('/').parse().unwrap(),
        );

        params.insert(
            "branch".to_owned(),
            breezyshim::urlutils::escape_utf8(remote_branch_name, Some("")),
        );

        breezyshim::urlutils::join_segment_parameters(&base_url, params)
    } else {
        url.clone()
    }
}

/// Check if two branch URLs refer to the same branch.
///
/// # Arguments
/// * `url_a` - The first branch URL
/// * `url_b` - The second branch URL
///
/// # Returns
/// `true` if the branches match, `false` otherwise
pub fn branches_match(url_a: Option<&url::Url>, url_b: Option<&url::Url>) -> bool {
    use silver_platter::vcs::{open_branch, BranchOpenError};
    if url_a == url_b {
        return true;
    }
    if url_a.is_none() || url_b.is_none() {
        return false;
    }
    let url_a = url_a.unwrap();
    let url_b = url_b.unwrap();
    let (base_url_a, _params_a) = breezyshim::urlutils::split_segment_parameters(
        &url_a.to_string().trim_end_matches('/').parse().unwrap(),
    );
    let (base_url_b, _params_b) = breezyshim::urlutils::split_segment_parameters(
        &url_b.to_string().trim_end_matches('/').parse().unwrap(),
    );
    // TODO(jelmer): Support following redirects
    if base_url_a.to_string().trim_end_matches('/') != base_url_b.to_string().trim_end_matches('/')
    {
        return false;
    }
    let branch_a = match open_branch(url_a, None, None, None) {
        Ok(branch) => branch,
        Err(BranchOpenError::Missing { .. }) => return false,
        Err(e) => panic!("Unexpected error: {:?}", e),
    };
    let branch_b = match open_branch(url_b, None, None, None) {
        Ok(branch) => branch,
        Err(BranchOpenError::Missing { .. }) => return false,
        Err(e) => panic!("Unexpected error: {:?}", e),
    };
    branch_a.name() == branch_b.name()
}

/// Get the URL for a user who merged a branch.
///
/// # Arguments
/// * `url` - The branch URL
/// * `user` - The username
///
/// # Returns
/// The user's URL, or None if not available
pub fn get_merged_by_user_url(url: &url::Url, user: &str) -> Result<Option<url::Url>, BrzError> {
    let hostname = if let Some(host) = url.host_str() {
        host
    } else {
        return Ok(None);
    };

    let forge = match breezyshim::forge::get_forge_by_hostname(hostname) {
        Ok(forge) => forge,
        Err(BrzError::UnsupportedForge(..)) => return Ok(None),
        Err(e) => return Err(e),
    };
    Ok(Some(forge.get_user_url(user)?))
}

/// Process the publish queue in a loop.
///
/// # Arguments
/// * `state` - The application state
/// * `interval` - The interval at which to process the queue
/// * `auto_publish` - Whether to automatically publish changes
pub async fn process_queue_loop(
    state: Arc<AppState>,
    interval: chrono::Duration,
    auto_publish: bool,
) {
    todo!();
}

/// Publish all pending ready changes.
///
/// # Arguments
/// * `state` - The application state
///
/// # Returns
/// Ok(()) if successful, or a PublishError
pub async fn publish_pending_ready(state: Arc<AppState>) -> Result<(), PublishError> {
    todo!();
}

/// Refresh the counts of merge proposals per bucket.
///
/// # Arguments
/// * `state` - The application state
///
/// # Returns
/// Ok(()) if successful, or a sqlx::Error
pub async fn refresh_bucket_mp_counts(state: Arc<AppState>) -> Result<(), sqlx::Error> {
    let mut per_bucket: HashMap<janitor::publish::MergeProposalStatus, HashMap<String, usize>> =
        HashMap::new();

    let rows = sqlx::query_as::<_, (String, String, i64)>(
        r#"
        SELECT
        rate_limit_bucket AS rate_limit_bucket,
        status AS status,
        count(*) as c
        FROM merge_proposal
        GROUP BY 1, 2
        "#,
    )
    .fetch_all(&state.conn)
    .await?;

    for row in rows {
        per_bucket
            .entry(row.1.parse().unwrap())
            .or_default()
            .insert(row.0, row.2 as usize);
    }
    state
        .bucket_rate_limiter
        .lock()
        .unwrap()
        .set_mps_per_bucket(&per_bucket);
    Ok(())
}

/// Listen to the runner for new changes to publish.
///
/// # Arguments
/// * `state` - The application state
pub async fn listen_to_runner(state: Arc<AppState>) {
    todo!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_calculate_next_try_time() {
        let finish_time = Utc::now();
        let attempt_count = 0;
        let next_try_time = calculate_next_try_time(finish_time, attempt_count);
        assert_eq!(finish_time, next_try_time);

        let attempt_count = 1;
        let next_try_time = calculate_next_try_time(finish_time, attempt_count);
        assert_eq!(finish_time + chrono::Duration::hours(2), next_try_time);

        let attempt_count = 2;
        let next_try_time = calculate_next_try_time(finish_time, attempt_count);
        assert_eq!(finish_time + chrono::Duration::hours(4), next_try_time);

        let attempt_count = 3;
        let next_try_time = calculate_next_try_time(finish_time, attempt_count);
        assert_eq!(finish_time + chrono::Duration::hours(8), next_try_time);

        // Verify that the maximum delay is 7 days
        let attempt_count = 10;
        let next_try_time = calculate_next_try_time(finish_time, attempt_count);
        assert_eq!(finish_time + chrono::Duration::days(7), next_try_time);
    }
}

/// Application state for the publish service.
pub struct AppState {
    /// Database connection pool.
    pub conn: sqlx::PgPool,
    /// Rate limiter for buckets.
    pub bucket_rate_limiter: Mutex<Box<dyn rate_limiter::RateLimiter>>,
    /// Rate limiter for forges.
    pub forge_rate_limiter: Arc<RwLock<HashMap<String, chrono::DateTime<Utc>>>>,
    /// Optional limit on the number of pushes.
    pub push_limit: Option<usize>,
    /// Optional Redis connection manager.
    pub redis: Option<redis::aio::ConnectionManager>,
    /// Configuration for the service.
    pub config: &'static janitor::config::Config,
    /// Worker for publishing changes.
    pub publish_worker: PublishWorker,
    /// Map of VCS managers by type.
    pub vcs_managers: HashMap<VcsType, Box<dyn VcsManager>>,
    /// Optional limit on the number of merge proposals to modify.
    pub modify_mp_limit: Option<i32>,
    /// Optional limit on the number of unexpected errors.
    pub unexpected_mp_limit: Option<i32>,
    /// GPG context for signing commits.
    pub gpg: breezyshim::gpg::GPGContext,
    /// Whether to require binary diffs.
    pub require_binary_diff: bool,
}

/// Errors that can occur when checking a merge proposal.
#[derive(Debug)]
pub enum CheckMpError {
    /// No run was found for the merge proposal.
    NoRunForMergeProposal(url::Url),
    /// The branch is rate limited.
    BranchRateLimited {
        /// Optional duration after which to retry.
        retry_after: Option<chrono::Duration>,
    },
    /// An unexpected HTTP status was received.
    UnexpectedHttpStatus,
    /// Login is required for the forge.
    ForgeLoginRequired,
}

impl From<BrzError> for CheckMpError {
    fn from(e: BrzError) -> Self {
        match e {
            BrzError::UnexpectedHttpStatus { .. } => CheckMpError::UnexpectedHttpStatus,
            BrzError::ForgeLoginRequired => CheckMpError::ForgeLoginRequired,
            _ => CheckMpError::UnexpectedHttpStatus,
        }
    }
}

impl std::fmt::Display for CheckMpError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CheckMpError::NoRunForMergeProposal(url) => {
                write!(f, "No run for merge proposal: {}", url)
            }
            CheckMpError::BranchRateLimited { retry_after } => write!(f, "Branch is rate limited"),
            CheckMpError::UnexpectedHttpStatus => write!(f, "Unexpected HTTP status"),
            CheckMpError::ForgeLoginRequired => write!(f, "Forge login required"),
        }
    }
}

impl std::error::Error for CheckMpError {}

async fn check_existing_mp(
    conn: &sqlx::PgPool,
    redis: Option<redis::aio::ConnectionManager>,
    config: &janitor::config::Config,
    publish_worker: &crate::PublishWorker,
    mp: &breezyshim::forge::MergeProposal,
    status: breezyshim::forge::MergeProposalStatus,
    vcs_managers: &HashMap<VcsType, Box<dyn VcsManager>>,
    bucket_rate_limiter: &Mutex<Box<dyn crate::rate_limiter::RateLimiter>>,
    check_only: bool,
    mps_per_bucket: Option<
        &mut HashMap<janitor::publish::MergeProposalStatus, HashMap<String, usize>>,
    >,
    possible_transports: Option<&mut Vec<breezyshim::transport::Transport>>,
) -> Result<bool, CheckMpError> {
    todo!()
}

/// Iterate over all merge proposals.
///
/// # Arguments
/// * `statuses` - Optional list of statuses to filter by
///
/// # Returns
/// An iterator over results containing the forge, merge proposal, and status
pub fn iter_all_mps(
    statuses: Option<&[breezyshim::forge::MergeProposalStatus]>,
) -> impl Iterator<
    Item = Result<
        (
            Forge,
            breezyshim::forge::MergeProposal,
            breezyshim::forge::MergeProposalStatus,
        ),
        BrzError,
    >,
> + '_ {
    let statuses = statuses.unwrap_or(&[
        breezyshim::forge::MergeProposalStatus::Open,
        breezyshim::forge::MergeProposalStatus::Closed,
        breezyshim::forge::MergeProposalStatus::Merged,
    ]);
    breezyshim::forge::iter_forge_instances().flat_map(|instance| {
        statuses.iter().flat_map(move |status| {
            let proposals = instance.iter_my_proposals(Some(*status), None).unwrap();
            let value = instance.clone();

            proposals.map(move |proposal| Ok((value.clone(), proposal, *status)))
        })
    })
}

async fn check_existing(
    conn: sqlx::PgPool,
    redis: Option<redis::aio::ConnectionManager>,
    config: &janitor::config::Config,
    publish_worker: &crate::PublishWorker,
    bucket_rate_limiter: &Mutex<Box<dyn crate::rate_limiter::RateLimiter>>,
    forge_rate_limiter: Arc<RwLock<HashMap<String, chrono::DateTime<Utc>>>>,
    vcs_managers: &HashMap<VcsType, Box<dyn VcsManager>>,
    modify_limit: Option<i32>,
    unexpected_limit: Option<i32>,
) {
    let mut mps_per_bucket = maplit::hashmap! {
        MergeProposalStatus::Open => maplit::hashmap! {},
        MergeProposalStatus::Closed => maplit::hashmap! {},
        MergeProposalStatus::Merged => maplit::hashmap! {},
        MergeProposalStatus::Applied => maplit::hashmap! {},
        MergeProposalStatus::Abandoned => maplit::hashmap! {},
        MergeProposalStatus::Rejected => maplit::hashmap! {},
    };
    let mut possible_transports = Vec::new();
    let mut status_count = maplit::hashmap! {
        MergeProposalStatus::Open => 0,
        MergeProposalStatus::Closed => 0,
        MergeProposalStatus::Merged => 0,
        MergeProposalStatus::Applied => 0,
        MergeProposalStatus::Abandoned => 0,
        MergeProposalStatus::Rejected => 0,
    };

    let mut modified_mps = 0;
    let mut unexpected = 0;
    let mut check_only = false;
    let mut was_forge_ratelimited = false;

    for (forge, mp, status) in iter_all_mps(None).filter_map(Result::ok) {
        *status_count.entry(status.into()).or_insert(0) += 1;
        if let Some(retry_after) = forge_rate_limiter.read().unwrap().get(&forge.forge_name()) {
            if chrono::Utc::now() < *retry_after {
                forge_rate_limiter
                    .write()
                    .unwrap()
                    .remove(&forge.forge_name());
            } else {
                was_forge_ratelimited = true;
                continue;
            }
        }
        let modified = match check_existing_mp(
            &conn,
            redis.clone(),
            config,
            publish_worker,
            &mp,
            status,
            vcs_managers,
            bucket_rate_limiter,
            check_only,
            Some(&mut mps_per_bucket),
            Some(&mut possible_transports),
        )
        .await
        {
            Ok(modified) => modified,
            Err(CheckMpError::NoRunForMergeProposal(url)) => {
                log::warn!("Unable to find metadata for {}, skipping.", url);
                false
            }
            Err(CheckMpError::ForgeLoginRequired) => {
                log::warn!("Login required, skipping.");
                false
            }
            Err(CheckMpError::BranchRateLimited { retry_after }) => {
                log::warn!(
                    "Rate-limited accessing {}. Skipping {:?} for this cycle.",
                    mp.url().unwrap(),
                    forge
                );
                let retry_after = if let Some(retry_after) = retry_after {
                    retry_after
                } else {
                    chrono::Duration::minutes(30)
                };
                forge_rate_limiter
                    .write()
                    .unwrap()
                    .insert(forge.forge_name(), chrono::Utc::now() + retry_after);
                continue;
            }
            Err(CheckMpError::UnexpectedHttpStatus) => {
                log::warn!("Got unexpected HTTP status for {}", mp.url().unwrap());
                // TODO(jelmer): print traceback?
                unexpected += 1;
                true
            }
        };

        if unexpected_limit.map(|ul| unexpected > ul).unwrap_or(false) {
            log::warn!(
                "Saw {} unexpected HTTP responses, over threshold of {}. Giving up for now.",
                unexpected,
                unexpected_limit.unwrap(),
            );
            return;
        }

        if modified {
            modified_mps += 1;
            if modify_limit.map(|ml| modified_mps > ml).unwrap_or(false) {
                log::warn!(
                    "Already modified {} merge proposals, waiting with the rest.",
                    modified_mps,
                );
                check_only = true;
            }
        }
    }

    log::info!("Successfully scanned existing merge proposals");

    if !was_forge_ratelimited {
        let mut total = 0;
        for (bucket, count) in mps_per_bucket
            .get(&janitor::publish::MergeProposalStatus::Open)
            .unwrap_or(&maplit::hashmap! {})
            .iter()
        {
            total += count;
        }
    } else {
        log::info!(
            "Rate-Limited for forges {:?}. Not updating stats",
            forge_rate_limiter
        );
    }
}

async fn consider_publish_run(
    conn: &sqlx::PgPool,
    redis: Option<redis::aio::ConnectionManager>,
    config: &janitor::config::Config,
    publish_worker: &crate::PublishWorker,
    vcs_managers: &HashMap<VcsType, Box<dyn VcsManager>>,
    bucket_rate_limiter: &Mutex<Box<dyn crate::rate_limiter::RateLimiter>>,
    run: &janitor::state::Run,
    rate_limit_bucket: &str,
    unpublished_branches: &[crate::state::UnpublishedBranch],
    command: &str,
    push_limit: Option<usize>,
    require_binary_diff: bool,
) -> Result<HashMap<String, Option<String>>, sqlx::Error> {
    let mut results = HashMap::new();
    
    log::info!("Considering publish for run {} (campaign: {}, codebase: {})", 
              run.id, run.suite, run.codebase);

    // Check if the run is successful
    if run.result_code.as_deref() != Some("success") {
        log::info!("Run {} not successful (result: {:?}), skipping publish", 
                  run.id, run.result_code);
        results.insert("status".to_string(), Some("not_successful".to_string()));
        return Ok(results);
    }

    // Check rate limiting
    let rate_limit_result = {
        let limiter = bucket_rate_limiter.lock().unwrap();
        limiter.check_allowed(rate_limit_bucket)
    };
    
    if !rate_limit_result.is_allowed() {
        log::info!("Rate limited for bucket {}, skipping publish for run {}", 
                  rate_limit_bucket, run.id);
        results.insert("status".to_string(), Some("rate_limited".to_string()));
        results.insert("rate_limit_bucket".to_string(), Some(rate_limit_bucket.to_string()));
        return Ok(results);
    }

    // Check if we should skip due to binary diff requirement
    if require_binary_diff {
        // For now, assume we have binary diff capability
        // In a full implementation, this would check if the diff contains binary changes
        log::debug!("Binary diff check passed for run {}", run.id);
    }

    // Check push limit
    if let Some(limit) = push_limit {
        // Get current number of active publishes
        let active_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM publish WHERE result_code IS NULL"
        )
        .fetch_one(conn)
        .await? as usize;
        
        if active_count >= limit {
            log::info!("Push limit reached ({}/{}), skipping publish for run {}", 
                      active_count, limit, run.id);
            results.insert("status".to_string(), Some("push_limit_reached".to_string()));
            results.insert("active_publishes".to_string(), Some(active_count.to_string()));
            return Ok(results);
        }
    }

    // Process each unpublished branch
    for branch in unpublished_branches {
        let branch_name = &branch.role;
        log::debug!("Processing branch {} for run {}", branch_name, run.id);

        // Create publish request for this branch
        match try_publish_branch(
            conn,
            redis.clone(),
            config,
            publish_worker,
            vcs_managers,
            bucket_rate_limiter,
            run,
            branch,
            command,
        ).await {
            Ok(publish_result) => {
                results.insert(
                    format!("branch_{}", branch_name), 
                    Some(publish_result)
                );
                log::info!("Successfully initiated publish for branch {} of run {}", 
                          branch_name, run.id);
            }
            Err(e) => {
                log::warn!("Failed to publish branch {} of run {}: {}", 
                          branch_name, run.id, e);
                results.insert(
                    format!("branch_{}_error", branch_name), 
                    Some(e.to_string())
                );
            }
        }
    }

    // Update rate limiter to track this publish attempt
    {
        let mut limiter = bucket_rate_limiter.lock().unwrap();
        match limiter.check_allowed(rate_limit_bucket) {
            crate::rate_limiter::RateLimitStatus::Allowed => {
                // Continue with publishing
            }
            status => {
                log::warn!("Rate limited for bucket {}: {:?}", rate_limit_bucket, status);
                return Ok(results);
            }
        }
    }

    results.insert("status".to_string(), Some("processing".to_string()));
    results.insert("run_id".to_string(), Some(run.id.clone()));
    
    Ok(results)
}

/// Helper function to attempt publishing a single branch.
async fn try_publish_branch(
    conn: &sqlx::PgPool,
    redis: Option<redis::aio::ConnectionManager>,
    config: &janitor::config::Config,
    publish_worker: &crate::PublishWorker,
    vcs_managers: &HashMap<VcsType, Box<dyn VcsManager>>,
    bucket_rate_limiter: &Mutex<Box<dyn crate::rate_limiter::RateLimiter>>,
    run: &janitor::state::Run,
    branch: &crate::state::UnpublishedBranch,
    command: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // Get campaign configuration
    let campaign = match config.campaign.iter().find(|c| c.name() == run.suite) {
        Some(campaign) => campaign,
        None => return Err(format!("No campaign configuration for suite {}", run.suite).into()),
    };

    // Determine publish mode based on branch role and campaign settings
    let mode = match branch.role.as_str() {
        "main" => Mode::BuildOnly, // Default to build-only for main branches
        "debian" => Mode::Push,    // Push Debian branches
        _ => Mode::BuildOnly,      // Conservative default
    };

    // Create publish record
    let publish_id = sqlx::query_scalar::<_, String>(
        r#"
        INSERT INTO publish (
            id, mode, branch_name, main_branch_revision, revision, 
            target_branch_url, result_code, description
        ) VALUES ($1, $2, $3, $4, $5, $6, NULL, 'Publishing in progress')
        RETURNING id
        "#
    )
    .bind(&run.id)
    .bind(mode.to_string())
    .bind(&branch.role)
    .bind(&run.main_branch_revision)
    .bind(&run.revision)
    .bind(&run.target_branch_url)
    .fetch_one(conn)
    .await?;

    log::info!("Created publish record {} for run {} branch {}", 
              publish_id, run.id, branch.role);

    // For now, return success. In a full implementation, this would:
    // 1. Queue the actual publishing work to the publish worker
    // 2. Handle different publish modes (propose, push, build-only)
    // 3. Create merge proposals if needed
    // 4. Update the publish record with results
    
    Ok(format!("publish_{}", publish_id))
}
