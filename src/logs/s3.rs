use async_trait::async_trait;
use chrono::{DateTime, Utc};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use reqwest::{Client, StatusCode};
use std::io::{self, Cursor, Read};
use std::path::Path;
use std::time::Duration;

use crate::logs::{Error, LogFileManager};

pub struct S3LogFileManager {
    base_url: String,
    bucket_name: String,
    client: Client,
}

impl S3LogFileManager {
    pub fn new(endpoint_url: &str, bucket_name: Option<&str>) -> Result<Self, Error> {
        let bucket_name = bucket_name.unwrap_or("debian-janitor");
        let base_url = format!("{}/{}/", endpoint_url.trim_end_matches('/'), bucket_name);

        let client = Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .map_err(|e| Error::Other(e.to_string()))?;

        Ok(Self {
            base_url,
            bucket_name: bucket_name.to_string(),
            client,
        })
    }

    fn get_key(&self, codebase: &str, run_id: &str, name: &str) -> String {
        format!("logs/{}/{}/{}.gz", codebase, run_id, name)
    }

    fn get_url(&self, codebase: &str, run_id: &str, name: &str) -> String {
        format!("{}{}", self.base_url, self.get_key(codebase, run_id, name))
    }
}

#[async_trait]
impl LogFileManager for S3LogFileManager {
    async fn has_log(&self, codebase: &str, run_id: &str, name: &str) -> Result<bool, Error> {
        let url = self.get_url(codebase, run_id, name);

        match self.client.head(&url).send().await {
            Ok(resp) => match resp.status() {
                StatusCode::OK => Ok(true),
                StatusCode::NOT_FOUND => Ok(false),
                StatusCode::FORBIDDEN => Ok(false),
                status => Err(Error::Other(format!(
                    "Unexpected response code: {}",
                    status
                ))),
            },
            Err(e) => Err(Error::ServiceUnavailable),
        }
    }

    async fn get_log(
        &self,
        codebase: &str,
        run_id: &str,
        name: &str,
    ) -> Result<Box<dyn Read + Send + Sync>, Error> {
        let url = self.get_url(codebase, run_id, name);

        let resp = self
            .client
            .get(&url)
            .timeout(Duration::from_secs(300))
            .send()
            .await
            .map_err(|_| Error::ServiceUnavailable)?;

        match resp.status() {
            StatusCode::OK => {
                let bytes = resp.bytes().await.map_err(|_| Error::ServiceUnavailable)?;
                let cursor = Cursor::new(bytes.to_vec());
                let decoder = GzDecoder::new(cursor);
                Ok(Box::new(decoder))
            }
            StatusCode::NOT_FOUND => Err(Error::NotFound),
            StatusCode::FORBIDDEN => Err(Error::PermissionDenied),
            status => Err(Error::Other(format!(
                "Unexpected response code: {}",
                status
            ))),
        }
    }

    async fn import_log(
        &self,
        codebase: &str,
        run_id: &str,
        orig_path: &str,
        mtime: Option<DateTime<Utc>>,
        basename: Option<&str>,
    ) -> Result<(), Error> {
        let data = tokio::fs::read(orig_path).await?;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        io::copy(&mut Cursor::new(&data), &mut encoder)?;
        let compressed_data = encoder.finish()?;

        let basename = basename.unwrap_or_else(|| {
            Path::new(orig_path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
        });

        let key = self.get_key(codebase, run_id, basename);
        let url = format!("{}{}", self.base_url, key);

        let resp = self
            .client
            .put(&url)
            .header("x-amz-acl", "public-read")
            .body(compressed_data)
            .send()
            .await
            .map_err(|_| Error::ServiceUnavailable)?;

        match resp.status() {
            StatusCode::OK | StatusCode::CREATED => Ok(()),
            StatusCode::FORBIDDEN => Err(Error::PermissionDenied),
            StatusCode::SERVICE_UNAVAILABLE => Err(Error::ServiceUnavailable),
            status => Err(Error::Other(format!(
                "Upload failed with status: {}",
                status
            ))),
        }
    }

    async fn delete_log(&self, codebase: &str, run_id: &str, name: &str) -> Result<(), Error> {
        let url = self.get_url(codebase, run_id, name);

        let resp = self
            .client
            .delete(&url)
            .send()
            .await
            .map_err(|_| Error::ServiceUnavailable)?;

        match resp.status() {
            StatusCode::OK | StatusCode::NO_CONTENT => Ok(()),
            StatusCode::NOT_FOUND => Err(Error::NotFound),
            StatusCode::FORBIDDEN => Err(Error::PermissionDenied),
            status => Err(Error::Other(format!(
                "Delete failed with status: {}",
                status
            ))),
        }
    }

    async fn iter_logs(&self) -> Box<dyn Iterator<Item = (String, String, Vec<String>)>> {
        // S3 listing is complex and would require XML parsing
        // For now, return empty iterator - this can be implemented later
        Box::new(std::iter::empty())
    }

    async fn get_ctime(
        &self,
        codebase: &str,
        run_id: &str,
        name: &str,
    ) -> Result<DateTime<Utc>, Error> {
        // S3 HEAD request doesn't reliably return creation time
        // Would need to implement GetObject with metadata
        Err(Error::Other("get_ctime not implemented for S3".to_string()))
    }

    async fn health_check(&self) -> Result<(), Error> {
        // Try to access the bucket root
        let resp = self
            .client
            .head(&self.base_url)
            .timeout(Duration::from_secs(10))
            .send()
            .await;

        match resp {
            Ok(r) if r.status().is_success() => Ok(()),
            Ok(r) if r.status() == StatusCode::FORBIDDEN => Err(Error::PermissionDenied),
            _ => Err(Error::ServiceUnavailable),
        }
    }
}
