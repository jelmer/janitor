use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::fs;
use std::io::{self, Read};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use tokio::fs as async_fs;

use crate::logs::{Error, LogFileManager};

#[derive(Debug)]
pub struct FileSystemLogFileManager {
    log_directory: PathBuf,
}

impl FileSystemLogFileManager {
    pub fn new<P: AsRef<Path>>(log_directory: P) -> Result<Self, Error> {
        let log_directory = log_directory.as_ref().to_path_buf();
        Ok(Self { log_directory })
    }

    fn get_paths(&self, codebase: &str, run_id: &str, name: &str) -> Vec<PathBuf> {
        if codebase.contains('/') || run_id.contains('/') || name.contains('/') {
            return vec![];
        }
        vec![
            self.log_directory.join(codebase).join(run_id).join(name),
            self.log_directory
                .join(codebase)
                .join(run_id)
                .join(format!("{}.gz", name)),
        ]
    }
}

#[async_trait]
impl LogFileManager for FileSystemLogFileManager {
    async fn has_log(&self, codebase: &str, run_id: &str, name: &str) -> Result<bool, Error> {
        Ok(self
            .get_paths(codebase, run_id, name)
            .iter()
            .any(|path| path.exists()))
    }

    async fn get_log(
        &self,
        codebase: &str,
        run_id: &str,
        name: &str,
    ) -> Result<Box<dyn Read + Send + Sync>, Error> {
        for path in self.get_paths(codebase, run_id, name) {
            if path.exists() {
                if path.extension().and_then(|ext| ext.to_str()) == Some("gz") {
                    let file = std::fs::File::open(path)?;
                    let gz = flate2::read::GzDecoder::new(file);
                    return Ok(Box::new(gz));
                } else {
                    let file = std::fs::File::open(path)?;
                    return Ok(Box::new(file));
                }
            }
        }
        Err(Error::NotFound)
    }

    async fn import_log(
        &self,
        codebase: &str,
        run_id: &str,
        orig_path: &str,
        mtime: Option<DateTime<Utc>>,
        basename: Option<&str>,
    ) -> Result<(), Error> {
        let dest_dir = self.log_directory.join(codebase).join(run_id);
        tokio::fs::create_dir_all(&dest_dir).await?;

        let data = tokio::fs::read(orig_path).await?;

        let basename =
            basename.unwrap_or_else(|| Path::new(orig_path).file_name().unwrap().to_str().unwrap());
        let dest_path = dest_dir.join(format!("{}.gz", basename));

        // Compress data in a blocking task to avoid blocking the executor
        let compressed = tokio::task::spawn_blocking(move || -> io::Result<Vec<u8>> {
            let mut encoder =
                flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
            io::copy(&mut io::Cursor::new(&data), &mut encoder)?;
            encoder.finish()
        })
        .await
        .map_err(|e| Error::Other(e.to_string()))??;

        tokio::fs::write(&dest_path, compressed).await?;

        if let Some(mtime) = mtime {
            let dest_path_clone = dest_path.clone();
            tokio::task::spawn_blocking(move || {
                filetime::set_file_times(
                    dest_path_clone,
                    filetime::FileTime::from_system_time(mtime.into()),
                    filetime::FileTime::from_system_time(mtime.into()),
                )
            })
            .await
            .map_err(|e| Error::Other(e.to_string()))??;
        }

        Ok(())
    }

    async fn iter_logs(&self) -> Box<dyn Iterator<Item = (String, String, Vec<String>)>> {
        let log_dir = self.log_directory.clone();
        let entries = fs::read_dir(log_dir).unwrap();
        let mut logs = Vec::new();

        for codebase_entry in entries {
            let codebase_entry = codebase_entry.unwrap();
            let codebase_name = codebase_entry.file_name().into_string().unwrap();

            let run_entries = fs::read_dir(codebase_entry.path()).unwrap();
            for run_entry in run_entries {
                let run_entry = run_entry.unwrap();
                let run_name = run_entry.file_name().into_string().unwrap();

                let log_names = fs::read_dir(run_entry.path())
                    .unwrap()
                    .filter_map(|entry| {
                        let entry = entry.unwrap();
                        let name = entry.file_name().into_string().unwrap();
                        if name.ends_with(".gz") {
                            Some(name[..name.len() - 3].to_string())
                        } else {
                            Some(name)
                        }
                    })
                    .collect();

                logs.push((codebase_name.clone(), run_name, log_names));
            }
        }

        Box::new(logs.into_iter())
    }

    async fn get_ctime(
        &self,
        codebase: &str,
        run_id: &str,
        name: &str,
    ) -> Result<DateTime<Utc>, Error> {
        for path in self.get_paths(codebase, run_id, name) {
            if let Ok(metadata) = fs::metadata(&path) {
                let ctime = metadata.ctime();
                return Ok(chrono::DateTime::from_timestamp(ctime, 0).unwrap());
            }
        }
        Err(Error::NotFound)
    }

    async fn delete_log(&self, codebase: &str, run_id: &str, name: &str) -> Result<(), Error> {
        for path in self.get_paths(codebase, run_id, name) {
            match async_fs::remove_file(&path).await {
                Ok(()) => return Ok(()),
                Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
                Err(e) => return Err(Error::from(e)),
            }
        }
        Err(Error::NotFound)
    }

    async fn health_check(&self) -> Result<(), Error> {
        // Check if the log directory exists and is accessible
        match fs::metadata(&self.log_directory) {
            Ok(metadata) => {
                if metadata.is_dir() {
                    // Try to list the directory to ensure we have read permissions
                    match fs::read_dir(&self.log_directory) {
                        Ok(_) => Ok(()),
                        Err(e) => match e.kind() {
                            std::io::ErrorKind::PermissionDenied => Err(Error::PermissionDenied),
                            _ => Err(Error::Io(e.to_string())),
                        },
                    }
                } else {
                    Err(Error::Other("Log directory is not a directory".to_string()))
                }
            }
            Err(e) => match e.kind() {
                std::io::ErrorKind::NotFound => {
                    // Try to create the directory
                    fs::create_dir_all(&self.log_directory)?;
                    Ok(())
                }
                std::io::ErrorKind::PermissionDenied => Err(Error::PermissionDenied),
                _ => Err(Error::Io(e.to_string())),
            },
        }
    }
}
