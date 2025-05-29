use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::io::{self, Read};

mod filesystem;
pub use filesystem::FileSystemLogFileManager;

#[cfg(feature = "gcs")]
mod gcs;
#[cfg(feature = "gcs")]
pub use gcs::GCSLogFileManager;

#[derive(Debug)]
pub enum Error {
    NotFound,
    ServiceUnavailable,
    PermissionDenied,
    Io(io::Error),
    Other(String),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(err)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::NotFound => write!(f, "Not found"),
            Error::ServiceUnavailable => write!(f, "Service unavailable"),
            Error::PermissionDenied => write!(f, "Permission denied"),
            Error::Io(err) => write!(f, "I/O error: {}", err),
            Error::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for Error {}

/// A trait for managing logs.
///
/// This trait is implemented by various log file managers, which
/// can be either local or remote.
#[async_trait]
pub trait LogFileManager: Send + Sync {
    /// Check if a log exists.
    async fn has_log(&self, codebase: &str, run_id: &str, name: &str) -> Result<bool, Error>;

    /// Get a log.
    ///
    /// # Arguments
    /// * `codebase` - The codebase name.
    /// * `run_id` - The run ID.
    /// * `name` - The log name.
    ///
    /// # Returns
    /// A reader for the log file.
    async fn get_log(
        &self,
        codebase: &str,
        run_id: &str,
        name: &str,
    ) -> Result<Box<dyn Read + Send + Sync>, Error>;

    /// Import a log.
    ///
    /// # Arguments
    /// * `codebase` - The codebase name.
    /// * `run_id` - The run ID.
    /// * `orig_path` - The original path of the log.
    /// * `mtime` - The modification time of the log.
    /// * `basename` - The basename of the log.
    async fn import_log(
        &self,
        codebase: &str,
        run_id: &str,
        orig_path: &str,
        mtime: Option<DateTime<Utc>>,
        basename: Option<&str>,
    ) -> Result<(), Error>;

    /// Delete a log.
    ///
    /// # Arguments
    /// * `codebase` - The codebase name.
    /// * `run_id` - The run ID.
    /// * `name` - The log name.
    async fn delete_log(&self, codebase: &str, run_id: &str, name: &str) -> Result<(), Error>;

    /// List logs.
    async fn iter_logs(&self) -> Box<dyn Iterator<Item = (String, String, Vec<String>)>>;

    /// Get the creation time of a log.
    ///
    /// # Arguments
    /// * `codebase` - The codebase name.
    /// * `run_id` - The run ID.
    /// * `name` - The log name.
    async fn get_ctime(
        &self,
        codebase: &str,
        run_id: &str,
        name: &str,
    ) -> Result<DateTime<Utc>, Error>;

    /// Perform a health check on the log manager.
    ///
    /// This method should verify that the log storage backend is accessible
    /// and functioning properly.
    async fn health_check(&self) -> Result<(), Error>;
}

/// Create a log file manager based on the location string.
pub async fn create_log_manager(location: &str) -> Result<Box<dyn LogFileManager>, Error> {
    if location.starts_with("gs://") {
        #[cfg(feature = "gcs")]
        {
            Ok(Box::new(
                GCSLogFileManager::new(location.trim_start_matches("gs://"), None).await?,
            ))
        }
        #[cfg(not(feature = "gcs"))]
        {
            Err(Error::Other("GCS support not compiled in".to_string()))
        }
    } else if location.starts_with("http://") || location.starts_with("https://") {
        // HTTP-based log managers could be implemented here
        Err(Error::Other(
            "HTTP log manager not yet implemented".to_string(),
        ))
    } else {
        // Default to filesystem
        Ok(Box::new(FileSystemLogFileManager::new(location)?))
    }
}

/// Import a log with primary and backup log managers
///
/// This function provides sophisticated error handling and fallback mechanisms
/// matching the Python implementation. It will attempt to use the primary manager
/// first, then fall back to the backup manager on service unavailability.
pub async fn import_log(
    primary_log_manager: &dyn LogFileManager,
    backup_log_manager: Option<&dyn LogFileManager>,
    codebase: &str,
    run_id: &str,
    path: &str,
    basename: Option<&str>,
) -> Result<(), Error> {
    // Validate input path (no slashes in components)
    if let Some(basename) = basename {
        if basename.contains('/') {
            return Err(Error::Other(
                "Basename cannot contain '/' characters".to_string(),
            ));
        }
    }

    let mtime = std::fs::metadata(path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .map(|time| DateTime::<Utc>::from(time));

    // Attempt with primary log manager
    match primary_log_manager
        .import_log(codebase, run_id, path, mtime, basename)
        .await
    {
        Ok(()) => {
            log::info!("Successfully imported log {} to primary manager", path);
            return Ok(());
        }
        Err(Error::ServiceUnavailable) => {
            log::warn!("Primary log manager unavailable, trying backup");
            if let Some(backup) = backup_log_manager {
                return backup
                    .import_log(codebase, run_id, path, mtime, basename)
                    .await;
            }
        }
        Err(Error::PermissionDenied) => {
            log::warn!("Permission denied for primary manager, trying with timestamp basename");
            // Try with timestamp-based alternative basename
            let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
            let alt_basename = basename.map(|b| format!("{}-{}", b, timestamp));

            match primary_log_manager
                .import_log(codebase, run_id, path, mtime, alt_basename.as_deref())
                .await
            {
                Ok(()) => return Ok(()),
                Err(_) => {
                    // If alternative basename fails, try backup manager
                    if let Some(backup) = backup_log_manager {
                        return backup
                            .import_log(codebase, run_id, path, mtime, basename)
                            .await;
                    }
                }
            }
        }
        Err(e) => {
            log::error!("Primary log manager failed: {}", e);
            // For other errors, try backup if available
            if let Some(backup) = backup_log_manager {
                log::info!("Trying backup log manager");
                return backup
                    .import_log(codebase, run_id, path, mtime, basename)
                    .await;
            }
            return Err(e);
        }
    }

    Err(Error::Other("All log managers failed".to_string()))
}

/// Import multiple logs concurrently
///
/// This function provides batch import capabilities with concurrent processing
/// similar to the Python implementation using asyncio.gather().
pub async fn import_logs(
    primary_log_manager: &dyn LogFileManager,
    backup_log_manager: Option<&dyn LogFileManager>,
    logs: Vec<(String, String, String, Option<String>)>, // (codebase, run_id, path, basename)
) -> Vec<Result<(), Error>> {
    use futures::future::join_all;

    let import_futures = logs
        .into_iter()
        .map(|(codebase, run_id, path, basename)| async move {
            import_log(
                primary_log_manager,
                backup_log_manager,
                &codebase,
                &run_id,
                &path,
                basename.as_deref(),
            )
            .await
        });

    join_all(import_futures).await
}
