use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub mod config_generator;
pub mod queue;

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
pub enum FindChangesError {
    NoChangesFile(PathBuf),
    InconsistentVersion(Vec<String>, debversion::Version, debversion::Version),
    InconsistentSource(Vec<String>, String, String),
    InconsistentDistribution(Vec<String>, String, String),
    MissingChangesFileFields(&'static str),
}

pub fn find_changes(
    path: &Path,
) -> Result<
    (
        Vec<String>,
        String,
        debversion::Version,
        String,
        Vec<String>,
    ),
    FindChangesError,
> {
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

    Ok((
        names,
        source.unwrap(),
        version.unwrap(),
        distribution.unwrap(),
        binary_packages,
    ))
}

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
}
