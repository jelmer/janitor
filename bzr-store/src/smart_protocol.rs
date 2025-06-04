//! Bazaar smart protocol handler
//!
//! This module implements the Bazaar smart protocol for efficient repository operations
//! over HTTP. The smart protocol allows for optimized clone, pull, and push operations.

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use crate::error::{BzrError, Result};
use crate::web::AppState;

/// Smart protocol handler state
pub struct SmartProtocolHandler {
    /// Python smart server instance (lazily initialized)
    smart_server: Arc<Mutex<Option<PyObject>>>,
}

impl Default for SmartProtocolHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl SmartProtocolHandler {
    /// Create a new smart protocol handler
    pub fn new() -> Self {
        Self {
            smart_server: Arc::new(Mutex::new(None)),
        }
    }

    /// Initialize the Python smart server if not already initialized
    async fn ensure_smart_server(&self, repo_path: &std::path::Path) -> Result<()> {
        let mut server_guard = self.smart_server.lock().await;

        if server_guard.is_none() {
            Python::with_gil(|py| {
                // Import the smart server module
                let smart_module = py.import_bound("breezy.bzr.smart").map_err(|e| {
                    BzrError::Python(format!("Failed to import smart module: {}", e))
                })?;

                let server_module = py.import_bound("breezy.bzr.smart.server").map_err(|e| {
                    BzrError::Python(format!("Failed to import smart.server: {}", e))
                })?;

                // Create a smart server instance
                let backing_transport = self.create_transport(py, repo_path)?;

                let smart_server = server_module
                    .getattr("SmartServerPipeStreamMedium")?
                    .call1((backing_transport,))?;

                *server_guard = Some(smart_server.unbind());
                info!(
                    "Initialized Bazaar smart server for: {}",
                    repo_path.display()
                );
                Ok::<(), BzrError>(())
            })?
        }

        Ok(())
    }

    /// Create a transport for the given path
    fn create_transport(&self, py: Python, path: &std::path::Path) -> PyResult<PyObject> {
        let transport_module = py.import_bound("breezy.transport")?;
        let path_str = format!("file://{}", path.display());

        Ok(transport_module
            .getattr("get_transport")?
            .call1((path_str,))?
            .unbind())
    }

    /// Process a smart protocol request
    pub async fn handle_request(
        &self,
        repo_path: std::path::PathBuf,
        headers: HeaderMap,
        body: Bytes,
    ) -> Result<Response> {
        debug!("Smart protocol request: {} bytes", body.len());

        // Ensure smart server is initialized
        self.ensure_smart_server(&repo_path).await?;

        // Process the request using PyO3
        // For now, use a simplified approach that recreates the server each time
        let response_bytes = Python::with_gil(|py| -> Result<Vec<u8>> {
            // Import the smart server module
            let server_module = py
                .import_bound("breezy.bzr.smart.server")
                .map_err(|e| BzrError::Python(format!("Failed to import smart.server: {}", e)))?;

            // Create a transport for this request
            let transport_module = py.import_bound("breezy.transport")?;
            let path_str = format!("file://{}", repo_path.display());
            let backing_transport = transport_module
                .getattr("get_transport")?
                .call1((path_str,))?;

            // Create a smart server instance for this request
            let smart_server = server_module
                .getattr("SmartServerPipeStreamMedium")?
                .call1((backing_transport,))?;

            // Convert request body to Python bytes
            let py_request = PyBytes::new_bound(py, &body);

            // Process the request
            match smart_server.call_method1("process_request", (py_request,)) {
                Ok(response) => {
                    // Extract response bytes as Vec<u8>
                    let response_bytes = response.extract::<Vec<u8>>().map_err(|e| {
                        BzrError::Python(format!("Failed to extract response: {}", e))
                    })?;

                    Ok(response_bytes)
                }
                Err(e) => {
                    error!("Smart protocol error: {}", e);
                    Err(BzrError::Python(format!(
                        "Smart protocol processing failed: {}",
                        e
                    )))
                }
            }
        })?;

        debug!("Smart protocol response: {} bytes", response_bytes.len());

        // Build response with appropriate headers
        Ok((
            StatusCode::OK,
            [
                ("content-type", "application/octet-stream"),
                ("content-length", &response_bytes.len().to_string()),
            ],
            response_bytes,
        )
            .into_response())
    }
}

/// Bazaar smart protocol endpoint handler
pub async fn smart_protocol_handler(
    State(state): State<AppState>,
    Path((campaign, codebase, role)): Path<(String, String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response> {
    info!(
        "Smart protocol request for {}/{}/{}",
        campaign, codebase, role
    );

    // Build repository path
    let repo_path = crate::repository::RepositoryPath::new(campaign, codebase, role);

    // Get the actual filesystem path
    let fs_path = state
        .config
        .repository_path
        .join(&repo_path.campaign)
        .join(&repo_path.codebase)
        .join(&repo_path.role);

    // Check if repository exists
    if !fs_path.exists() {
        warn!("Repository not found: {}", fs_path.display());
        return Err(BzrError::PathNotFound {
            path: repo_path.relative_path(),
        });
    }

    // Create handler and process request
    let handler = SmartProtocolHandler::new();
    handler.handle_request(fs_path, headers, body).await
}

/// Alternative smart protocol implementation using subprocess
pub async fn smart_protocol_subprocess_handler(
    repo_path: std::path::PathBuf,
    body: Bytes,
) -> Result<Response> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::process::Command;

    debug!("Smart protocol subprocess request: {} bytes", body.len());

    // Launch bzr serve process
    let mut child = Command::new("brz")
        .args([
            "serve",
            "--inet",
            "--directory",
            &repo_path.to_string_lossy(),
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| BzrError::Subprocess(format!("Failed to spawn bzr serve: {}", e)))?;

    // Write request to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(&body).await.map_err(BzrError::Io)?;
        stdin.shutdown().await.map_err(BzrError::Io)?;
    }

    // Read response from stdout
    let mut response_bytes = Vec::new();
    if let Some(mut stdout) = child.stdout.take() {
        stdout
            .read_to_end(&mut response_bytes)
            .await
            .map_err(BzrError::Io)?;
    }

    // Wait for process to complete
    let status = child
        .wait()
        .await
        .map_err(|e| BzrError::Subprocess(format!("Process wait failed: {}", e)))?;

    if !status.success() {
        // Read stderr for error details
        let mut stderr_bytes = Vec::new();
        if let Some(mut stderr) = child.stderr.take() {
            stderr.read_to_end(&mut stderr_bytes).await.ok();
        }
        let stderr = String::from_utf8_lossy(&stderr_bytes);

        return Err(BzrError::Subprocess(format!(
            "bzr serve failed with status {}: {}",
            status.code().unwrap_or(-1),
            stderr
        )));
    }

    debug!(
        "Smart protocol subprocess response: {} bytes",
        response_bytes.len()
    );

    Ok((
        StatusCode::OK,
        [
            ("content-type", "application/octet-stream"),
            ("content-length", &response_bytes.len().to_string()),
        ],
        response_bytes,
    )
        .into_response())
}

/// Repository file serving handler for .bzr directory access
pub async fn serve_bzr_file_handler(
    State(state): State<AppState>,
    Path((campaign, codebase, role, file_path)): Path<(String, String, String, String)>,
) -> Result<Response> {
    use tokio::fs;

    // Validate that the requested path is within .bzr directory
    if !file_path.starts_with(".bzr/") {
        return Err(BzrError::InvalidRequest {
            message: "Only .bzr directory files can be accessed".to_string(),
        });
    }

    // Build repository path
    let repo_path = crate::repository::RepositoryPath::new(campaign, codebase, role);

    // Get the actual filesystem path
    let fs_path = state
        .config
        .repository_path
        .join(&repo_path.campaign)
        .join(&repo_path.codebase)
        .join(&repo_path.role)
        .join(&file_path);

    // Security check: ensure the resolved path is still within the repository
    let repo_base = state
        .config
        .repository_path
        .join(&repo_path.campaign)
        .join(&repo_path.codebase)
        .join(&repo_path.role);

    let fs_path = fs_path.canonicalize().map_err(|_| BzrError::PathNotFound {
        path: file_path.clone(),
    })?;

    if !fs_path.starts_with(&repo_base) {
        return Err(BzrError::InvalidRequest {
            message: "Path traversal attempt detected".to_string(),
        });
    }

    // Check if file exists
    if !fs_path.exists() {
        return Err(BzrError::PathNotFound { path: file_path });
    }

    // Read file content
    let content = fs::read(&fs_path).await?;

    // Determine content type based on file extension
    let content_type = if file_path.ends_with(".pack") {
        "application/octet-stream"
    } else if file_path.ends_with(".rix") || file_path.ends_with(".iix") {
        "application/octet-stream"
    } else {
        "text/plain"
    };

    Ok((
        StatusCode::OK,
        [
            ("content-type", content_type),
            ("content-length", &content.len().to_string()),
        ],
        content,
    )
        .into_response())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smart_protocol_handler_creation() {
        let handler = SmartProtocolHandler::new();
        assert!(handler.smart_server.try_lock().is_ok());
    }
}
