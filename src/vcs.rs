use breezyshim::branch::Branch;
use breezyshim::error::Error as BrzError;
use pyo3::exceptions::PyAttributeError;
use pyo3::prelude::*;

pub fn is_authenticated_url(url: &url::Url) -> bool {
    ["git+ssh", "bzr+ssh"].contains(&url.scheme())
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
    .map_err(|e| BrzError::from(e))
}
