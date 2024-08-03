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
}
