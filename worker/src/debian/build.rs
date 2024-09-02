use crate::debian::BuildFailure;
use crate::debian::DebianBuildConfig;
use breezyshim::tree::{Tree, WorkingTree};
use ognibuild::debian::build::{BuildOnceError, BuildOnceResult};
use ognibuild::debian::context::Phase;
use ognibuild::debian::fix_build::IterateBuildError;
use ognibuild::session::Session;

pub fn build(
    local_tree: &WorkingTree,
    subpath: &std::path::Path,
    output_directory: &std::path::Path,
    committer: Option<&str>,
    update_changelog: crate::debian::DebUpdateChangelog,
    config: &DebianBuildConfig,
) -> Result<DebianBuildResult, BuildFailure> {
    if !local_tree.has_filename(&subpath.join("debian/changelog")) {
        return Err(BuildFailure {
            code: "missing-changelog".to_string(),
            description: "Missing changelog".to_string(),
            stage: vec!["pre-check".to_string()],
            details: None,
        });
    }

    #[cfg(target_os = "linux")]
    let session: Box<dyn Session> = if let Some(chroot) = config.chroot {
        Box::new(
            ognibuild::session::schroot::SchrootSession::new(chroot, Some("janitor-worker"))
                .map_err(|e| match e {
                    ognibuild::session::Error::SetupFailure(n, e) => {
                        return Err(BuildFailure {
                            code: "session-setup-failure".to_string(),
                            description: format!("Error setting up schroot session: {}", e),
                            details: None,
                            stage: vec![],
                        });
                    }
                    e => unreachable!(),
                })?,
        ) as Box<dyn Session>
    } else {
        Box::new(ognibuild::session::plain::PlainSession::new()) as Box<dyn Session>
    };

    #[cfg(not(target_os = "linux"))]
    let session: Box<dyn Session> = if config.chroot.is_some() {
        return Err(BuildFailure {
            code: "unsupported-schroot".to_string(),
            description: "Schroot is not supported on this platform".to_string(),
            stage: vec!["pre-check".to_string()],
            details: None,
        });
    } else {
        Box::new(ognibuild::session::plain::PlainSession::new()) as Box<dyn Session>
    };

    let source_date_epoch = local_tree
        .branch()
        .repository()
        .get_revision(&local_tree.branch().last_revision())
        .unwrap()
        .datetime();

    let source_date_epoch = source_date_epoch.to_utc();

    let apt = ognibuild::debian::apt::AptManager::new(session.as_ref(), None);
    if let Some(command) = config.build_command.as_ref() {
        if let Some(last_build_version) = config.last_build_version.as_ref() {
            // Update the changelog entry with the previous build version;
            // This allows us to upload incremented versions for subsequent
            // runs.
            crate::debian::tree_set_changelog_version(local_tree, &last_build_version, subpath)
                .unwrap();
        }

        let result: Result<BuildOnceResult, IterateBuildError> =
            if let Some(suffix) = config.build_suffix.as_ref() {
                let packaging_context = ognibuild::debian::context::DebianPackagingContext::new(
                    local_tree.clone(),
                    subpath,
                    committer.map(|c| breezyshim::config::parse_username(c)),
                    update_changelog == crate::debian::DebUpdateChangelog::Update,
                    Box::new(breezyshim::commit::NullCommitReporter::new()),
                );
                let fixers = ognibuild::debian::fixers::default_fixers(&packaging_context, &apt);

                ognibuild::debian::fix_build::build_incrementally(
                    local_tree,
                    config
                        .build_suffix
                        .as_ref()
                        .map(|s| format!("~{}", s))
                        .as_deref(),
                    config.build_distribution.as_deref(),
                    output_directory,
                    &command,
                    fixers
                        .iter()
                        .map(|f| f.as_ref())
                        .collect::<Vec<_>>()
                        .as_slice(),
                    Some("Build for debian-janitor apt repository."),
                    Some(crate::debian::MAX_BUILD_ITERATIONS),
                    subpath,
                    Some(source_date_epoch),
                    config.apt_repository.as_deref(),
                    config.apt_repository_key.as_deref(),
                    config
                        .extra_repositories
                        .as_ref()
                        .map(|m| m.iter().map(|r| r.as_str()).collect::<Vec<_>>()),
                    update_changelog == crate::debian::DebUpdateChangelog::Leave,
                )
            } else {
                ognibuild::debian::build::build_once(
                    local_tree,
                    config.build_distribution.as_deref(),
                    output_directory,
                    &command,
                    subpath,
                    Some(source_date_epoch),
                    config.apt_repository.as_deref(),
                    config.apt_repository_key.as_deref(),
                    config
                        .extra_repositories
                        .as_ref()
                        .map(|m| m.iter().map(|r| r.as_str()).collect::<Vec<_>>())
                        .as_ref(),
                )
                .map_err(|e| match e {
                    BuildOnceError::Detailed {
                        stage: _,
                        phase,
                        retcode: _,
                        command: _,
                        error,
                        description: _,
                    } => IterateBuildError::Persistent(phase.unwrap(), error),
                    BuildOnceError::Unidentified {
                        stage: _,
                        phase,
                        retcode,
                        command,
                        description,
                    } => IterateBuildError::Unidentified {
                        retcode,
                        lines: todo!(),
                        secondary: todo!(),
                        phase,
                    },
                })
            };

        let build_result = match result {
            Ok(result) => result,
            Err(IterateBuildError::ResetTree(e)) => {
                return Err(BuildFailure {
                    code: "reset-tree".to_string(),
                    description: format!("Error resetting tree: {}", e),
                    stage: vec!["build".to_string()],
                    details: None,
                });
            }
            Err(IterateBuildError::Persistent(phase, e)) => {
                let mut stage = vec!["build".to_string()];
                match phase {
                    Phase::Build => stage.push("build".to_string()),
                    Phase::BuildEnv => stage.push("build-env".to_string()),
                    Phase::AutoPkgTest(name) => {
                        stage.push("autopkgtest".to_string());
                        stage.push(name);
                    }
                    Phase::CreateSession => stage.push("create-session".to_string()),
                }
                return Err(BuildFailure {
                    code: e.kind().to_string(),
                    description: e.to_string(),
                    stage,
                    details: None,
                });
            }
            Err(IterateBuildError::MissingPhase) => {
                return Err(BuildFailure {
                    code: "missing-phase".to_string(),
                    description: "Missing phase".to_string(),
                    stage: vec!["build".to_string()],
                    details: None,
                });
            }
            Err(IterateBuildError::FixerLimitReached(n)) => {
                return Err(BuildFailure {
                    code: "fixer-limit-reached".to_string(),
                    description: format!("Fixer limit reached: {}", n),
                    stage: vec!["build".to_string()],
                    details: None,
                });
            }
            Err(IterateBuildError::Other(o)) => {
                return Err(BuildFailure {
                    code: "other".to_string(),
                    description: format!("Other error: {}", o),
                    stage: vec!["build".to_string()],
                    details: None,
                });
            }
            Err(IterateBuildError::Unidentified {
                retcode: _,
                lines,
                secondary,
                phase,
            }) => {
                return Err(BuildFailure {
                    code: "unidentified".to_string(),
                    description: format!("Unidentified error: {}", lines.join("\n")),
                    stage: vec!["build".to_string()],
                    details: None,
                });
            }
        };
        log::info!("Built {:?}.", build_result.changes_names);

        let lintian_result = crate::debian::lintian::run_lintian(
            output_directory,
            build_result
                .changes_names
                .iter()
                .map(|s| s.as_path())
                .collect(),
            config.lintian.profile.as_deref(),
            config
                .lintian
                .suppress_tags
                .as_ref()
                .map(|tags| tags.iter().map(|tag| tag.as_str()).collect()),
        )
        .map_err(|e| BuildFailure {
            code: "lintian".to_string(),
            description: format!("Error running lintian: {}", e),
            stage: vec!["lintian".to_string()],
            details: None,
        })?;
        Ok(DebianBuildResult {
            lintian: lintian_result,
        })
    } else {
        Ok(DebianBuildResult {
            lintian: crate::debian::lintian::LintianResult::default(),
        })
    }
}

#[derive(serde::Serialize)]
pub struct DebianBuildResult {
    lintian: crate::debian::lintian::LintianResult,
}
