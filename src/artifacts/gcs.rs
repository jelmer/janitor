use async_trait::async_trait;
use google_cloud_auth::credentials::CredentialsFile;
use google_cloud_storage::client::{Client, ClientConfig};
use google_cloud_storage::http::objects::{
    download::Range, get::GetObjectRequest, list::ListObjectsRequest, upload::Media,
    upload::UploadObjectRequest, upload::UploadType,
};
use google_cloud_storage::http::Error as GcsError;
use std::collections::HashSet;
use std::fs::File;
use std::path::Path;

use crate::artifacts::{ArtifactManager, Error};

pub struct GCSArtifactManager {
    bucket_name: String,
    client: Client,
}

impl std::fmt::Debug for GCSArtifactManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GCSArtifactManager")
            .field("bucket_name", &self.bucket_name)
            .finish()
    }
}

impl GCSArtifactManager {
    pub async fn from_url(
        location: &url::Url,
        creds: Option<CredentialsFile>,
    ) -> Result<Self, Error> {
        if location.scheme() != "gs" {
            return Err(Error::Other(format!(
                "Invalid URL scheme: {}",
                location.scheme()
            )));
        }
        let bucket_name = location
            .host_str()
            .ok_or_else(|| Error::Other("Missing bucket name".to_string()))?;

        Self::new(bucket_name.to_string(), creds).await
    }

    pub async fn new(bucket_name: String, creds: Option<CredentialsFile>) -> Result<Self, Error> {
        let config = ClientConfig::default();
        let config = if let Some(creds) = creds {
            config
                .with_credentials(creds)
                .await
                .map_err(|e| Error::Other(e.to_string()))?
        } else {
            config.anonymous()
        };

        let client = Client::new(config);

        Ok(Self {
            bucket_name,
            client,
        })
    }
}

#[async_trait]
impl ArtifactManager for GCSArtifactManager {
    async fn store_artifacts(
        &self,
        run_id: &str,
        local_path: &Path,
        names: Option<&[String]>,
    ) -> Result<(), Error> {
        let files_to_upload = match names {
            Some(names) => names.to_vec(),
            None => {
                let mut entries = vec![];
                let mut rd = tokio::fs::read_dir(local_path).await?;
                while let Some(entry) = rd.next_entry().await? {
                    if entry.file_type().await?.is_file() {
                        entries.push(entry.file_name().into_string().unwrap());
                    }
                }
                entries
            }
        };

        let tasks: Vec<_> = files_to_upload
            .into_iter()
            .map(|name| {
                let file_path = local_path.join(&name);
                let bucket_name = self.bucket_name.clone();
                let client = self.client.clone();
                let run_id = run_id.to_string();
                let name = name.clone();
                tokio::spawn(async move {
                    let file = tokio::fs::File::open(&file_path).await?;
                    let request = UploadObjectRequest {
                        bucket: bucket_name,
                        ..Default::default()
                    };

                    let upload_type =
                        UploadType::Simple(Media::new(format!("{}/{}", run_id, name)));

                    client
                        .upload_object(&request, file, &upload_type)
                        .await
                        .map_err(|e| {
                            if let GcsError::Response(ref e) = e {
                                if e.code == 503 {
                                    return Error::ServiceUnavailable;
                                }
                            }
                            Error::Other(e.to_string())
                        })
                })
            })
            .collect();

        for task in tasks {
            task.await.map_err(|e| Error::Other(e.to_string()))??;
        }

        Ok(())
    }

    async fn delete_artifacts(&self, run_id: &str) -> Result<(), Error> {
        let prefix = format!("{}/", run_id);
        let request = ListObjectsRequest {
            bucket: self.bucket_name.clone(),
            prefix: Some(prefix.clone()),
            ..Default::default()
        };

        let objects = match self.client.list_objects(&request).await {
            Ok(response) => response.items.unwrap_or_default(),
            Err(GcsError::Response(e)) if e.code == 503 => return Err(Error::ServiceUnavailable),
            Err(GcsError::Response(e)) if e.code == 404 => return Err(Error::ArtifactsMissing),
            Err(e) => return Err(Error::Other(e.to_string())),
        };

        for object in objects {
            let name = object.name;
            let request = google_cloud_storage::http::objects::delete::DeleteObjectRequest {
                bucket: self.bucket_name.clone(),
                object: name,
                ..Default::default()
            };

            match self.client.delete_object(&request).await {
                Ok(_) => (),
                Err(GcsError::Response(e)) if e.code == 503 => {
                    return Err(Error::ServiceUnavailable)
                }
                Err(e) => return Err(Error::Other(e.to_string())),
            }
        }

        Ok(())
    }

    async fn get_artifact(
        &self,
        run_id: &str,
        filename: &str,
    ) -> Result<Box<dyn std::io::Read + Send + Sync>, Error> {
        let object_name = format!("{}/{}", run_id, filename);
        let request = GetObjectRequest {
            bucket: self.bucket_name.clone(),
            object: object_name.clone(),
            ..Default::default()
        };

        match self
            .client
            .download_object(&request, &Range::default())
            .await
        {
            Ok(response) => Ok(Box::new(std::io::Cursor::new(response))),
            Err(GcsError::Response(e)) if e.code == 503 => Err(Error::ServiceUnavailable),
            Err(GcsError::Response(e)) if e.code == 404 => Err(Error::ArtifactsMissing),
            Err(e) => Err(Error::Other(e.to_string())),
        }
    }

    fn public_artifact_url(&self, run_id: &str, filename: &str) -> url::Url {
        let object_name = format!("{}/{}", run_id, filename);
        let encoded_object_name =
            percent_encoding::utf8_percent_encode(&object_name, percent_encoding::CONTROLS);
        format!(
            "https://storage.googleapis.com/{}/{}/{}",
            self.bucket_name, run_id, encoded_object_name
        )
        .parse()
        .unwrap()
    }

    async fn retrieve_artifacts(
        &self,
        run_id: &str,
        local_path: &Path,
        filter_fn: Option<&(dyn for<'a> Fn(&'a str) -> bool + Sync + Send)>,
    ) -> Result<(), Error> {
        let prefix = format!("{}/", run_id);
        let request = ListObjectsRequest {
            bucket: self.bucket_name.clone(),
            prefix: Some(prefix.clone()),
            ..Default::default()
        };

        let objects = match self.client.list_objects(&request).await {
            Ok(response) => response.items.unwrap_or_default(),
            Err(GcsError::Response(e)) if e.code == 503 => return Err(Error::ServiceUnavailable),
            Err(GcsError::Response(e)) if e.code == 404 => return Err(Error::ArtifactsMissing),
            Err(e) => return Err(Error::Other(e.to_string())),
        };

        for object in objects {
            let name = object.name;
            let file_name = name.trim_start_matches(&prefix);
            if filter_fn.is_none_or(|f| f(file_name)) {
                let mut content = self.get_artifact(run_id, file_name).await?;
                let mut file = File::create(local_path.join(file_name))?;
                std::io::copy(&mut content, &mut file)?;
            }
        }

        Ok(())
    }

    async fn iter_ids(&self) -> Box<dyn Iterator<Item = String> + Send> {
        let mut ids = HashSet::new();
        let request = ListObjectsRequest {
            bucket: self.bucket_name.clone(),
            ..Default::default()
        };

        let objects = self
            .client
            .list_objects(&request)
            .await
            .map_err(|e| match e {
                GcsError::Response(e) if e.code == 503 => Error::ServiceUnavailable,
                e => Error::Other(e.to_string()),
            })
            .unwrap();

        for object in objects.items.unwrap_or_default() {
            let id = object.name.split('/').next().unwrap().to_string();
            ids.insert(id);
        }

        Box::new(ids.into_iter())
    }
}
