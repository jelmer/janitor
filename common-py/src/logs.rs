use crate::io::Readable;
use chrono::{DateTime, Utc};
use pyo3::create_exception;
use pyo3::exceptions::{PyRuntimeError, PyTimeoutError};
use pyo3::prelude::*;
use std::sync::Arc;

create_exception!(
    janitor.logs,
    ServiceUnavailable,
    pyo3::exceptions::PyException
);

fn convert_logs_error_to_py(err: janitor::logs::Error) -> PyErr {
    match err {
        janitor::logs::Error::ServiceUnavailable => {
            ServiceUnavailable::new_err("Service unavailable")
        }
        janitor::logs::Error::NotFound => pyo3::exceptions::PyKeyError::new_err("Log not found"),
        janitor::logs::Error::PermissionDenied => PyRuntimeError::new_err("Permission denied"),
        janitor::logs::Error::Io(e) => e.into(),
        janitor::logs::Error::Other(e) => PyRuntimeError::new_err(e),
    }
}

#[pyclass(subclass)]
pub struct LogFileManager(Arc<dyn janitor::logs::LogFileManager + Send + Sync>);

#[pymethods]
impl LogFileManager {
    #[pyo3(signature = (codebase, run_id, name, timeout=None))]
    fn has_log<'a>(
        &self,
        py: Python<'a>,
        codebase: String,
        run_id: String,
        name: String,
        timeout: Option<u64>,
    ) -> PyResult<Bound<'a, PyAny>> {
        let z = self.0.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let f = z.has_log(&codebase, &run_id, &name);
            let r = if let Some(timeout) = timeout {
                tokio::time::timeout(std::time::Duration::from_secs(timeout), f)
                    .await
                    .map_err(|_| PyTimeoutError::new_err("Timeout"))?
            } else {
                f.await
            };
            r.map_err(|e| convert_logs_error_to_py(e))
                .map(|r| Python::with_gil(|py| r.into_py(py)))
        })
    }

    #[pyo3(signature = (codebase, run_id, name, timeout=None))]
    fn get_log<'a>(
        &self,
        py: Python<'a>,
        codebase: String,
        run_id: String,
        name: String,
        timeout: Option<u64>,
    ) -> PyResult<Bound<'a, PyAny>> {
        let z = self.0.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let f = z.get_log(&codebase, &run_id, &name);
            let r = if let Some(timeout) = timeout {
                tokio::time::timeout(std::time::Duration::from_secs(timeout), f)
                    .await
                    .map_err(|_| PyTimeoutError::new_err("Timeout"))?
            } else {
                f.await
            };
            let readable = r.map_err(|e| convert_logs_error_to_py(e))?;
            Ok(Readable::new(readable))
        })
    }

    #[pyo3(signature = (codebase, run_id, orig_path, timeout=None, mtime=None, basename=None))]
    fn import_log<'a>(
        &self,
        py: Python<'a>,
        codebase: String,
        run_id: String,
        orig_path: String,
        timeout: Option<u64>,
        mtime: Option<DateTime<Utc>>,
        basename: Option<String>,
    ) -> PyResult<Bound<'a, PyAny>> {
        let z = self.0.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let f = z.import_log(&codebase, &run_id, &orig_path, mtime, basename.as_deref());
            let r = if let Some(timeout) = timeout {
                tokio::time::timeout(std::time::Duration::from_secs(timeout), f)
                    .await
                    .map_err(|_| PyTimeoutError::new_err("Timeout"))?
            } else {
                f.await
            };
            r.map_err(|e| convert_logs_error_to_py(e))?;
            Ok(Python::with_gil(|py| py.None()))
        })
    }

    #[pyo3(signature = (codebase, run_id, name))]
    fn get_ctime<'a>(
        &self,
        py: Python<'a>,
        codebase: String,
        run_id: String,
        name: String,
    ) -> PyResult<Bound<'a, PyAny>> {
        let z = self.0.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let r = z
                .get_ctime(&codebase, &run_id, &name)
                .await
                .map_err(|e| convert_logs_error_to_py(e))?;
            Ok(Python::with_gil(|py| r.into_py(py)))
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

    #[pyo3(signature = ())]
    fn iter_logs<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let z = self.0.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let r = z.iter_logs().await.collect::<Vec<_>>();

            Python::with_gil(|py| {
                let list = pyo3::types::PyList::empty_bound(py);
                for (codebase, run_id, name) in r {
                    let tuple = pyo3::types::PyTuple::empty_bound(py);
                    tuple.set_item(0, codebase)?;
                    tuple.set_item(1, run_id)?;
                    tuple.set_item(2, name)?;
                    list.append(tuple)?;
                }
                Ok(list.into_py(py))
            })
        })
    }
}

#[pyclass(extends=LogFileManager)]
pub struct FileSystemLogFileManager;

#[pymethods]
impl FileSystemLogFileManager {
    #[new]
    fn new(log_directory: std::path::PathBuf) -> PyResult<(Self, LogFileManager)> {
        let z = janitor::logs::FileSystemLogFileManager::new(log_directory).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyIOError, _>(format!(
                "Failed to create log manager: {}",
                e
            ))
        })?;
        Ok((FileSystemLogFileManager, LogFileManager(Arc::new(z))))
    }
}

pub(crate) fn init(py: Python, module: &Bound<PyModule>) -> PyResult<()> {
    module.add_class::<LogFileManager>()?;
    module.add_class::<FileSystemLogFileManager>()?;
    module.add(
        "ServiceUnavailable",
        py.get_type_bound::<ServiceUnavailable>(),
    )?;
    Ok(())
}
