use pyo3::prelude::*;
use std::collections::HashMap;

#[pyfunction]
#[pyo3(signature = (committer=None))]
fn committer_env(committer: Option<&str>) -> HashMap<String, String> {
    janitor_runner::committer_env(committer)
}

#[pyfunction]
fn is_log_filename(filename: &str) -> bool {
    janitor_runner::is_log_filename(filename)
}

#[pymodule]
fn _runner(m: &Bound<PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(committer_env, m)?)?;
    m.add_function(wrap_pyfunction!(is_log_filename, m)?)?;
    Ok(())
}
