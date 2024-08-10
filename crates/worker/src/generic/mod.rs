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
use ognibuild::session::{Error as SessionError, Session};
use ognibuild::session::plain::PlainSession;
use ognibuild::session::schroot::SchrootSession;
use ognibuild::build::{run_build, BuildError as OgniBuildError};
use ognibuild::resolver::auto_resolver;
use ognibuild::fixer::InstallFixer;
use ognibuild::log::DirectoryLogManager;
use ognibuild::buildsystems::{BuildSystem,detect_buildsystems};


pub fn generic_make_changes(
    local_tree: &WorkingTree,
    subpath: &Path,
    argv: &[&str],
    env: &HashMap<String, String>,
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
        Some(env.clone()),
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
    _env: &HashMap<String, String>,
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
    let session = if let Some(chroot) = chroot {
        log::info!("Using schroot {}", chroot);
        Box::new(SchrootSession::new(chroot, None).map_err(|e| {
            match e {
                SessionError::SetupFailure(summary, lines) => WorkerFailure {
                    code: "session-setup-failure".to_string(),
                    description: summary,
                    details: Some(lines),
                    stage: vec!["build".to_string()],
                    transient: None,
                },
                _ => unreachable!(),
            }
        })?) as Box<dyn Session>
    } else {
        Box::new(PlainSession::new()) as Box<dyn Session>
    };
    let resolver = auto_resolver(session, dep_server_url=dep_server_url);
    let fixers = vec![InstallFixer(resolver)];
    let (external_dir, internal_dir) = session.setup_from_vcs(local_tree);
    let bss = detect_buildsystems(external_dir.join(subpath)).collect::<Vec<_>>();
    session.chdir(internal_dir.join(subpath));
    let build_log_manager = =DirectoryLogManager::new(
        output_directory.join(BUILD_LOG_FILENAME),
        "redirect",
    );
    match run_build(
            session,
            bss,
            resolver,
            fixers,
            build_log_manager
        ) {
        Err(BuildError::NotImplemented) => {
            return Err(BuildFailure(
                "build-action-unknown", str(e), stage=("build",)
            ) from e

            except NoBuildToolsFound as e:
                raise BuildFailure("no-build-tools-found", str(e)) from e
            except DetailedFailure as f:
                raise BuildFailure(
                    f.error.kind, str(f.error), details={"command": f.argv}
                ) from f
            except UnidentifiedError as e:
                lines = [line for line in e.lines if line]
                if e.secondary:
                    raise BuildFailure("build-failed", e.secondary.line) from e
                elif len(lines) == 1:
                    raise BuildFailure("build-failed", lines[0]) from e
                else:
                    raise BuildFailure(
                        "build-failed",
                        "%r failed with unidentified error "
                        "(return code %d)" % (e.argv, e.retcode),
                    ) from e
                try:
                    run_test(
                        session,
                        buildsystems=bss,
                        resolver=resolver,
                        fixers=fixers,
                        log_manager=DirectoryLogManager(
                            os.path.join(output_directory, "test.log"), "redirect"
                        ),
                    )
                except NotImplementedError as e:
                    traceback.print_exc()
                    raise BuildFailure(
                        "test-action-unknown", str(e), stage=("test",)
                    ) from e

    return {}
}

pub struct GenericTarget {
    env: HashMap<String, String>,
}

impl GenericTarget {
    pub fn new(env: HashMap<String, String>) -> Self {
        Self { env }
    }
}

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
        build_from_config(local_tree, subpath, output_directory, &config, &self.env)
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
        resume_metadata: Option<&serde_json::Value>,
    ) -> Result<Box<dyn silver_platter::CodemodResult>, WorkerFailure> {
        generic_make_changes(
            local_tree,
            subpath,
            argv,
            &self.env,
            log_directory,
            resume_metadata,
        )
        .map(|x| Box::new(x) as Box<dyn silver_platter::CodemodResult>)
    }
}
