use chrono::NaiveDateTime;
use janitor_worker::{AssignmentError, Remote, RevisionId};
use pyo3::create_exception;
use pyo3::exceptions::{PyException, PyNotImplementedError, PyTypeError};
use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use std::path::Path;

create_exception!(
    janitor._worker,
    AssignmentFailure,
    pyo3::exceptions::PyException
);
create_exception!(
    janitor_worker.debian.lintian,
    LintianOutputInvalid,
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
        Err(PyTypeError::new_err(("unsupported type",)))
    }
}

fn serde_json_to_py(value: &serde_json::Value) -> PyObject {
    Python::with_gil(|py| match value {
        serde_json::Value::Null => py.None(),
        serde_json::Value::Bool(b) => pyo3::types::PyBool::new(py, *b).into(),
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

            Ok(serde_json_to_py(&assignment))
        })
    }

    fn upload_results<'a>(
        &self,
        py: Python<'a>,
        run_id: &str,
        metadata: &'a Metadata,
        output_directory: Option<std::path::PathBuf>,
    ) -> PyResult<&'a PyAny> {
        let client = self.0.clone();
        let run_id = run_id.to_string();
        let metadata = metadata.0.clone();
        pyo3_asyncio::tokio::future_into_py(py, async move {
            let result = client
                .upload_results(run_id.as_str(), &metadata, output_directory.as_deref())
                .await
                .map_err(|e| ResultUploadFailure::new_err(format!("{:?}", e)))?;

            Ok(serde_json_to_py(&result))
        })
    }
}

#[pyfunction]
fn abort_run<'a>(
    py: Python<'a>,
    client: &Client,
    run_id: &str,
    metadata: &Metadata,
    description: &str,
) -> PyResult<&'a PyAny> {
    let client = client.0.clone();
    let run_id = run_id.to_string();
    let description = description.to_string();
    let metadata = metadata.0.clone();
    pyo3_asyncio::tokio::future_into_py(py, async move {
        janitor_worker::abort_run(&client, run_id.as_str(), &metadata, description.as_str()).await;
        Ok(())
    })
}

#[pyfunction]
fn run_lintian(
    output_directory: &str,
    changes_names: Vec<&str>,
    profile: Option<&str>,
    suppress_tags: Option<Vec<&str>>,
) -> PyResult<PyObject> {
    let result = janitor_worker::debian::lintian::run_lintian(
        output_directory,
        changes_names,
        profile,
        suppress_tags,
    )
    .map_err(|e| match e {
        janitor_worker::debian::lintian::Error::LintianFailed(e) => e.into(),
        janitor_worker::debian::lintian::Error::LintianOutputInvalid(e) => {
            LintianOutputInvalid::new_err((e,))
        }
    })?;
    Ok(serde_json_to_py(&result))
}

create_exception!(janitor_worker, WorkerFailure, pyo3::exceptions::PyException);

#[pyclass]
struct Assignment(janitor_worker::Assignment);

#[pymethods]
impl Assignment {}

#[pyclass]
struct MetadataTarget(janitor_worker::Target);

#[pymethods]
impl MetadataTarget {
    #[getter]
    fn get_name(&self) -> PyResult<&str> {
        Ok(self.0.name.as_str())
    }

    #[new]
    fn new(name: &str) -> Self {
        Self(janitor_worker::Target::new(
            name.to_string(),
            serde_json::Value::Null,
        ))
    }

    #[getter]
    fn get_details(&self) -> PyResult<PyObject> {
        Ok(serde_json_to_py(&self.0.details))
    }

    #[setter]
    fn set_details(&mut self, details: &PyAny) -> PyResult<()> {
        self.0.details = py_to_serde_json(details)?;
        Ok(())
    }
}

#[pyclass]
struct Metadata(janitor_worker::Metadata);

#[pymethods]
impl Metadata {
    #[new]
    fn new() -> PyResult<Self> {
        Ok(Self(janitor_worker::Metadata::default()))
    }

    #[getter]
    fn get_command(&self) -> PyResult<Option<Vec<String>>> {
        Ok(self.0.command.clone())
    }

    #[setter]
    fn set_command(&mut self, command: Option<Vec<String>>) -> PyResult<()> {
        self.0.command = command;
        Ok(())
    }

    #[getter]
    fn get_codebase(&self) -> PyResult<Option<&str>> {
        Ok(self.0.codebase.as_deref())
    }

    #[setter]
    fn set_codebase(&mut self, codebase: Option<&str>) -> PyResult<()> {
        self.0.codebase = codebase.map(|s| s.to_string());
        Ok(())
    }

    #[getter]
    fn get_code(&self) -> PyResult<Option<&str>> {
        Ok(self.0.code.as_deref())
    }

    #[setter]
    fn set_code(&mut self, code: Option<&str>) -> PyResult<()> {
        self.0.code = code.map(|s| s.to_string());
        Ok(())
    }

    #[getter]
    fn get_description(&self) -> PyResult<Option<&str>> {
        Ok(self.0.description.as_deref())
    }

    #[setter]
    fn set_description(&mut self, description: Option<&str>) -> PyResult<()> {
        self.0.description = description.map(|s| s.to_string());
        Ok(())
    }

    #[getter]
    fn get_start_time(&self) -> PyResult<Option<NaiveDateTime>> {
        Ok(self.0.start_time)
    }

    #[setter]
    fn set_start_time(&mut self, start_time: Option<NaiveDateTime>) -> PyResult<()> {
        self.0.start_time = start_time;
        Ok(())
    }

    #[getter]
    fn get_finish_time(&self) -> PyResult<Option<NaiveDateTime>> {
        Ok(self.0.finish_time)
    }

    #[setter]
    fn set_finish_time(&mut self, finish_time: Option<NaiveDateTime>) -> PyResult<()> {
        self.0.finish_time = finish_time;
        Ok(())
    }

    #[getter]
    fn get_queue_id(&self) -> PyResult<Option<u64>> {
        Ok(self.0.queue_id)
    }

    #[setter]
    fn set_queue_id(&mut self, queue_id: Option<u64>) -> PyResult<()> {
        self.0.queue_id = queue_id;
        Ok(())
    }

    #[getter]
    fn get_branch_url(&self) -> PyResult<Option<&str>> {
        Ok(self.0.branch_url.as_ref().map(|s| s.as_str()))
    }

    #[setter]
    fn set_branch_url(&mut self, branch_url: Option<&str>) -> PyResult<()> {
        self.0.branch_url = branch_url.map(|s| s.parse().unwrap());
        Ok(())
    }

    #[getter]
    fn get_vcs_type(&self) -> PyResult<Option<&str>> {
        Ok(self.0.vcs_type.as_deref())
    }

    #[setter]
    fn set_vcs_type(&mut self, vcs_type: Option<&str>) -> PyResult<()> {
        self.0.vcs_type = vcs_type.map(|s| s.to_string());
        Ok(())
    }

    #[getter]
    fn get_subpath(&self) -> PyResult<Option<&str>> {
        Ok(self.0.subpath.as_deref())
    }

    #[setter]
    fn set_subpath(&mut self, subpath: Option<&str>) -> PyResult<()> {
        self.0.subpath = subpath.map(|s| s.to_string());
        Ok(())
    }

    #[getter]
    fn get_revision(&self) -> PyResult<Option<RevisionId>> {
        Ok(self.0.revision.clone())
    }

    #[setter]
    fn set_revision(&mut self, revision: Option<RevisionId>) -> PyResult<()> {
        self.0.revision = revision;
        Ok(())
    }

    #[getter]
    fn get_main_branch_revision(&self) -> PyResult<Option<RevisionId>> {
        Ok(self.0.main_branch_revision.clone())
    }

    #[setter]
    fn set_main_branch_revision(&mut self, revision: Option<RevisionId>) -> PyResult<()> {
        self.0.main_branch_revision = revision;
        Ok(())
    }

    #[getter]
    fn get_refreshed(&self) -> PyResult<Option<bool>> {
        Ok(self.0.refreshed)
    }

    #[setter]
    fn set_refreshed(&mut self, refreshed: Option<bool>) -> PyResult<()> {
        self.0.refreshed = refreshed;
        Ok(())
    }

    #[getter]
    fn get_value(&self) -> PyResult<Option<u64>> {
        Ok(self.0.value)
    }

    #[setter]
    fn set_value(&mut self, value: Option<u64>) -> PyResult<()> {
        self.0.value = value;
        Ok(())
    }

    #[getter]
    fn get_campaign(&self) -> PyResult<Option<&str>> {
        Ok(self.0.campaign.as_deref())
    }

    #[setter]
    fn set_campaign(&mut self, campaign: Option<&str>) -> PyResult<()> {
        self.0.campaign = campaign.map(|s| s.to_string());
        Ok(())
    }

    #[getter]
    fn get_target_branch_url(&self) -> PyResult<Option<&str>> {
        Ok(self.0.target_branch_url.as_ref().map(|s| s.as_str()))
    }

    #[setter]
    fn set_target_branch_url(&mut self, target_branch_url: Option<&str>) -> PyResult<()> {
        self.0.target_branch_url = target_branch_url.map(|s| s.parse().unwrap());
        Ok(())
    }

    fn add_remote(&mut self, name: &str, url: &str) -> PyResult<()> {
        self.0.remotes.insert(
            name.to_string(),
            Remote {
                url: url.parse().unwrap(),
            },
        );
        Ok(())
    }

    fn add_branch(
        &mut self,
        function: &str,
        name: Option<String>,
        base_revision: Option<RevisionId>,
        revision: Option<RevisionId>,
    ) -> PyResult<()> {
        self.0
            .branches
            .push((function.to_string(), name, base_revision, revision));
        Ok(())
    }

    fn add_tag(&mut self, name: &str, revision: RevisionId) -> PyResult<()> {
        self.0.tags.push((name.to_string(), revision));
        Ok(())
    }

    #[getter]
    fn get_codemod(&self) -> PyResult<Option<PyObject>> {
        Ok(self.0.codemod.as_ref().map(serde_json_to_py))
    }

    #[setter]
    fn set_codemod(&mut self, codemod: Option<&PyAny>) -> PyResult<()> {
        self.0.codemod = codemod.map(|c| py_to_serde_json(c).unwrap());
        Ok(())
    }

    fn update(&mut self, py: Python, failure: &WorkerFailure) -> PyResult<()> {
        let args: (String, String, Option<PyObject>, Vec<String>, Option<bool>) =
            failure.extract()?;
        let failure = janitor_worker::WorkerFailure {
            code: args.0,
            description: args.1,
            details: args.2.map(|d| py_to_serde_json(d.as_ref(py)).unwrap()),
            stage: args.3,
            transient: args.4,
        };
        self.0.update(&failure);
        Ok(())
    }

    #[setter]
    fn set_target_name(&mut self, name: &str) -> PyResult<()> {
        self.0.target = Some(janitor_worker::Target::new(
            name.to_string(),
            serde_json::Value::Null,
        ));
        Ok(())
    }

    #[setter]
    fn set_target_details(&mut self, details: &PyAny) -> PyResult<()> {
        if let Some(t) = self.0.target.as_mut() {
            t.details = py_to_serde_json(details).unwrap();
        }
        Ok(())
    }

    fn json(&self) -> PyObject {
        let json = serde_json::to_value(&self.0).unwrap();
        serde_json_to_py(&json)
    }
}

#[pyclass]
struct DebianCommandResult(silver_platter::debian::codemod::CommandResult);

#[pyfunction]
fn debian_make_changes(
    local_tree: PyObject,
    subpath: std::path::PathBuf,
    argv: Vec<&str>,
    env: std::collections::HashMap<String, String>,
    log_directory: std::path::PathBuf,
    resume_metadata: Option<PyObject>,
    committer: Option<&str>,
    update_changelog: Option<bool>,
) -> PyResult<DebianCommandResult> {
    Python::with_gil(|py| {
        janitor_worker::debian::debian_make_changes(
            &breezyshim::tree::WorkingTree::new(local_tree).unwrap(),
            &subpath,
            argv.as_slice(),
            env,
            &log_directory,
            resume_metadata
                .map(|m| py_to_serde_json(m.as_ref(py)).unwrap())
                .as_ref(),
            committer,
            update_changelog,
        )
    })
    .map(DebianCommandResult)
    .map_err(|e| {
        WorkerFailure::new_err((
            e.code,
            e.description,
            e.details.map(|e| serde_json_to_py(&e)),
            e.stage,
            e.transient,
        ))
    })
}

#[pymodule]
pub fn _worker(py: Python, m: &PyModule) -> PyResult<()> {
    pyo3_log::init();
    m.add_class::<Metadata>()?;
    m.add("WorkerFailure", py.get_type::<WorkerFailure>())?;
    m.add_function(wrap_pyfunction!(is_gce_instance, m)?)?;
    m.add_function(wrap_pyfunction!(gce_external_ip, m)?)?;
    m.add_class::<Client>()?;
    m.add("ResultUploadFailure", py.get_type::<ResultUploadFailure>())?;
    m.add("AssignmentFailure", py.get_type::<AssignmentFailure>())?;
    m.add("EmptyQueue", py.get_type::<EmptyQueue>())?;
    m.add_function(wrap_pyfunction!(abort_run, m)?)?;
    m.add_function(wrap_pyfunction!(run_lintian, m)?)?;
    m.add_function(wrap_pyfunction!(debian_make_changes, m)?)?;
    m.add(
        "LintianOutputInvalid",
        py.get_type::<LintianOutputInvalid>(),
    )?;
    Ok(())
}
