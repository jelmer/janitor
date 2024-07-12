use pyo3::prelude::*;

#[pymodule]
pub fn _common(py: Python, m: &Bound<PyModule>) -> PyResult<()> {
    pyo3_log::init();
    Ok(())
}
