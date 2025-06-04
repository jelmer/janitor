use crate::io::Readable;
use pyo3::create_exception;
use pyo3::exceptions::{PyRuntimeError, PyTimeoutError};
use pyo3::prelude::*;
use pyo3::types::PyAny;
use std::sync::Arc;

create_exception!(
    janitor.artifacts,
    ServiceUnavailable,
    pyo3::exceptions::PyException
);
create_exception!(
    janitor.artifacts,
    ArtifactsMissing,
    pyo3::exceptions::PyException
);

fn artifact_err_to_py_err(e: janitor::artifacts::Error) -> PyErr {
    match e {
        janitor::artifacts::Error::ServiceUnavailable => {
            ServiceUnavailable::new_err("Service unavailable")
        }
        janitor::artifacts::Error::ArtifactsMissing => {
            ArtifactsMissing::new_err("Artifacts missing")
        }
        janitor::artifacts::Error::IoError(e) => e.into(),
        janitor::artifacts::Error::Other(e) => PyRuntimeError::new_err(e),
    }
}

#[pyclass(subclass)]
pub struct ArtifactManager(Arc<dyn janitor::artifacts::ArtifactManager>);

#[pymethods]
impl ArtifactManager {
    /// Store a set of artifacts.
    ///
    /// Args:
    ///   run_id: The run id
    ///   local_path: Local path to retrieve files from
    ///   names: Optional list of filenames in local_path to upload.
    ///     Defaults to all files in local_path.
    #[pyo3(signature = (run_id, local_path, names=None))]
    fn store_artifacts<'a>(
        &self,
        py: Python<'a>,
        run_id: &str,
        local_path: &str,
        names: Option<Vec<String>>,
    ) -> PyResult<Bound<'a, PyAny>> {
        let local_path = std::path::PathBuf::from(local_path);
        let run_id = run_id.to_string();
        let z = self.0.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            z.store_artifacts(&run_id, &local_path, names.as_deref())
                .await
                .map_err(artifact_err_to_py_err)
        })
    }

    #[pyo3(signature = (run_id, filename, timeout=None))]
    fn get_artifact<'a>(
        &self,
        py: Python<'a>,
        run_id: &str,
        filename: &str,
        timeout: Option<u64>,
    ) -> PyResult<Bound<'a, PyAny>> {
        let run_id = run_id.to_string();
        let filename = filename.to_string();
        let z = self.0.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let r = tokio::time::timeout(
                std::time::Duration::from_secs(timeout.unwrap_or(60)),
                z.get_artifact(&run_id, &filename),
            )
            .await
            .map_err(|_| PyTimeoutError::new_err("Timeout"))?
            .map_err(artifact_err_to_py_err)?;

            Ok(Readable::new(r))
        })
    }

    fn public_artifact_url(&self, run_id: &str, filename: &str) -> String {
        self.0.public_artifact_url(run_id, filename).to_string()
    }

    #[pyo3(signature = (run_id, ))]
    fn delete_artifacts<'a>(&self, py: Python<'a>, run_id: &str) -> PyResult<Bound<'a, PyAny>> {
        let run_id = run_id.to_string();
        let z = self.0.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            z.delete_artifacts(&run_id)
                .await
                .map_err(artifact_err_to_py_err)
        })
    }

    #[pyo3(signature = (run_id, local_path, filter_fn=None))]
    fn retrieve_artifacts<'a>(
        &self,
        py: Python<'a>,
        run_id: &str,
        local_path: &str,
        filter_fn: Option<PyObject>,
    ) -> PyResult<Bound<'a, PyAny>> {
        let run_id = run_id.to_string();
        let local_path = std::path::PathBuf::from(local_path);
        let z = self.0.clone();
        let filter_fn: Option<Box<dyn Fn(&str) -> bool + Sync + Send>> = filter_fn.map(|f| {
            Box::new({
                move |x: &str| -> bool {
                    Python::with_gil(|py| {
                        let r = f.call1(py, (x,))?;
                        r.is_truthy(py)
                    })
                    .unwrap()
                }
            }) as Box<dyn Fn(&str) -> bool + Sync + Send>
        });
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            z.retrieve_artifacts(&run_id, &local_path, filter_fn.as_deref())
                .await
                .map_err(artifact_err_to_py_err)
        })
    }

    fn __aenter__<'a>(slf: pyo3::Bound<Self>, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let slf = slf.clone().to_object(py);
        pyo3_async_runtimes::tokio::future_into_py(py, async move { Ok(slf) })
    }

    fn __aexit__<'a>(
        &self,
        py: Python<'a>,
        _exc_type: PyObject,
        _exc_value: PyObject,
        _traceback: PyObject,
    ) -> PyResult<Bound<'a, PyAny>> {
        let none = py.None();
        pyo3_async_runtimes::tokio::future_into_py(py, async move { Ok(none) })
    }
}

/// Local Artifact Manager
#[pyclass(extends=ArtifactManager)]
pub struct LocalArtifactManager;

#[pymethods]
impl LocalArtifactManager {
    #[new]
    fn new(path: std::path::PathBuf) -> PyResult<(Self, ArtifactManager)> {
        let artifact_manager = janitor::artifacts::LocalArtifactManager::new(path.as_path())?;
        Ok((
            LocalArtifactManager,
            ArtifactManager(Arc::new(artifact_manager)),
        ))
    }
}

/// Google Cloud Storage Artifact Manager
#[pyclass(extends=ArtifactManager)]
pub struct GCSArtifactManager {}

#[pyfunction]
fn list_ids<'a>(py: Python<'a>, artifact_manager: &ArtifactManager) -> PyResult<Bound<'a, PyAny>> {
    let z = artifact_manager.0.clone();
    pyo3_async_runtimes::tokio::future_into_py(py, async move {
        janitor::artifacts::list_ids(z.as_ref())
            .await
            .map_err(artifact_err_to_py_err)
    })
}

#[pyfunction]
fn get_artifact_manager(location: &str) -> PyResult<ArtifactManager> {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let artifact_manager = runtime
        .block_on(janitor::artifacts::get_artifact_manager(location))
        .map_err(artifact_err_to_py_err)?;
    Ok(ArtifactManager(Arc::from(artifact_manager)))
}

#[pyfunction]
#[pyo3(signature = (backup_artifact_manager, artifact_manager, timeout=None))]
fn upload_backup_artifacts<'a>(
    py: Python<'a>,
    backup_artifact_manager: &ArtifactManager,
    artifact_manager: &ArtifactManager,
    timeout: Option<u64>,
) -> PyResult<Bound<'a, PyAny>> {
    let z = backup_artifact_manager.0.clone();
    let y = artifact_manager.0.clone();
    pyo3_async_runtimes::tokio::future_into_py(py, async move {
        let r = tokio::time::timeout(
            std::time::Duration::from_secs(timeout.unwrap_or(60)),
            janitor::artifacts::upload_backup_artifacts(z.as_ref(), y.as_ref()),
        )
        .await
        .map_err(|_| PyTimeoutError::new_err("Timeout"))?
        .map_err(artifact_err_to_py_err)?;

        Ok(r.into_iter().collect::<Vec<_>>())
    })
}

#[pyfunction]
#[pyo3(signature = (manager, backup_manager, from_dir, run_id, names=None))]
fn store_artifacts_with_backup<'a>(
    py: Python<'a>,
    manager: &ArtifactManager,
    backup_manager: Option<&ArtifactManager>,
    from_dir: &str,
    run_id: &str,
    names: Option<Vec<String>>,
) -> PyResult<Bound<'a, PyAny>> {
    let from_dir = std::path::PathBuf::from(from_dir);
    let run_id = run_id.to_string();
    let z = manager.0.clone();
    let y = backup_manager.as_ref().map(|x| x.0.clone());
    pyo3_async_runtimes::tokio::future_into_py(py, async move {
        janitor::artifacts::store_artifacts_with_backup(
            z.as_ref(),
            y.as_deref(),
            &from_dir,
            &run_id,
            names.as_deref(),
        )
        .await
        .map_err(artifact_err_to_py_err)
    })
}

pub(crate) fn init(py: Python, module: &Bound<PyModule>) -> PyResult<()> {
    module.add_class::<ArtifactManager>()?;
    module.add_class::<LocalArtifactManager>()?;
    module.add_class::<GCSArtifactManager>()?;

    module.add_function(wrap_pyfunction_bound!(list_ids, module)?)?;
    module.add_function(wrap_pyfunction_bound!(get_artifact_manager, module)?)?;

    module.add_function(wrap_pyfunction_bound!(upload_backup_artifacts, module)?)?;
    module.add_function(wrap_pyfunction_bound!(store_artifacts_with_backup, module)?)?;

    module.add(
        "ServiceUnavailable",
        py.get_type_bound::<ServiceUnavailable>(),
    )?;
    module.add("ArtifactsMissing", py.get_type_bound::<ArtifactsMissing>())?;
    Ok(())
}
