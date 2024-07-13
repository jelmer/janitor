use backoff::ExponentialBackoff;

pub use breezyshim::RevisionId;
use log::debug;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::multipart::{Form, Part};
use reqwest::{Error as ReqwestError, Response, StatusCode, Url};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;

use std::net::IpAddr;
use std::path::Path;
use tokio::io::AsyncReadExt;
use tokio::net::lookup_host;
use tokio::time::Duration;

pub const DEFAULT_USER_AGENT: &str = concat!("janitor/worker (", env!("CARGO_PKG_VERSION"), ")");

#[cfg(feature = "debian")]
pub mod debian;

pub mod generic;

pub async fn is_gce_instance() -> bool {
    match lookup_host("metadata.google.internal").await {
        Ok(lookup_result) => {
            for addr in lookup_result {
                if let IpAddr::V4(ipv4) = addr.ip() {
                    if ipv4.is_private() {
                        return true;
                    }
                }
            }
            false
        }
        Err(_) => false,
    }
}

pub async fn gce_external_ip() -> Result<Option<String>, reqwest::Error> {
    let url = "http://metadata.google.internal/computeMetadata/v1/instance/network-interfaces/0/access-configs/0/external-ip";
    let mut headers = HeaderMap::new();
    headers.insert("Metadata-Flavor", HeaderValue::from_static("Google"));

    let client = reqwest::Client::new();
    let resp = client.get(url).headers(headers).send().await?;

    match resp.status().as_u16() {
        200 => Ok(Some(resp.text().await?)),
        404 => Ok(None),
        _ => panic!("Unexpected response status: {}", resp.status()),
    }
}

pub fn get_build_arch() -> String {
    String::from_utf8(
        std::process::Command::new("dpkg-architecture")
            .arg("-qDEB_BUILD_ARCH")
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .to_owned()
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerFailure {
    pub code: String,
    pub description: String,
    pub details: Option<serde_json::Value>,
    pub stage: Vec<String>,
    pub transient: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Codemod {
    pub command: String,
    pub environment: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Build {
    pub target: String,
    pub config: HashMap<String, String>,
    pub environment: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Branch {
    pub cached_url: Option<Url>,
    pub vcs_type: String,
    pub url: Url,
    pub subpath: Option<String>,
    pub additional_colocated_branches: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TargetRepository {
    pub url: Url,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Assignment {
    pub id: String,
    pub queue_id: u64,
    pub campaign: String,
    pub codebase: String,
    #[serde(rename = "force-build")]
    pub force_build: bool,
    pub branch: Branch,
    pub resume: Option<Branch>,
    pub target_repository: TargetRepository,
    #[serde(rename = "skip-setup-validation")]
    pub skip_setup_validation: bool,
    #[serde(rename = "default-empty")]
    pub default_empty: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Remote {
    pub url: Url,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TargetDetails {
    pub name: String,
    pub details: serde_json::Value,
}

impl TargetDetails {
    pub fn new(name: String, details: serde_json::Value) -> Self {
        Self { name, details }
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Metadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queue_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub campaign: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<chrono::NaiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_time: Option<chrono::NaiveDateTime>,
    pub command: Option<Vec<String>>,
    pub codebase: Option<String>,
    pub vcs_type: Option<String>,
    pub branch_url: Option<Url>,
    pub subpath: Option<String>,
    pub main_branch_revision: Option<RevisionId>,
    pub revision: Option<RevisionId>,
    pub codemod: Option<serde_json::Value>,
    pub remotes: HashMap<String, Remote>,
    pub refreshed: Option<bool>,
    pub value: Option<u64>,
    pub target_branch_url: Option<Url>,
    pub branches: Vec<(
        String,
        Option<String>,
        Option<RevisionId>,
        Option<RevisionId>,
    )>,
    pub tags: Vec<(String, RevisionId)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetDetails>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "details")]
    pub failure_details: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transient: Option<bool>,
}

impl Metadata {
    pub fn update(&mut self, failure: &WorkerFailure) {
        self.code = Some(failure.code.clone());
        self.description = Some(failure.description.clone());
        self.failure_details = failure.details.clone();
        self.stage = Some(failure.stage.join("/"));
        self.transient = failure.transient;
    }
}

#[derive(Debug)]
pub enum AssignmentError {
    Failure(String),
    EmptyQueue,
}

impl std::fmt::Display for AssignmentError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AssignmentError::Failure(msg) => write!(f, "AssignmentError: {}", msg),
            AssignmentError::EmptyQueue => write!(f, "AssignmentError: EmptyQueue"),
        }
    }
}

impl std::error::Error for AssignmentError {}

/// Get an assignment from the queue.
pub async fn get_assignment_raw(
    session: &reqwest::Client,
    credentials: &Credentials,
    my_url: Option<Url>,
    base_url: &Url,
    node_name: &str,
    jenkins_build_url: Option<&str>,
    codebase: Option<&str>,
    campaign: Option<&str>,
) -> Result<serde_json::Value, AssignmentError> {
    let assign_url = base_url
        .join("active-runs")
        .map_err(|e| AssignmentError::Failure(format!("Failed to build assignment URL: {}", e)))?;
    let build_arch = get_build_arch();
    let mut json = serde_json::json!({
        "node": node_name,
        "archs": [build_arch],
        "worker_link": null,
        "codebase": codebase,
        "campaign": campaign,
    });
    json["backchannel"] = if let Some(ref url) = my_url {
        serde_json::json!({
            "kind": "http",
            "url": url.to_string(),
        })
    } else if let Some(url) = jenkins_build_url {
        serde_json::json!({
            "kind": "jenkins",
            "url": url,
        })
    } else {
        serde_json::json![null]
    };
    if let Some(url) = jenkins_build_url.or_else(|| my_url.as_ref().map(|u| u.as_str())) {
        json["worker_link"] = serde_json::Value::String(url.to_string());
    }
    async fn send_assignment_request(
        session: reqwest::Client,
        assign_url: Url,
        credentials: &Credentials,
        json: &serde_json::Value,
    ) -> Result<Response, ReqwestError> {
        let mut builder = session.post(assign_url);
        builder = credentials.set_credentials(builder);
        builder = builder.header("Content-Type", "application/json");
        debug!("Sending assignment request: {:?}", json);
        builder = builder.json(json);
        builder.send().await
    }
    let assignment = backoff::future::retry(ExponentialBackoff::default(), || async {
        let session = session.clone();
        let assign_url = assign_url.clone();
        match send_assignment_request(session, assign_url, credentials, &json).await {
            Ok(resp) => {
                if resp.status().as_u16() == StatusCode::CREATED {
                    let data = resp.json::<serde_json::Value>().await.map_err(|e| {
                        backoff::Error::Permanent(AssignmentError::Failure(format!(
                            "Failed to parse assignment response: {}",
                            e
                        )))
                    })?;
                    Ok(data)
                } else if resp.status().as_u16() == 503 {
                    let json = resp.json::<serde_json::Value>().await.map_err(|e| {
                        backoff::Error::Permanent(AssignmentError::Failure(format!(
                            "Failed to parse assignment response: {}",
                            e
                        )))
                    })?;
                    if json["reason"] == "queue empty" {
                        Err(backoff::Error::Permanent(AssignmentError::EmptyQueue))
                    } else {
                        Err(backoff::Error::transient(AssignmentError::Failure(
                            format!("Failed to get assignment: {:?}", json),
                        )))
                    }
                } else if resp.status().is_server_error() {
                    Err(backoff::Error::transient(AssignmentError::Failure(
                        format!("Failed to get assignment: {:?}", resp),
                    )))
                } else {
                    let data = resp.text().await.map_err(|e| {
                        backoff::Error::permanent(AssignmentError::Failure(format!(
                            "Failed to read assignment response: {}",
                            e
                        )))
                    })?;
                    Err(backoff::Error::permanent(AssignmentError::Failure(data)))
                }
            }
            Err(e) => Err(backoff::Error::transient(AssignmentError::Failure(
                e.to_string(),
            ))),
        }
    })
    .await?;
    debug!("Got assignment: {:?}", assignment);
    Ok(assignment)
}

/// Get an assignment from the queue.
///
/// # Arguments
/// * `session` - A reqwest session.
/// * `credentials` - Credentials for the server.
/// * `my_url` - The URL of the worker.
/// * `base_url` - The base URL of the server.
/// * `node_name` - The name of the node.
/// * `jenkins_build_url` - The URL of the Jenkins build.
/// * `codebase` - Request an assignment for a specific codebase.
/// * `campaign` - Request an assignment for a specific campaign.
///
/// # Returns
///
/// An assignment.
pub async fn get_assignment(
    session: &reqwest::Client,
    credentials: &Credentials,
    my_url: Option<Url>,
    base_url: &Url,
    node_name: &str,
    jenkins_build_url: Option<&str>,
    codebase: Option<&str>,
    campaign: Option<&str>,
) -> Result<Assignment, AssignmentError> {
    let assignment = get_assignment_raw(
        session,
        credentials,
        my_url,
        base_url,
        node_name,
        jenkins_build_url,
        codebase,
        campaign,
    )
    .await?;
    serde_json::from_value(assignment)
        .map_err(|e| AssignmentError::Failure(format!("Failed to parse assignment: {}", e)))
}

pub struct Client {
    client: reqwest::Client,
    base_url: Url,
    credentials: Credentials,
}

pub enum Credentials {
    None,
    Bearer {
        token: String,
    },
    Basic {
        username: String,
        password: Option<String>,
    },
}

impl Credentials {
    fn set_credentials(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match self {
            Credentials::None => builder,
            Credentials::Basic { username, password } => {
                builder.basic_auth(username, password.as_ref())
            }
            Credentials::Bearer { token } => builder.bearer_auth(token),
        }
    }
}

impl Client {
    pub fn new(base_url: Url, credentials: Credentials, user_agent: &str) -> Self {
        let mut builder = reqwest::Client::builder();
        builder = builder.user_agent(user_agent);
        Self {
            client: builder.build().unwrap(),
            base_url,
            credentials,
        }
    }

    pub async fn get_assignment(
        &self,
        my_url: Option<Url>,
        node_name: &str,
        jenkins_build_url: Option<&str>,
        codebase: Option<&str>,
        campaign: Option<&str>,
    ) -> Result<Assignment, AssignmentError> {
        get_assignment(
            &self.client,
            &self.credentials,
            my_url,
            &self.base_url,
            node_name,
            jenkins_build_url,
            codebase,
            campaign,
        )
        .await
    }

    pub async fn get_assignment_raw(
        &self,
        my_url: Option<Url>,
        node_name: &str,
        jenkins_build_url: Option<&str>,
        codebase: Option<&str>,
        campaign: Option<&str>,
    ) -> Result<serde_json::Value, AssignmentError> {
        get_assignment_raw(
            &self.client,
            &self.credentials,
            my_url,
            &self.base_url,
            node_name,
            jenkins_build_url,
            codebase,
            campaign,
        )
        .await
    }

    pub async fn upload_results(
        &self,
        run_id: &str,
        metadata: &Metadata,
        output_directory: Option<&Path>,
    ) -> Result<serde_json::Value, UploadFailure> {
        upload_results(
            &self.client,
            &self.credentials,
            &self.base_url,
            run_id,
            metadata,
            output_directory,
        )
        .await
    }
}

pub async fn bundle_results<'a>(
    metadata: &'a Metadata,
    directory: Option<&'a Path>,
) -> Result<Form, Box<dyn Error + Send + Sync>> {
    let mut form = Form::new();

    let json_part = Part::text(serde_json::to_string(metadata)?)
        .file_name("result.json")
        .mime_str("application/json")?;
    form = form.part("metadata", json_part);

    if let Some(directory) = directory {
        let mut dir = tokio::fs::read_dir(directory).await?;

        while let Some(entry) = dir.next_entry().await? {
            if entry.file_type().await?.is_file() {
                let mut file = tokio::fs::File::open(entry.path()).await?;
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer).await?;

                let part = Part::bytes(buffer)
                    .file_name(entry.file_name().to_string_lossy().into_owned())
                    .mime_str("application/octet-stream")?;
                form = form.part("file", part);
            }
        }
    }

    Ok(form)
}

#[derive(Debug)]
pub struct UploadFailure(String);

impl std::fmt::Display for UploadFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for UploadFailure {}

pub async fn upload_results(
    client: &reqwest::Client,
    credentials: &Credentials,
    base_url: &Url,
    run_id: &str,
    metadata: &Metadata,
    output_directory: Option<&Path>,
) -> Result<serde_json::Value, UploadFailure> {
    backoff::future::retry(ExponentialBackoff::default(), || async {
        let finish_url = base_url
            .join(&format!("active-runs/{}/finish", run_id))
            .map_err(|e| UploadFailure(format!("Error building finish URL: {}", e)))?;
        log::info!("Uploading results to {}", &finish_url);
        let builder = client.post(finish_url).timeout(Duration::from_secs(60));
        let builder = credentials.set_credentials(builder);
        log::debug!("Uploading results: {}", serde_json::to_string(metadata).unwrap());
        let bundle: Form = bundle_results(metadata, output_directory)
            .await
            .map_err(|e| {
                backoff::Error::permanent(UploadFailure(format!("Error creating multipart: {}", e)))
            })?;
        let response = builder.multipart(bundle).send().await.map_err(|e| {
            backoff::Error::permanent(UploadFailure(format!("Error creating multipart: {}", e)))
        })?;

        match response.status() {
            StatusCode::NOT_FOUND => {
                let resp_json = response.json::<serde_json::Value>().await.map_err(|e| {
                    backoff::Error::permanent(UploadFailure(format!(
                        "Error parsing 404 response: {}",
                        e
                    )))
                })?;
                let reason = resp_json
                    .get("reason")
                    .and_then(|r| r.as_str())
                    .unwrap_or("Runner returned 404");
                Err(backoff::Error::permanent(
                    UploadFailure(reason.to_string()),
                ))
            }
            StatusCode::BAD_GATEWAY | StatusCode::SERVICE_UNAVAILABLE => {
                let status = response.status();
                let text = response.text().await.map_err(|e| {
                    backoff::Error::transient(UploadFailure(format!(
                        "Error reading response text for {}: {}",
                        status,
                        e
                    )))
                })?;
                Err(backoff::Error::transient(UploadFailure(format!(
                    "RetriableResultUploadFailure: Unable to submit result: {}: {}",
                    text, status
                ))))
            }
            StatusCode::OK | StatusCode::CREATED => {
                let json = response.json::<serde_json::Value>().await.map_err(|e| {
                    backoff::Error::permanent(UploadFailure(format!(
                        "Error parsing response: {}",
                        e
                    )))
                })?;
                if let Some(output_directory) = output_directory {
                    let mut local_filenames: std::collections::HashSet<_> =
                        std::collections::HashSet::new();
                    let mut read_dir =
                        tokio::fs::read_dir(output_directory).await.map_err(|e| {
                            backoff::Error::permanent(UploadFailure(format!(
                                "Error reading output directory: {}",
                                e
                            )))
                        })?;
                    while let Some(entry) = read_dir.next_entry().await.map_err(|e| {
                        backoff::Error::permanent(UploadFailure(format!(
                            "Error reading output directory: {}",
                            e
                        )))
                    })? {
                        let file_type = entry.file_type().await.map_err(|e| {
                            backoff::Error::permanent(UploadFailure(format!(
                                "Error reading output directory: {}",
                                e
                            )))
                        })?;
                        if file_type.is_file() {
                            local_filenames.insert(entry.file_name().to_string_lossy().to_string());
                        }
                    }

                    let runner_filenames: std::collections::HashSet<_> = json
                        .get("filenames")
                        .and_then(|f| f.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|name| name.as_str())
                                .map(|name| name.to_owned())
                                .collect()
                        })
                        .unwrap_or_else(std::collections::HashSet::new);

                    if local_filenames != runner_filenames {
                        log::warn!(
                        "Difference between local filenames and runner reported filenames: {:?} != {:?}",
                        local_filenames,
                        runner_filenames
                    );
                    }
                }

                Ok(json)
            }
            _ => {
                let status = response.status();
                let text = response.text().await.map_err(|e| {
                    backoff::Error::permanent(UploadFailure(format!(
                        "Error reading response text for {}: {}",
                        status,
                        e
                    )))
                })?;
                log::warn!("Error uploading results: {}: {}", status, text);
                Err(backoff::Error::transient(UploadFailure(format!(
                    "ResultUploadFailure: Unable to submit result: {}: {}",
                    text, status
                ))))
            }
        }
    }).await
}

pub async fn abort_run(client: &Client, run_id: &str, metadata: &Metadata, description: &str) {
    let mut metadata: Metadata = metadata.clone();
    metadata.code = Some("aborted".to_string());
    metadata.description = Some(description.to_string());
    metadata.finish_time = Some(chrono::Utc::now().naive_utc());

    match client.upload_results(run_id, &metadata, None).await {
        Ok(_) => {}
        Err(e) => {
            log::warn!("Result upload for abort of {} failed: {}", run_id, e);
        }
    }
}

pub fn convert_codemod_script_failed(i: i32, command: &str) -> WorkerFailure {
    match i {
        127 => WorkerFailure {
            code: "command-not-found".to_string(),
            description: format!("Command {} not found", command),
            details: None,
            stage: vec!["codemod".to_string()],
            transient: None,
        },
        137 => WorkerFailure {
            code: "killed".to_string(),
            description: "Process was killed (by OOM killer?)".to_string(),
            details: None,
            stage: vec!["codemod".to_string()],
            transient: None,
        },
        _ => WorkerFailure {
            code: "command-failed".to_string(),
            description: format!("Script {} failed to run with code {}", command, i),
            details: None,
            stage: vec!["codemod".to_string()],
            transient: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WorkerFailure;

    #[test]
    fn test_convert_codemod_script_failed() {
        assert_eq!(
            convert_codemod_script_failed(127, "foobar"),
            WorkerFailure {
                code: "command-not-found".to_string(),
                description: "Command foobar not found".to_string(),
                stage: vec!["codemod".to_string()],
                details: None,
                transient: None,
            }
        );
        assert_eq!(
            convert_codemod_script_failed(137, "foobar"),
            WorkerFailure {
                code: "killed".to_string(),
                description: "Process was killed (by OOM killer?)".to_string(),
                stage: vec!["codemod".to_string()],

                details: None,
                transient: None,
            }
        );
        assert_eq!(
            convert_codemod_script_failed(1, "foobar"),
            WorkerFailure {
                code: "command-failed".to_string(),
                description: "Script foobar failed to run with code 1".to_string(),
                stage: vec!["codemod".to_string()],
                details: None,
                transient: None,
            }
        );
    }
}

#[derive(Debug, Default)]
enum DebUpdateChangelog {
    #[default]
    Auto,
    Update,
    Leave,
}

impl std::str::FromStr for DebUpdateChangelog {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "auto" => Ok(DebUpdateChangelog::Auto),
            "update" => Ok(DebUpdateChangelog::Update),
            "leave" => Ok(DebUpdateChangelog::Leave),
            _ => Err(format!("Invalid value for deb-update-changelog: {}", s)),
        }
    }
}

impl std::fmt::Display for DebUpdateChangelog {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DebUpdateChangelog::Auto => write!(f, "auto"),
            DebUpdateChangelog::Update => write!(f, "update"),
            DebUpdateChangelog::Leave => write!(f, "leave"),
        }
    }
}
