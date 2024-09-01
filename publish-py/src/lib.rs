use chrono::{DateTime, Utc};
use pyo3::prelude::*;

#[pyfunction]
fn calculate_next_try_time(finish_time: DateTime<Utc>, attempt_count: usize) -> DateTime<Utc> {
    janitor_publish::calculate_next_try_time(finish_time, attempt_count)
}

#[pymodule]
pub fn _publish(m: &Bound<PyModule>) -> PyResult<()> {
    pyo3_log::init();
    m.add_function(wrap_pyfunction!(calculate_next_try_time, m)?)?;
    Ok(())
}
