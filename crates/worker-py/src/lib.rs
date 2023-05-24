use janitor_worker::{AssignmentError};
use pyo3::create_exception;
use pyo3::prelude::*;

create_exception!(
    janitor._worker,
    AssignmentFailure,
    pyo3::exceptions::PyException
);
create_exception!(janitor._worker, EmptyQueue, pyo3::exceptions::PyException);
create_exception!(
    janitor._worker,
    ResultUploadFailure,
    pyo3::exceptions::PyException
);

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

fn py_to_serde_json(obj: &PyAny) -> PyResult<serde_json::Value> {
    if obj.is_none() {
        Ok(serde_json::Value::Null)
    } else if let Ok(b) = obj.downcast::<pyo3::types::PyBool>() {
        Ok(serde_json::Value::Bool(b.is_true()))
    } else if let Ok(f) = obj.downcast::<pyo3::types::PyFloat>() {
        Ok(serde_json::Value::Number(
            serde_json::Number::from_f64(f.value()).unwrap(),
        ))
    } else if let Ok(s) = obj.downcast::<pyo3::types::PyString>() {
        Ok(serde_json::Value::String(s.to_string_lossy().to_string()))
    } else if let Ok(l) = obj.downcast::<pyo3::types::PyList>() {
        Ok(serde_json::Value::Array(
            l.iter()
                .map(py_to_serde_json)
                .collect::<PyResult<Vec<_>>>()?,
        ))
    } else if let Ok(d) = obj.downcast::<pyo3::types::PyDict>() {
        let mut ret = serde_json::Map::new();
        for (k, v) in d.iter() {
            let k = k.extract::<String>()?;
            let v = py_to_serde_json(v)?;
            ret.insert(k, v);
        }
        Ok(serde_json::Value::Object(ret))
    } else {
        Err(pyo3::exceptions::PyTypeError::new_err(
            ("unsupported type",),
        ))
    }
}

fn serde_json_to_py(value: serde_json::Value) -> PyObject {
    Python::with_gil(|py| match value {
        serde_json::Value::Null => py.None(),
        serde_json::Value::Bool(b) => pyo3::types::PyBool::new(py, b).into(),
        serde_json::Value::Number(n) => pyo3::types::PyFloat::new(py, n.as_f64().unwrap()).into(),
        serde_json::Value::String(s) => pyo3::types::PyString::new(py, s.as_str()).into(),
        serde_json::Value::Array(a) => {
            pyo3::types::PyList::new(py, a.into_iter().map(serde_json_to_py)).into()
        }
        serde_json::Value::Object(o) => {
            let ret = pyo3::types::PyDict::new(py);
            for (k, v) in o.into_iter() {
                ret.set_item(k, serde_json_to_py(v)).unwrap();
            }
            ret.into()
        }
    })
}

#[pyclass]
struct Client(std::sync::Arc<janitor_worker::Client>);

#[pymethods]
impl Client {
    #[new]
    fn new(
        base_url: &str,
        username: Option<&str>,
        password: Option<&str>,
        user_agent: &str,
    ) -> Self {
        if username.is_none() && password.is_some() {
            panic!("password specified without username");
        }
        let credentials = if let Some(username) = username {
            janitor_worker::Credentials::Basic {
                username: username.to_string(),
                password: password.map(|s| s.to_string()),
            }
        } else {
            janitor_worker::Credentials::None
        };

        Self(std::sync::Arc::new(janitor_worker::Client::new(
            reqwest::Url::parse(base_url).unwrap(),
            credentials,
            user_agent,
        )))
    }

    fn get_assignment_raw<'a>(
        &self,
        py: Python<'a>,
        my_url: Option<&str>,
        node_name: &str,
        jenkins_build_url: Option<&str>,
        codebase: Option<&str>,
        campaign: Option<&str>,
    ) -> PyResult<&'a PyAny> {
        let campaign = campaign.map(|s| s.to_string());
        let codebase = codebase.map(|s| s.to_string());
        let node_name = node_name.to_string();
        let jenkins_build_url = jenkins_build_url.map(|s| s.to_string());
        let my_url = my_url.map(|s| s.to_string());
        let client = self.0.clone();
        pyo3_asyncio::tokio::future_into_py(py, async move {
            let assignment = client
                .get_assignment_raw(
                    my_url.map(|u| reqwest::Url::parse(u.as_str()).unwrap()),
                    node_name.as_str(),
                    jenkins_build_url.as_deref(),
                    codebase.as_deref(),
                    campaign.as_deref(),
                )
                .await
                .map_err(|e| match e {
                    AssignmentError::Failure(msg) => AssignmentFailure::new_err(msg),
                    AssignmentError::EmptyQueue => EmptyQueue::new_err(()),
                })?;

            Ok(serde_json_to_py(assignment))
        })
    }

    fn upload_results<'a>(
        &self,
        py: Python<'a>,
        run_id: &str,
        metadata: PyObject,
        output_directory: Option<std::path::PathBuf>,
    ) -> PyResult<&'a PyAny> {
        let client = self.0.clone();
        let run_id = run_id.to_string();
        let metadata = py_to_serde_json(metadata.as_ref(py))?;
        pyo3_asyncio::tokio::future_into_py(py, async move {
            let result = client
                .upload_results(run_id.as_str(), &metadata, output_directory.as_deref())
                .await
                .map_err(|e| ResultUploadFailure::new_err(format!("{:?}", e)))?;

            Ok(serde_json_to_py(result))
        })
    }
}

#[pymodule]
pub fn _worker(py: Python, m: &PyModule) -> PyResult<()> {
    pyo3_log::init();
    m.add_function(wrap_pyfunction!(is_gce_instance, m)?)?;
    m.add_function(wrap_pyfunction!(gce_external_ip, m)?)?;
    m.add_class::<Client>()?;
    m.add("ResultUploadFailure", py.get_type::<ResultUploadFailure>())?;
    m.add("AssignmentFailure", py.get_type::<AssignmentFailure>())?;
    m.add("EmptyQueue", py.get_type::<EmptyQueue>())?;
    Ok(())
}
