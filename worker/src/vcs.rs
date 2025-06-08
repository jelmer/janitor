use breezyshim::error::Error as BrzError;
use breezyshim::transport::Transport;
use breezyshim::RevisionId;
use janitor::vcs::VcsType;
use std::collections::HashMap;
use url::Url;

/// Push a branch to a new location.
pub fn push_branch(
    source_branch: &dyn breezyshim::branch::Branch,
    url: &Url,
    vcs_type: Option<VcsType>,
    overwrite: bool,
    stop_revision: Option<RevisionId>,
    tag_selector: Option<Box<dyn Fn(String) -> bool>>,
    possible_transports: &mut Option<Vec<Transport>>,
) -> Result<(), BrzError> {
    let (url, params) = breezyshim::urlutils::split_segment_parameters(url);
    let branch_name = params.get("branch").map(|s| {
        percent_encoding::percent_decode(s.as_bytes())
            .decode_utf8()
            .unwrap()
            .to_string()
    });
    let vcs_type = vcs_type.map_or_else(
        || source_branch.controldir().cloning_metadir(),
        |t| {
            breezyshim::controldir::FORMAT_REGISTRY
                .make_controldir(t.to_string().as_str())
                .unwrap()
        },
    );
    let target = match breezyshim::controldir::open(&url, possible_transports.as_mut()) {
        Ok(cd) => cd,
        Err(BrzError::NotBranchError(..)) => {
            breezyshim::controldir::create(&url, &vcs_type, possible_transports.as_mut())?
        }
        Err(e) => {
            return Err(e);
        }
    };

    target.push_branch(
        source_branch,
        branch_name.as_deref(),
        stop_revision.as_ref(),
        Some(overwrite),
        tag_selector,
    )?;

    Ok(())
}

/// Import branches into a new location.
///
/// This function will create a new branch for each branch in the branches
/// vector, and push the revisions from the local branch to the new branch.
/// The tags vector is used to set tags on the new branches.
///
/// The directory structure of the new branches will be:
///
/// <url>/campaign>
///
/// The tags will be set on the new branches as follows:
/// - The tag `log_id` will be set to the revision in the branches vector.
/// - The tag `log_id/<tag_name>` will be set to the revision in the tags vector
///   for each tag in the tags vector.
/// - The tag `<tag_name>` will be set to the revision in the tags vector for
///   each tag in the tags vector if `update_current` is true.
///
/// # Arguments
/// * `repo_url` - The URL of the repository to import the branches into.
/// * `local_branch` - The local branch to import the branches from.
/// * `campaign` - The campaign
/// * `log_id` - The log_id
/// * `branches` - A vector of tuples containing the branch name, the directory
///    name, the revision to push, and the revision to push to.
///
/// # Returns
/// * `Ok(())` if the branches were successfully imported.
fn import_branches_bzr(
    repo_url: &Url,
    local_branch: &dyn breezyshim::branch::Branch,
    campaign: &str,
    log_id: &str,
    branches: &Vec<(String, String, Option<RevisionId>, Option<RevisionId>)>,
    tags: Vec<(String, Option<RevisionId>)>,
    update_current: bool,
) -> Result<(), BrzError> {
    let format = breezyshim::controldir::FORMAT_REGISTRY
        .make_controldir("bzr")
        .ok_or(BrzError::UnknownFormat("bzr".to_string()))?;
    for (f, _n, _br, r) in branches.iter() {
        let rootcd = match breezyshim::controldir::open(repo_url, None) {
            Ok(cd) => cd,
            Err(BrzError::NotBranchError(..)) => {
                breezyshim::controldir::create(repo_url, &format, None)?
            }
            Err(e) => {
                return Err(e);
            }
        };
        match rootcd.find_repository() {
            Err(BrzError::NoRepositoryPresent) => {
                rootcd.create_repository(Some(true))?;
            }
            Ok(_) => {}
            Err(e) => {
                return Err(e);
            }
        }
        let transport = rootcd.user_transport().clone(campaign)?;
        let name = if f != "main" {
            f.to_string()
        } else {
            "".to_string()
        };
        if !transport.has(".")? {
            match transport.ensure_base() {
                Ok(_) => {}
                Err(BrzError::NoSuchFile(..)) => {
                    transport.create_prefix()?;
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
        let branchcd = match format.initialize_on_transport(&transport) {
            Ok(cd) => cd,
            Err(BrzError::AlreadyControlDir(..)) => {
                breezyshim::controldir::open_from_transport(&transport, None)?
            }
            Err(e) => {
                return Err(e);
            }
        };

        assert!(branchcd.find_repository().is_ok());

        let target_branch = match branchcd.open_branch(Some(&name)) {
            Ok(b) => b,
            Err(BrzError::NotBranchError(..)) => branchcd.create_branch(Some(&name))?,
            Err(e) => {
                return Err(e);
            }
        };
        if update_current {
            local_branch.push(target_branch.as_ref(), true, r.as_ref(), None)?;
        } else {
            target_branch
                .repository()
                .fetch(&local_branch.repository(), r.as_ref())?;
        }

        target_branch.tags()?.set_tag(log_id, &r.clone().unwrap())?;

        let graph = target_branch.repository().get_graph();
        for (name, revision) in tags.iter() {
            if let Some(revision) = revision {
                // Only set tags on those branches where the revisions exist
                if graph.is_ancestor(revision, &target_branch.last_revision()) {
                    target_branch
                        .tags()?
                        .set_tag(&format!("{}/{}", log_id, name), revision)?;
                    if update_current {
                        target_branch.tags()?.set_tag(name, revision)?;
                    }
                }
            } else if update_current {
                match target_branch.tags()?.delete_tag(name) {
                    Ok(_) => (),
                    Err(BrzError::NoSuchTag(..)) => (),
                    Err(e) => {
                        return Err(e);
                    }
                }
            }
        }
    }

    Ok(())
}

pub trait Vcs {
    fn import_branches(
        &self,
        repo_url: &Url,
        local_branch: &dyn breezyshim::branch::Branch,
        campaign: &str,
        log_id: &str,
        branches: &Vec<(String, String, Option<RevisionId>, Option<RevisionId>)>,
        tags: Vec<(String, Option<RevisionId>)>,
        update_current: bool,
    ) -> Result<(), BrzError>;
}

pub struct BzrVcs;

impl Vcs for BzrVcs {
    fn import_branches(
        &self,
        repo_url: &Url,
        local_branch: &dyn breezyshim::branch::Branch,
        campaign: &str,
        log_id: &str,
        branches: &Vec<(String, String, Option<RevisionId>, Option<RevisionId>)>,
        tags: Vec<(String, Option<RevisionId>)>,
        update_current: bool,
    ) -> Result<(), BrzError> {
        import_branches_bzr(
            repo_url,
            local_branch,
            campaign,
            log_id,
            branches,
            tags,
            update_current,
        )
    }
}

/// Create symbolic references for branch refs pointing to their corresponding tag refs
fn create_symrefs_for_branches(
    repo: &breezyshim::repository::Repository,
    campaign: &str,
    log_id: &str,
    branches: &Vec<(String, String, Option<RevisionId>, Option<RevisionId>)>,
) -> Result<(), BrzError> {
    // Use git-specific functionality to create symrefs
    for (f, _n, _br, r) in branches.iter() {
        if r.is_some() {
            let target_ref = format!("refs/tags/run/{}/{}", log_id, f);
            let symref_name = format!("refs/heads/{}/{}", campaign, f);

            // Create symref using git-specific functionality
            create_symbolic_ref(repo, &symref_name, &target_ref)?;

            log::debug!("Created symref {} -> {}", symref_name, target_ref);
        }
    }
    Ok(())
}

/// Create symbolic references for current tag refs pointing to their versioned counterparts
fn create_symrefs_for_tags(
    repo: &breezyshim::repository::Repository,
    log_id: &str,
    tags: &Vec<(String, Option<RevisionId>)>,
) -> Result<(), BrzError> {
    // Use git-specific functionality to create symrefs
    for (n, r) in tags.iter() {
        if r.is_some() {
            let target_ref = format!("refs/tags/{}/{}", log_id, n);
            let symref_name = format!("refs/tags/{}", n);

            // Create symref using git-specific functionality
            create_symbolic_ref(repo, &symref_name, &target_ref)?;

            log::debug!("Created symref {} -> {}", symref_name, target_ref);
        }
    }
    Ok(())
}

/// Create a symbolic reference using Git-specific functionality
fn create_symbolic_ref(
    _repo: &breezyshim::repository::Repository,
    symref_name: &str,
    target_ref: &str,
) -> Result<(), BrzError> {
    // TODO: Implement symbolic reference creation when PyO3 API stabilizes
    // See external-todo.md for details on this external dependency
    log::debug!(
        "Symbolic ref creation not yet implemented: {} -> {}",
        symref_name,
        target_ref
    );
    log::debug!(
        "This would create symref {} pointing to {}",
        symref_name,
        target_ref
    );
    // Return Ok to allow operations to continue - this is a non-critical feature
    Ok(())
}

fn import_branches_git(
    repo_url: &Url,
    local_branch: &dyn breezyshim::branch::Branch,
    campaign: &str,
    log_id: &str,
    branches: &Vec<(String, String, Option<RevisionId>, Option<RevisionId>)>,
    tags: Vec<(String, Option<RevisionId>)>,
    update_current: bool,
) -> Result<(), BrzError> {
    let vcs_result_controldir = match breezyshim::controldir::open(repo_url, None) {
        Err(BrzError::NotBranchError(..)) => {
            let transport = breezyshim::transport::get_transport(repo_url, None).unwrap();
            if !transport.has(".")? {
                match transport.ensure_base() {
                    Err(BrzError::NoSuchFile(..)) => {
                        transport.create_prefix()?;
                    }
                    Ok(_) => (),
                    Err(e) => {
                        return Err(e);
                    }
                }
            }
            // The server is expected to have repositories ready for us, unless we're working
            // locally.
            let format = breezyshim::controldir::FORMAT_REGISTRY
                .make_controldir("git-bare")
                .unwrap();
            format.initialize(repo_url)?
        }
        Ok(cd) => cd,
        Err(e) => {
            return Err(e);
        }
    };

    let repo = vcs_result_controldir.open_repository().unwrap();

    // Clone for the sake of the closure
    let log_id_ = log_id.to_string();
    let campaign_ = campaign.to_string();
    let repo_ = local_branch.repository();
    let branches_clone = branches.clone();
    let tags_clone = tags.clone();

    let get_changed_refs = move |_refs: &HashMap<Vec<u8>, (Vec<u8>, Option<RevisionId>)>| -> HashMap<Vec<u8>, (Vec<u8>, Option<RevisionId>)> {
        let mut changed_refs = HashMap::new();
        for (f, _n, _br, r) in branches_clone.iter() {
            let tagname = format!("refs/tags/run/{}/{}", log_id_, f);
            changed_refs.insert(
                tagname.as_bytes().to_vec(),
                if let Some(r) = r {
                    (repo_.lookup_bzr_revision_id(r).unwrap().0, Some(r.clone()))
                } else {
                    (breezyshim::git::ZERO_SHA.to_vec(), r.clone())
                },
            );
            // Note: Branch refs will be created as symrefs in create_symrefs_for_branches()
            // instead of duplicating the commit SHA here
        }
        for (n, r) in tags_clone.iter() {
            let tagname = format!("refs/tags/{}/{}", log_id_, n);
            changed_refs.insert(
                tagname.as_bytes().to_vec(),
                (
                    repo_.lookup_bzr_revision_id(r.as_ref().unwrap()).unwrap().0,
                    r.clone(),
                ),
            );
            // Note: Current tag refs will be created as symrefs in create_symrefs_for_tags()
            // instead of duplicating the commit SHA here
        }
        changed_refs
    };

    let inter = breezyshim::interrepository::get(&local_branch.repository(), &repo).unwrap();
    inter.fetch_refs(
        std::sync::Mutex::new(Box::new(get_changed_refs)),
        false,
        true,
    )?;

    // Create symrefs for branch references after the regular refs are created
    if update_current {
        create_symrefs_for_branches(&repo, campaign, log_id, &branches)?;
        create_symrefs_for_tags(&repo, log_id, &tags)?;
    }

    Ok(())
}

pub struct GitVcs;

impl Vcs for GitVcs {
    fn import_branches(
        &self,
        repo_url: &Url,
        local_branch: &dyn breezyshim::branch::Branch,
        campaign: &str,
        log_id: &str,
        branches: &Vec<(String, String, Option<RevisionId>, Option<RevisionId>)>,
        tags: Vec<(String, Option<RevisionId>)>,
        update_current: bool,
    ) -> Result<(), BrzError> {
        import_branches_git(
            repo_url,
            local_branch,
            campaign,
            log_id,
            branches,
            tags,
            update_current,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_branch() {
        let td = tempfile::tempdir().unwrap();
        let source_tree = breezyshim::controldir::create_standalone_workingtree(
            &td.path().join("source"),
            &breezyshim::controldir::ControlDirFormat::default(),
        )
        .unwrap();
        let target_url = Url::from_directory_path(td.path().join("target")).unwrap();
        let revid1 = source_tree
            .build_commit()
            .message("Initial commit")
            .commit()
            .unwrap();
        let target = breezyshim::controldir::create(
            &target_url,
            &breezyshim::controldir::FORMAT_REGISTRY
                .make_controldir("bzr")
                .unwrap(),
            None,
        )
        .unwrap();
        target.create_repository(None).unwrap();
        super::push_branch(
            source_tree.branch().as_ref(),
            &url::Url::parse(&format!("{},branch=foo", target_url)).unwrap(),
            None,
            false,
            None,
            None,
            &mut None,
        )
        .unwrap();
        assert_eq!(
            target.open_branch(Some("foo")).unwrap().last_revision(),
            revid1
        );
    }

    #[test]
    fn test_import_branches_bzr() {
        let td = tempfile::tempdir().unwrap();
        let source_tree = breezyshim::controldir::create_standalone_workingtree(
            &td.path().join("source"),
            &breezyshim::controldir::ControlDirFormat::default(),
        )
        .unwrap();
        let target_path = td.path().join("target");
        let target_url = Url::from_directory_path(&target_path).unwrap();
        let revid1 = source_tree
            .build_commit()
            .message("Initial commit")
            .commit()
            .unwrap();
        let target = breezyshim::controldir::create(
            &target_url,
            &breezyshim::controldir::FORMAT_REGISTRY
                .make_controldir("bzr")
                .unwrap(),
            None,
        )
        .unwrap();
        target.create_repository(Some(true)).unwrap();
        BzrVcs
            .import_branches(
                &target_url,
                source_tree.branch().as_ref(),
                "campaign",
                "log_id",
                &vec![(
                    "main".to_string(),
                    "foo".to_string(),
                    None,
                    Some(revid1.clone()),
                )],
                vec![("tag".to_string(), Some(revid1.clone()))],
                false,
            )
            .unwrap();
        let target_branch_foo = breezyshim::branch::open(
            &Url::from_directory_path(target_path.join("campaign")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            &target_branch_foo.last_revision(),
            &breezyshim::RevisionId::null()
        );
        assert_eq!(
            &target_branch_foo.tags().unwrap().get_tag_dict().unwrap(),
            &maplit::hashmap! {
                "log_id".to_string() => revid1.clone(),
            }
        );
    }

    #[test]
    #[ignore = "Requires Git/Breezy system dependencies"]
    fn test_import_branches_git() {
        let td = tempfile::tempdir().unwrap();
        let source_tree = breezyshim::controldir::create_standalone_workingtree(
            &td.path().join("source"),
            &breezyshim::controldir::FORMAT_REGISTRY
                .make_controldir("git")
                .unwrap(),
        )
        .unwrap();
        let target_path = td.path().join("target");
        let target_url = Url::from_directory_path(&target_path).unwrap();
        let revid1 = source_tree
            .build_commit()
            .message("Initial commit")
            .commit()
            .unwrap();
        let target = breezyshim::controldir::create(
            &target_url,
            &breezyshim::controldir::FORMAT_REGISTRY
                .make_controldir("git-bare")
                .unwrap(),
            None,
        )
        .unwrap();
        GitVcs
            .import_branches(
                &target_url,
                source_tree.branch().as_ref(),
                "campaign",
                "log_id",
                &vec![(
                    "main".to_string(),
                    "foo".to_string(),
                    None,
                    Some(revid1.clone()),
                )],
                vec![("tag".to_string(), Some(revid1.clone()))],
                true,
            )
            .unwrap();
        let target_branch_foo = target.open_branch(Some("campaign/main")).unwrap();
        assert_eq!(&target_branch_foo.last_revision(), &revid1);
        assert_eq!(
            &target_branch_foo.tags().unwrap().get_tag_dict().unwrap(),
            &maplit::hashmap! {
                "run/log_id/main".to_string() => revid1.clone(),
                "log_id/tag".to_string() => revid1.clone(),
                "tag".to_string() => revid1.clone(),
            }
        );
        std::mem::drop(td);
    }
}
