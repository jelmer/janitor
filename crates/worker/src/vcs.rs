use breezyshim::error::Error as BrzError;
use breezyshim::transport::Transport;
use breezyshim::RevisionId;
use url::Url;

/// Push a branch to a new location.
pub fn push_branch(
    source_branch: &dyn breezyshim::branch::Branch,
    url: &Url,
    vcs_type: Option<&str>,
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
                .make_controldir(t)
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
    branches: Vec<(String, String, Option<RevisionId>, Option<RevisionId>)>,
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
        branches: Vec<(String, String, Option<RevisionId>, Option<RevisionId>)>,
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
        branches: Vec<(String, String, Option<RevisionId>, Option<RevisionId>)>,
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
            .commit("Initial commit", None, None, None)
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
            .commit("Initial commit", None, None, None)
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
                vec![(
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
}
