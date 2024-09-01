use chrono::{DateTime, Utc};
use janitor_worker::{RevisionId};
use janitor::api::worker::{Remote};
use pyo3::create_exception;
use pyo3::prelude::*;
use janitor_worker::debian::DebUpdateChangelog;
use janitor_worker::client::{AssignmentError};

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
fn is_gce_instance(py: Python) -> PyResult<Bound<PyAny>> {
    pyo3_asyncio::tokio::future_into_py(py, async { Ok(janitor_worker::is_gce_instance().await) })
}

#[pyfunction]
fn gce_external_ip(py: Python) -> PyResult<Bound<PyAny>> {
    pyo3_asyncio::tokio::future_into_py(py, async {
        janitor_worker::gce_external_ip()
            .await
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{}", e)))
    })
}

#[pyclass]
struct Client(std::sync::Arc<janitor_worker::client::Client>);

#[pymethods]
impl Client {
    #[new]
    #[pyo3(signature = (base_url, username=None, password=None, user_agent=None))]
    fn new(
        base_url: &str,
        username: Option<&str>,
        password: Option<&str>,
        user_agent: Option<&str>,
    ) -> Self {
        if username.is_none() && password.is_some() {
            panic!("password specified without username");
        }
        let credentials = if let Some(username) = username {
            janitor_worker::client::Credentials::Basic {
                username: username.to_string(),
                password: password.map(|s| s.to_string()),
            }
        } else {
            janitor_worker::client::Credentials::None
        };

        let user_agent = user_agent.unwrap_or(janitor_worker::DEFAULT_USER_AGENT);

        Self(std::sync::Arc::new(janitor_worker::client::Client::new(
            reqwest::Url::parse(base_url).unwrap(),
            credentials,
            user_agent,
        )))
    }

    #[pyo3(signature = (node_name, my_url=None, jenkins_build_url=None, codebase=None, campaign=None))]
    fn get_assignment_raw<'a>(
        &self,
        py: Python<'a>,
        node_name: &str,
        my_url: Option<&str>,
        jenkins_build_url: Option<&str>,
        codebase: Option<&str>,
        campaign: Option<&str>,
    ) -> PyResult<Bound<'a, PyAny>> {
        let campaign = campaign.map(|s| s.to_string());
        let codebase = codebase.map(|s| s.to_string());
        let node_name = node_name.to_string();
        let jenkins_build_url = jenkins_build_url.map(|s| s.to_string());
        let my_url = my_url.map(|s| s.to_string());
        let client = self.0.clone();
        pyo3_asyncio::tokio::future_into_py(py, async move {
            let assignment = client
                .get_assignment_raw(
                    my_url.map(|u| reqwest::Url::parse(u.as_str()).unwrap()).as_ref(),
                    node_name.as_str(),
                    jenkins_build_url.map(|u| reqwest::Url::parse(u.as_str()).unwrap()).as_ref(),
                    codebase.as_deref(),
                    campaign.as_deref(),
                )
                .await
                .map_err(|e| match e {
                    AssignmentError::Failure(msg) => AssignmentFailure::new_err(msg),
                    AssignmentError::EmptyQueue => EmptyQueue::new_err(()),
                })?;

            Ok(janitor_worker::serde_json_to_py(&assignment))
        })
    }

    #[pyo3(signature = (run_id, metadata, output_directory=None))]
    fn upload_results<'a>(
        &self,
        py: Python<'a>,
        run_id: &str,
        metadata: &'a Metadata,
        output_directory: Option<std::path::PathBuf>,
    ) -> PyResult<Bound<'a, PyAny>> {
        let client = self.0.clone();
        let run_id = run_id.to_string();
        let metadata = metadata.0.clone();
        pyo3_asyncio::tokio::future_into_py(py, async move {
            let result = client
                .upload_results(run_id.as_str(), &metadata, output_directory.as_deref())
                .await
                .map_err(|e| ResultUploadFailure::new_err(format!("{:?}", e)))?;

            Ok(janitor_worker::serde_json_to_py(&result))
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
) -> PyResult<Bound<'a, PyAny>> {
    let client = client.0.clone();
    let run_id = run_id.to_string();
    let description = description.to_string();
    let metadata = metadata.0.clone();
    pyo3_asyncio::tokio::future_into_py(py, async move {
        janitor_worker::client::abort_run(&client, run_id.as_str(), &metadata, description.as_str()).await;
        Ok(())
    })
}

#[pyfunction]
#[pyo3(signature = (output_directory, changes_names, profile=None, suppress_tags=None))]
fn run_lintian(
    output_directory: &'_ str,
    changes_names: Vec<String>,
    profile: Option<&'_ str>,
    suppress_tags: Option<Vec<String>>,
) -> PyResult<Py<PyAny>> {
    let changes_names = changes_names.iter().map(|s| s.as_str()).collect::<Vec<_>>();
    let suppress_tags = suppress_tags
        .as_ref()
        .map(|v| v.iter().map(|s| s.as_str()).collect());
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
    Ok(janitor_worker::serde_json_to_py(&result))
}

create_exception!(janitor_worker, WorkerFailure, pyo3::exceptions::PyException);

#[pyclass]
struct Assignment(janitor::api::worker::Assignment);

#[pymethods]
impl Assignment {}

#[pyclass]
struct MetadataTarget(janitor::api::worker::TargetDetails);

#[pymethods]
impl MetadataTarget {
    #[getter]
    fn get_name(&self) -> PyResult<&str> {
        Ok(self.0.name.as_str())
    }

    #[new]
    fn new(name: &str) -> Self {
        Self(janitor::api::worker::TargetDetails::new(
            name.to_string(),
            serde_json::Value::Null,
        ))
    }

    #[getter]
    fn get_details(&self) -> PyResult<Py<PyAny>> {
        Ok(janitor_worker::serde_json_to_py(&self.0.details))
    }

    #[setter]
    fn set_details(&mut self, details: &Bound<PyAny>) -> PyResult<()> {
        self.0.details = janitor_worker::py_to_serde_json(details)?;
        Ok(())
    }
}

#[pyclass]
struct Metadata(janitor::api::worker::Metadata);

#[pymethods]
impl Metadata {
    #[new]
    fn new() -> PyResult<Self> {
        Ok(Self(janitor::api::worker::Metadata::default()))
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
    fn get_start_time(&self) -> PyResult<Option<DateTime<Utc>>> {
        Ok(self.0.start_time)
    }

    #[setter]
    fn set_start_time(&mut self, start_time: Option<DateTime<Utc>>) -> PyResult<()> {
        self.0.start_time = start_time;
        Ok(())
    }

    #[getter]
    fn get_finish_time(&self) -> PyResult<Option<DateTime<Utc>>> {
        Ok(self.0.finish_time)
    }

    #[setter]
    fn set_finish_time(&mut self, finish_time: Option<DateTime<Utc>>) -> PyResult<()> {
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
    fn get_vcs_type(&self) -> PyResult<Option<String>> {
        Ok(self.0.vcs_type.map(|s| s.to_string()))
    }

    #[setter]
    fn set_vcs_type(&mut self, vcs_type: Option<&str>) -> PyResult<()> {
        self.0.vcs_type = vcs_type.map(|s| s.parse().unwrap());
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

    #[pyo3(signature = (function, name=None, base_revision=None, revision=None))]
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
    fn get_codemod(&self) -> PyResult<Option<Py<PyAny>>> {
        Ok(self
            .0
            .codemod
            .as_ref()
            .map(janitor_worker::serde_json_to_py))
    }

    #[setter]
    fn set_codemod(&mut self, codemod: Option<Bound<PyAny>>) -> PyResult<()> {
        self.0.codemod = codemod.map(|c| janitor_worker::py_to_serde_json(&c).unwrap());
        Ok(())
    }

    fn update(&mut self, failure: Bound<WorkerFailure>) -> PyResult<()> {
        let args: (
            String,
            String,
            Option<Bound<PyAny>>,
            Vec<String>,
            Option<bool>,
        ) = failure.extract()?;
        let failure = janitor::api::worker::WorkerFailure {
            code: args.0,
            description: args.1,
            details: args
                .2
                .map(|d| janitor_worker::py_to_serde_json(&d).unwrap()),
            stage: args.3,
            transient: args.4,
        };
        self.0.update(&failure);
        Ok(())
    }

    #[setter]
    fn set_target_name(&mut self, name: &str) -> PyResult<()> {
        self.0.target = Some(janitor::api::worker::TargetDetails::new(
            name.to_string(),
            serde_json::Value::Null,
        ));
        Ok(())
    }

    #[setter]
    fn set_target_details(&mut self, details: &Bound<PyAny>) -> PyResult<()> {
        if let Some(t) = self.0.target.as_mut() {
            t.details = janitor_worker::py_to_serde_json(details).unwrap();
        }
        Ok(())
    }

    fn json(&self) -> Py<PyAny> {
        let json = serde_json::to_value(&self.0).unwrap();
        janitor_worker::serde_json_to_py(&json)
    }
}

#[pyclass]
struct DebianCommandResult(silver_platter::debian::codemod::CommandResult);

#[pymethods]
impl DebianCommandResult {
    #[getter]
    fn value(&self) -> Option<u32> {
        self.0.value
    }

    #[getter]
    fn description(&self) -> &str {
        self.0.description.as_str()
    }

    #[getter]
    fn serialized_context(&self) -> Option<&str> {
        self.0.serialized_context.as_deref()
    }

    #[getter]
    fn tags(&self) -> Vec<(String, Option<RevisionId>)> {
        self.0.tags.clone()
    }

    #[getter]
    fn target_branch_url(&self) -> Option<&str> {
        self.0.target_branch_url.as_ref().map(|u| u.as_str())
    }

    #[getter]
    fn old_revision(&self) -> RevisionId {
        self.0.old_revision.clone()
    }

    #[getter]
    fn new_revision(&self) -> RevisionId {
        self.0.new_revision.clone()
    }

    #[getter]
    fn context(&self) -> Option<Py<PyAny>> {
        self.0
            .context
            .as_ref()
            .map(janitor_worker::serde_json_to_py)
    }
}

#[pyfunction]
#[pyo3(signature = (local_tree, subpath, argv, env, log_directory, resume_metadata=None, committer=None, update_changelog=None))]
fn debian_make_changes(
    py: Python,
    local_tree: Bound<PyAny>,
    subpath: std::path::PathBuf,
    argv: Vec<String>,
    env: std::collections::HashMap<String, String>,
    log_directory: std::path::PathBuf,
    resume_metadata: Option<Bound<PyAny>>,
    committer: Option<&str>,
    update_changelog: Option<bool>,
) -> PyResult<DebianCommandResult> {
    let argv = argv.iter().map(|s| s.as_str()).collect::<Vec<_>>();
    janitor_worker::debian::debian_make_changes(
        &breezyshim::tree::WorkingTree::from(local_tree.into_py(py)),
        &subpath,
        argv.as_slice(),
        env,
        &log_directory,
        resume_metadata
            .map(|m| janitor_worker::py_to_serde_json(&m).unwrap())
            .as_ref(),
        committer,
        match update_changelog {
            None => DebUpdateChangelog::Auto,
            Some(true) => DebUpdateChangelog::Update,
            Some(false) => DebUpdateChangelog::Leave,
        }
    )
    .map(DebianCommandResult)
    .map_err(|e| {
        WorkerFailure::new_err((
            e.code,
            e.description,
            e.details.map(|e| janitor_worker::serde_json_to_py(&e)),
            e.stage,
            e.transient,
        ))
    })
}

#[pyclass]
struct GenericCommandResult(silver_platter::codemod::CommandResult);

#[pymethods]
impl GenericCommandResult {
    #[getter]
    fn value(&self) -> Option<u32> {
        self.0.value
    }

    #[getter]
    fn description(&self) -> Option<&str> {
        self.0.description.as_deref()
    }

    #[getter]
    fn serialized_context(&self) -> Option<&str> {
        self.0.serialized_context.as_deref()
    }

    #[getter]
    fn tags(&self) -> Vec<(String, Option<RevisionId>)> {
        self.0.tags.clone()
    }

    #[getter]
    fn target_branch_url(&self) -> Option<&str> {
        self.0.target_branch_url.as_ref().map(|u| u.as_str())
    }

    #[getter]
    fn old_revision(&self) -> RevisionId {
        self.0.old_revision.clone()
    }

    #[getter]
    fn new_revision(&self) -> RevisionId {
        self.0.new_revision.clone()
    }

    #[getter]
    fn context(&self) -> Option<Py<PyAny>> {
        self.0
            .context
            .as_ref()
            .map(janitor_worker::serde_json_to_py)
    }
}

#[pyfunction]
#[pyo3(signature = (local_tree, subpath, argv, env, log_directory, resume_metadata=None))]
fn generic_make_changes(
    py: Python,
    local_tree: Bound<PyAny>,
    subpath: std::path::PathBuf,
    argv: Vec<String>,
    env: std::collections::HashMap<String, String>,
    log_directory: std::path::PathBuf,
    resume_metadata: Option<Bound<PyAny>>,
) -> PyResult<GenericCommandResult> {
    let argv = argv.iter().map(|s| s.as_str()).collect::<Vec<_>>();
    janitor_worker::generic::generic_make_changes(
        &breezyshim::tree::WorkingTree::from(local_tree.into_py(py)),
        &subpath,
        argv.as_slice(),
        &env,
        &log_directory,
        resume_metadata
            .map(|m| janitor_worker::py_to_serde_json(&m).unwrap())
            .as_ref(),
    )
    .map(GenericCommandResult)
    .map_err(|e| {
        WorkerFailure::new_err((
            e.code,
            e.description,
            e.details.map(|e| janitor_worker::serde_json_to_py(&e)),
            e.stage,
            e.transient,
        ))
    })
}

#[pymodule]
pub fn _worker(py: Python, m: &Bound<PyModule>) -> PyResult<()> {
    pyo3_log::init();
    m.add_class::<Metadata>()?;
    m.add("WorkerFailure", py.get_type_bound::<WorkerFailure>())?;
    m.add_function(wrap_pyfunction!(is_gce_instance, m)?)?;
    m.add_function(wrap_pyfunction!(gce_external_ip, m)?)?;
    m.add_class::<Client>()?;
    m.add(
        "ResultUploadFailure",
        py.get_type_bound::<ResultUploadFailure>(),
    )?;
    m.add(
        "AssignmentFailure",
        py.get_type_bound::<AssignmentFailure>(),
    )?;
    m.add("EmptyQueue", py.get_type_bound::<EmptyQueue>())?;
    m.add_function(wrap_pyfunction!(abort_run, m)?)?;
    m.add_function(wrap_pyfunction!(run_lintian, m)?)?;
    m.add_function(wrap_pyfunction!(debian_make_changes, m)?)?;
    m.add_function(wrap_pyfunction!(generic_make_changes, m)?)?;
    m.add(
        "LintianOutputInvalid",
        py.get_type_bound::<LintianOutputInvalid>(),
    )?;
    Ok(())
}
