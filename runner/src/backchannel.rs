use async_trait::async_trait;

#[derive(Debug)]
/// Error types for backchannel operations.
pub enum Error {
    /// Timeout while pinging job.
    PingTimeout,

    /// The requested resource was not found.
    NotFound,

    /// Failure in the communication with the intermediary.
    IntermediaryFailure(reqwest::Error),

    /// Failure to ping the job that's not retriable.
    FatalFailure(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::PingTimeout => write!(f, "Timeout while pinging job"),
            Error::NotFound => write!(f, "Job not found"),
            Error::IntermediaryFailure(e) => write!(f, "Intermediary failure: {}", e),
            Error::FatalFailure(e) => {
                write!(f, "Failure to ping the job that's not retriable: {}", e)
            }
        }
    }
}

impl std::error::Error for Error {}

#[async_trait]
/// Interface for communication with the worker processes.
pub trait Backchannel {
    /// Kill the worker process.
    async fn kill(&self) -> Result<(), Error>;
    /// List available log files from the worker.
    async fn list_log_files(&self) -> Result<Vec<String>, Error>;
    /// Get the contents of a specific log file from the worker.
    async fn get_log_file(&self, name: &str) -> Result<Vec<u8>, Error>;
    /// Check if the worker is still alive and processing the expected log.
    async fn ping(&self, log_id: &str) -> Result<(), Error>;

    /// Serialize the backchannel to JSON.
    fn to_json(&self) -> serde_json::Value;

    /// Create a backchannel from JSON representation.
    fn from_json(js: serde_json::Value) -> impl Backchannel;
}

/// Backchannel implementation for Jenkins workers.
pub struct JenkinsBackchannel {
    my_url: url::Url,
    metadata: serde_json::Value,
}

impl JenkinsBackchannel {
    /// Create a new Jenkins backchannel with the specified URL and metadata.
    pub fn new(my_url: url::Url, metadata: serde_json::Value) -> Self {
        Self { my_url, metadata }
    }

    async fn get_job(&self, session: reqwest::Client) -> Result<serde_json::Value, reqwest::Error> {
        let url = self.my_url.join("api/json").unwrap();
        log::info!("Fetching Jenkins URL {}", url);
        let resp = session.get(url).send().await?;
        Ok(resp.json().await?)
    }
}

#[async_trait]
impl Backchannel for JenkinsBackchannel {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "my_url": self.my_url.to_string(),
            "metadata": self.metadata,
        })
    }

    fn from_json(js: serde_json::Value) -> impl Backchannel {
        Self {
            my_url: url::Url::parse(js["my_url"].as_str().unwrap()).unwrap(),
            metadata: js["metadata"].clone(),
        }
    }

    async fn kill(&self) -> Result<(), Error> {
        unimplemented!()
    }

    async fn list_log_files(&self) -> Result<Vec<String>, Error> {
        Ok(vec!["worker.log".to_string()])
    }

    async fn get_log_file(&self, name: &str) -> Result<Vec<u8>, Error> {
        if name != "worker.log" {
            return Err(Error::NotFound);
        }

        let resp = reqwest::get(self.my_url.join("logText/progressiveText").unwrap())
            .await
            .map_err(Error::IntermediaryFailure)?;
        Ok(resp
            .bytes()
            .await
            .map_err(Error::IntermediaryFailure)?
            .to_vec())
    }

    async fn ping(&self, expected_log_id: &str) -> Result<(), Error> {
        let session = reqwest::Client::new();
        let job = match self.get_job(session).await {
            Ok(job) => job,
            Err(e) if e.status() == Some(reqwest::StatusCode::NOT_FOUND) => {
                return Err(Error::NotFound)
            }
            Err(_) => return Err(Error::PingTimeout),
        };
        // If Jenkins has listed the job as having failed, then we can't expect anything to be
        // uploaded
        if job["result"] == "FAILURE" {
            return Err(Error::FatalFailure(format!(
                "Jenkins lists job {} for run {} as failed",
                job["id"], expected_log_id
            )));
        }
        Ok(())
    }
}

/// Backchannel implementation that polls a worker via HTTP.
pub struct PollingBackchannel {
    my_url: url::Url,
}

impl PollingBackchannel {
    /// Create a new polling backchannel with the specified URL.
    pub fn new(my_url: url::Url) -> Self {
        Self { my_url }
    }

    async fn get_log_id(&self, session: reqwest::Client) -> Result<String, reqwest::Error> {
        let url = self.my_url.join("log-id").unwrap();
        log::info!("Fetching log ID from URL {}", url);
        let resp = session.get(url).send().await?;
        Ok(resp.text().await?)
    }
}

#[async_trait]
impl Backchannel for PollingBackchannel {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "my_url": self.my_url.to_string(),
        })
    }

    fn from_json(js: serde_json::Value) -> impl Backchannel {
        Self {
            my_url: url::Url::parse(js["my_url"].as_str().unwrap()).unwrap(),
        }
    }

    async fn kill(&self) -> Result<(), Error> {
        let session = reqwest::Client::new();
        let url = self.my_url.join("kill").unwrap();

        log::info!("Killing worker at URL {}", url);

        session
            .post(url)
            .send()
            .await
            .map_err(Error::IntermediaryFailure)?;
        Ok(())
    }

    async fn list_log_files(&self) -> Result<Vec<String>, Error> {
        let session = reqwest::Client::new();
        let url = self.my_url.join("logs").unwrap();

        log::info!("Listing log files at URL {}", url);

        let resp = session
            .get(url)
            .send()
            .await
            .map_err(Error::IntermediaryFailure)?;
        Ok(resp.json().await.map_err(Error::IntermediaryFailure)?)
    }

    async fn get_log_file(&self, name: &str) -> Result<Vec<u8>, Error> {
        let session = reqwest::Client::new();
        let url = self.my_url.join("logs").unwrap().join(name).unwrap();

        log::info!("Fetching log file at URL {}", url);

        let resp = session
            .get(url)
            .send()
            .await
            .map_err(Error::IntermediaryFailure)?;
        Ok(resp
            .bytes()
            .await
            .map_err(Error::IntermediaryFailure)?
            .to_vec())
    }

    async fn ping(&self, expected_log_id: &str) -> Result<(), Error> {
        let session = reqwest::Client::new();

        let log_id = match self.get_log_id(session).await {
            Ok(log_id) => log_id,
            Err(_e) => return Err(Error::PingTimeout),
        };

        if log_id != expected_log_id {
            return Err(Error::FatalFailure(format!(
                "Worker started processing new run {} rather than {}",
                log_id, expected_log_id
            )));
        }

        Ok(())
    }
}
