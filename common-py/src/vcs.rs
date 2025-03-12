use breezyshim::RevisionId;
use pyo3::exceptions::{PyNotImplementedError, PyValueError};
use pyo3::prelude::*;
use std::path::PathBuf;

#[pyclass(subclass)]
pub struct VcsManager(Box<dyn janitor::vcs::VcsManager>);

#[pymethods]
impl VcsManager {
    fn get_branch_url(&self, codebase: &str, branch_name: &str) -> String {
        let url = self.0.get_branch_url(codebase, branch_name);
        url.to_string()
    }

    fn get_repository_url(&self, codebase: &str) -> String {
        let url = self.0.get_repository_url(codebase);
        url.to_string()
    }

    fn list_repositories(&self) -> Vec<String> {
        self.0.list_repositories()
    }
}

#[pyclass(extends=VcsManager)]
pub struct LocalGitVcsManager {}

#[pymethods]
impl LocalGitVcsManager {
    #[new]
    fn new(base_path: &str) -> PyResult<(Self, VcsManager)> {
        let manager = LocalGitVcsManager {};
        let base_path = PathBuf::from(base_path);
        let vcs_manager = Box::new(janitor::vcs::LocalGitVcsManager::new(base_path));
        Ok((manager, VcsManager(vcs_manager)))
    }
}

#[pyclass(extends=VcsManager)]
pub struct RemoteGitVcsManager {}

#[pymethods]
impl RemoteGitVcsManager {
    #[new]
    fn new(base_url: &str) -> PyResult<(Self, VcsManager)> {
        let manager = RemoteGitVcsManager {};
        let base_url =
            url::Url::parse(base_url).map_err(|e| PyValueError::new_err(format!("{}", e)))?;
        let vcs_manager = Box::new(janitor::vcs::RemoteGitVcsManager::new(base_url));
        Ok((manager, VcsManager(vcs_manager)))
    }

    pub fn get_diff_url(
        &self,
        codebase: &str,
        old_revid: RevisionId,
        new_revid: RevisionId,
    ) -> PyResult<String> {
        Err(PyNotImplementedError::new_err("Not implemented"))
    }
}

#[pyclass(extends=VcsManager)]
pub struct LocalBzrVcsManager {}

#[pymethods]
impl LocalBzrVcsManager {
    #[new]
    fn new(base_path: &str) -> PyResult<(Self, VcsManager)> {
        let manager = LocalBzrVcsManager {};
        let base_path = PathBuf::from(base_path);
        let vcs_manager = Box::new(janitor::vcs::LocalBzrVcsManager::new(base_path));
        Ok((manager, VcsManager(vcs_manager)))
    }
}

#[pyclass(extends=VcsManager)]
pub struct RemoteBzrVcsManager {}

#[pymethods]
impl RemoteBzrVcsManager {
    #[new]
    fn new(base_url: &str) -> PyResult<(Self, VcsManager)> {
        let manager = RemoteBzrVcsManager {};
        let base_url =
            url::Url::parse(base_url).map_err(|e| PyValueError::new_err(format!("{}", e)))?;
        let vcs_manager = Box::new(janitor::vcs::RemoteBzrVcsManager::new(base_url));
        Ok((manager, VcsManager(vcs_manager)))
    }

    pub fn get_diff_url(
        &self,
        codebase: &str,
        old_revid: RevisionId,
        new_revid: RevisionId,
    ) -> PyResult<String> {
        Err(PyNotImplementedError::new_err("Not implemented"))
    }
}

pub(crate) fn init(py: Python, module: &Bound<PyModule>) -> PyResult<()> {
    module.add_class::<VcsManager>()?;
    Ok(())
}
