use pyo3::exceptions::PyTimeoutError;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::path::{Path, PathBuf};

#[pyfunction]
fn get_branch_vcs_type(branch: PyObject) -> PyResult<String> {
    let branch = breezyshim::branch::GenericBranch::new(branch);
    janitor::vcs::get_branch_vcs_type(&branch)
        .map_err(|e| PyValueError::new_err((format!("{}", e),)))
        .map(|vcs| vcs.to_string())
}

#[pyfunction]
fn is_authenticated_url(url: &str) -> PyResult<bool> {
    Ok(janitor::vcs::is_authenticated_url(
        &url::Url::parse(url)
            .map_err(|e| PyValueError::new_err((format!("Invalid URL: {}", e),)))?,
    ))
}

#[pyfunction]
fn is_alioth_url(url: &str) -> PyResult<bool> {
    Ok(janitor::vcs::is_alioth_url(&url::Url::parse(url).map_err(
        |e| PyValueError::new_err((format!("Invalid URL: {}", e),)),
    )?))
}

#[pyclass(subclass)]
struct ArtifactManager {
    inner: std::sync::Arc<dyn janitor::artifacts::ArtifactManager>,
}

#[pymethods]
impl ArtifactManager {
    #[pyo3(signature = (run_id, local_path, names=None))]
    fn store_artifacts<'a>(
        &self,
        py: Python<'a>,
        run_id: String,
        local_path: PathBuf,
        names: Option<Vec<String>>,
    ) -> PyResult<Bound<'a, PyAny>> {
        let inner = self.inner.clone();
        pyo3_asyncio::tokio::future_into_py(py, async move {
            inner
                .store_artifacts(&run_id, local_path.as_path(), names.as_deref())
                .await
                .map_err(|e| PyValueError::new_err((format!("{}", e),)))?;
            Ok(())
        })
    }

    fn public_artifact_url(&self, run_id: String, filename: String) -> PyResult<String> {
        Ok(self
            .inner
            .public_artifact_url(&run_id, &filename)
            .to_string())
    }

    #[pyo3(signature = (run_id, local_path, filter_fn=None, timeout=None))]
    fn retrieve_artifacts(
        &self,
        run_id: String,
        local_path: PathBuf,
        filter_fn: Option<PyObject>,
        timeout: Option<u64>,
    ) -> PyResult<()> {
        self.inner
            .retrieve_artifacts(&run_id, local_path.as_path(), names.as_deref())
            .map_err(|e| PyValueError::new_err((format!("{}", e),)))
    }

    fn iter_ids(&self) -> PyResult<Vec<String>> {
        Ok(self.inner.iter_ids())
    }
}

#[pyclass(extends=ArtifactManager)]
struct LocalArtifactManager;

#[pymethods]
impl LocalArtifactManager {
    #[new]
    fn new(path: &str) -> (Self, ArtifactManager) {
        let path = PathBuf::from(path);
        let manager = janitor::artifacts::LocalArtifactManager::new(&path)
            .map_err(|e| PyValueError::new_err((format!("{}", e),)))
            .unwrap();
        (
            Self,
            ArtifactManager {
                inner: std::sync::Arc::new(manager),
            },
        )
    }
}

#[pyfunction]
#[pyo3(signature = (manager, backup_manager, from_dir, run_id, names=None))]
fn store_artifacts_with_backup<'a>(
    py: Python<'a>,
    manager: &ArtifactManager,
    backup_manager: Option<&ArtifactManager>,
    from_dir: PathBuf,
    run_id: String,
    names: Option<Vec<String>>,
) -> PyResult<Bound<'a, PyAny>> {
    let manager = manager.inner.clone();
    let backup_manager = backup_manager.map(|m| m.inner.clone());
    pyo3_asyncio::tokio::future_into_py(py, async move {
        janitor::artifacts::store_artifacts_with_backup(
            manager.as_ref(),
            backup_manager.as_deref(),
            from_dir.as_path(),
            &run_id,
            names.as_deref(),
        )
        .await
        .map_err(|e| PyValueError::new_err((format!("{}", e),)))?;
        Ok(())
    })
}

#[pyfunction]
#[pyo3(signature = (backup_manager, manager, timeout=None))]
fn upload_backup_artifacts<'a>(
    py: Python<'a>,
    backup_manager: &ArtifactManager,
    manager: &ArtifactManager,
    timeout: Option<u64>,
) -> PyResult<Bound<'a, PyAny>> {
    let manager = manager.inner.clone();
    let backup_manager = backup_manager.inner.clone();
    pyo3_asyncio::tokio::future_into_py(py, async move {
        let upload_fn =
            janitor::artifacts::upload_backup_artifacts(backup_manager.as_ref(), manager.as_ref());
        tokio::time::timeout(
            std::time::Duration::from_secs(timeout.unwrap_or(60)),
            upload_fn,
        )
        .await
        .map_err(|e| PyTimeoutError::new_err((format!("{}", e),)))?
        .map_err(|e| PyValueError::new_err((format!("{}", e),)))?;
        Ok(())
    })
}

#[pyfunction]
fn get_artifact_manager<'a>(py: Python<'a>, location: String) -> PyResult<Bound<'a, PyAny>> {
    pyo3_asyncio::tokio::future_into_py(py, async move {
        let manager = janitor::artifacts::get_artifact_manager(location.as_str())
            .await
            .map_err(|e| PyValueError::new_err((format!("{}", e),)))?;
        Ok(ArtifactManager {
            inner: std::sync::Arc::from(manager),
        })
    })
}

#[pymodule]
pub fn _common(m: &Bound<PyModule>) -> PyResult<()> {
    pyo3_log::init();
    m.add_function(wrap_pyfunction!(is_authenticated_url, m)?)?;
    m.add_function(wrap_pyfunction!(is_alioth_url, m)?)?;
    m.add_function(wrap_pyfunction!(get_branch_vcs_type, m)?)?;
    m.add_class::<LocalArtifactManager>()?;
    m.add_function(wrap_pyfunction!(store_artifacts_with_backup, m)?)?;
    m.add_function(wrap_pyfunction!(upload_backup_artifacts, m)?)?;
    m.add_function(wrap_pyfunction!(get_artifact_manager, m)?)?;
    Ok(())
}
