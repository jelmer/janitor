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
    } else {
        // Default to filesystem
        Ok(Box::new(FileSystemLogFileManager::new(location)?))
    }
}
