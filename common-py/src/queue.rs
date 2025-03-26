use pyo3::prelude::*;
use std::sync::Arc;
use pyo3::types::PyType;
use std::collections::HashSet;
use janitor::queue::QueueId;

#[pyclass]
pub struct ETA(janitor::queue::ETA);

#[pymethods]
impl ETA {
    #[getter]
    fn position(&self) -> i64 {
        self.0.position
    }
}

#[pyclass]
pub struct QueueItem(janitor::queue::QueueItem);

#[pyclass]
pub struct VcsInfo(janitor::queue::VcsInfo);

#[pyclass]
pub struct Queue(Arc<janitor::queue::Queue>);

#[pymethods]
impl Queue {
    #[classmethod]
    pub fn from_config<'a>(
        tp: &Bound<'a, PyType>,
        py: Python<'a>,
        config: crate::config::Config,
    ) -> PyResult<Bound<'a, PyAny>> {
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            Ok(Queue(Arc::new(
                janitor::queue::Queue::from_config(&config.0).await,
            )))
        })
    }

    pub fn get_position<'a>(
        &self,
        py: Python<'a>,
        campaign: String,
        codebase: String,
    ) -> PyResult<Bound<'a, PyAny>> {
        let z = self.0.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let r = z
                .get_position(&campaign, &codebase)
                .await
                .map(|eta| eta.map(ETA)).map_err(crate::convert_sqlx_error)?;
            Ok(Python::with_gil(|py| r.map(|r| r.into_py(py))))
        })
    }

    pub fn get_item<'a>(
        &self,
        py: Python<'a>,
        queue_id: QueueId
    ) -> PyResult<Bound<'a, PyAny>> {
        let z = self.0.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let r = z
                .get_item(queue_id)
                .await
                .map(|item| item.map(QueueItem)).map_err(crate::convert_sqlx_error)?;
            Ok(Python::with_gil(|py| r.map(|r| r.into_py(py))))
        })
    }

    /// Get the next item from the queue
    ///
    /// # Arguments
    /// * `codebase` - The codebase to get the next item for
    /// * `campaign` - The campaign to get the next item for
    /// * `exclude_hosts` - A list of hosts to exclude from the search
    /// * `assigned_queue_items` - A list of queue items that are already assigned to a host
    ///
    /// # Returns
    /// A tuple containing the next item and the VCS info for the codebase
    #[pyo3(signature = (codebase=None, campaign=None, exclude_hosts=None, assigned_queue_items=None))]
    pub fn next_item<'a>(
        &self,
        py: Python<'a>,
        codebase: Option<String>,
        campaign: Option<String>,
        exclude_hosts: Option<HashSet<String>>,
        assigned_queue_items: Option<HashSet<QueueId>>,
    ) -> PyResult<Bound<'a, PyAny>> {
        let z = self.0.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let (item, vcs_info) = z
                .next_item(
                    codebase.as_deref(),
                    campaign.as_deref(),
                    exclude_hosts,
                    assigned_queue_items,
                )
                .await
                .map(|(item, vcs_info)| {
                    (item.map(QueueItem), vcs_info.map(VcsInfo))
                }).map_err(crate::convert_sqlx_error)?;
            Ok(Python::with_gil(|py| (item.into_py(py), vcs_info.into_py(py))))
        })
    }
}

pub(crate) fn init(py: Python, module: &Bound<PyModule>) -> PyResult<()> {
    module.add_class::<Queue>()?;
    module.add_class::<QueueItem>()?;
    module.add_class::<VcsInfo>()?;
    module.add_class::<ETA>()?;
    Ok(())
}
