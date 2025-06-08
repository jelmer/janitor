//! API for the worker to communicate with the runner.

use crate::vcs::VcsType;
use breezyshim::RevisionId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

/// Build metadata as produced by the worker.
#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq, Eq)]
pub struct Metadata {
    /// The ID of the item in the runner's queue.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queue_id: Option<u64>,
    /// Campaign name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub campaign: Option<String>,
    /// Error code, if any. None for success.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// Description of the result.
    pub description: Option<String>,

    /// Start time of the run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,

    /// Finish time of the run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_time: Option<chrono::DateTime<chrono::Utc>>,

    /// The command that was run.
    pub command: Option<Vec<String>>,

    /// Name of the codebase.
    pub codebase: Option<String>,

    /// VCS Type
    pub vcs_type: Option<VcsType>,

    /// URL of the branch.
    pub branch_url: Option<Url>,

    /// Subpath of the branch.
    pub subpath: Option<String>,

    /// Revision ID of the base branch.
    pub main_branch_revision: Option<RevisionId>,

    /// Revision ID of the resulting branch.
    pub revision: Option<RevisionId>,

    /// Codemod result.
    pub codemod: Option<serde_json::Value>,

    /// Remotes.
    pub remotes: HashMap<String, Remote>,

    /// Whether the branch was refreshed, i.e. old resume branch was discarded.
    pub refreshed: Option<bool>,

    /// Value of the branch.
    pub value: Option<u64>,

    /// Target branch URL.
    pub target_branch_url: Option<Url>,

    /// Branches that were created.
    pub branches: Vec<(
        String,
        Option<String>,
        Option<RevisionId>,
        Option<RevisionId>,
    )>,

    /// Tags that were created.
    pub tags: Vec<(String, RevisionId)>,
    #[serde(skip_serializing_if = "Option::is_none")]

    /// Details about the target.
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
        self.failure_details.clone_from(&failure.details);
        self.stage = Some(failure.stage.join("/"));
        self.transient = failure.transient;
    }

    pub fn add_branch(
        &mut self,
        function: String,
        name: String,
        base_revision: Option<RevisionId>,
        revision: Option<RevisionId>,
    ) {
        self.branches
            .push((function, Some(name), base_revision, revision));
    }

    pub fn add_tag(&mut self, name: String, revision: Option<RevisionId>) {
        if let Some(rev) = revision {
            self.tags.push((name, rev));
        }
    }

    pub fn add_remote(&mut self, name: String, url: Url) {
        self.remotes.insert(name, Remote { url });
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Remote {
    pub url: Url,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct WorkerFailure {
    pub code: String,
    pub description: String,
    pub details: Option<serde_json::Value>,
    pub stage: Vec<String>,
    pub transient: Option<bool>,
}

impl std::fmt::Display for WorkerFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.description)
    }
}

impl std::error::Error for WorkerFailure {}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct TargetDetails {
    pub name: String,
    pub details: serde_json::Value,
}

impl TargetDetails {
    pub fn new(name: String, details: serde_json::Value) -> Self {
        Self { name, details }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Codemod {
    pub command: String,
    pub environment: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Build {
    pub target: String,
    pub config: serde_json::Value,
    pub environment: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Branch {
    pub cached_url: Option<Url>,
    pub vcs_type: VcsType,

    // The URL for the branch. Note that this can be None for nascent branches.
    pub url: Option<Url>,

    /// Path inside of the branch
    pub subpath: std::path::PathBuf,

    /// Any additional branches that are colocated with this branch that should be checked out.
    pub additional_colocated_branches: Option<Vec<String>>,

    #[serde(rename = "default-empty")]
    pub default_empty: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TargetRepository {
    pub url: Url,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResumeBranch {
    pub result: serde_json::Value,
    pub branch_url: Url,
    pub branches: Vec<(String, String, Option<RevisionId>, Option<RevisionId>)>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Assignment {
    pub id: String,
    pub queue_id: u64,
    pub campaign: String,
    pub codebase: String,
    #[serde(rename = "force-build")]
    pub force_build: bool,
    pub branch: Branch,
    pub resume: Option<ResumeBranch>,
    pub target_repository: TargetRepository,
    #[serde(rename = "skip-setup-validation")]
    pub skip_setup_validation: bool,
    pub codemod: Codemod,
    /// Environment used for both the build and the codemod.
    pub env: HashMap<String, String>,
    pub build: Build,
}

#[derive(serde::Deserialize, serde::Serialize, Default)]
pub struct GenericBuildConfig {
    pub chroot: Option<String>,
    pub dep_server_url: Option<url::Url>,
}

#[cfg(feature = "debian")]
#[derive(serde::Deserialize, serde::Serialize, Debug, Default)]
pub struct LintianConfig {
    pub profile: Option<String>,
    #[serde(rename = "suppress-tags")]
    pub suppress_tags: Option<Vec<String>>,
}

#[cfg(feature = "debian")]
#[derive(serde::Deserialize, serde::Serialize, Debug, Default)]
pub struct DebianBuildConfig {
    #[serde(rename = "build-distribution")]
    pub build_distribution: Option<String>,
    #[serde(rename = "build-command")]
    pub build_command: Option<String>,
    #[serde(rename = "build-suffix")]
    pub build_suffix: Option<String>,
    #[serde(rename = "last-build-version")]
    pub last_build_version: Option<debversion::Version>,
    pub chroot: Option<String>,
    pub lintian: LintianConfig,
    #[serde(rename = "base-apt-repository")]
    pub apt_repository: Option<String>,
    #[serde(rename = "base-apt-repository-signed-by")]
    pub apt_repository_key: Option<String>,
    #[serde(rename = "build-extra-repositories")]
    pub extra_repositories: Option<Vec<String>>,
    #[serde(rename = "dep_server_url")]
    pub dep_server_url: Option<String>,
}
