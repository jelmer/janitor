use async_trait::async_trait;
use std::path::Path;

mod local;

pub use local::LocalArtifactManager;

#[cfg(feature = "gcs")]
mod gcs;

#[cfg(feature = "gcs")]
pub use gcs::GCSArtifactManager;

#[derive(Debug)]
pub enum Error {
    ServiceUnavailable,
    ArtifactsMissing,
    IoError(std::io::Error),
    Other(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::ServiceUnavailable => write!(f, "Service unavailable"),
            Error::ArtifactsMissing => write!(f, "Artifacts missing"),
            Error::IoError(e) => write!(f, "IO error: {}", e),
            Error::Other(e) => write!(f, "Error: {}", e),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::IoError(e)
    }
}

#[async_trait]
pub trait ArtifactManager {
    async fn store_artifacts(&self, run_id: &str, local_path: &Path, names: Option<&[String]>, timeout: Option<std::time::Duration>) -> Result<(), Error>;
    async fn get_artifact(&self, run_id: &str, filename: &str, timeout: Option<std::time::Duration>) -> Result<Box<dyn std::io::Read>, Error>;
    fn public_artifact_url(&self, run_id: &str, filename: &str) -> url::Url;
    async fn retrieve_artifacts(&self, run_id: &str, local_path: &Path, filter_fn: Option<&(dyn for<'a> Fn(&'a str) -> bool + Sync)>, timeout: Option<std::time::Duration>) -> Result<(), Error>;
    async fn iter_ids(&self) -> Box<dyn Iterator<Item=String>>;
}


