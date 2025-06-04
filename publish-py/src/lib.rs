use chrono::{DateTime, Utc};
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use std::collections::HashMap;

#[pyclass(extends=PyException,subclass)]
pub struct RateLimited {
    #[pyo3(get)]
    message: String,
}

#[pymethods]
impl RateLimited {
    #[new]
    fn new(message: String) -> Self {
        Self { message }
    }
}

#[pyclass(extends=RateLimited)]
pub struct BucketRateLimited {
    #[pyo3(get)]
    bucket: String,

    #[pyo3(get)]
    open_mps: usize,

    #[pyo3(get)]
    max_open_mps: usize,
}

#[pymethods]
impl BucketRateLimited {
    #[new]
    fn new(bucket: String, open_mps: usize, max_open_mps: usize) -> (Self, RateLimited) {
        (
            Self {
                bucket,
                open_mps,
                max_open_mps,
            },
            RateLimited::new(format!(
                "Bucket rate limited: {} open_mps, {} max_open_mps",
                open_mps, max_open_mps
            )),
        )
    }
}

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

#[pyfunction]
#[pyo3(signature = (url, remote_branch_name = None))]
fn role_branch_url(url: &str, remote_branch_name: Option<&str>) -> PyResult<String> {
    let url = url.parse().map_err(|e: url::ParseError| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string())
    })?;
    Ok(janitor_publish::role_branch_url(&url, remote_branch_name).to_string())
}

#[pyclass(subclass)]
pub struct RateLimiter(Box<dyn janitor_publish::rate_limiter::RateLimiter>);

#[pymethods]
impl RateLimiter {
    fn check_allowed(&self, bucket: &str) -> PyResult<()> {
        match self.0.check_allowed(bucket) {
            janitor_publish::rate_limiter::RateLimitStatus::Allowed => Ok(()),
            janitor_publish::rate_limiter::RateLimitStatus::RateLimited => {
                Err(PyErr::new::<RateLimited, _>("Rate limited"))
            }
            janitor_publish::rate_limiter::RateLimitStatus::BucketRateLimited {
                bucket,
                open_mps,
                max_open_mps,
            } => {
                let e = Python::with_gil(|py| {
                    Py::new(py, BucketRateLimited::new(bucket, open_mps, max_open_mps)).unwrap()
                });
                Err(PyErr::new::<BucketRateLimited, _>(e))
            }
        }
    }

    fn inc(&mut self, bucket: &str) {
        self.0.inc(bucket)
    }

    fn get_max_open(&self, bucket: &str) -> Option<usize> {
        self.0.get_max_open(bucket)
    }

    fn set_mps_per_bucket(&mut self, mps_per_bucket: HashMap<String, HashMap<String, usize>>) {
        let mps_per_bucket = mps_per_bucket
            .into_iter()
            .map(|(k, v)| (k.parse().unwrap(), v.into_iter().collect()))
            .collect();
        self.0.set_mps_per_bucket(&mps_per_bucket)
    }

    fn get_stats(&self) -> HashMap<String, usize> {
        let stats = self.0.get_stats();
        if let Some(stats) = stats {
            stats.per_bucket
        } else {
            HashMap::new()
        }
    }
}

#[pyclass(extends=RateLimiter)]
pub struct SlowStartRateLimiter;

#[pymethods]
impl SlowStartRateLimiter {
    #[new]
    #[pyo3(signature = (max_mps_per_bucket = None))]
    fn new(max_mps_per_bucket: Option<usize>) -> (Self, RateLimiter) {
        let limiter = janitor_publish::rate_limiter::SlowStartRateLimiter::new(max_mps_per_bucket);
        (Self, RateLimiter(Box::new(limiter)))
    }
}

#[pyclass(extends=RateLimiter)]
pub struct FixedRateLimiter;

#[pymethods]
impl FixedRateLimiter {
    #[new]
    fn new(max_mps_per_bucket: usize) -> (Self, RateLimiter) {
        let limiter = janitor_publish::rate_limiter::FixedRateLimiter::new(max_mps_per_bucket);
        (Self, RateLimiter(Box::new(limiter)))
    }
}

#[pyclass(extends=RateLimiter)]
pub struct NonRateLimiter;

#[pymethods]
impl NonRateLimiter {
    #[new]
    fn new() -> (Self, RateLimiter) {
        let limiter = janitor_publish::rate_limiter::NonRateLimiter::new();
        (Self, RateLimiter(Box::new(limiter)))
    }
}

#[pymodule]
pub fn _publish(py: Python, m: &Bound<PyModule>) -> PyResult<()> {
    pyo3_log::init();
    m.add_function(wrap_pyfunction!(calculate_next_try_time, m)?)?;
    m.add_function(wrap_pyfunction!(get_merged_by_user_url, m)?)?;
    m.add_function(wrap_pyfunction!(branches_match, m)?)?;
    m.add_function(wrap_pyfunction!(role_branch_url, m)?)?;

    m.add_class::<RateLimiter>()?;
    m.add_class::<SlowStartRateLimiter>()?;
    m.add_class::<FixedRateLimiter>()?;
    m.add_class::<NonRateLimiter>()?;

    m.add("RateLimited", py.get_type_bound::<RateLimited>())?;
    m.add(
        "BucketRateLimited",
        py.get_type_bound::<BucketRateLimited>(),
    )?;
    Ok(())
}
