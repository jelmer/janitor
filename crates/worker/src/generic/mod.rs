use crate::{convert_codemod_script_failed, WorkerFailure};
use breezyshim::tree::WorkingTree;
use silver_platter::codemod::{
    script_runner as generic_script_runner, CommandResult as GenericCommandResult,
    Error as GenericCodemodError,
};
use silver_platter::CommitPending;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

pub fn generic_make_changes(
    local_tree: &WorkingTree,
    subpath: &Path,
    argv: &[&str],
    env: HashMap<String, String>,
    log_directory: &Path,
    resume_metadata: Option<&serde_json::Value>,
) -> std::result::Result<GenericCommandResult, WorkerFailure> {
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

    let codemod_log_path = log_directory.join("codemod.log");

    let f = File::create(codemod_log_path).unwrap();

    let committer = env.get("COMMITTER").map(|s| s.to_string());

    match generic_script_runner(
        local_tree,
        argv,
        subpath,
        CommitPending::Auto,
        resume_metadata,
        committer.as_deref(),
        Some(env),
        f.into(),
    ) {
        Ok(r) => Ok(r),
        Err(GenericCodemodError::ScriptMadeNoChanges) => Err(WorkerFailure {
            code: "nothing-to-do".to_string(),
            description: "No changes made".to_string(),
            details: None,
            stage: vec!["codemod".to_string()],
            transient: Some(false),
        }),
        Err(GenericCodemodError::ExitCode(i)) => Err(convert_codemod_script_failed(
            i,
            shlex::try_join(argv.to_vec()).unwrap().as_str(),
        )),
        Err(GenericCodemodError::ScriptNotFound) => Err(WorkerFailure {
            code: "codemod-not-found".to_string(),
            description: format!(
                "Codemod script {} not found",
                shlex::try_join(argv.to_vec()).unwrap()
            ),
            details: None,
            stage: vec!["codemod".to_string()],
            transient: None,
        }),
        Err(GenericCodemodError::Detailed(f)) => {
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
        Err(GenericCodemodError::Io(e)) => Err(WorkerFailure {
            code: "io-error".to_string(),
            description: format!("IO error: {}", e),
            details: None,
            stage: vec!["codemod".to_string()],
            transient: None,
        }),
        Err(GenericCodemodError::Json(e)) => Err(WorkerFailure {
            code: "result-file-format".to_string(),
            description: format!("JSON error: {}", e),
            details: None,
            stage: vec!["codemod".to_string()],
            transient: None,
        }),
        Err(GenericCodemodError::Utf8(e)) => Err(WorkerFailure {
            code: "utf8-error".to_string(),
            description: format!("UTF8 error: {}", e),
            details: None,
            stage: vec!["codemod".to_string()],
            transient: None,
        }),
        Err(GenericCodemodError::Other(e)) => Err(WorkerFailure {
            code: "unknown-error".to_string(),
            description: format!("Unknown error: {}", e),
            details: None,
            stage: vec!["codemod".to_string()],
            transient: None,
        }),
    }
}

pub fn build_from_config(
    local_tree: &WorkingTree,
    subpath: &std::path::Path,
    output_directory: &std::path::Path,
    config: &serde_json::Value,
    _env: HashMap<String, String>,
) -> Result<serde_json::Value, WorkerFailure> {
    let chroot = config.get("chroot").and_then(|v| v.as_str());
    let dep_server_url = config.get("dep_server_url").and_then(|v| v.as_str());

    build(
        local_tree,
        subpath,
        output_directory,
        chroot,
        dep_server_url,
    )
}

pub fn build(
    local_tree: &WorkingTree,
    subpath: &std::path::Path,
    output_directory: &std::path::Path,
    chroot: Option<&str>,
    dep_server_url: Option<&str>,
) -> Result<serde_json::Value, WorkerFailure> {
    pyo3::import_exception!(janitor.generic.build, BuildFailure);
    pyo3::Python::with_gil(|py| -> Result<serde_json::Value, WorkerFailure> {
        use pyo3::prelude::*;
        let m = py.import_bound("janitor.generic.build").unwrap();
        let build = m.getattr("build").unwrap();

        match build.call1((
            local_tree.to_object(py),
            subpath,
            output_directory,
            chroot,
            dep_server_url,
        )) {
            Ok(_) => Ok(serde_json::Value::Null),
            Err(e) => {
                if e.is_instance_of::<BuildFailure>(py) {
                    let value = e.value_bound(py);
                    let code = value.getattr("code").unwrap().extract::<String>().unwrap();
                    let description = value
                        .getattr("description")
                        .unwrap()
                        .extract::<String>()
                        .unwrap();
                    let details = value
                        .getattr("details")
                        .unwrap()
                        .extract::<Option<PyObject>>()
                        .unwrap()
                        .map(|x| crate::py_to_serde_json(x.bind(py)).unwrap());
                    let stage = value
                        .getattr("stage")
                        .unwrap()
                        .extract::<Vec<String>>()
                        .unwrap();
                    Err(WorkerFailure {
                        code,
                        description,
                        details,
                        stage: vec!["build".to_string()].into_iter().chain(stage).collect(),
                        transient: None,
                    })
                } else {
                    Err(WorkerFailure {
                        code: "internal-error".to_string(),
                        description: e.to_string(),
                        details: None,
                        stage: vec!["build".to_string()],
                        transient: None,
                    })
                }
            }
        }
    })
}

pub struct GenericTarget {}

impl crate::Target for GenericTarget {
    fn name(&self) -> String {
        "generic".to_string()
    }

    fn build(
        &self,
        local_tree: &WorkingTree,
        subpath: &std::path::Path,
        output_directory: &std::path::Path,
        config: &crate::BuildConfig,
    ) -> Result<serde_json::Value, WorkerFailure> {
        build_from_config(
            local_tree,
            subpath,
            output_directory,
            &config,
            HashMap::new(),
        )
    }

    fn validate(
        &self,
        _local_tree: &WorkingTree,
        _subpath: &std::path::Path,
        _config: &crate::ValidateConfig,
    ) -> Result<(), WorkerFailure> {
        Ok(())
    }

    fn make_changes(
        &self,
        local_tree: &WorkingTree,
        subpath: &std::path::Path,
        argv: &[&str],
        log_directory: &std::path::Path,
        resume_metadata: Option<&crate::Metadata>,
    ) -> Result<serde_json::Value, WorkerFailure> {
        generic_make_changes(
            local_tree,
            subpath,
            argv,
            HashMap::new(),
            log_directory,
            resume_metadata
                .as_ref()
                .map(|x| serde_json::to_value(x).unwrap())
                .as_ref(),
        )
        .map(|x| serde_json::to_value(&x).unwrap())
    }
}
