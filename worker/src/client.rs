use crate::get_build_arch;
use backoff::ExponentialBackoff;
use janitor::api::worker::{Assignment, Metadata};
use reqwest::multipart::{Form, Part};
use reqwest::{Error as ReqwestError, Response, StatusCode, Url};
use std::error::Error;
use std::path::Path;

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
    my_url: Option<&Url>,
    base_url: &Url,
    node_name: &str,
    jenkins_build_url: Option<&Url>,
    codebase: Option<&str>,
    campaign: Option<&str>,
) -> Result<serde_json::Value, AssignmentError> {
    let assign_url = base_url
        .join("active-runs")
        .map_err(|e| AssignmentError::Failure(format!("Failed to build assignment URL: {}", e)))?;
    let build_arch = match get_build_arch() {
        Ok(arch) => arch,
        Err(e) => {
            return Err(AssignmentError::Failure(format!(
                "Failed to get build arch: {}",
                e
            )))
        }
    };
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
    if let Some(url) = jenkins_build_url.or(my_url) {
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
        log::debug!("Sending assignment request: {:?}", json);
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
    log::debug!("Got assignment: {:?}", assignment);
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
    my_url: Option<&Url>,
    base_url: &Url,
    node_name: &str,
    jenkins_build_url: Option<&Url>,
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

    pub fn from_url(url: &Url) -> Self {
        let mut credentials = Credentials::None;
        if !url.username().is_empty() {
            let password = url.password().map(|p| p.to_string());
            credentials = Credentials::Basic {
                username: url.username().to_string(),
                password,
            };
        }
        credentials
    }
}

impl Client {
    pub fn new(
        base_url: Url,
        credentials: Credentials,
        user_agent: &str,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let mut builder = reqwest::Client::builder();
        builder = builder.user_agent(user_agent);
        let client = builder
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
        Ok(Self {
            client,
            base_url,
            credentials,
        })
    }

    pub async fn get_assignment(
        &self,
        my_url: Option<&Url>,
        node_name: &str,
        jenkins_build_url: Option<&Url>,
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
        my_url: Option<&Url>,
        node_name: &str,
        jenkins_build_url: Option<&Url>,
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
                let file_path = entry.path();
                let file_name = entry.file_name().to_string_lossy().into_owned();

                // Always use streaming - more memory efficient and simpler code
                let file = tokio::fs::File::open(&file_path).await?;
                let file_size = file.metadata().await?.len();
                let stream = tokio_util::io::ReaderStream::new(file);
                let body = reqwest::Body::wrap_stream(stream);

                let part = Part::stream_with_length(body, file_size)
                    .file_name(file_name)
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
        let builder = client.post(finish_url).timeout(std::time::Duration::from_secs(60));
        let builder = credentials.set_credentials(builder);
        if let Ok(metadata_str) = serde_json::to_string(metadata) {
            log::debug!("Uploading results: {}", metadata_str);
        } else {
            log::debug!("Uploading results: <serialization failed>");
        }
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
    metadata.finish_time = Some(chrono::Utc::now());

    match client.upload_results(run_id, &metadata, None).await {
        Ok(_) => {}
        Err(e) => {
            log::warn!("Result upload for abort of {} failed: {}", run_id, e);
        }
    }
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod client_tests;
