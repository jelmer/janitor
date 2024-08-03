pub mod lintian;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::{convert_codemod_script_failed, WorkerFailure};
use breezyshim::tree::{Tree, WorkingTree};
use silver_platter::debian::codemod::{
    script_runner as debian_script_runner, CommandResult as DebianCommandResult,
    Error as DebianCodemodError,
};
use silver_platter::CommitPending;
use std::collections::HashMap;
use std::fs::File;

use std::path::Path;

pub const MAX_BUILD_ITERATIONS: usize = 50;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum DebUpdateChangelog {
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

pub fn debian_make_changes(
    local_tree: &WorkingTree,
    subpath: &Path,
    argv: &[&str],
    env: HashMap<String, String>,
    log_directory: &Path,
    resume_metadata: Option<&serde_json::Value>,
    committer: Option<&str>,
    update_changelog: DebUpdateChangelog,
) -> std::result::Result<DebianCommandResult, WorkerFailure> {
    use pyo3::prelude::*;
    if argv.is_empty() {
        return Err(WorkerFailure {
            code: "no-changes".to_string(),
            description: "No change build".to_string(),
            details: None,
            stage: vec!["codemod".to_string()],
            transient: None,
        });
    }

    log::info!("Running {:?}", argv);

    // TODO(jelmer): This is only necessary for deb-new-upstream
    let sys_path = pyo3::Python::with_gil(|py| {
        let sys = py.import_bound("sys").unwrap();
        Ok::<String, pyo3::PyErr>(
            sys.getattr("path")
                .unwrap()
                .extract::<Vec<String>>()
                .unwrap()
                .join(":"),
        )
    })
    .unwrap();

    let sys_executable = pyo3::Python::with_gil(|py| {
        let sys = py.import_bound("sys").unwrap();
        Ok::<String, pyo3::PyErr>(
            sys.getattr("executable")
                .unwrap()
                .extract::<String>()
                .unwrap(),
        )
    })
    .unwrap();

    let mut dist_command = format!(
        "PYTHONPATH={} {} -m janitor.debian.dist --log-directory={} '",
        sys_path,
        sys_executable,
        log_directory.display()
    );

    if let Some(chroot) = env.get("CHROOT") {
        dist_command = format!("SCHROOT={} {}", chroot, dist_command);
    }

    let debian_path = subpath.join("debian");

    if local_tree.has_filename(&debian_path) {
        dist_command.push_str(
            format!(
                " --packaging={}",
                local_tree.abspath(&debian_path).unwrap().display()
            )
            .as_str(),
        );
    }

    // Prevent 404s because files have gone away:
    dist_command.push_str(" --apt-update --apt-dist-upgrade");

    let mut extra_env = HashMap::new();
    extra_env.insert("DIST".to_string(), dist_command);
    for (k, v) in env {
        extra_env.insert(k, v);
    }

    let codemod_log_path = log_directory.join("codemod.log");

    let f = File::create(codemod_log_path).unwrap();

    match debian_script_runner(
        local_tree,
        argv,
        subpath,
        CommitPending::Auto,
        resume_metadata,
        committer,
        Some(extra_env),
        f.into(),
        match update_changelog {
            DebUpdateChangelog::Auto => None,
            DebUpdateChangelog::Update => Some(true),
            DebUpdateChangelog::Leave => Some(false),
        },
    ) {
        Ok(r) => Ok(r),
        Err(DebianCodemodError::ScriptMadeNoChanges) => Err(WorkerFailure {
            code: "nothing-to-do".to_string(),
            description: "No changes made".to_string(),
            details: None,
            stage: vec!["codemod".to_string()],
            transient: Some(false),
        }),
        Err(DebianCodemodError::MissingChangelog(p)) => Err(WorkerFailure {
            code: "missing-changelog".to_string(),
            description: format!("No changelog present: {}", p.display()),
            details: None,
            stage: vec!["codemod".to_string()],
            transient: Some(false),
        }),
        Err(DebianCodemodError::ExitCode(i)) => Err(convert_codemod_script_failed(
            i,
            shlex::try_join(argv.to_vec()).unwrap().as_str(),
        )),
        Err(DebianCodemodError::ScriptNotFound) => Err(WorkerFailure {
            code: "codemod-not-found".to_string(),
            description: format!(
                "Codemod script {} not found",
                shlex::try_join(argv.to_vec()).unwrap()
            ),
            details: None,
            stage: vec!["codemod".to_string()],
            transient: None,
        }),
        Err(DebianCodemodError::Detailed(f)) => {
            let mut stage = vec!["codemod".to_string()];
            if let Some(extra_stage) = f.stage {
                stage.extend(extra_stage);
            }
            Err(WorkerFailure {
                code: f.result_code,
                description: f
                    .description
                    .unwrap_or_else(|| "Codemod failed".to_string()),
                details: f.details,
                stage,
                transient: None,
            })
        }
        Err(DebianCodemodError::Io(e)) => Err(WorkerFailure {
            code: "io-error".to_string(),
            description: format!("IO error: {}", e),
            details: None,
            stage: vec!["codemod".to_string()],
            transient: None,
        }),
        Err(DebianCodemodError::Json(e)) => Err(WorkerFailure {
            code: "result-file-format".to_string(),
            description: format!("JSON error: {}", e),
            details: None,
            stage: vec!["codemod".to_string()],
            transient: None,
        }),
        Err(DebianCodemodError::Utf8(e)) => Err(WorkerFailure {
            code: "utf8-error".to_string(),
            description: format!("UTF8 error: {}", e),
            details: None,
            stage: vec!["codemod".to_string()],
            transient: None,
        }),
        Err(DebianCodemodError::ChangelogParse(e)) => Err(WorkerFailure {
            code: "changelog-parse-error".to_string(),
            description: format!("Changelog parse error: {}", e),
            details: None,
            stage: vec!["codemod".to_string()],
            transient: None,
        }),
        Err(DebianCodemodError::Other(e)) => Err(WorkerFailure {
            code: "unknown-error".to_string(),
            description: format!("Unknown error: {}", e),
            details: None,
            stage: vec!["codemod".to_string()],
            transient: None,
        }),
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct Lintian {
    profile: Option<String>,
    #[serde(rename = "suppress-tags")]
    suppress_tags: Option<Vec<String>>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct DebianBuildConfig {
    #[serde(rename = "build-distribution")]
    build_distribution: Option<String>,
    #[serde(rename = "build-command")]
    build_command: Option<String>,
    #[serde(rename = "build-suffix")]
    build_suffix: Option<String>,
    #[serde(rename = "last-build-version")]
    last_build_version: Option<String>,
    chroot: Option<String>,
    lintian: Lintian,
    #[serde(rename = "base-apt-repository")]
    apt_repository: Option<String>,
    #[serde(rename = "base-apt-repository-signed-by")]
    apt_repository_key: Option<String>,
    #[serde(rename = "build-extra-repositories")]
    extra_repositories: Option<Vec<String>>,
    #[serde(rename = "dep_server_url")]
    dep_server_url: Option<String>,
}

#[derive(Debug)]
pub struct BuildFailure {
    pub code: String,
    pub description: String,
    pub details: Option<serde_json::Value>,
    pub stage: Vec<String>,
}

impl std::fmt::Display for BuildFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.description)
    }
}

impl std::error::Error for BuildFailure {}

fn build(
    local_tree: &WorkingTree,
    subpath: &std::path::Path,
    output_directory: &std::path::Path,
    committer: Option<&str>,
    update_changelog: DebUpdateChangelog,
    config: &DebianBuildConfig,
) -> Result<serde_json::Value, BuildFailure> {
    pyo3::import_exception!(janitor.debian.build, BuildFailure);
    Python::with_gil(|py| {
        let m = py.import_bound("janitor.debian.build").unwrap();
        let build = m.getattr("build").unwrap();

        let kwargs = PyDict::new_bound(py);

        kwargs.set_item("local_tree", local_tree).unwrap();
        kwargs.set_item("subpath", subpath).unwrap();
        kwargs
            .set_item("output_directory", output_directory)
            .unwrap();

        kwargs.set_item("chroot", config.chroot.as_ref()).unwrap();
        kwargs
            .set_item(
                "command",
                config
                    .build_command
                    .as_ref()
                    .map_or("sbuild -A -s -v".to_string(), |x| x.clone()),
            )
            .unwrap();
        kwargs.set_item("suffix", &config.build_suffix).unwrap();
        kwargs
            .set_item("distribution", &config.build_distribution)
            .unwrap();
        kwargs
            .set_item("last_build_version", &config.last_build_version)
            .unwrap();
        kwargs
            .set_item("lintian_profile", &config.lintian.profile)
            .unwrap();
        kwargs
            .set_item("lintian_suppress_tags", &config.lintian.suppress_tags)
            .unwrap();
        kwargs.set_item("committer", committer).unwrap();
        kwargs
            .set_item("apt_repository", &config.apt_repository)
            .unwrap();
        kwargs
            .set_item("apt_repository_key", &config.apt_repository_key)
            .unwrap();
        kwargs
            .set_item("extra_repositories", &config.extra_repositories)
            .unwrap();
        kwargs
            .set_item(
                "update_changelog",
                match update_changelog {
                    DebUpdateChangelog::Auto => None,
                    DebUpdateChangelog::Update => Some(true),
                    DebUpdateChangelog::Leave => Some(false),
                },
            )
            .unwrap();
        kwargs
            .set_item("dep_server_url", &config.dep_server_url)
            .unwrap();

        build
            .call((), Some(&kwargs))
            .map(|x| crate::py_to_serde_json(&x).unwrap())
            .map_err(|e| {
                if e.is_instance_of::<BuildFailure>(py) {
                    let value = e.value_bound(py);
                    let details: Option<PyObject> =
                        value.getattr("details").unwrap().extract().unwrap();
                    crate::debian::BuildFailure {
                        code: value.getattr("code").unwrap().extract().unwrap(),
                        description: value.getattr("description").unwrap().extract().unwrap(),
                        details: details.map(|x| crate::py_to_serde_json(x.bind(py)).unwrap()),
                        stage: value.getattr("stage").unwrap().extract().unwrap(),
                    }
                } else {
                    panic!("Unexpected exception: {:?}", e)
                }
            })
    })
}

pub fn build_from_config(
    local_tree: &WorkingTree,
    subpath: &std::path::Path,
    output_directory: &std::path::Path,
    config: &serde_json::Value,
    env: &std::collections::HashMap<String, String>,
) -> Result<serde_json::Value, WorkerFailure> {
    let config: DebianBuildConfig = serde_json::from_value(config.clone()).unwrap();
    let committer = env.get("COMMITTER");
    let update_changelog: DebUpdateChangelog = match env
        .get("DEB_UPDATE_CHANGELOG")
        .map_or("auto", |x| x.as_str())
        .parse()
    {
        Ok(x) => x,
        Err(e) => {
            log::warn!(
                "Invalid value for DEB_UPDATE_CHANGELOG: {}, defaulting to auto.",
                e
            );
            DebUpdateChangelog::Auto
        }
    };
    build(
        local_tree,
        subpath,
        output_directory,
        committer.as_ref().map(|x| x.as_str()),
        update_changelog,
        &config,
    )
    .map_err(|e| WorkerFailure {
        code: e.code,
        description: e.description,
        details: e.details,
        stage: vec!["build".to_string()]
            .into_iter()
            .chain(e.stage)
            .collect(),
        transient: None,
    })
}

pub struct DebianTarget {
    env: HashMap<String, String>,
    committer: Option<String>,
    update_changelog: DebUpdateChangelog,
}

impl DebianTarget {
    pub fn new(env: HashMap<String, String>) -> Self {
        let committer = env.get("COMMITTER").cloned();
        let update_changelog = match env
            .get("DEB_UPDATE_CHANGELOG")
            .map_or("auto", |x| x.as_str())
            .parse()
        {
            Ok(x) => x,
            Err(e) => {
                log::warn!(
                    "Invalid value for DEB_UPDATE_CHANGELOG: {}, defaulting to auto.",
                    e
                );
                DebUpdateChangelog::Auto
            }
        };
        Self {
            env,
            update_changelog,
            committer,
        }
    }
}

impl crate::Target for DebianTarget {
    fn name(&self) -> String {
        "debian".to_string()
    }

    fn build(
        &self,
        local_tree: &WorkingTree,
        subpath: &std::path::Path,
        output_directory: &std::path::Path,
        config: &crate::BuildConfig,
    ) -> Result<serde_json::Value, WorkerFailure> {
        build_from_config(local_tree, subpath, output_directory, config, &self.env)
    }

    fn validate(
        &self,
        local_tree: &WorkingTree,
        subpath: &std::path::Path,
        config: &crate::ValidateConfig,
    ) -> Result<(), WorkerFailure> {
        validate_from_config(local_tree, subpath, config).map_err(|e| WorkerFailure {
            code: e.code,
            description: e.description,
            details: None,
            stage: vec!["validate".to_string()],
            transient: None,
        })
    }

    fn make_changes(
        &self,
        local_tree: &WorkingTree,
        subpath: &std::path::Path,
        argv: &[&str],
        log_directory: &std::path::Path,
        resume_metadata: Option<&crate::Metadata>,
    ) -> Result<serde_json::Value, WorkerFailure> {
        debian_make_changes(
            local_tree,
            subpath,
            argv,
            self.env.clone(),
            log_directory,
            resume_metadata
                .as_ref()
                .map(|x| serde_json::to_value(x).unwrap())
                .as_ref(),
            self.committer.as_deref(),
            self.update_changelog,
        )
        .map(|x| serde_json::to_value(&x).unwrap())
    }
}

#[derive(Debug)]
struct ValidateError {
    code: String,
    description: String,
}

impl std::fmt::Display for ValidateError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.description)
    }
}

impl std::error::Error for ValidateError {}

fn validate_from_config(
    local_tree: &WorkingTree,
    subpath: &std::path::Path,
    config: &serde_json::Value,
) -> Result<(), ValidateError> {
    let config: DebianBuildConfig = serde_json::from_value(config.clone()).unwrap();
    if let Some(apt_repository) = config.apt_repository.as_ref() {
        let apt = breezyshim::debian::apt::RemoteApt::from_string(
            apt_repository,
            config
                .apt_repository_key
                .as_deref()
                .map(std::path::Path::new),
        )
        .unwrap();
        match breezyshim::debian::vcs_up_to_date::check_up_to_date(local_tree, subpath, &apt)
            .unwrap()
        {
            breezyshim::debian::vcs_up_to_date::UpToDateStatus::UpToDate => {}
            breezyshim::debian::vcs_up_to_date::UpToDateStatus::MissingChangelog => {
                if !local_tree.has_filename(&subpath.join("debian")) {
                    return Err(ValidateError {
                        code: "not-debian-package".to_string(),
                        description: "Not a Debian package".to_string(),
                    });
                } else {
                    return Err(ValidateError {
                        code: "missing-changelog".to_string(),
                        description: "Missing changelog".to_string(),
                    });
                }
            }
            breezyshim::debian::vcs_up_to_date::UpToDateStatus::PackageMissingInArchive {
                package,
            } => {
                log::warn!("Package {} is not present in archive", package);
            }
            breezyshim::debian::vcs_up_to_date::UpToDateStatus::TreeVersionNotInArchive {
                tree_version,
                archive_versions: _,
            } => {
                log::warn!(
                    "Last tree version {} not present in the archive",
                    tree_version
                );
            }
            breezyshim::debian::vcs_up_to_date::UpToDateStatus::NewArchiveVersion {
                archive_version,
                tree_version,
            } => {
                return Err(ValidateError {
                    code: "new-archive-version".to_string(),
                    description: format!(
                        "New archive version {} (last tree version {})",
                        archive_version, tree_version
                    ),
                })
            }
        }
    }
    Ok(())
}
