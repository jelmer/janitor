use breezyshim::branch::Branch;
use breezyshim::error::Error as BrzError;
use pyo3::exceptions::PyAttributeError;
use pyo3::prelude::*;

pub fn is_authenticated_url(url: &url::Url) -> bool {
    ["git+ssh", "bzr+ssh"].contains(&url.scheme())
}

pub fn get_branch_vcs_type(branch: &dyn Branch) -> Result<String, BrzError> {
    let repository = branch.repository();
    Python::with_gil(|py| {
        let object = repository.to_object(py);
        match object.getattr(py, "vcs") {
            Ok(vcs) => vcs
                .getattr(py, "abbreviation")
                .unwrap()
                .extract::<String>(py),
            Err(e) if e.is_instance_of::<PyAttributeError>(py) => Ok("bzr".to_string()),
            Err(e) => Err(e),
        }
    })
    .map_err(BrzError::from)
}

pub fn is_alioth_url(url: &url::Url) -> bool {
    matches!(
        url.host_str(),
        Some("svn.debian.org")
            | Some("bzr.debian.org")
            | Some("anonscm.debian.org")
            | Some("hg.debian.org")
            | Some("git.debian.org")
            | Some("alioth.debian.org")
    )
}

#[cfg(test)]
mod is_authenticated_url_tests {
    #[test]
    fn test_simple() {
        assert!(super::is_authenticated_url(
            &url::Url::parse("git+ssh://example.com").unwrap()
        ));
        assert!(super::is_authenticated_url(
            &url::Url::parse("bzr+ssh://example.com").unwrap()
        ));
        assert!(!super::is_authenticated_url(
            &url::Url::parse("http://example.com").unwrap()
        ));
    }
}

#[cfg(test)]
mod is_alioth_url_tests {
    #[test]
    fn test_simple() {
        assert!(super::is_alioth_url(
            &url::Url::parse(
                "https://anonscm.debian.org/cgit/pkg-ocaml-maint/packages/ocamlbuild.git"
            )
            .unwrap()
        ));
        assert!(super::is_alioth_url(
            &url::Url::parse("https://git.debian.org/git/pkg-ocaml-maint/packages/ocamlbuild.git")
                .unwrap()
        ));
        assert!(super::is_alioth_url(
            &url::Url::parse(
                "https://alioth.debian.org/anonscm/git/pkg-ocaml-maint/packages/ocamlbuild.git"
            )
            .unwrap()
        ));
        assert!(!super::is_alioth_url(
            &url::Url::parse("https://example.com").unwrap()
        ));
    }
}
