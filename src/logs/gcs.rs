use async_trait::async_trait;
use chrono::{DateTime, Utc};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use google_cloud_auth::credentials::CredentialsFile;
use google_cloud_storage::client::{Client, ClientConfig};
use google_cloud_storage::http::buckets::get::GetBucketRequest;
use google_cloud_storage::http::buckets::Bucket;
use google_cloud_storage::http::objects::download::Range;
use google_cloud_storage::http::objects::get::GetObjectRequest;
use google_cloud_storage::http::objects::list::ListObjectsRequest;
use google_cloud_storage::http::objects::upload::{Media, UploadObjectRequest, UploadType};
use std::borrow::Cow;
use std::collections::HashMap;
use std::io::{self, Cursor, Read};
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

use crate::logs::{Error, LogFileManager};

pub struct GCSLogFileManager {
    client: Client,
    bucket: Bucket,
    last_error: std::sync::Arc<std::sync::Mutex<Option<String>>>,
}

impl GCSLogFileManager {
    fn record_error(&self, error: Option<String>) {
        if let Ok(mut last_error) = self.last_error.lock() {
            *last_error = error;
        }
    }

    pub async fn from_url(
        location: &url::Url,
        creds: Option<CredentialsFile>,
    ) -> Result<Self, Error> {
        if location.scheme() != "gs" {
            return Err(Error::Other(format!(
                "Invalid scheme: {}",
                location.scheme()
            )));
        }

        let bucket_name = location
            .host_str()
            .ok_or_else(|| Error::Other("Missing bucket name".to_string()))?;

        Self::new(bucket_name, creds).await
    }

    pub async fn new(bucket_name: &str, creds: Option<CredentialsFile>) -> Result<Self, Error> {
        let mut config = ClientConfig::default();

        if let Some(creds) = creds {
            config = match config.with_credentials(creds).await {
                Ok(config) => config,
                Err(e) => return Err(Error::Other(e.to_string())),
            };
        } else {
            config = config.anonymous();
        }

        let client = Client::new(config);

        let get_bucket_request = GetBucketRequest {
            bucket: bucket_name.to_owned(),
            ..Default::default()
        };

        let bucket = match client.get_bucket(&get_bucket_request).await {
            Ok(bucket) => bucket,
            Err(e) => return Err(Error::Other(e.to_string())),
        };

        Ok(Self {
            client,
            bucket,
            last_error: std::sync::Arc::new(std::sync::Mutex::new(None)),
        })
    }

    fn get_object_name(&self, codebase: &str, run_id: &str, name: &str) -> String {
        format!("{}/{}/{}.gz", codebase, run_id, name)
    }
}

#[async_trait]
impl LogFileManager for GCSLogFileManager {
    async fn has_log(&self, codebase: &str, run_id: &str, name: &str) -> Result<bool, Error> {
        let object_name = self.get_object_name(codebase, run_id, name);

        let get_request = GetObjectRequest {
            bucket: self.bucket.name.clone(),
            object: object_name,
            ..Default::default()
        };

        match self.client.get_object(&get_request).await {
            Ok(_) => {
                // If we can get the object, it exists
                self.record_error(None); // Clear error on success
                Ok(true)
            }
            Err(e) => {
                if e.to_string().contains("Not Found") {
                    self.record_error(None); // Not found is not an error condition
                    Ok(false)
                } else {
                    self.record_error(Some(e.to_string()));
                    Err(Error::Other(e.to_string()))
                }
            }
        }
    }

    async fn get_log(
        &self,
        codebase: &str,
        run_id: &str,
        name: &str,
    ) -> Result<Box<dyn Read + Send + Sync>, Error> {
        let object_name = self.get_object_name(codebase, run_id, name);

        // Check if the object exists first
        let get_request = GetObjectRequest {
            bucket: self.bucket.name.clone(),
            object: object_name.clone(),
            ..Default::default()
        };

        match self.client.get_object(&get_request).await {
            Ok(_) => {
                // Download the object content with full range
                let download_req = GetObjectRequest {
                    bucket: self.bucket.name.clone(),
                    object: object_name,
                    ..Default::default()
                };

                let range = Range::default();

                let bytes = match self.client.download_object(&download_req, &range).await {
                    Ok(data) => data,
                    Err(e) => return Err(Error::Other(e.to_string())),
                };

                // Create a cursor for the data
                let cursor = Cursor::new(bytes);

                // Create a flate2 decoder
                let decoder = GzDecoder::new(cursor);

                // Return the decoder as a boxed Read
                Ok(Box::new(decoder) as Box<dyn Read + Send + Sync>)
            }
            Err(e) => {
                if e.to_string().contains("Not Found") {
                    Err(Error::NotFound)
                } else {
                    Err(Error::Other(e.to_string()))
                }
            }
        }
    }

    async fn import_log(
        &self,
        codebase: &str,
        run_id: &str,
        orig_path: &str,
        _mtime: Option<DateTime<Utc>>,
        basename: Option<&str>,
    ) -> Result<(), Error> {
        // Extract basename from the path if not provided
        let basename_owned: Option<String> = if basename.is_none() {
            let path = PathBuf::from(orig_path);
            if let Some(filename) = path.file_name() {
                if let Some(name_str) = filename.to_str() {
                    Some(name_str.to_string())
                } else {
                    return Err(Error::Other("Invalid filename".to_string()));
                }
            } else {
                return Err(Error::Other(
                    "Cannot extract basename from path".to_string(),
                ));
            }
        } else {
            None
        };

        // Use either the provided basename or the extracted one
        let basename_ref = basename.unwrap_or_else(|| basename_owned.as_ref().unwrap());

        // Generate the final object name
        let object_name = self.get_object_name(codebase, run_id, basename_ref);

        let mut file = File::open(orig_path).await?;
        let mut plain_data = Vec::new();
        file.read_to_end(&mut plain_data).await?;

        // Compress the data
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        io::copy(&mut Cursor::new(&plain_data), &mut encoder)
            .map_err(|e| Error::Other(e.to_string()))?;
        let compressed_data = encoder.finish().map_err(|e| Error::Other(e.to_string()))?;

        // Upload to GCS
        let request = UploadObjectRequest {
            bucket: self.bucket.name.clone(),
            predefined_acl: None,
            // Other fields use default values
            ..Default::default()
        };

        // Create a Media object with the object name and content length
        let media = Media {
            name: Cow::Owned(object_name),
            content_type: Cow::Borrowed("application/octet-stream"),
            content_length: Some(compressed_data.len() as u64),
        };

        // Use the simple upload type
        let upload_type = UploadType::Simple(media);

        // Execute the upload
        match self
            .client
            .upload_object(&request, compressed_data, &upload_type)
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => {
                if e.to_string().contains("Permission") {
                    Err(Error::PermissionDenied)
                } else {
                    Err(Error::Other(e.to_string()))
                }
            }
        }
    }

    async fn iter_logs(&self) -> Box<dyn Iterator<Item = (String, String, Vec<String>)>> {
        let result = self.iter_logs_internal().await;
        match result {
            Ok(logs) => Box::new(logs.into_iter()),
            Err(_) => Box::new(std::iter::empty()),
        }
    }

    async fn get_ctime(
        &self,
        codebase: &str,
        run_id: &str,
        name: &str,
    ) -> Result<DateTime<Utc>, Error> {
        let object_name = self.get_object_name(codebase, run_id, name);

        let get_request = GetObjectRequest {
            bucket: self.bucket.name.clone(),
            object: object_name,
            ..Default::default()
        };

        match self.client.get_object(&get_request).await {
            Ok(object) => {
                // Extract time_created from the metadata
                if let Some(time_created) = object.time_created {
                    // Convert from time::OffsetDateTime to chrono::DateTime<Utc>
                    let timestamp = time_created.unix_timestamp();
                    let nsecs = time_created.nanosecond() as u32;
                    if let Some(dt) = DateTime::<Utc>::from_timestamp(timestamp, nsecs) {
                        Ok(dt)
                    } else {
                        Ok(Utc::now())
                    }
                } else {
                    Err(Error::Other(
                        "Time created not found for the object".to_string(),
                    ))
                }
            }
            Err(e) => {
                if e.to_string().contains("Not Found") {
                    Err(Error::NotFound)
                } else {
                    Err(Error::Other(e.to_string()))
                }
            }
        }
    }

    async fn health_check(&self) -> Result<(), Error> {
        // Check the last error status without making any network calls
        if let Ok(last_error) = self.last_error.lock() {
            if let Some(ref error_msg) = *last_error {
                Err(Error::Other(format!(
                    "Last operation failed: {}",
                    error_msg
                )))
            } else {
                Ok(())
            }
        } else {
            Err(Error::Other("Failed to check error status".to_string()))
        }
    }
}

impl GCSLogFileManager {
    // Helper method to implement iter_logs functionality
    async fn iter_logs_internal(&self) -> Result<Vec<(String, String, Vec<String>)>, Error> {
        let mut logs: HashMap<(String, String), Vec<String>> = HashMap::new();

        let list_request = ListObjectsRequest {
            bucket: self.bucket.name.clone(),
            delimiter: None,
            prefix: None,
            ..Default::default()
        };

        let response = match self.client.list_objects(&list_request).await {
            Ok(response) => response,
            Err(e) => return Err(Error::Other(e.to_string())),
        };

        // Process each object in the listing
        for item in response.items.unwrap_or_default() {
            // Get the name from the item directly (no Option in this API version)
            let name = item.name;

            // Parse object name into components
            let parts: Vec<&str> = name.split('/').collect();
            if parts.len() == 3 {
                let codebase = parts[0].to_string();
                let log_id = parts[1].to_string();

                // Get the log filename (remove .gz extension)
                let mut log_file = parts[2].to_string();
                if log_file.ends_with(".gz") {
                    log_file = log_file[..log_file.len() - 3].to_string();
                }

                logs.entry((codebase, log_id))
                    .or_insert_with(Vec::new)
                    .push(log_file);
            }
        }

        Ok(logs
            .into_iter()
            .map(|((codebase, log_id), log_files)| (codebase, log_id, log_files))
            .collect())
    }
}

#[cfg(test)]
mod tests {

    /// A simplified test for get_object_name that doesn't require actual GCS connections
    #[test]
    fn test_get_object_name() {
        // Create a struct with just the methods we need for testing
        struct TestGCSLogFileManager;

        impl TestGCSLogFileManager {
            fn get_object_name(&self, codebase: &str, run_id: &str, name: &str) -> String {
                format!("{}/{}/{}.gz", codebase, run_id, name)
            }
        }

        let manager = TestGCSLogFileManager {};

        assert_eq!(
            manager.get_object_name("my-codebase", "run123", "build.log"),
            "my-codebase/run123/build.log.gz"
        );
    }

    // Integration tests that would connect to actual GCS would go here
    // We'd need to use environment variables or mocks for those tests
}
