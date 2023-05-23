use pyo3::prelude::*;

#[pyfunction]
fn parse_plain_text_body(text: &str) -> PyResult<Option<String>> {
    Ok(janitor_mail_filter::parse_plain_text_body(text))
}

#[pyfunction]
fn parse_html_body(html: &str) -> PyResult<Option<String>> {
    Ok(janitor_mail_filter::parse_html_body(html))
}

#[pymodule]
pub fn _mail_filter(py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parse_plain_text_body, m)?)?;
    m.add_function(wrap_pyfunction!(parse_html_body, m)?)?;
    Ok(())
}
