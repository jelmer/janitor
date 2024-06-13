pub mod lintian;

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

pub fn debian_make_changes(
    local_tree: &WorkingTree,
    subpath: &Path,
    argv: &[&str],
    env: HashMap<String, String>,
    log_directory: &Path,
    resume_metadata: Option<&serde_json::Value>,
    committer: Option<&str>,
    update_changelog: Option<bool>,
) -> std::result::Result<DebianCommandResult, WorkerFailure> {
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
        let sys = py.import("sys").unwrap();
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
        let sys = py.import("sys").unwrap();
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
        update_changelog,
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
