use breezyshim::RevisionId;
use pyo3::exceptions::{PyNotImplementedError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyBytes};
use std::path::PathBuf;
use std::sync::Arc;

#[pyclass]
pub struct RevisionInfo(pub(crate) janitor::vcs::RevisionInfo);

#[pymethods]
impl RevisionInfo {
    #[getter]
    fn commit_id<'a>(&self, py: Python<'a>) -> Option<Bound<'a, PyBytes>> {
        self.0.commit_id.as_ref().map(|x| PyBytes::new_bound(py, x))
    }

    #[getter]
    fn revision_id(&self) -> RevisionId {
        self.0.revision_id.clone()
    }

    #[getter]
    fn message(&self) -> String {
        self.0.message.clone()
    }

    #[getter]
    fn link(&self) -> Option<String> {
        self.0.link.as_ref().map(|x| x.to_string())
    }
}

#[pyclass(subclass)]
pub struct VcsManager(Arc<dyn janitor::vcs::VcsManager>);

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

    fn get_diff<'a>(
        &self,
        py: Python<'a>,
        codebase: String,
        old_revid: RevisionId,
        new_revid: RevisionId,
    ) -> PyResult<Bound<'a, PyAny>> {
        let z = self.0.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let diff = z.get_diff(&codebase, &old_revid, &new_revid).await;
            Ok(Python::with_gil(|py| {
                PyBytes::new_bound(py, &diff).to_object(py)
            }))
        })
    }

    fn get_revision_info<'a>(
        &self,
        py: Python<'a>,
        codebase: String,
        old_revid: RevisionId,
        new_revid: RevisionId,
    ) -> PyResult<Bound<'a, PyAny>> {
        let z = self.0.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let infos = z.get_revision_info(&codebase, &old_revid, &new_revid).await;
            Ok(infos
                .into_iter()
                .map(|info| RevisionInfo(info))
                .collect::<Vec<_>>())
        })
    }

    fn get_repository<'a>(&self, py: Python<'a>, codebase: String) -> PyResult<PyObject> {
        let repo = self.0.get_repository(&codebase)?;

        Ok(repo.to_object(py))
    }

    fn get_branch<'a>(
        &self,
        py: Python<'a>,
        codebase: String,
        branch_name: String,
    ) -> PyResult<Option<PyObject>> {
        let branch = self.0.get_branch(&codebase, &branch_name)?;

        Ok(branch.map(|x| x.to_object(py)))
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
        let vcs_manager = Arc::new(janitor::vcs::LocalGitVcsManager::new(base_path));
        Ok((manager, VcsManager(vcs_manager)))
    }
}

#[pyclass(extends=VcsManager)]
pub struct RemoteGitVcsManager(Arc<janitor::vcs::RemoteGitVcsManager>);

#[pymethods]
impl RemoteGitVcsManager {
    #[new]
    fn new(base_url: &str) -> PyResult<(Self, VcsManager)> {
        let base_url =
            url::Url::parse(base_url).map_err(|e| PyValueError::new_err(format!("{}", e)))?;
        let vcs_manager = Arc::new(janitor::vcs::RemoteGitVcsManager::new(base_url));
        let manager = RemoteGitVcsManager(vcs_manager.clone());
        Ok((manager, VcsManager(vcs_manager)))
    }

    pub fn get_diff_url(
        &self,
        codebase: &str,
        old_revid: RevisionId,
        new_revid: RevisionId,
    ) -> PyResult<String> {
        Ok(self.0
            .get_diff_url(codebase, &old_revid, &new_revid)
            .to_string())
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
        let vcs_manager = Arc::new(janitor::vcs::LocalBzrVcsManager::new(base_path));
        Ok((manager, VcsManager(vcs_manager)))
    }
}

#[pyclass(extends=VcsManager)]
pub struct RemoteBzrVcsManager(Arc<janitor::vcs::RemoteBzrVcsManager>);

#[pymethods]
impl RemoteBzrVcsManager {
    #[new]
    fn new(base_url: &str) -> PyResult<(Self, VcsManager)> {
        let base_url =
            url::Url::parse(base_url).map_err(|e| PyValueError::new_err(format!("{}", e)))?;
        let vcs_manager = Arc::new(janitor::vcs::RemoteBzrVcsManager::new(base_url));
        let manager = RemoteBzrVcsManager(vcs_manager.clone());
        Ok((manager, VcsManager(vcs_manager)))
    }

    pub fn get_diff_url(
        &self,
        codebase: &str,
        old_revid: RevisionId,
        new_revid: RevisionId,
    ) -> PyResult<String> {
        Ok(self.0
            .get_diff_url(codebase, &old_revid, &new_revid)
            .to_string())
    }
}

pub(crate) fn init(py: Python, module: &Bound<PyModule>) -> PyResult<()> {
    module.add_class::<VcsManager>()?;
    module.add_class::<LocalGitVcsManager>()?;
    module.add_class::<RemoteGitVcsManager>()?;
    module.add_class::<LocalBzrVcsManager>()?;
    module.add_class::<RemoteBzrVcsManager>()?;
    Ok(())
}
