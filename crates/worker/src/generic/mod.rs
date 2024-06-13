use crate::{convert_codemod_script_failed, WorkerFailure};
use breezyshim::tree::WorkingTree;
use silver_platter::CommitPending;
use silver_platter::codemod::{
    script_runner as generic_script_runner, CommandResult as GenericCommandResult,
    Error as GenericCodemodError,
};
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
            description: format!("Codemod script {} not found", shlex::try_join(argv.to_vec()).unwrap()),
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
