use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

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

#[pymodule]
pub fn _common(m: &Bound<PyModule>) -> PyResult<()> {
    pyo3_log::init();
    m.add_function(wrap_pyfunction!(is_authenticated_url, m)?)?;
    m.add_function(wrap_pyfunction!(is_alioth_url, m)?)?;
    Ok(())
}
