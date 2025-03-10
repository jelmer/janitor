use async_trait::async_trait;
use std::fs;
use std::fs::File;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

use crate::artifacts::{ArtifactManager, Error};

#[derive(Debug)]
pub struct LocalArtifactManager {
    path: PathBuf,
}

impl LocalArtifactManager {
    pub fn new(path: &Path) -> io::Result<Self> {
        if !path.is_dir() {
            fs::create_dir_all(&path)?;
        }
        Ok(Self {
            path: path.canonicalize()?,
        })
    }
}

#[async_trait]
impl ArtifactManager for LocalArtifactManager {
    async fn store_artifacts(
        &self,
        run_id: &str,
        local_path: &Path,
        names: Option<&[String]>,
    ) -> Result<(), Error> {
        let run_dir = self.path.join(run_id);
        fs::create_dir(&run_dir).or_else(|e| {
            if e.kind() == ErrorKind::AlreadyExists {
                Ok(())
            } else {
                Err(e)
            }
        })?;
        let names = names.map_or_else(
            || {
                fs::read_dir(local_path)
                    .unwrap()
                    .map(|entry| entry.unwrap().file_name().into_string().unwrap())
                    .collect::<Vec<_>>()
            },
            |names| names.to_vec(),
        );
        for name in names {
            fs::copy(local_path.join(&name), run_dir.join(&name))?;
        }
        Ok(())
    }

    async fn get_artifact(
        &self,
        run_id: &str,
        filename: &str,
    ) -> Result<Box<dyn std::io::Read + Send + Sync>, Error> {
        let path = self.path.join(run_id).join(filename);
        Ok(Box::new(File::open(path)?))
    }

    fn public_artifact_url(&self, run_id: &str, filename: &str) -> url::Url {
        url::Url::from_file_path(self.path.join(run_id).join(filename)).unwrap()
    }

    async fn retrieve_artifacts(
        &self,
        run_id: &str,
        local_path: &Path,
        filter_fn: Option<&(dyn for<'a> Fn(&'a str) -> bool + Sync + Send)>,
    ) -> Result<(), Error> {
        let run_path = self.path.join(run_id);
        if !run_path.is_dir() {
            return Err(Error::ArtifactsMissing);
        }

        for entry in fs::read_dir(run_path)? {
            let entry = entry?;
            let name = entry.file_name();
            let filter_fn = filter_fn.unwrap_or(&|_| true);
            if filter_fn(&name.to_str().unwrap()) {
                fs::copy(entry.path(), local_path.join(&name))?;
            }
        }
        Ok(())
    }

    async fn iter_ids(&self) -> Box<dyn Iterator<Item = String> + Send> {
        let entries = fs::read_dir(&self.path)
            .unwrap()
            .filter_map(|entry| {
                let entry = entry.unwrap();
                if entry.file_type().unwrap().is_dir() {
                    Some(entry.file_name().into_string().unwrap())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        Box::new(entries.into_iter())
    }

    async fn delete_artifacts(&self, run_id: &str) -> Result<(), Error> {
        let run_path = self.path.join(run_id);
        if !run_path.is_dir() {
            return Err(Error::ArtifactsMissing);
        }
        fs::remove_dir_all(run_path)?;
        Ok(())
    }
}
