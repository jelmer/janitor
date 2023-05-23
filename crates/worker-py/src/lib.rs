use pyo3::prelude::*;

#[pyfunction]
fn is_gce_instance(py: Python) -> PyResult<&PyAny> {
    pyo3_asyncio::tokio::future_into_py(py, async { Ok(janitor_worker::is_gce_instance().await) })
}

#[pyfunction]
fn gce_external_ip(py: Python) -> PyResult<&PyAny> {
    pyo3_asyncio::tokio::future_into_py(py, async {
        janitor_worker::gce_external_ip()
            .await
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{}", e)))
    })
}

#[pymodule]
pub fn _worker(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(is_gce_instance, m)?)?;
    m.add_function(wrap_pyfunction!(gce_external_ip, m)?)?;
    Ok(())
}
