use tracing::{debug, error, info};

pub const TMP_PREFIX: &str = "janitor-apt";
pub const DEFAULT_GCS_TIMEOUT: usize = 60 * 30;

pub mod scanner;

// TODO(jelmer): Generate contents file
