use backoff::ExponentialBackoff;
use log::debug;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Error as ReqwestError, Response, StatusCode, Url};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::io::Read;
use std::net::IpAddr;
use std::path::Path;
use tokio::io::AsyncReadExt;
use tokio::net::lookup_host;
use tokio::time::Duration;

use reqwest::multipart::{Form, Part};

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

#[derive(Debug, Serialize, Deserialize)]
pub struct Assignment {
    id: String,
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
        metadata: &serde_json::Value,
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
    metadata: &'a serde_json::Value,
    directory: Option<&'a Path>,
) -> Result<Form, Box<dyn Error + Send + Sync>> {
    let mut form = Form::new();

    let json_part = Part::text(metadata.to_string())
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

pub async fn upload_results(
    client: &reqwest::Client,
    credentials: &Credentials,
    base_url: &Url,
    run_id: &str,
    metadata: &serde_json::Value,
    output_directory: Option<&Path>,
) -> Result<serde_json::Value, UploadFailure> {
    backoff::future::retry(ExponentialBackoff::default(), || async {
        let finish_url = base_url
            .join(&format!("active-runs/{}/finish", run_id))
            .map_err(|e| UploadFailure(format!("Error building finish URL: {}", e)))?;
        let builder = client.post(finish_url).timeout(Duration::from_secs(60));
        let builder = credentials.set_credentials(builder);
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
                Err(backoff::Error::transient(UploadFailure(format!(
                    "ResultUploadFailure: Unable to submit result: {}: {}",
                    text, status
                ))))
            }
        }
    }).await
}
