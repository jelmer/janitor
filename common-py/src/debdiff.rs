use pyo3::prelude::*;

#[pyfunction]
fn debdiff_is_empty(debdiff: &str) -> PyResult<bool> {
    Ok(janitor::debdiff::debdiff_is_empty(debdiff))
}

pub(crate) fn init_module(py: Python, m: &Bound<PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(debdiff_is_empty, m)?)?;
    Ok(())
}
