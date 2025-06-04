use breezyshim::RevisionId;

use pyo3::basic::CompareOp;
use pyo3::create_exception;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyBytes};
use std::collections::HashMap;
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
            Ok(infos.into_iter().map(RevisionInfo).collect::<Vec<_>>())
        })
    }

    fn get_repository(&self, py: Python<'_>, codebase: String) -> PyResult<PyObject> {
        let repo = self.0.get_repository(&codebase)?;

        Ok(repo.to_object(py))
    }

    fn get_branch(
        &self,
        py: Python<'_>,
        codebase: String,
        branch_name: String,
    ) -> PyResult<Option<PyObject>> {
        let branch = self.0.get_branch(&codebase, &branch_name)?;

        Ok(branch.map(|x| x.to_object(py)))
    }
}

#[pyclass(extends=VcsManager)]
pub struct LocalGitVcsManager(Arc<janitor::vcs::LocalGitVcsManager>);

#[pymethods]
impl LocalGitVcsManager {
    #[new]
    fn new(base_path: PathBuf) -> PyResult<(Self, VcsManager)> {
        let vcs_manager = Arc::new(janitor::vcs::LocalGitVcsManager::new(base_path));
        let manager = LocalGitVcsManager(vcs_manager.clone());
        Ok((manager, VcsManager(vcs_manager)))
    }

    #[getter]
    pub fn base_path(&self) -> String {
        self.0.base_path().to_string_lossy().to_string()
    }

    fn __richcmp__(&self, other: &Self, op: CompareOp) -> bool {
        match op {
            CompareOp::Eq => self.0.base_path() == other.0.base_path(),
            _ => false,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "<LocalGitVcsManager({})>",
            self.0.base_path().to_string_lossy()
        )
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
        Ok(self
            .0
            .get_diff_url(codebase, &old_revid, &new_revid)
            .to_string())
    }

    #[getter]
    pub fn base_url(&self) -> String {
        self.0.base_url().to_string()
    }

    fn __richcmp__(&self, other: &Self, op: CompareOp) -> bool {
        match op {
            CompareOp::Eq => self.0.base_url() == other.0.base_url(),
            _ => false,
        }
    }

    fn __repr__(&self) -> String {
        format!("<RemoteGitVcsManager({})>", self.0.base_url())
    }
}

#[pyclass(extends=VcsManager)]
pub struct LocalBzrVcsManager(Arc<janitor::vcs::LocalBzrVcsManager>);

#[pymethods]
impl LocalBzrVcsManager {
    #[new]
    fn new(base_path: PathBuf) -> PyResult<(Self, VcsManager)> {
        let vcs_manager = Arc::new(janitor::vcs::LocalBzrVcsManager::new(base_path));
        let manager = LocalBzrVcsManager(vcs_manager.clone());
        Ok((manager, VcsManager(vcs_manager)))
    }

    #[getter]
    pub fn base_path(&self) -> String {
        self.0.base_path().to_string_lossy().to_string()
    }

    fn __richcmp__(&self, other: &Self, op: CompareOp) -> bool {
        match op {
            CompareOp::Eq => self.0.base_path() == other.0.base_path(),
            _ => false,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "<LocalBzrVcsManager({})>",
            self.0.base_path().to_string_lossy()
        )
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
        Ok(self
            .0
            .get_diff_url(codebase, &old_revid, &new_revid)
            .to_string())
    }

    #[getter]
    pub fn base_url(&self) -> String {
        self.0.base_url().to_string()
    }

    fn __richcmp__(&self, other: &Self, op: CompareOp) -> bool {
        match op {
            CompareOp::Eq => self.0.base_url() == other.0.base_url(),
            _ => false,
        }
    }

    fn __repr__(&self) -> String {
        format!("<RemoteBzrVcsManager({})>", self.0.base_url())
    }
}

#[pyfunction]
pub fn get_local_vcs_manager(py: Python, name: &str, location: PathBuf) -> PyResult<PyObject> {
    match name {
        "bzr" => Ok(Py::new(py, LocalBzrVcsManager::new(location).unwrap())?.to_object(py)),
        "git" => Ok(Py::new(py, LocalGitVcsManager::new(location).unwrap())?.to_object(py)),
        _ => Err(UnsupportedVcs::new_err((
            name.to_string(),
            location.to_string_lossy().to_string(),
        ))),
    }
}

#[pyfunction]
pub fn get_remote_vcs_manager(py: Python, name: &str, location: &str) -> PyResult<PyObject> {
    match name {
        "bzr" => Ok(Py::new(py, RemoteBzrVcsManager::new(location).unwrap())?.to_object(py)),
        "git" => Ok(Py::new(py, RemoteGitVcsManager::new(location).unwrap())?.to_object(py)),
        _ => Err(UnsupportedVcs::new_err((
            name.to_string(),
            location.to_string(),
        ))),
    }
}

#[pyfunction]
pub fn get_vcs_manager(py: Python, name: &str, location: &str) -> PyResult<PyObject> {
    if !location.contains(':') {
        get_local_vcs_manager(py, name, PathBuf::from(location))
    } else {
        get_remote_vcs_manager(py, name, location)
    }
}

#[pyfunction]
pub fn get_vcs_managers(py: Python, location: &str) -> PyResult<HashMap<String, PyObject>> {
    if !location.contains('=') {
        Ok(maplit::hashmap! {
            "bzr".to_string() => get_vcs_manager(py, "bzr", &(location.trim_end_matches('/').to_owned() + "/bzr")).unwrap(),
            "git".to_string() => get_vcs_manager(py, "git", &(location.trim_end_matches('/').to_owned() + "/git")).unwrap(),
        })
    } else {
        let mut managers = std::collections::HashMap::new();
        for part in location.split(',') {
            let (name, path) = part.split_once('=').unwrap();
            let vcs = get_vcs_manager(py, name, path)?;
            managers.insert(name.to_string(), vcs);
        }
        Ok(managers)
    }
}

create_exception!(
    janitor.vcs,
    BranchOpenFailure,
    pyo3::exceptions::PyException
);
create_exception!(janitor.vcs, UnsupportedVcs, pyo3::exceptions::PyException);

#[pyfunction]
#[pyo3(signature = (vcs_url, possible_transports=None, probers=None))]
pub fn open_branch_ext(
    py: Python,
    vcs_url: &str,
    possible_transports: Option<Vec<PyObject>>,
    probers: Option<Vec<PyObject>>,
) -> Result<PyObject, PyErr> {
    let vcs_url = url::Url::parse(vcs_url).map_err(|e| PyValueError::new_err(format!("{}", e)))?;
    // TODO: support possible_transports
    // TODO: support probers
    match janitor::vcs::open_branch_ext(&vcs_url, None, None) {
        Ok(b) => Ok(b.to_object(py)),
        Err(e) => Err(BranchOpenFailure::new_err((
            e.code,
            e.description,
            e.retry_after,
        ))),
    }
}

pub(crate) fn init(py: Python, module: &Bound<PyModule>) -> PyResult<()> {
    module.add_class::<VcsManager>()?;
    module.add_class::<LocalGitVcsManager>()?;
    module.add_class::<RemoteGitVcsManager>()?;
    module.add_class::<LocalBzrVcsManager>()?;
    module.add_class::<RemoteBzrVcsManager>()?;
    module.add_class::<RevisionInfo>()?;
    module.add_function(wrap_pyfunction_bound!(get_local_vcs_manager, module)?)?;
    module.add_function(wrap_pyfunction_bound!(get_remote_vcs_manager, module)?)?;
    module.add_function(wrap_pyfunction_bound!(get_vcs_manager, module)?)?;
    module.add_function(wrap_pyfunction_bound!(get_vcs_managers, module)?)?;
    module.add_function(wrap_pyfunction_bound!(open_branch_ext, module)?)?;
    module.add(
        "BranchOpenFailure",
        py.get_type_bound::<BranchOpenFailure>(),
    )?;
    module.add("UnsupportedVcs", py.get_type_bound::<UnsupportedVcs>())?;
    Ok(())
}
