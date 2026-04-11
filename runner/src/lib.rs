//! Runner crate for the Janitor project.
//!
//! This crate provides functionality for running code quality checks and tests.

#![deny(missing_docs)]

use breezyshim::RevisionId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use url::Url;

/// Module for handling backchannel communication with the worker.
pub mod backchannel;
/// Module for generating configuration files.
pub mod config_generator;
/// Module for the web interface.
pub mod web;

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
        }
    }
}

#[cfg(feature = "debian")]
impl std::error::Error for FindChangesError {}

#[cfg(feature = "debian")]
/// Summary of changes files.
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
    for entry in std::fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        if !entry.file_name().to_str().unwrap().ends_with(".changes") {
            continue;
        }
        let f = std::fs::File::open(entry.path()).unwrap();
        let changes = debian_control::changes::Changes::read(&f).unwrap();
        names.push(entry.file_name().to_string_lossy().to_string());
        if let Some(version) = &version {
            if changes.version().as_ref() != Some(version) {
                return Err(FindChangesError::InconsistentVersion(
                    names,
                    changes.version().unwrap(),
                    version.clone(),
                ));
            }
        }
        version = changes.version();
        if let Some(source) = &source {
            if changes.source().as_ref() != Some(source) {
                return Err(FindChangesError::InconsistentSource(
                    names,
                    changes.source().unwrap(),
                    source.to_string(),
                ));
            }
        }
        source = changes.source();

        if let Some(distribution) = &distribution {
            if changes.distribution().as_ref() != Some(distribution) {
                return Err(FindChangesError::InconsistentDistribution(
                    names,
                    changes.distribution().unwrap(),
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
                        Some(file.filename.split('_').next().unwrap().to_string())
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
    if parts.last() == Some(&"log") {
        true
    } else if parts.len() == 3 {
        let mut rev = parts.iter().rev();
        rev.next().unwrap().chars().all(char::is_numeric) && rev.next() == Some(&"log")
    } else {
        false
    }
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
    log_id: String,
    /// URL of the branch that was processed.
    branch_url: Url,
    /// Optional subpath within the repository.
    subpath: Option<String>,
    /// Result code.
    code: String,
    /// Whether the result is transient.
    transient: Option<bool>,
    /// Name of the codebase.
    codebase: String,
    /// Name of the campaign.
    campaign: String,
    /// Human-readable description of the result.
    description: String,
    /// Result of the codemod.
    codemod: serde_json::Value,
    /// Optional value associated with the result.
    value: Option<u64>,
    /// Names of log files.
    logfilenames: Vec<String>,

    /// Time when the run started.
    start_time: chrono::DateTime<chrono::Utc>,
    /// Time when the run finished.
    finish_time: chrono::DateTime<chrono::Utc>,
    /// Duration of the run.
    duration: std::time::Duration,

    /// Revision ID of the branch after processing.
    revision: Option<RevisionId>,
    /// Revision ID of the main branch.
    main_branch_revision: Option<RevisionId>,

    /// Optional changeset ID.
    change_set: Option<String>,

    /// Optional tags with revision IDs.
    tags: Option<Vec<(String, Option<RevisionId>)>>,
    /// Optional remote repositories.
    remotes: Option<HashMap<String, ResultRemote>>,

    /// Optional branches information.
    branches: Option<Vec<(String, String, Option<RevisionId>, Option<RevisionId>)>>,

    /// Optional details about the failure.
    failure_details: Option<serde_json::Value>,
    /// Optional stages where failure occurred.
    failure_stage: Option<Vec<String>>,

    /// Optional information about resuming a previous run.
    resume: Option<ResultResume>,

    /// Optional target information.
    target: Option<ResultTarget>,
}

/// Information about resuming a previous run.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResultResume {
    /// ID of the run to resume.
    run_id: String,
}

/// Target information for a result.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResultTarget {
    /// Name of the target.
    name: String,
    /// Additional details about the target.
    details: serde_json::Value,
}

/// Remote repository information for a result.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResultRemote {
    /// URL of the remote repository.
    url: Url,
}

/// Application state for the runner.
pub struct AppState {}

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

    #[test]
    fn test_committer_env_name_only() {
        // Committer with name but no email
        let env = committer_env(Some("John Doe"));
        assert_eq!(env.get("DEBFULLNAME"), Some(&"John Doe".to_string()));
        assert_eq!(env.get("COMMITTER"), Some(&"John Doe".to_string()));
        assert_eq!(env.get("BRZ_EMAIL"), Some(&"John Doe".to_string()));
    }

    #[test]
    fn test_is_log_filename_extensions() {
        // Standard .log files
        assert!(is_log_filename("build.log"));
        assert!(is_log_filename("worker.log"));
        assert!(is_log_filename("a.log"));

        // Rotated logs
        assert!(is_log_filename("build.log.1"));
        assert!(is_log_filename("build.log.42"));

        // Nested number format (e.g., build.1.log)
        assert!(is_log_filename("build.1.log"));

        // Not log files
        assert!(!is_log_filename("build.txt"));
        assert!(!is_log_filename("build.log.bak"));
        assert!(!is_log_filename("build"));
        // ".log" has extension "log", so it is treated as a log file
        assert!(is_log_filename(".log"));
    }

    #[test]
    fn test_gather_logs_with_files() {
        let td = tempfile::tempdir().unwrap();
        // gather_logs looks for *directories* that match is_log_filename
        std::fs::create_dir(td.path().join("build.log")).unwrap();
        std::fs::create_dir(td.path().join("worker.log")).unwrap();
        std::fs::create_dir(td.path().join("not-a-log")).unwrap();
        // Regular file should be ignored
        std::fs::write(td.path().join("output.log"), "content").unwrap();

        let logs: Vec<_> = gather_logs(td.path()).collect();
        let mut names: Vec<String> = logs
            .iter()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        names.sort();
        assert_eq!(names, vec!["build.log", "worker.log"]);
    }

    #[test]
    fn test_gather_logs_empty_directory() {
        let td = tempfile::tempdir().unwrap();
        let logs: Vec<_> = gather_logs(td.path()).collect();
        assert_eq!(logs.len(), 0);
    }

    #[test]
    fn test_janitor_result_serde() {
        let result = JanitorResult {
            log_id: "log-123".to_string(),
            branch_url: url::Url::parse("https://example.com/repo").unwrap(),
            subpath: Some("debian/".to_string()),
            code: "success".to_string(),
            transient: Some(false),
            codebase: "mycodebase".to_string(),
            campaign: "lintian-fixes".to_string(),
            description: "Fixed 3 lintian issues".to_string(),
            codemod: serde_json::json!({"applied": 3}),
            value: Some(30),
            logfilenames: vec!["build.log".to_string(), "worker.log".to_string()],
            start_time: chrono::Utc::now(),
            finish_time: chrono::Utc::now(),
            duration: std::time::Duration::from_secs(120),
            revision: Some(breezyshim::RevisionId::from(b"rev-1".to_vec())),
            main_branch_revision: Some(breezyshim::RevisionId::from(b"rev-0".to_vec())),
            change_set: Some("cs-1".to_string()),
            tags: None,
            remotes: None,
            branches: None,
            failure_details: None,
            failure_stage: None,
            resume: None,
            target: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        let roundtripped: JanitorResult = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtripped.log_id, "log-123");
        assert_eq!(roundtripped.code, "success");
        assert_eq!(roundtripped.codebase, "mycodebase");
        assert_eq!(roundtripped.campaign, "lintian-fixes");
        assert_eq!(roundtripped.value, Some(30));
        assert_eq!(roundtripped.logfilenames, vec!["build.log", "worker.log"]);
    }

    #[test]
    fn test_janitor_result_with_failure() {
        let result = JanitorResult {
            log_id: "log-456".to_string(),
            branch_url: url::Url::parse("https://example.com/repo").unwrap(),
            subpath: None,
            code: "build-failed".to_string(),
            transient: Some(true),
            codebase: "failcodebase".to_string(),
            campaign: "fresh-releases".to_string(),
            description: "Build failed".to_string(),
            codemod: serde_json::json!(null),
            value: None,
            logfilenames: vec![],
            start_time: chrono::Utc::now(),
            finish_time: chrono::Utc::now(),
            duration: std::time::Duration::from_secs(30),
            revision: None,
            main_branch_revision: None,
            change_set: None,
            tags: None,
            remotes: None,
            branches: None,
            failure_details: Some(serde_json::json!({"error": "compilation failed"})),
            failure_stage: Some(vec!["build".to_string()]),
            resume: None,
            target: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        let roundtripped: JanitorResult = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtripped.code, "build-failed");
        assert_eq!(roundtripped.transient, Some(true));
        assert!(roundtripped.failure_details.is_some());
        assert_eq!(roundtripped.failure_stage, Some(vec!["build".to_string()]));
    }

    #[test]
    fn test_janitor_result_with_resume() {
        let result = JanitorResult {
            log_id: "log-789".to_string(),
            branch_url: url::Url::parse("https://example.com/repo").unwrap(),
            subpath: None,
            code: "success".to_string(),
            transient: None,
            codebase: "test".to_string(),
            campaign: "test".to_string(),
            description: "OK".to_string(),
            codemod: serde_json::json!({}),
            value: None,
            logfilenames: vec![],
            start_time: chrono::Utc::now(),
            finish_time: chrono::Utc::now(),
            duration: std::time::Duration::from_secs(10),
            revision: None,
            main_branch_revision: None,
            change_set: None,
            tags: None,
            remotes: Some(maplit::hashmap! {
                "origin".to_string() => ResultRemote {
                    url: url::Url::parse("https://example.com/origin").unwrap(),
                }
            }),
            branches: Some(vec![(
                "main".to_string(),
                "refs/heads/main".to_string(),
                None,
                None,
            )]),
            failure_details: None,
            failure_stage: None,
            resume: Some(ResultResume {
                run_id: "prev-run".to_string(),
            }),
            target: Some(ResultTarget {
                name: "debian".to_string(),
                details: serde_json::json!({"dist": "unstable"}),
            }),
        };
        let json = serde_json::to_string(&result).unwrap();
        let roundtripped: JanitorResult = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtripped.resume.as_ref().unwrap().run_id, "prev-run");
        assert_eq!(roundtripped.target.as_ref().unwrap().name, "debian");
        assert!(roundtripped.remotes.is_some());
        assert!(roundtripped.branches.is_some());
    }

    #[test]
    fn test_find_changes_error_display() {
        let err = FindChangesError::NoChangesFile(std::path::PathBuf::from("/tmp/output"));
        assert_eq!(err.to_string(), "No changes file found in /tmp/output");

        let err = FindChangesError::MissingChangesFileFields("Source");
        assert_eq!(err.to_string(), "Missing field Source in changes files");
    }
}
