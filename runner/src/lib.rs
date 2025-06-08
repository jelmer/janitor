//! Runner crate for the Janitor project.
//!
//! This crate provides functionality for running code quality checks and tests.

#![deny(missing_docs)]

use breezyshim::RevisionId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

// Re-export VcsInfo from the main crate to avoid duplication
pub use janitor::queue::VcsInfo;

// Re-export builder types
pub use builder::{
    get_builder, Builder, BuilderError, CampaignConfig, DebianBuildConfig, DebianBuilder,
    DistroConfig, GenericBuildConfig, GenericBuilder,
};

// Re-export backchannel types
pub use backchannel::{
    Backchannel as BackchannelTrait, Error as BackchannelError, HealthStatus, JenkinsBackchannel,
    PollingBackchannel,
};

// Re-export watchdog types
pub use watchdog::{RunHealthStatus, TerminationReason, Watchdog, WatchdogConfig, WatchdogStats};

/// Module for application initialization and orchestration.
pub mod application;
/// Module for worker authentication and security.
pub mod auth;
/// Module for handling backchannel communication with the worker.
pub mod backchannel;
/// Module for build system implementations.
pub mod builder;
/// Module for comprehensive configuration management.
pub mod config;
/// Module for generating configuration files.
pub mod config_generator;
/// Module for database operations.
pub mod database;
/// Module for comprehensive error tracking and logging.
pub mod error_tracking;
/// Module for log file management.
pub mod logs;
/// Module for Prometheus metrics collection.
pub mod metrics;
/// Module for performance monitoring and system health tracking.
pub mod performance;
/// Module for database resume logic for interrupted runs.
pub mod resume;
/// Module for production-ready logging and tracing integration.
pub mod tracing;
/// Module for handling file uploads and multipart forms.
pub mod upload;
/// Module for VCS integration and coordination.
pub mod vcs;
/// Module for monitoring active runs.
pub mod watchdog;
/// Module for the web interface.
pub mod web;

/// Test utilities for the runner module.
pub mod test_utils;

/// Generate environment variables for committing changes.
///
/// # Arguments
/// * `committer` - Optional committer string in the format "Name <email>"
///
/// # Returns
/// A HashMap containing environment variables for committing
pub fn committer_env(committer: Option<&str>) -> HashMap<String, String> {
    let mut env = HashMap::new();
    if let Some(committer) = committer {
        let (user, email) = breezyshim::config::parse_username(committer);
        if !user.is_empty() {
            env.insert("DEBFULLNAME".to_string(), user.to_string());
            env.insert("GIT_COMMITTER_NAME".to_string(), user.to_string());
            env.insert("GIT_AUTHOR_NAME".to_string(), user.to_string());
        }
        if !email.is_empty() {
            env.insert("DEBEMAIL".to_string(), email.to_string());
            env.insert("GIT_COMMITTER_EMAIL".to_string(), email.to_string());
            env.insert("GIT_AUTHOR_EMAIL".to_string(), email.to_string());
            env.insert("EMAIL".to_string(), email.to_string());
        }
        env.insert("COMMITTER".to_string(), committer.to_string());
        env.insert("BRZ_EMAIL".to_string(), committer.to_string());
    }
    env
}

#[cfg(feature = "debian")]
/// Errors that can occur when finding changes files.
#[derive(Debug)]
pub enum FindChangesError {
    /// No changes file was found in the specified directory.
    NoChangesFile(PathBuf),
    /// Inconsistent versions were found in multiple changes files.
    InconsistentVersion(Vec<String>, debversion::Version, debversion::Version),
    /// Inconsistent source names were found in multiple changes files.
    InconsistentSource(Vec<String>, String, String),
    /// Inconsistent distributions were found in multiple changes files.
    InconsistentDistribution(Vec<String>, String, String),
    /// A required field was missing in the changes file.
    MissingChangesFileFields(&'static str),
    /// I/O error when accessing the directory or files.
    IoError(PathBuf, std::io::Error),
    /// Error parsing a changes file.
    ParseError(PathBuf, Box<dyn std::error::Error + Send + Sync>),
    /// Filename cannot be converted to UTF-8.
    InvalidFilename(PathBuf),
}

#[cfg(feature = "debian")]
impl std::fmt::Display for FindChangesError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            FindChangesError::NoChangesFile(path) => {
                write!(f, "No changes file found in {}", path.display())
            }
            FindChangesError::InconsistentVersion(names, found, expected) => write!(
                f,
                "Inconsistent version in changes files {:?}: found {} expected {}",
                names, found, expected
            ),
            FindChangesError::InconsistentSource(names, found, expected) => write!(
                f,
                "Inconsistent source in changes files {:?}: found {} expected {}",
                names, found, expected
            ),
            FindChangesError::InconsistentDistribution(names, found, expected) => write!(
                f,
                "Inconsistent distribution in changes files {:?}: found {} expected {}",
                names, found, expected
            ),
            FindChangesError::MissingChangesFileFields(field) => {
                write!(f, "Missing field {} in changes files", field)
            }
            FindChangesError::IoError(path, err) => {
                write!(f, "I/O error accessing {}: {}", path.display(), err)
            }
            FindChangesError::ParseError(path, err) => {
                write!(f, "Error parsing changes file {}: {}", path.display(), err)
            }
            FindChangesError::InvalidFilename(path) => {
                write!(
                    f,
                    "Invalid filename that cannot be converted to UTF-8: {}",
                    path.display()
                )
            }
        }
    }
}

#[cfg(feature = "debian")]
impl std::error::Error for FindChangesError {}

#[cfg(feature = "debian")]
#[cfg(test)]
#[path = "find_changes_tests.rs"]
mod find_changes_tests;

#[cfg(feature = "debian")]
/// Summary of changes files.
#[derive(Debug)]
pub struct ChangesSummary {
    /// Names of the changes files.
    pub names: Vec<String>,
    /// Source package name.
    pub source: String,
    /// Package version.
    pub version: debversion::Version,
    /// Distribution name.
    pub distribution: String,
    /// Names of binary packages included in the changes.
    pub binary_packages: Vec<String>,
}

/// Find and parse Debian changes files in a directory.
///
/// # Arguments
/// * `path` - Directory to search for changes files
///
/// # Returns
/// A summary of the changes files, or an error if not found or inconsistent
pub fn find_changes(path: &Path) -> Result<ChangesSummary, FindChangesError> {
    let mut names: Vec<String> = Vec::new();
    let mut source: Option<String> = None;
    let mut version: Option<debversion::Version> = None;
    let mut distribution: Option<String> = None;
    let mut binary_packages: Vec<String> = Vec::new();

    let read_dir =
        std::fs::read_dir(path).map_err(|e| FindChangesError::IoError(path.to_path_buf(), e))?;

    for entry_result in read_dir {
        let entry = entry_result.map_err(|e| FindChangesError::IoError(path.to_path_buf(), e))?;

        // Check if filename can be converted to UTF-8 and ends with .changes
        let file_name = entry.file_name();
        let file_name_str = file_name
            .to_str()
            .ok_or_else(|| FindChangesError::InvalidFilename(entry.path()))?;

        if !file_name_str.ends_with(".changes") {
            continue;
        }

        let file_path = entry.path();
        let f = std::fs::File::open(&file_path)
            .map_err(|e| FindChangesError::IoError(file_path.clone(), e))?;

        let changes = debian_control::changes::Changes::read(&f)
            .map_err(|e| FindChangesError::ParseError(file_path, e.into()))?;
        names.push(entry.file_name().to_string_lossy().to_string());
        if let Some(version) = &version {
            if changes.version().as_ref() != Some(version) {
                let found_version = changes
                    .version()
                    .ok_or_else(|| FindChangesError::MissingChangesFileFields("Version"))?;
                return Err(FindChangesError::InconsistentVersion(
                    names,
                    found_version,
                    version.clone(),
                ));
            }
        }
        version = changes.version();
        if let Some(source) = &source {
            if changes.source().as_ref() != Some(source) {
                let found_source = changes
                    .source()
                    .ok_or_else(|| FindChangesError::MissingChangesFileFields("Source"))?;
                return Err(FindChangesError::InconsistentSource(
                    names,
                    found_source,
                    source.to_string(),
                ));
            }
        }
        source = changes.source();

        if let Some(distribution) = &distribution {
            if changes.distribution().as_ref() != Some(distribution) {
                let found_distribution = changes
                    .distribution()
                    .ok_or_else(|| FindChangesError::MissingChangesFileFields("Distribution"))?;
                return Err(FindChangesError::InconsistentDistribution(
                    names,
                    found_distribution,
                    distribution.to_string(),
                ));
            }
        }
        distribution = changes.distribution();

        binary_packages.extend(
            changes
                .files()
                .unwrap_or_default()
                .iter()
                .filter_map(|file| {
                    if file.filename.ends_with(".deb") {
                        // Extract package name from filename (everything before first underscore)
                        file.filename.split('_').next().map(|s| s.to_string())
                    } else {
                        None
                    }
                }),
        );
    }
    if names.is_empty() {
        return Err(FindChangesError::NoChangesFile(path.to_path_buf()));
    }

    if source.is_none() {
        return Err(FindChangesError::MissingChangesFileFields("Source"));
    }

    if version.is_none() {
        return Err(FindChangesError::MissingChangesFileFields("Version"));
    }

    if distribution.is_none() {
        return Err(FindChangesError::MissingChangesFileFields("Distribution"));
    }

    Ok(ChangesSummary {
        names,
        source: source.unwrap(),
        version: version.unwrap(),
        distribution: distribution.unwrap(),
        binary_packages,
    })
}

/// Check if a filename is a log file.
///
/// # Arguments
/// * `name` - Filename to check
///
/// # Returns
/// `true` if the filename is a log file, `false` otherwise
pub fn is_log_filename(name: &str) -> bool {
    let parts = name.split('.').collect::<Vec<_>>();

    // Must have at least one extension and filename must not be empty
    if parts.len() < 2 || parts[0].is_empty() {
        return false;
    }

    // Handle simple .log files (foo.log)
    if parts.last() == Some(&"log") {
        return true;
    }

    // Handle compressed log files (.log.gz, .log.bz2, etc.)
    if parts.len() >= 3 {
        let compression_extensions = ["gz", "bz2", "xz", "lzma", "Z"];
        if let Some(&last_part) = parts.last() {
            if compression_extensions.contains(&last_part) {
                // Check if the second-to-last part is "log"
                if parts[parts.len() - 2] == "log" {
                    return true;
                }
            }
        }
    }

    // Handle numbered log files (foo.log.1, foo.1.log)
    if parts.len() == 3 {
        let mut rev = parts.iter().rev();
        let last = rev.next().unwrap();
        let middle = rev.next().unwrap();

        // foo.log.1 pattern
        if last.chars().all(char::is_numeric) && *middle == "log" {
            return true;
        }
    }

    false
}

#[cfg(feature = "debian")]
/// Get the current Debian vendor.
///
/// # Returns
/// The vendor name, or None if it could not be determined
pub fn dpkg_vendor() -> Option<String> {
    std::process::Command::new("dpkg-vendor")
        .arg("--query")
        .arg("vendor")
        .output()
        .map(|output| {
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap()
}

#[cfg(feature = "debian")]
/// Read the source filenames from a changes file.
pub fn changes_filenames(changes_location: &Path) -> Vec<String> {
    let mut f = std::fs::File::open(changes_location).unwrap();
    let changes = debian_control::changes::Changes::read(&mut f).unwrap();
    changes
        .files()
        .unwrap_or_default()
        .iter()
        .map(|file| file.filename.clone())
        .collect()
}

/// Scan a directory for log files.
///
/// # Arguments
/// * `output_directory` - Directory to scan
pub fn gather_logs(output_directory: &std::path::Path) -> impl Iterator<Item = std::fs::DirEntry> {
    std::fs::read_dir(output_directory)
        .unwrap()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            if entry.file_type().unwrap().is_dir()
                && is_log_filename(entry.file_name().to_str().unwrap())
            {
                Some(entry)
            } else {
                None
            }
        })
}

/// Result of a Janitor run.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JanitorResult {
    /// Unique identifier for the log.
    pub log_id: String,
    /// URL of the branch that was processed.
    pub branch_url: String,
    /// Optional subpath within the repository.
    pub subpath: Option<String>,
    /// Result code.
    pub code: String,
    /// Whether the result is transient.
    pub transient: Option<bool>,
    /// Name of the codebase.
    pub codebase: String,
    /// Name of the campaign.
    pub campaign: String,
    /// Human-readable description of the result.
    pub description: Option<String>,
    /// Result of the codemod.
    pub codemod: Option<serde_json::Value>,
    /// Optional value associated with the result.
    pub value: Option<u64>,
    /// Names of log files.
    pub logfilenames: Vec<String>,

    /// Time when the run started.
    pub start_time: DateTime<Utc>,
    /// Time when the run finished.
    pub finish_time: DateTime<Utc>,

    /// Revision ID of the branch after processing.
    pub revision: Option<RevisionId>,
    /// Revision ID of the main branch.
    pub main_branch_revision: Option<RevisionId>,

    /// Optional changeset ID.
    pub change_set: Option<String>,

    /// Optional tags with revision IDs.
    pub tags: Option<Vec<(String, Option<RevisionId>)>>,
    /// Optional remote repositories.
    pub remotes: Option<HashMap<String, ResultRemote>>,

    /// Optional branches information.
    pub branches: Option<
        Vec<(
            Option<String>,
            Option<String>,
            Option<RevisionId>,
            Option<RevisionId>,
        )>,
    >,

    /// Optional details about the failure.
    pub failure_details: Option<serde_json::Value>,
    /// Optional stages where failure occurred.
    pub failure_stage: Option<Vec<String>>,

    /// Optional information about resuming a previous run.
    pub resume: Option<ResultResume>,

    /// Optional target information.
    pub target: Option<ResultTarget>,

    /// Optional worker name.
    pub worker_name: Option<String>,
    /// Optional VCS type.
    pub vcs_type: Option<String>,
    /// Optional target branch URL.
    pub target_branch_url: Option<String>,
    /// Optional context information.
    pub context: Option<serde_json::Value>,
    /// Optional builder result.
    pub builder_result: Option<BuilderResult>,
}

impl JanitorResult {
    /// Calculate the duration of the run.
    pub fn duration(&self) -> Duration {
        let duration = self.finish_time - self.start_time;
        Duration::from_secs(duration.num_seconds().max(0) as u64)
    }

    /// Convert to JSON representation.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "codebase": self.codebase,
            "campaign": self.campaign,
            "change_set": self.change_set,
            "log_id": self.log_id,
            "description": self.description,
            "code": self.code,
            "failure_details": self.failure_details,
            "failure_stage": self.failure_stage,
            "duration": self.duration().as_secs_f64(),
            "finish_time": self.finish_time.to_rfc3339(),
            "start_time": self.start_time.to_rfc3339(),
            "transient": self.transient,
            "target": self.target.as_ref().map(|t| serde_json::json!({
                "name": t.name,
                "details": t.details
            })).unwrap_or_else(|| serde_json::json!({})),
            "logfilenames": self.logfilenames,
            "codemod": self.codemod,
            "value": self.value,
            "remotes": self.remotes,
            "branch_url": self.branch_url,
            "resume": self.resume.as_ref().map(|r| serde_json::json!({"run_id": r.run_id})),
            "branches": self.branches.as_ref().map(|branches| {
                branches.iter().map(|(fn_name, name, br, r)| {
                    serde_json::json!([
                        fn_name,
                        name,
                        br.as_ref().map(|b| b.to_string()),
                        r.as_ref().map(|r| r.to_string())
                    ])
                }).collect::<Vec<_>>()
            }),
            "tags": self.tags.as_ref().map(|tags| {
                tags.iter().map(|(name, rev)| {
                    serde_json::json!([
                        name,
                        rev.as_ref().map(|r| r.to_string())
                    ])
                }).collect::<Vec<_>>()
            }),
            "revision": self.revision.as_ref().map(|r| r.to_string()),
            "main_branch_revision": self.main_branch_revision.as_ref().map(|r| r.to_string())
        })
    }
}

/// Information about resuming a previous run.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResultResume {
    /// ID of the run to resume.
    pub run_id: String,
}

/// Target information for a result.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResultTarget {
    /// Name of the target.
    pub name: String,
    /// Additional details about the target.
    pub details: serde_json::Value,
}

/// Remote repository information for a result.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResultRemote {
    /// URL of the remote repository.
    pub url: String,
}

/// Result from a worker.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkerResult {
    /// Result code.
    pub code: String,
    /// Human-readable description.
    pub description: Option<String>,
    /// Context information.
    pub context: Option<serde_json::Value>,
    /// Codemod result.
    pub codemod: Option<serde_json::Value>,
    /// Main branch revision ID.
    pub main_branch_revision: Option<RevisionId>,
    /// Current revision ID.
    pub revision: Option<RevisionId>,
    /// Optional value associated with the result.
    pub value: Option<i64>,
    /// Branch information.
    pub branches: Option<
        Vec<(
            Option<String>,
            Option<String>,
            Option<RevisionId>,
            Option<RevisionId>,
        )>,
    >,
    /// Tag information.
    pub tags: Option<Vec<(String, Option<RevisionId>)>>,
    /// Remote repository information.
    pub remotes: Option<HashMap<String, HashMap<String, serde_json::Value>>>,
    /// Failure details.
    pub details: Option<serde_json::Value>,
    /// Failure stage.
    pub stage: Option<Vec<String>>,
    /// Builder result.
    pub builder_result: Option<BuilderResult>,
    /// Start time.
    pub start_time: Option<DateTime<Utc>>,
    /// Finish time.
    pub finish_time: Option<DateTime<Utc>>,
    /// Queue ID.
    pub queue_id: Option<i64>,
    /// Worker name.
    pub worker_name: Option<String>,
    /// Whether the run was refreshed.
    pub refreshed: bool,
    /// Target branch URL.
    pub target_branch_url: Option<String>,
    /// Branch URL.
    pub branch_url: Option<String>,
    /// VCS type.
    pub vcs_type: Option<String>,
    /// Subpath within repository.
    pub subpath: Option<String>,
    /// Whether the result is transient.
    pub transient: Option<bool>,
    /// Codebase name.
    pub codebase: Option<String>,
}

/// Information about an active run.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ActiveRun {
    /// Worker name.
    pub worker_name: String,
    /// Optional worker link.
    pub worker_link: Option<String>,
    /// Queue ID.
    pub queue_id: i64,
    /// Unique log ID.
    pub log_id: String,
    /// Start time.
    pub start_time: DateTime<Utc>,
    /// Optional finish time.
    pub finish_time: Option<DateTime<Utc>>,
    /// Optional estimated duration.
    #[serde(with = "serde_with::As::<Option<serde_with::DurationSeconds<f64>>>")]
    pub estimated_duration: Option<Duration>,
    /// Campaign name.
    pub campaign: String,
    /// Optional change set.
    pub change_set: Option<String>,
    /// Command being executed.
    pub command: String,
    /// Backchannel for communication.
    pub backchannel: Backchannel,
    /// VCS information.
    pub vcs_info: VcsInfo,
    /// Codebase name.
    pub codebase: String,
    /// Instigated context.
    pub instigated_context: Option<serde_json::Value>,
    /// Optional resume from run ID.
    pub resume_from: Option<String>,
}

impl ActiveRun {
    /// Calculate current duration of the run.
    pub fn current_duration(&self) -> Duration {
        let now = Utc::now();
        let duration = now - self.start_time;
        Duration::from_secs(duration.num_seconds().max(0) as u64)
    }

    /// Get VCS type.
    pub fn vcs_type(&self) -> Option<&str> {
        self.vcs_info.vcs_type.as_deref()
    }

    /// Get main branch URL.
    pub fn main_branch_url(&self) -> Option<&str> {
        self.vcs_info.branch_url.as_deref()
    }

    /// Get subpath.
    pub fn subpath(&self) -> Option<&str> {
        self.vcs_info.subpath.as_deref()
    }

    /// Create a JanitorResult from this active run.
    pub fn create_result(&self, code: String, description: Option<String>) -> JanitorResult {
        JanitorResult {
            log_id: self.log_id.clone(),
            branch_url: self.vcs_info.branch_url.clone().unwrap_or_default(),
            subpath: self.vcs_info.subpath.clone(),
            code,
            transient: None,
            codebase: self.codebase.clone(),
            campaign: self.campaign.clone(),
            description,
            codemod: None,
            value: None,
            logfilenames: vec![],
            start_time: self.start_time,
            finish_time: Utc::now(),
            revision: None,
            main_branch_revision: None,
            change_set: self.change_set.clone(),
            tags: None,
            remotes: None,
            branches: None,
            failure_details: None,
            failure_stage: None,
            resume: self
                .resume_from
                .as_ref()
                .map(|id| ResultResume { run_id: id.clone() }),
            target: None,
            worker_name: Some(self.worker_name.clone()),
            vcs_type: self.vcs_info.vcs_type.clone(),
            target_branch_url: None,
            context: self.instigated_context.clone(),
            builder_result: None,
        }
    }

    /// Convert to JSON representation.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "queue_id": self.queue_id,
            "id": self.log_id,
            "codebase": self.codebase,
            "change_set": self.change_set,
            "campaign": self.campaign,
            "command": self.command,
            "estimated_duration": self.estimated_duration.map(|d| d.as_secs_f64()),
            "current_duration": self.current_duration().as_secs_f64(),
            "start_time": self.start_time.to_rfc3339(),
            "worker": self.worker_name,
            "worker_link": self.worker_link,
            "vcs": self.vcs_info,
            "backchannel": self.backchannel.to_json(),
            "instigated_context": self.instigated_context,
            "resume_from": self.resume_from
        })
    }

    /// Ping the worker to check if it's still alive.
    pub async fn ping(&self) -> Result<(), PingError> {
        self.backchannel.ping(&self.log_id).await
    }
}

/// Backchannel communication types.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Backchannel {
    /// Jenkins backchannel.
    Jenkins {
        /// Jenkins URL.
        my_url: String,
        /// Jenkins metadata.
        jenkins: Option<serde_json::Value>,
    },
    /// Polling backchannel.
    Polling {
        /// Worker URL.
        my_url: String,
    },
    /// No backchannel (default).
    None {},
}

impl Default for Backchannel {
    fn default() -> Self {
        Backchannel::None {}
    }
}

impl Backchannel {
    /// Ping the worker.
    pub async fn ping(&self, expected_log_id: &str) -> Result<(), PingError> {
        match self {
            Backchannel::None {} => {
                // No ping available
                Err(PingError::Retriable(
                    "No backchannel available for ping".to_string(),
                ))
            }
            Backchannel::Jenkins { my_url, .. } => {
                // Implement Jenkins ping by checking job status
                use reqwest::Client;
                use std::time::Duration;

                let client = Client::builder()
                    .timeout(Duration::from_secs(60))
                    .build()
                    .map_err(|e| {
                        PingError::Retriable(format!("Failed to create HTTP client: {}", e))
                    })?;

                let api_url = format!("{}/api/json", my_url);

                match client.get(&api_url).send().await {
                    Ok(response) => {
                        if response.status() == 404 {
                            Err(PingError::Fatal(format!(
                                "Jenkins job {} has disappeared",
                                my_url
                            )))
                        } else if !response.status().is_success() {
                            Err(PingError::Retriable(format!(
                                "Failed to ping Jenkins {}: HTTP {}",
                                my_url,
                                response.status()
                            )))
                        } else {
                            match response.json::<serde_json::Value>().await {
                                Ok(job) => {
                                    // Check if job failed
                                    if let Some(result) = job.get("result") {
                                        if result == "FAILURE" {
                                            if let Some(job_id) = job.get("id") {
                                                return Err(PingError::Fatal(format!(
                                                    "Jenkins lists job {} for run {} as failed",
                                                    job_id, expected_log_id
                                                )));
                                            }
                                        }
                                    }
                                    Ok(())
                                }
                                Err(e) => Err(PingError::Retriable(format!(
                                    "Failed to parse Jenkins response from {}: {}",
                                    my_url, e
                                ))),
                            }
                        }
                    }
                    Err(e) => {
                        if e.is_timeout() {
                            Err(PingError::Timeout(format!(
                                "Failed to ping Jenkins {}: {}",
                                my_url, e
                            )))
                        } else {
                            Err(PingError::Retriable(format!(
                                "Failed to ping Jenkins {}: {}",
                                my_url, e
                            )))
                        }
                    }
                }
            }
            Backchannel::Polling { my_url } => {
                // Implement polling ping by checking worker health
                use reqwest::Client;
                use std::time::Duration;

                let client = Client::builder()
                    .timeout(Duration::from_secs(60))
                    .build()
                    .map_err(|e| {
                        PingError::Retriable(format!("Failed to create HTTP client: {}", e))
                    })?;

                let health_url = format!("{}/log-id", my_url);
                log::info!("Pinging URL {} for run {}", health_url, expected_log_id);

                match client.get(&health_url).send().await {
                    Ok(response) => {
                        if !response.status().is_success() {
                            Err(PingError::Retriable(format!(
                                "Failed to ping worker {}: HTTP {}",
                                my_url,
                                response.status()
                            )))
                        } else {
                            match response.text().await {
                                Ok(log_id) => {
                                    let log_id = log_id.trim();
                                    if log_id != expected_log_id {
                                        Err(PingError::Fatal(format!(
                                            "Worker started processing new run {} rather than {}",
                                            log_id, expected_log_id
                                        )))
                                    } else {
                                        Ok(())
                                    }
                                }
                                Err(e) => Err(PingError::Retriable(format!(
                                    "Failed to read response from {}: {}",
                                    my_url, e
                                ))),
                            }
                        }
                    }
                    Err(e) => {
                        if e.is_timeout() {
                            Err(PingError::Timeout(format!(
                                "Failed to ping worker {}: {}",
                                my_url, e
                            )))
                        } else {
                            Err(PingError::Retriable(format!(
                                "Failed to ping worker {}: {}",
                                my_url, e
                            )))
                        }
                    }
                }
            }
        }
    }

    /// Get health status from the worker.
    pub async fn get_health_status(
        &self,
        expected_log_id: &str,
    ) -> Result<crate::HealthStatus, crate::BackchannelError> {
        match self {
            Backchannel::None {} => Err(crate::BackchannelError::WorkerUnreachable(
                "No backchannel available for health check".to_string(),
            )),
            Backchannel::Jenkins { my_url, jenkins } => {
                let url = url::Url::parse(my_url).map_err(|_| {
                    crate::BackchannelError::FatalFailure("Invalid URL".to_string())
                })?;
                let jenkins_bc =
                    crate::JenkinsBackchannel::new(url, jenkins.clone().unwrap_or_default());
                jenkins_bc.get_health_status(expected_log_id).await
            }
            Backchannel::Polling { my_url } => {
                let url = url::Url::parse(my_url).map_err(|_| {
                    crate::BackchannelError::FatalFailure("Invalid URL".to_string())
                })?;
                let polling_bc = crate::PollingBackchannel::new(url);
                polling_bc.get_health_status(expected_log_id).await
            }
        }
    }

    /// Terminate the worker gracefully.
    pub async fn terminate(&self, log_id: &str) -> Result<(), crate::BackchannelError> {
        match self {
            Backchannel::None {} => Err(crate::BackchannelError::WorkerUnreachable(
                "No backchannel available for termination".to_string(),
            )),
            Backchannel::Jenkins { my_url, jenkins } => {
                let url = url::Url::parse(my_url).map_err(|_| {
                    crate::BackchannelError::FatalFailure("Invalid URL".to_string())
                })?;
                let jenkins_bc =
                    crate::JenkinsBackchannel::new(url, jenkins.clone().unwrap_or_default());
                jenkins_bc.terminate(log_id).await
            }
            Backchannel::Polling { my_url } => {
                let url = url::Url::parse(my_url).map_err(|_| {
                    crate::BackchannelError::FatalFailure("Invalid URL".to_string())
                })?;
                let polling_bc = crate::PollingBackchannel::new(url);
                polling_bc.terminate(log_id).await
            }
        }
    }

    /// Kill the worker.
    pub async fn kill(&self) -> Result<(), PingError> {
        match self {
            Backchannel::None {} => Err(PingError::Retriable(
                "No backchannel available for kill".to_string(),
            )),
            Backchannel::Jenkins { .. } => Err(PingError::Fatal(
                "Jenkins kill not supported - Jenkins jobs cannot be killed via API".to_string(),
            )),
            Backchannel::Polling { my_url } => {
                // Implement polling kill by sending POST request to /kill endpoint
                use reqwest::Client;
                use std::time::Duration;

                let client = Client::builder()
                    .timeout(Duration::from_secs(30))
                    .build()
                    .map_err(|e| {
                        PingError::Retriable(format!("Failed to create HTTP client: {}", e))
                    })?;

                let kill_url = format!("{}/kill", my_url);

                match client
                    .post(&kill_url)
                    .header("Accept", "application/json")
                    .send()
                    .await
                {
                    Ok(response) => {
                        if response.status().is_success() {
                            Ok(())
                        } else {
                            Err(PingError::Retriable(format!(
                                "Failed to kill worker at {}: HTTP {}",
                                my_url,
                                response.status()
                            )))
                        }
                    }
                    Err(e) => {
                        if e.is_timeout() {
                            Err(PingError::Timeout(format!(
                                "Timeout killing worker at {}: {}",
                                my_url, e
                            )))
                        } else {
                            Err(PingError::Retriable(format!(
                                "Failed to kill worker at {}: {}",
                                my_url, e
                            )))
                        }
                    }
                }
            }
        }
    }

    /// List available log files.
    pub async fn list_log_files(&self) -> Result<Vec<String>, PingError> {
        match self {
            Backchannel::None {} => Ok(vec![]),
            Backchannel::Jenkins { .. } => Ok(vec!["worker.log".to_string()]),
            Backchannel::Polling { my_url } => {
                // Implement polling list_log_files by querying /logs endpoint
                use reqwest::Client;
                use std::time::Duration;

                let client = Client::builder()
                    .timeout(Duration::from_secs(30))
                    .build()
                    .map_err(|e| {
                        PingError::Retriable(format!("Failed to create HTTP client: {}", e))
                    })?;

                let logs_url = format!("{}/logs", my_url);

                match client.get(&logs_url).send().await {
                    Ok(response) => {
                        if response.status().is_success() {
                            match response.json::<Vec<String>>().await {
                                Ok(log_files) => Ok(log_files),
                                Err(e) => Err(PingError::Retriable(format!(
                                    "Failed to parse log files response from {}: {}",
                                    my_url, e
                                ))),
                            }
                        } else {
                            Err(PingError::Retriable(format!(
                                "Failed to list log files from {}: HTTP {}",
                                my_url,
                                response.status()
                            )))
                        }
                    }
                    Err(e) => {
                        if e.is_timeout() {
                            Err(PingError::Timeout(format!(
                                "Timeout listing log files from {}: {}",
                                my_url, e
                            )))
                        } else {
                            Err(PingError::Retriable(format!(
                                "Failed to list log files from {}: {}",
                                my_url, e
                            )))
                        }
                    }
                }
            }
        }
    }

    /// Get a specific log file.
    pub async fn get_log_file(&self, name: &str) -> Result<Vec<u8>, PingError> {
        match self {
            Backchannel::None {} => {
                Err(PingError::Retriable("No backchannel available".to_string()))
            }
            Backchannel::Jenkins { my_url, .. } => {
                // Jenkins only supports getting "worker.log" via progressiveText endpoint
                if name != "worker.log" {
                    return Err(PingError::Fatal(format!(
                        "Jenkins log file not found: {}",
                        name
                    )));
                }

                use reqwest::Client;
                use std::time::Duration;

                let client = Client::builder()
                    .timeout(Duration::from_secs(60))
                    .build()
                    .map_err(|e| {
                        PingError::Retriable(format!("Failed to create HTTP client: {}", e))
                    })?;

                let log_url = format!("{}/logText/progressiveText", my_url);

                match client.get(&log_url).send().await {
                    Ok(response) => {
                        if response.status().is_success() {
                            match response.bytes().await {
                                Ok(bytes) => Ok(bytes.to_vec()),
                                Err(e) => Err(PingError::Retriable(format!(
                                    "Failed to read log file content from {}: {}",
                                    my_url, e
                                ))),
                            }
                        } else if response.status() == 404 {
                            Err(PingError::Fatal(format!("Log file not found: {}", name)))
                        } else {
                            Err(PingError::Retriable(format!(
                                "Failed to get log file from {}: HTTP {}",
                                my_url,
                                response.status()
                            )))
                        }
                    }
                    Err(e) => {
                        if e.is_timeout() {
                            Err(PingError::Timeout(format!(
                                "Timeout getting log file from {}: {}",
                                my_url, e
                            )))
                        } else {
                            Err(PingError::Retriable(format!(
                                "Failed to get log file from {}: {}",
                                my_url, e
                            )))
                        }
                    }
                }
            }
            Backchannel::Polling { my_url } => {
                // Polling gets log files via /logs/{name} endpoint
                use reqwest::Client;
                use std::time::Duration;

                let client = Client::builder()
                    .timeout(Duration::from_secs(60))
                    .build()
                    .map_err(|e| {
                        PingError::Retriable(format!("Failed to create HTTP client: {}", e))
                    })?;

                let log_url = format!("{}/logs/{}", my_url, name);

                match client.get(&log_url).send().await {
                    Ok(response) => {
                        if response.status().is_success() {
                            match response.bytes().await {
                                Ok(bytes) => Ok(bytes.to_vec()),
                                Err(e) => Err(PingError::Retriable(format!(
                                    "Failed to read log file content from {}: {}",
                                    my_url, e
                                ))),
                            }
                        } else if response.status() == 404 {
                            Err(PingError::Fatal(format!("Log file not found: {}", name)))
                        } else {
                            Err(PingError::Retriable(format!(
                                "Failed to get log file from {}: HTTP {}",
                                my_url,
                                response.status()
                            )))
                        }
                    }
                    Err(e) => {
                        if e.is_timeout() {
                            Err(PingError::Timeout(format!(
                                "Timeout getting log file from {}: {}",
                                my_url, e
                            )))
                        } else {
                            Err(PingError::Retriable(format!(
                                "Failed to get log file from {}: {}",
                                my_url, e
                            )))
                        }
                    }
                }
            }
        }
    }

    /// Convert to JSON representation.
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            Backchannel::None {} => serde_json::json!({}),
            Backchannel::Jenkins { my_url, jenkins } => serde_json::json!({
                "my_url": my_url,
                "jenkins": jenkins
            }),
            Backchannel::Polling { my_url } => serde_json::json!({
                "my_url": my_url
            }),
        }
    }
}

/// Builder result types.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum BuilderResult {
    /// Generic build result.
    #[serde(rename = "generic")]
    Generic,
    /// Debian build result.
    #[serde(rename = "debian")]
    Debian {
        /// Source package name.
        source: Option<String>,
        /// Build version.
        build_version: Option<String>,
        /// Build distribution.
        build_distribution: Option<String>,
        /// Changes filenames.
        changes_filenames: Option<Vec<String>>,
        /// Lintian result.
        lintian_result: Option<serde_json::Value>,
        /// Binary packages.
        binary_packages: Option<Vec<String>>,
    },
}

impl BuilderResult {
    /// Get the kind of builder result.
    pub fn kind(&self) -> &'static str {
        match self {
            BuilderResult::Generic => "generic",
            BuilderResult::Debian { .. } => "debian",
        }
    }

    /// Convert to JSON.
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            BuilderResult::Generic => serde_json::json!({}),
            BuilderResult::Debian {
                source,
                build_version,
                build_distribution,
                changes_filenames,
                lintian_result,
                binary_packages,
            } => serde_json::json!({
                "source": source,
                "build_version": build_version,
                "build_distribution": build_distribution,
                "changes_filenames": changes_filenames,
                "lintian": lintian_result,
                "binary_packages": binary_packages
            }),
        }
    }

    /// Get artifact filenames.
    pub fn artifact_filenames(&self) -> Vec<String> {
        match self {
            BuilderResult::Generic => vec![],
            BuilderResult::Debian {
                changes_filenames, ..
            } => changes_filenames.clone().unwrap_or_default(),
        }
    }
}

/// Ping failure types.
#[derive(Debug)]
pub enum PingError {
    /// Timeout while pinging.
    Timeout(String),
    /// Fatal failure that's not retriable.
    Fatal(String),
    /// Retriable failure.
    Retriable(String),
}

impl std::fmt::Display for PingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PingError::Timeout(msg) => write!(f, "Ping timeout: {}", msg),
            PingError::Fatal(msg) => write!(f, "Fatal ping failure: {}", msg),
            PingError::Retriable(msg) => write!(f, "Ping failure: {}", msg),
        }
    }
}

impl std::error::Error for PingError {}

/// A queue item representing work to be done.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QueueItem {
    /// Unique identifier for the queue item.
    pub id: i64,
    /// Context information for the run.
    pub context: Option<serde_json::Value>,
    /// Command to execute.
    pub command: String,
    /// Estimated duration of the run.
    #[serde(with = "serde_with::As::<Option<serde_with::DurationSeconds<f64>>>")]
    pub estimated_duration: Option<Duration>,
    /// Campaign name.
    pub campaign: String,
    /// Whether to refresh the run.
    pub refresh: bool,
    /// Who requested this run.
    pub requester: Option<String>,
    /// Optional change set identifier.
    pub change_set: Option<String>,
    /// Name of the codebase.
    pub codebase: String,
}

impl QueueItem {
    /// Convert to JSON representation.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "context": self.context,
            "command": self.command,
            "estimated_duration": self.estimated_duration.map(|d| d.as_secs_f64()),
            "campaign": self.campaign,
            "refresh": self.refresh,
            "requester": self.requester,
            "change_set": self.change_set,
            "codebase": self.codebase
        })
    }
}

/// Queue assignment result.
#[derive(Debug)]
pub struct QueueAssignment {
    /// The assigned queue item.
    pub queue_item: QueueItem,
    /// VCS information for the codebase.
    pub vcs_info: VcsInfo,
}

/// Application state for the runner.
pub struct AppState {
    /// Database connection pool.
    pub database: Arc<database::RunnerDatabase>,
    /// VCS management system.
    pub vcs_manager: Arc<vcs::RunnerVcsManager>,
    /// Log file management system.
    pub log_manager: Arc<dyn logs::LogFileManager>,
    /// Artifact storage management system.
    pub artifact_manager: Arc<dyn janitor::artifacts::ArtifactManager>,
    /// Performance monitoring system.
    pub performance_monitor: Arc<performance::PerformanceMonitor>,
    /// Error tracking system.
    pub error_tracker: Arc<error_tracking::ErrorTracker>,
    /// Metrics collector.
    pub metrics: Arc<metrics::MetricsCollector>,
    /// Configuration.
    pub config: Arc<janitor::config::Config>,
    /// Upload processor for multipart forms.
    pub upload_processor: Arc<upload::UploadProcessor>,
    /// Worker authentication service.
    pub auth_service: Arc<auth::WorkerAuthService>,
    /// Security service for rate limiting and access control.
    pub security_service: Arc<auth::SecurityService>,
    /// Resume service for handling interrupted runs.
    pub resume_service: Arc<resume::ResumeService>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_committer_env() {
        let committer = Some("John Doe <john@example.com>");

        let expected = maplit::hashmap! {
            "DEBFULLNAME".to_string() => "John Doe".to_string(),
            "GIT_COMMITTER_NAME".to_string() => "John Doe".to_string(),
            "GIT_AUTHOR_NAME".to_string() => "John Doe".to_string(),
            "DEBEMAIL".to_string() => "john@example.com".to_string(),
            "GIT_COMMITTER_EMAIL".to_string() => "john@example.com".to_string(),
            "GIT_AUTHOR_EMAIL".to_string() => "john@example.com".to_string(),
            "EMAIL".to_string() => "john@example.com".to_string(),
            "COMMITTER".to_string() => "John Doe <john@example.com>".to_string(),
            "BRZ_EMAIL".to_string() => "John Doe <john@example.com>".to_string(),
        };

        assert_eq!(committer_env(committer), expected);
    }

    #[test]
    fn test_active_run_creation() {
        let vcs_info = VcsInfo {
            vcs_type: Some("git".to_string()),
            branch_url: Some("https://github.com/example/repo.git".to_string()),
            subpath: None,
        };

        let active_run = ActiveRun {
            worker_name: "test-worker".to_string(),
            worker_link: None,
            queue_id: 123,
            log_id: "log-456".to_string(),
            start_time: Utc::now(),
            finish_time: None,
            estimated_duration: Some(Duration::from_secs(300)),
            campaign: "test-campaign".to_string(),
            change_set: None,
            command: "test command".to_string(),
            backchannel: Backchannel::default(),
            vcs_info,
            codebase: "test-codebase".to_string(),
            instigated_context: None,
            resume_from: None,
        };

        assert_eq!(active_run.vcs_type(), Some("git"));
        assert_eq!(
            active_run.main_branch_url(),
            Some("https://github.com/example/repo.git")
        );
        assert!(active_run.current_duration().as_secs() >= 0);

        let result =
            active_run.create_result("success".to_string(), Some("Test completed".to_string()));
        assert_eq!(result.code, "success");
        assert_eq!(result.description, Some("Test completed".to_string()));
        assert_eq!(result.codebase, "test-codebase");
    }

    #[test]
    fn test_builder_result_serialization() {
        let generic = BuilderResult::Generic;
        assert_eq!(generic.kind(), "generic");
        assert_eq!(generic.artifact_filenames(), Vec::<String>::new());

        let debian = BuilderResult::Debian {
            source: Some("test-package".to_string()),
            build_version: Some("1.0.0".to_string()),
            build_distribution: Some("bullseye".to_string()),
            changes_filenames: Some(vec!["test.changes".to_string()]),
            lintian_result: None,
            binary_packages: Some(vec!["test-bin".to_string()]),
        };

        assert_eq!(debian.kind(), "debian");
        assert_eq!(debian.artifact_filenames(), vec!["test.changes"]);
    }

    #[test]
    fn test_committer_env_no_committer() {
        let committer = None;

        let expected = maplit::hashmap! {};

        assert_eq!(committer_env(committer), expected);
    }

    #[test]
    fn is_log_filename_test() {
        assert!(is_log_filename("foo.log"));
        assert!(is_log_filename("foo.log.1"));
        assert!(is_log_filename("foo.1.log"));
        assert!(!is_log_filename("foo.1"));
        assert!(!is_log_filename("foo.1.log.1"));
        assert!(!is_log_filename("foo.1.notlog"));
        assert!(!is_log_filename("foo.log.notlog"));
    }

    #[test]
    fn test_dpkg_vendor() {
        let vendor = dpkg_vendor();
        assert!(vendor.is_some());
    }
}
