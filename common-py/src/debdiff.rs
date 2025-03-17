use pyo3::prelude::*;

#[pyfunction]
fn debdiff_is_empty(debdiff: &str) -> PyResult<bool> {
    Ok(janitor::debdiff::debdiff_is_empty(debdiff))
}

#[pyfunction]
fn filter_boring(debdiff: &str, old_version: &str, new_version: &str) -> PyResult<String> {
    Ok(janitor::debdiff::filter_boring(debdiff, old_version, new_version))
}

#[pyfunction]
fn section_is_wdiff(title: &str) -> PyResult<(bool, Option<&str>)> {
    Ok(janitor::debdiff::section_is_wdiff(title))
}

pub(crate) fn init_module(py: Python, m: &Bound<PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(debdiff_is_empty, m)?)?;
    m.add_function(wrap_pyfunction!(filter_boring, m)?)?;
    m.add_function(wrap_pyfunction!(section_is_wdiff, m)?)?;
    Ok(())
}
