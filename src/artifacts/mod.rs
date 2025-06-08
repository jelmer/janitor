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
pub trait ArtifactManager: std::fmt::Debug + Send + Sync {
    async fn store_artifacts(
        &self,
        run_id: &str,
        local_path: &Path,
        names: Option<&[String]>,
    ) -> Result<(), Error>;
    async fn get_artifact(
        &self,
        run_id: &str,
        filename: &str,
    ) -> Result<Box<dyn std::io::Read + Sync + Send>, Error>;
    fn public_artifact_url(&self, run_id: &str, filename: &str) -> url::Url;
    async fn retrieve_artifacts(
        &self,
        run_id: &str,
        local_path: &Path,
        filter_fn: Option<&(dyn for<'a> Fn(&'a str) -> bool + Sync + Send)>,
    ) -> Result<(), Error>;
    async fn iter_ids(&self) -> Box<dyn Iterator<Item = String> + Send>;
    async fn delete_artifacts(&self, run_id: &str) -> Result<(), Error>;
}

pub async fn list_ids(manager: &dyn ArtifactManager) -> Result<(), Error> {
    for id in manager.iter_ids().await {
        println!("{}", id);
    }

    Ok(())
}

pub async fn get_artifact_manager(location: &str) -> Result<Box<dyn ArtifactManager>, Error> {
    if location.starts_with("gs://") {
        #[cfg(feature = "gcs")]
        {
            Ok(Box::new(
                GCSArtifactManager::new(location.parse().unwrap(), None).await?,
            ))
        }
        #[cfg(not(feature = "gcs"))]
        {
            Err(Error::ServiceUnavailable)
        }
    } else {
        Ok(Box::new(LocalArtifactManager::new(Path::new(location))?))
    }
}

/// Upload all backup artifacts to the main artifact manager and delete them from the backup
/// manager.
///
/// # Arguments
/// * `backup_artifact_manager` - The backup artifact manager to retrieve artifacts from.
/// * `artifact_manager` - The main artifact manager to store artifacts to.
///
/// # Returns
/// A list of run IDs for which the backup artifacts were successfully uploaded.
pub async fn upload_backup_artifacts(
    backup_artifact_manager: &dyn ArtifactManager,
    artifact_manager: &dyn ArtifactManager,
) -> Result<Vec<String>, Error> {
    use futures::stream::{self, StreamExt};

    // Process artifacts in parallel with a concurrency limit
    const MAX_CONCURRENT: usize = 3;

    let ids: Vec<_> = backup_artifact_manager.iter_ids().await.collect();

    let results = stream::iter(ids)
        .map(|id| async move {
            let result =
                process_single_backup_artifact(&id, backup_artifact_manager, artifact_manager)
                    .await;
            (id, result)
        })
        .buffer_unordered(MAX_CONCURRENT)
        .collect::<Vec<_>>()
        .await;

    let mut done = vec![];
    for (id, result) in results {
        match result {
            Ok(_) => done.push(id),
            Err(e) => log::warn!("Unable to upload backup artifacts for {}: {}", id, e),
        }
    }

    Ok(done)
}

async fn process_single_backup_artifact(
    id: &str,
    backup_artifact_manager: &dyn ArtifactManager,
    artifact_manager: &dyn ArtifactManager,
) -> Result<(), Error> {
    let td = tempfile::NamedTempFile::new()?;
    backup_artifact_manager
        .retrieve_artifacts(id, td.path(), None)
        .await?;

    match artifact_manager.store_artifacts(id, td.path(), None).await {
        Ok(_) => {
            backup_artifact_manager.delete_artifacts(id).await?;
            Ok(())
        }
        Err(Error::ArtifactsMissing) => unreachable!(),
        Err(e) => Err(e),
    }
}

pub async fn store_artifacts_with_backup(
    manager: &dyn ArtifactManager,
    backup_manager: Option<&dyn ArtifactManager>,
    from_dir: &Path,
    run_id: &str,
    names: Option<&[String]>,
) -> Result<(), Error> {
    match manager.store_artifacts(run_id, from_dir, names).await {
        Ok(_) => Ok(()),
        Err(Error::ArtifactsMissing) => unreachable!(),
        Err(e) => {
            log::warn!("Unable to upload artifacts for {}: {}", run_id, e);
            if let Some(backup_manager) = backup_manager {
                backup_manager
                    .store_artifacts(run_id, from_dir, names)
                    .await?;
                log::info!(
                    "Uploading results to backup artifact location {:?}",
                    backup_manager
                );
            } else {
                log::warn!("No backup artifact manager set.");
            }
            Err(e)
        }
    }
}
