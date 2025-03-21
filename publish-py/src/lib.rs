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

#[pyfunction]
#[pyo3(signature = (url_a, url_b))]
fn branches_match(url_a: Option<&str>, url_b: Option<&str>) -> PyResult<bool> {
    let url_a = if let Some(url) = url_a {
        Some(url.parse().map_err(|e: url::ParseError| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string())
        })?)
    } else {
        None
    };
    let url_b = if let Some(url) = url_b {
        Some(url.parse().map_err(|e: url::ParseError| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string())
        })?)
    } else {
        None
    };

    Ok(janitor_publish::branches_match(
        url_a.as_ref(),
        url_b.as_ref(),
    ))
}

#[pymodule]
pub fn _publish(m: &Bound<PyModule>) -> PyResult<()> {
    pyo3_log::init();
    m.add_function(wrap_pyfunction!(calculate_next_try_time, m)?)?;
    m.add_function(wrap_pyfunction!(get_merged_by_user_url, m)?)?;
    m.add_function(wrap_pyfunction!(branches_match, m)?)?;
    Ok(())
}
