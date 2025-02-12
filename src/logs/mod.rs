use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::io::{self, Read};

mod filesystem;
pub use filesystem::FileSystemLogFileManager;

/*
#[cfg(feature = "gcs")]
mod gcs;
#[cfg(feature = "gcs")]
pub use gcs::GCSLogFileManager;
*/

#[derive(Debug)]
pub enum Error {
    NotFound,
    ServiceUnavailable,
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
            Error::Io(err) => write!(f, "I/O error: {}", err),
            Error::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for Error {}

#[async_trait]
pub trait LogFileManager {
    async fn has_log(&self, codebase: &str, run_id: &str, name: &str) -> Result<bool, Error>;

    async fn get_log(
        &self,
        codebase: &str,
        run_id: &str,
        name: &str,
    ) -> Result<Box<dyn Read + Send>, Error>;

    async fn import_log(
        &self,
        codebase: &str,
        run_id: &str,
        orig_path: &str,
        mtime: Option<DateTime<Utc>>,
        basename: Option<&str>,
    ) -> Result<(), Error>;

    async fn iter_logs(&self) -> Box<dyn Iterator<Item = (String, String, Vec<String>)>>;

    async fn get_ctime(
        &self,
        codebase: &str,
        run_id: &str,
        name: &str,
    ) -> Result<DateTime<Utc>, Error>;
}
