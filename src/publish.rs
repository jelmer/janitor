use chrono::Duration;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use url::Url;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum MergeProposalStatus {
    #[serde(rename = "open")]
    Open,
    #[serde(rename = "merged")]
    Merged,
    #[serde(rename = "closed")]
    Closed,
}

#[derive(Serialize, Deserialize)]
pub struct MergeProposalNotification {
    pub url: Url,
    pub web_url: Option<Url>,
    pub rate_limit_bucket: Option<String>,
    pub status: MergeProposalStatus,
    pub merged_by: Option<String>,
    pub merged_by_url: Option<Url>,
    pub merged_at: Option<String>,
    pub codebase: String,
    pub campaign: String,
    pub target_branch_url: Url,
    pub target_branch_web_url: Option<Url>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum PublishMode {
    #[serde(rename = "skip")]
    Skip,
    #[serde(rename = "build-only")]
    BuildOnly,
    #[serde(rename = "push")]
    Push,
    #[serde(rename = "push-derived")]
    PushDerived,
    #[serde(rename = "propose")]
    Propose,
    #[serde(rename = "attempt-push")]
    AttemptPush,
    #[serde(rename = "bts")]
    Bts,
}

fn serialize_duration<S>(duration: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if let Some(d) = duration {
        serializer.serialize_f64(d.num_seconds() as f64)
    } else {
        serializer.serialize_none()
    }
}

fn deserialize_duration<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
where
    D: Deserializer<'de>,
{
    if let Some(d) = Option::<f64>::deserialize(deserializer)? {
        Ok(Some(Duration::seconds(d as i64)))
    } else {
        Ok(None)
    }
}

#[derive(Serialize, Deserialize)]
pub struct PublishNotification {
    pub id: String,
    pub codebase: String,
    pub campaign: String,
    pub proposal_url: Option<Url>,
    pub mode: PublishMode,
    pub main_branch_url: Option<Url>,
    pub main_branch_web_url: Option<Url>,
    pub branch_name: Option<String>,
    pub result_code: String,
    pub result: serde_json::Value,
    pub run_id: String,
    #[serde(
        serialize_with = "serialize_duration",
        deserialize_with = "deserialize_duration"
    )]
    pub publish_delay: Option<Duration>,
}
