use breezyshim::tree::WorkingTree;
use crate::debian::DebianBuildConfig;

pub fn build(
    local_tree: &WorkingTree,
    subpath: &std::path::Path,
    output_directory: &std::path::Path,
    committer: Option<&str>,
    update_changelog: crate::debian::DebUpdateChangelog,
    config: &DebianBuildConfig,
) {
    if !local_tree.has_filename(subpath.join("debian/changelog")) {
        return BuildFailure("missing-changelog", "Missing changelog", stage="pre-check")
    }

    let session: Session = if let Some(chroot) = config.chroot {
        SchrootSession(chroot)
    } else {
        PlainSession()
    };

    let source_date_epoch = local_tree.branch().repository().get_revision(
        local_tree.branch.last_revision()
    ).timestamp;

    try:
        with session:
            apt = AptManager(session)
            if let Some(command) = config.command {
                if let Some(last_build_version) = config.last_build_version {
                    // Update the changelog entry with the previous build version;
                    // This allows us to upload incremented versions for subsequent
                    // runs.
                    tree_set_changelog_version(local_tree, last_build_version, subpath)
                }

                try:
                    if let Some(suffix) = config.suffix {
                        (changes_names, cl_entry) = build_once(
                            local_tree,
                            distribution,
                            output_directory,
                            build_command=command,
                            subpath=subpath,
                            source_date_epoch=source_date_epoch,
                            apt_repository=apt_repository,
                            apt_repository_key=apt_repository_key,
                            extra_repositories=extra_repositories,
                        );
                    } else {
                        fixers = default_fixers(
                            local_tree,
                            subpath=subpath,
                            apt=apt,
                            committer=committer,
                            update_changelog=update_changelog,
                            dep_server_url=dep_server_url,
                        )

                        (changes_names, cl_entry) = build_incrementally(
                            local_tree=local_tree,
                            suffix="~" + suffix,
                            build_suite=distribution,
                            output_directory=output_directory,
                            build_command=command,
                            build_changelog_entry="Build for debian-janitor apt repository.",
                            fixers=fixers,
                            subpath=subpath,
                            source_date_epoch=source_date_epoch,
                            max_iterations=MAX_BUILD_ITERATIONS,
                            apt_repository=apt_repository,
                            apt_repository_key=apt_repository_key,
                            extra_repositories=extra_repositories,
                            run_gbp_dch=(update_changelog is False),
                        )
                    }
                except ChangelogNotEditable as e:
                    raise BuildFailure("build-changelog-not-editable", str(e)) from e
                except MissingUpstreamTarball as e:
                    raise BuildFailure(
                        "build-missing-upstream-source",
                        "unable to find upstream source",
                    ) from e
                except MissingChangesFile as e:
                    raise BuildFailure(
                        "build-missing-changes",
                        f"Expected changes path {e.filename} does not exist.",
                        details={"filename": e.filename},
                    ) from e
                except DetailedDebianBuildFailure as e:
                    try:
                        details = e.error.json()
                    except NotImplementedError:
                        details = None
                    raise BuildFailure(
                        e.error.kind, e.description, stage=e.stage, details=details
                    ) from e
                except UnidentifiedDebianBuildError as e:
                    raise BuildFailure("failed", e.description, stage=e.stage) from e
                logging.info("Built %r.", changes_names)
    except OSError as e:
        if e.errno == errno.EMFILE:
            raise BuildFailure(
                "too-many-open-files", str(e), stage="session-setup"
            ) from e
        raise
    except SessionSetupFailure as e:
        if e.errlines:
            sys.stderr.buffer.writelines(e.errlines)
        raise BuildFailure(
            "session-setup-failure",
            str(e),
            stage="session-setup",
        ) from e

    lintian_result = crate::debian::lintian::run_lintian(
        output_directory,
        changes_names,
        profile=lintian_profile,
        suppress_tags=lintian_suppress_tags,
    )
    return {"lintian": lintian_result}
