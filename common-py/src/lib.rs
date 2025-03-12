// Necessary since create_exception!() uses cfg!(feature = "gil-refs"),
// but we don't have that feature.
#![allow(unexpected_cfgs)]
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

mod artifacts;
mod vcs;

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

#[pymodule]
pub fn _common(py: Python, m: &Bound<PyModule>) -> PyResult<()> {
    pyo3_log::init();
    m.add_function(wrap_pyfunction!(is_authenticated_url, m)?)?;
    m.add_function(wrap_pyfunction!(is_alioth_url, m)?)?;
    m.add_function(wrap_pyfunction!(get_branch_vcs_type, m)?)?;

    let artifactsm = pyo3::types::PyModule::new_bound(py, "artifacts")?;
    crate::artifacts::init(py, &artifactsm)?;
    m.add_submodule(&artifactsm)?;

    let vcsm = pyo3::types::PyModule::new_bound(py, "vcs")?;
    crate::vcs::init(py, &vcsm)?;
    m.add_submodule(&vcsm)?;

    Ok(())
}
