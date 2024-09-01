use breezyshim::RevisionId;
use chrono::{DateTime, Utc};
use reqwest::header::HeaderMap;
use serde::ser::SerializeStruct;
use std::collections::HashMap;

pub mod publish_one;

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

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    #[serde(rename = "attempt-push")]
    AttemptPush,
    #[serde(rename = "push-derived")]
    PushDerived,
    #[serde(rename = "propose")]
    Propose,
    #[serde(rename = "push")]
    Push,
    #[serde(rename = "build-only")]
    BuildOnly,
    #[serde(rename = "skip")]
    Skip,
    #[serde(rename = "bts")]
    BTS,
}

impl TryFrom<Mode> for silver_platter::Mode {
    type Error = String;

    fn try_from(value: Mode) -> Result<Self, Self::Error> {
        match value {
            Mode::PushDerived => Ok(silver_platter::Mode::PushDerived),
            Mode::Propose => Ok(silver_platter::Mode::Propose),
            Mode::Push => Ok(silver_platter::Mode::Push),
            Mode::BuildOnly => Err("Mode::BuildOnly is not supported".to_string()),
            Mode::Skip => Err("Mode::Skip is not supported".to_string()),
            Mode::BTS => Err("Mode::BTS is not supported".to_string()),
            Mode::AttemptPush => Ok(silver_platter::Mode::AttemptPush),
        }
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
    pub commit_message_tempalte: Option<String>,
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
}

impl PublishError {
    pub fn code(&self) -> &str {
        match self {
            PublishError::Failure { code, .. } => code,
            PublishError::NothingToDo(_) => "nothing-to-do",
        }
    }

    pub fn description(&self) -> &str {
        match self {
            PublishError::Failure { description, .. } => description,
            PublishError::NothingToDo(description) => description,
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
        }
    }
}

impl std::error::Error for PublishError {}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct PublishResult {
    proposal_url: Option<url::Url>,
    proposal_web_url: Option<url::Url>,
    is_new: Option<bool>,
    branch_name: String,
    target_branch_url: url::Url,
    target_branch_web_url: Option<url::Url>,
    mode: Mode,
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
