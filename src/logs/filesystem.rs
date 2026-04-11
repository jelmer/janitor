use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::fs;
use std::io::Read;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use crate::logs::{Error, LogFileManager};

#[derive(Debug)]
pub struct FileSystemLogFileManager {
    log_directory: PathBuf,
}

impl FileSystemLogFileManager {
    pub fn new(log_directory: PathBuf) -> Self {
        Self { log_directory }
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
        fs::create_dir_all(&dest_dir)?;

        let mut inf = fs::File::open(orig_path)?;

        let basename =
            basename.unwrap_or_else(|| Path::new(orig_path).file_name().unwrap().to_str().unwrap());
        let dest_path = dest_dir.join(format!("{}.gz", basename));

        let mut outf = fs::File::create(&dest_path)?;
        let mut encoder = flate2::write::GzEncoder::new(&mut outf, flate2::Compression::default());
        std::io::copy(&mut inf, &mut encoder)?;
        encoder.finish()?;

        std::mem::drop(outf);

        if let Some(mtime) = mtime {
            filetime::set_file_times(
                dest_path,
                filetime::FileTime::from_system_time(mtime.into()),
                filetime::FileTime::from_system_time(mtime.into()),
            )?;
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logs::LogFileManager;
    use tempfile::TempDir;

    fn setup() -> (TempDir, FileSystemLogFileManager) {
        let td = TempDir::new().unwrap();
        let mgr = FileSystemLogFileManager::new(td.path().to_path_buf());
        (td, mgr)
    }

    #[tokio::test]
    async fn test_has_log_not_found() {
        let (_td, mgr) = setup();
        assert_eq!(
            mgr.has_log("codebase", "run-1", "build.log").await.unwrap(),
            false
        );
    }

    #[tokio::test]
    async fn test_get_log_not_found() {
        let (_td, mgr) = setup();
        let result = mgr.get_log("codebase", "run-1", "build.log").await;
        assert!(matches!(result, Err(Error::NotFound)));
    }

    #[tokio::test]
    async fn test_import_and_has_log() {
        let (_td, mgr) = setup();

        // Create a source log file
        let source_dir = TempDir::new().unwrap();
        let source_path = source_dir.path().join("build.log");
        std::fs::write(&source_path, "some log content\n").unwrap();

        mgr.import_log(
            "codebase",
            "run-1",
            source_path.to_str().unwrap(),
            None,
            None,
        )
        .await
        .unwrap();

        assert_eq!(
            mgr.has_log("codebase", "run-1", "build.log").await.unwrap(),
            true
        );
    }

    #[tokio::test]
    async fn test_import_and_get_log() {
        let (_td, mgr) = setup();

        let source_dir = TempDir::new().unwrap();
        let source_path = source_dir.path().join("build.log");
        std::fs::write(&source_path, "log line 1\nlog line 2\n").unwrap();

        mgr.import_log(
            "codebase",
            "run-1",
            source_path.to_str().unwrap(),
            None,
            None,
        )
        .await
        .unwrap();

        let mut reader = mgr.get_log("codebase", "run-1", "build.log").await.unwrap();
        let mut content = String::new();
        reader.read_to_string(&mut content).unwrap();
        assert_eq!(content, "log line 1\nlog line 2\n");
    }

    #[tokio::test]
    async fn test_import_with_custom_basename() {
        let (_td, mgr) = setup();

        let source_dir = TempDir::new().unwrap();
        let source_path = source_dir.path().join("original.log");
        std::fs::write(&source_path, "content").unwrap();

        mgr.import_log(
            "codebase",
            "run-1",
            source_path.to_str().unwrap(),
            None,
            Some("renamed.log"),
        )
        .await
        .unwrap();

        assert_eq!(
            mgr.has_log("codebase", "run-1", "renamed.log")
                .await
                .unwrap(),
            true
        );
        assert_eq!(
            mgr.has_log("codebase", "run-1", "original.log")
                .await
                .unwrap(),
            false
        );
    }

    #[tokio::test]
    async fn test_import_with_mtime() {
        let (_td, mgr) = setup();

        let source_dir = TempDir::new().unwrap();
        let source_path = source_dir.path().join("build.log");
        std::fs::write(&source_path, "content").unwrap();

        let mtime = chrono::DateTime::from_timestamp(1700000000, 0).unwrap();
        mgr.import_log(
            "codebase",
            "run-1",
            source_path.to_str().unwrap(),
            Some(mtime),
            None,
        )
        .await
        .unwrap();

        assert_eq!(
            mgr.has_log("codebase", "run-1", "build.log").await.unwrap(),
            true
        );
    }

    #[tokio::test]
    async fn test_get_ctime() {
        let (_td, mgr) = setup();

        let source_dir = TempDir::new().unwrap();
        let source_path = source_dir.path().join("build.log");
        std::fs::write(&source_path, "content").unwrap();

        mgr.import_log(
            "codebase",
            "run-1",
            source_path.to_str().unwrap(),
            None,
            None,
        )
        .await
        .unwrap();

        let ctime = mgr
            .get_ctime("codebase", "run-1", "build.log")
            .await
            .unwrap();
        // ctime should be recent (within the last minute)
        let now = Utc::now();
        let diff = now - ctime;
        assert!(diff.num_seconds() >= 0);
        assert!(diff.num_seconds() < 60);
    }

    #[tokio::test]
    async fn test_get_ctime_not_found() {
        let (_td, mgr) = setup();
        let result = mgr.get_ctime("codebase", "run-1", "build.log").await;
        assert!(matches!(result, Err(Error::NotFound)));
    }

    #[tokio::test]
    async fn test_iter_logs() {
        let (_td, mgr) = setup();

        let source_dir = TempDir::new().unwrap();
        let source1 = source_dir.path().join("build.log");
        std::fs::write(&source1, "content1").unwrap();
        let source2 = source_dir.path().join("worker.log");
        std::fs::write(&source2, "content2").unwrap();

        mgr.import_log("codebase-a", "run-1", source1.to_str().unwrap(), None, None)
            .await
            .unwrap();
        mgr.import_log("codebase-a", "run-1", source2.to_str().unwrap(), None, None)
            .await
            .unwrap();
        mgr.import_log("codebase-b", "run-2", source1.to_str().unwrap(), None, None)
            .await
            .unwrap();

        let mut logs: Vec<(String, String, Vec<String>)> = mgr.iter_logs().await.collect();
        logs.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));

        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0].0, "codebase-a");
        assert_eq!(logs[0].1, "run-1");
        let mut names = logs[0].2.clone();
        names.sort();
        assert_eq!(names, vec!["build.log", "worker.log"]);

        assert_eq!(logs[1].0, "codebase-b");
        assert_eq!(logs[1].1, "run-2");
        assert_eq!(logs[1].2, vec!["build.log"]);
    }

    #[tokio::test]
    async fn test_iter_logs_empty() {
        let (_td, mgr) = setup();
        let logs: Vec<_> = mgr.iter_logs().await.collect();
        assert_eq!(logs.len(), 0);
    }

    #[tokio::test]
    async fn test_path_traversal_rejected() {
        let (_td, mgr) = setup();
        // Paths containing '/' should return empty paths list
        assert_eq!(
            mgr.has_log("code/base", "run-1", "build.log")
                .await
                .unwrap(),
            false
        );
        assert_eq!(
            mgr.has_log("codebase", "run/1", "build.log").await.unwrap(),
            false
        );
        assert_eq!(
            mgr.has_log("codebase", "run-1", "build/log").await.unwrap(),
            false
        );
    }
}
