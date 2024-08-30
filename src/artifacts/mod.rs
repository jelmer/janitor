use async_trait::async_trait;
use std::io;
use std::path::Path;

mod local;

pub use local::LocalArtifactManager;

pub enum Error {
    ServiceUnavailable,
    ArtifactsMissing,
    IoError(io::Error),
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::IoError(e)
    }
}

#[async_trait]
pub trait ArtifactManager {
    async fn store_artifacts(&self, run_id: &str, local_path: &Path, names: Option<&[String]>) -> Result<(), Error>;
    async fn get_artifact(&self, run_id: &str, filename: &str) -> Result<Box<dyn std::io::Read>, Error>;
    fn public_artifact_url(&self, run_id: &str, filename: &str) -> url::Url;
    async fn retrieve_artifacts(&self, run_id: &str, local_path: &Path, filter_fn: Option<&(dyn for<'a> Fn(&'a str) -> bool + Sync)>) -> Result<(), Error>;
    async fn iter_ids(&self) -> Box<dyn Iterator<Item=String>>;
}


