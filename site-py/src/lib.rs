use pyo3::prelude::*;

#[pyfunction]
fn find_dist_log_failure(logf: &str, length: usize) -> (usize, (usize, usize), Option<Vec<usize>>) {
    let r = janitor_site::analyze::find_dist_log_failure(logf, length);
    (r.total_lines, r.include_lines, r.highlight_lines)
}

#[pyfunction]
fn find_build_log_failure(
    logf: &[u8],
    length: usize,
) -> (usize, (usize, usize), Option<Vec<usize>>) {
    let r = janitor_site::analyze::find_build_log_failure(logf, length);
    (r.total_lines, r.include_lines, r.highlight_lines)
}

#[pymodule]
fn _site(_py: Python, m: &Bound<PyModule>) -> PyResult<()> {
    pyo3_log::init();
    m.add_function(wrap_pyfunction!(find_dist_log_failure, m)?)?;
    m.add_function(wrap_pyfunction!(find_build_log_failure, m)?)?;
    Ok(())
}
