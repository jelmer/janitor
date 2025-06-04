use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use std::io::Read;

#[pyclass]
pub(crate) struct Readable(Box<dyn std::io::Read + Send + Sync>);

impl Readable {
    pub fn new(read: Box<dyn std::io::Read + Send + Sync>) -> Self {
        Self(read)
    }
}

#[pymethods]
impl Readable {
    #[pyo3(signature = (size=None))]
    fn read(&mut self, py: Python, size: Option<usize>) -> PyResult<PyObject> {
        let mut buf = vec![0; size.unwrap_or(4096)];
        let n = self.0.read(&mut buf).map_err(PyRuntimeError::new_err)?;
        buf.truncate(n);
        Ok(PyBytes::new_bound(py, &buf).into())
    }
}
