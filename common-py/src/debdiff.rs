use pyo3::create_exception;
use pyo3::prelude::*;
use pyo3::types::PyBytes;

#[pyfunction]
fn debdiff_is_empty(debdiff: &str) -> PyResult<bool> {
    Ok(janitor::debdiff::debdiff_is_empty(debdiff))
}

#[pyfunction]
fn filter_boring(debdiff: &str, old_version: &str, new_version: &str) -> PyResult<String> {
    Ok(janitor::debdiff::filter_boring(
        debdiff,
        old_version,
        new_version,
    ))
}

#[pyfunction]
fn section_is_wdiff(title: &str) -> PyResult<(bool, Option<&str>)> {
    Ok(janitor::debdiff::section_is_wdiff(title))
}

#[pyfunction]
fn markdownify_debdiff(debdiff: &str) -> PyResult<String> {
    Ok(janitor::debdiff::markdownify_debdiff(debdiff))
}

#[pyfunction]
fn htmlize_debdiff(debdiff: &str) -> PyResult<String> {
    Ok(janitor::debdiff::htmlize_debdiff(debdiff))
}

create_exception!(
    janitor.debian.debdiff,
    DebdiffError,
    pyo3::exceptions::PyException
);

#[pyfunction]
fn run_debdiff(
    py: Python<'_>,
    old_binaries: Vec<String>,
    new_binaries: Vec<String>,
) -> PyResult<Bound<'_, PyAny>> {
    pyo3_async_runtimes::tokio::future_into_py(py, async move {
        let r = janitor::debdiff::run_debdiff(
            old_binaries.iter().map(|x| x.as_str()).collect::<Vec<_>>(),
            new_binaries.iter().map(|x| x.as_str()).collect::<Vec<_>>(),
        )
        .await
        .map_err(|e| DebdiffError::new_err((e.to_string(),)))?;

        Ok(Python::with_gil(|py| {
            PyBytes::new_bound(py, &r).to_object(py)
        }))
    })
}

pub(crate) fn init_module(py: Python, m: &Bound<PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(debdiff_is_empty, m)?)?;
    m.add_function(wrap_pyfunction!(filter_boring, m)?)?;
    m.add_function(wrap_pyfunction!(section_is_wdiff, m)?)?;
    m.add_function(wrap_pyfunction!(markdownify_debdiff, m)?)?;
    m.add_function(wrap_pyfunction!(htmlize_debdiff, m)?)?;
    m.add_function(wrap_pyfunction!(run_debdiff, m)?)?;
    m.add("DebdiffError", py.get_type_bound::<DebdiffError>())?;
    Ok(())
}
