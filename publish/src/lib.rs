use breezyshim::error::Error as BrzError;
use breezyshim::forge::Forge;
use breezyshim::RevisionId;
use chrono::{DateTime, Utc};
use janitor::config::Campaign;
use janitor::publish::Mode;
use janitor::vcs::VcsManager;
use reqwest::header::HeaderMap;
use serde::ser::SerializeStruct;
use std::collections::HashMap;
use std::path::PathBuf;

pub mod publish_one;
pub mod rate_limiter;
pub mod state;
pub mod web;

use rate_limiter::RateLimiter;

pub fn calculate_next_try_time(finish_time: DateTime<Utc>, attempt_count: usize) -> DateTime<Utc> {
    if attempt_count == 0 {
        finish_time
    } else {
        let delta = chrono::Duration::hours(2usize.pow(attempt_count as u32).min(7 * 24) as i64);

        finish_time + delta
    }
}

#[derive(Debug)]
pub enum DebdiffError {
    Http(reqwest::Error),
    MissingRun(String),
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

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct PublishOneRequest {
    pub campaign: String,
    pub target_branch_url: url::Url,
    pub role: String,
    pub log_id: String,
    pub reviewers: Option<Vec<String>>,
    pub revision_id: RevisionId,
    pub unchanged_id: String,
    #[serde(rename = "require-binary-diff")]
    pub require_binary_diff: bool,
    pub differ_url: url::Url,
    pub derived_branch_name: String,
    pub tags: Option<HashMap<String, RevisionId>>,
    pub allow_create_proposal: bool,
    pub source_branch_url: url::Url,
    pub codemod_result: serde_json::Value,
    pub commit_message_template: Option<String>,
    pub title_template: Option<String>,
    pub existing_mp_url: Option<url::Url>,
    pub extra_context: Option<serde_json::Value>,
    pub mode: Mode,
    pub command: String,
    pub external_url: Option<url::Url>,
    pub derived_owner: Option<String>,
}

#[derive(Debug)]
pub enum PublishError {
    Failure { code: String, description: String },
    NothingToDo(String),
    BranchBusy(url::Url),
}

impl PublishError {
    pub fn code(&self) -> &str {
        match self {
            PublishError::Failure { code, .. } => code,
            PublishError::NothingToDo(_) => "nothing-to-do",
            PublishError::BranchBusy(_) => "branch-busy",
        }
    }

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

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct PublishOneResult {
    proposal_url: Option<url::Url>,
    proposal_web_url: Option<url::Url>,
    is_new: Option<bool>,
    branch_name: String,
    target_branch_url: url::Url,
    target_branch_web_url: Option<url::Url>,
    mode: Mode,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct PublishOneError {
    code: String,
    description: String,
}

pub struct PublishWorker {
    pub template_env_path: Option<PathBuf>,
    pub external_url: Option<url::Url>,
    pub differ_url: url::Url,
    pub redis: Option<redis::aio::MultiplexedConnection>,
    pub lock_manager: Option<rslock::LockManager>,
}

#[derive(Debug)]
pub enum WorkerInvalidResponse {
    Io(std::io::Error),
    Serde(serde_json::Error),
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

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
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
    async fn new(
        template_env_path: Option<PathBuf>,
        external_url: Option<url::Url>,
        differ_url: url::Url,
        redis: Option<redis::Client>,
        lock_manager: Option<rslock::LockManager>,
    ) -> Self {
        let redis = if let Some(redis) = redis {
            Some(redis.get_multiplexed_async_connection().await.unwrap())
        } else {
            None
        };
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

fn run_sufficient_for_proposal(campaign_config: &Campaign, run_value: Option<i32>) -> bool {
    if let (Some(run_value), Some(threshold)) =
        (run_value, &campaign_config.merge_proposal.value_threshold)
    {
        run_value >= *threshold
    } else {
        // Assume yes, if the run doesn't have an associated value or if there is no threshold configured.
        true
    }
}

fn role_branch_url(url: &url::Url, remote_branch_name: Option<&str>) -> url::Url {
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

fn branches_match(url_a: Option<&url::Url>, url_b: Option<&url::Url>) -> bool {
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

fn get_merged_by_user_url(url: &url::Url, user: &str) -> Result<Option<url::Url>, BrzError> {
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

pub async fn process_queue_loop(
    db: &sqlx::PgPool,
    redis: &redis::aio::MultiplexedConnection,
    config: &janitor::config::Config,
    publish_worker: &PublishWorker,
    bucket_rate_limiter: &mut dyn rate_limiter::RateLimiter,
    forge_rate_limiter: &mut HashMap<Forge, chrono::DateTime<Utc>>,
    vcs_managers: Vec<Box<dyn VcsManager>>,
    interval: chrono::Duration,
    auto_publish: bool,
    push_limit: Option<i32>,
    modify_mp_limit: Option<i32>,
    require_binary_diff: bool,
) {
    todo!();
}

pub async fn publish_pending_ready(
    db: &sqlx::PgPool,
    redis: &redis::aio::MultiplexedConnection,
    config: &janitor::config::Config,
    publish_worker: &PublishWorker,
    bucket_rate_limiter: &mut dyn rate_limiter::RateLimiter,
    vcs_managers: Vec<Box<dyn VcsManager>>,
    push_limit: Option<i32>,
    require_binary_diff: bool,
) {
    todo!();
}

pub async fn refresh_bucket_mp_counts(
    db: &sqlx::PgPool,
    bucket_rate_limiter: &mut dyn rate_limiter::RateLimiter,
) {
    todo!();
}

pub async fn listen_to_runner(
    db: &sqlx::PgPool,
    redis: &redis::aio::MultiplexedConnection,
    config: &janitor::config::Config,
    publish_worker: &PublishWorker,
    bucket_rate_limiter: &mut dyn rate_limiter::RateLimiter,
    vcs_managers: Vec<Box<dyn VcsManager>>,
    require_binary_diff: bool,
) {
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
