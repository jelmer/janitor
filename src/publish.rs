use chrono::Duration;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use url::Url;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy, std::hash::Hash)]
pub enum MergeProposalStatus {
    #[serde(rename = "open")]
    Open,
    #[serde(rename = "merged")]
    Merged,
    #[serde(rename = "applied")]
    Applied,
    #[serde(rename = "closed")]
    Closed,
}

impl std::fmt::Display for MergeProposalStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            MergeProposalStatus::Open => write!(f, "open"),
            MergeProposalStatus::Merged => write!(f, "merged"),
            MergeProposalStatus::Applied => write!(f, "applied"),
            MergeProposalStatus::Closed => write!(f, "closed"),
        }
    }
}

impl std::str::FromStr for MergeProposalStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "open" => Ok(MergeProposalStatus::Open),
            "merged" => Ok(MergeProposalStatus::Merged),
            "applied" => Ok(MergeProposalStatus::Applied),
            "closed" => Ok(MergeProposalStatus::Closed),
            _ => Err(format!("Unknown merge proposal status: {}", s)),
        }
    }
}

impl From<breezyshim::forge::MergeProposalStatus> for MergeProposalStatus {
    fn from(status: breezyshim::forge::MergeProposalStatus) -> Self {
        match status {
            breezyshim::forge::MergeProposalStatus::Open => MergeProposalStatus::Open,
            breezyshim::forge::MergeProposalStatus::Merged => MergeProposalStatus::Merged,
            breezyshim::forge::MergeProposalStatus::Closed => MergeProposalStatus::Closed,
            breezyshim::forge::MergeProposalStatus::All => unreachable!(),
        }
    }
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

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
pub enum Mode {
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

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Mode::PushDerived => write!(f, "push-derived"),
            Mode::Propose => write!(f, "propose"),
            Mode::Push => write!(f, "push"),
            Mode::BuildOnly => write!(f, "build-only"),
            Mode::Skip => write!(f, "skip"),
            Mode::Bts => write!(f, "bts"),
            Mode::AttemptPush => write!(f, "attempt-push"),
        }
    }
}

impl std::str::FromStr for Mode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "push-derived" => Ok(Mode::PushDerived),
            "propose" => Ok(Mode::Propose),
            "push" => Ok(Mode::Push),
            "build-only" => Ok(Mode::BuildOnly),
            "skip" => Ok(Mode::Skip),
            "bts" => Ok(Mode::Bts),
            "attempt-push" => Ok(Mode::AttemptPush),
            _ => Err(format!("Unknown mode: {}", s)),
        }
    }
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
            Mode::Bts => Err("Mode::BTS is not supported".to_string()),
            Mode::AttemptPush => Ok(silver_platter::Mode::AttemptPush),
        }
    }
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
    pub mode: Mode,
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
