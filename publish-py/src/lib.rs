use chrono::{DateTime, Utc};
use pyo3::prelude::*;

#[pyfunction]
fn calculate_next_try_time(finish_time: DateTime<Utc>, attempt_count: usize) -> DateTime<Utc> {
    janitor_publish::calculate_next_try_time(finish_time, attempt_count)
}

#[pyfunction]
fn get_merged_by_user_url(url: &str, user: &str) -> PyResult<Option<String>> {
    let url: url::Url = url.parse().map_err(|e: url::ParseError| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string())
    })?;
    Ok(janitor_publish::get_merged_by_user_url(&url, user)?.map(|u| u.to_string()))
}

#[pymodule]
pub fn _publish(m: &Bound<PyModule>) -> PyResult<()> {
    pyo3_log::init();
    m.add_function(wrap_pyfunction!(calculate_next_try_time, m)?)?;
    m.add_function(wrap_pyfunction!(get_merged_by_user_url, m)?)?;
    Ok(())
}
