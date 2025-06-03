//! Git HTTP protocol implementation

use crate::error::{GitStoreError, Result};
use axum::{
    body::Body,
    extract::{Path, Query, Request, State},
    http::{header, HeaderMap, StatusCode},
    response::Response,
};
use futures_util::TryStreamExt;
use serde::Deserialize;
use std::collections::HashMap;
use std::process::Stdio;
use tokio::{io::{AsyncBufReadExt, AsyncReadExt}, process::Command};
use tokio_util::io::StreamReader;
use tracing::{debug, warn};

const GIT_BACKEND_CHUNK_SIZE: usize = 4096;

/// Query parameters for git diff
#[derive(Debug, Deserialize)]
pub struct DiffQuery {
    old: String,
    new: String,
    path: Option<String>,
}

/// Handle git diff requests
pub async fn git_diff(
    State(state): State<crate::web::AppState>,
    Path(codebase): Path<String>,
    Query(params): Query<DiffQuery>,
) -> Result<Response> {
    // Validate SHAs
    crate::repository::RepositoryManager::validate_sha(&params.old)?;
    crate::repository::RepositoryManager::validate_sha(&params.new)?;

    let repo_path = state.repo_manager.repo_path(&codebase);
    
    if !repo_path.exists() {
        return Err(GitStoreError::RepositoryNotFound(codebase));
    }

    let mut cmd = Command::new("git");
    cmd.arg("diff")
        .arg(&params.old)
        .arg(&params.new)
        .current_dir(&repo_path)
        .kill_on_drop(true);

    if let Some(path) = params.path {
        cmd.arg("--").arg(path);
    }

    let output = tokio::time::timeout(
        std::time::Duration::from_secs(state.config.git_timeout),
        cmd.output(),
    )
    .await
    .map_err(|_| GitStoreError::Timeout)??;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!("git diff failed: {}", stderr);
        return Err(GitStoreError::GitError(git2::Error::from_str(&stderr)));
    }

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/x-diff")
        .body(Body::from(output.stdout))
        .unwrap())
}

/// Query parameters for revision info
#[derive(Debug, Deserialize)]
pub struct RevisionQuery {
    rev: String,
}

/// Handle revision info requests
pub async fn revision_info(
    State(state): State<crate::web::AppState>,
    Path(codebase): Path<String>,
    Query(params): Query<RevisionQuery>,
) -> Result<Response> {
    let repo_path = state.repo_manager.repo_path(&codebase);
    
    if !repo_path.exists() {
        return Err(GitStoreError::RepositoryNotFound(codebase));
    }

    // Use git log to get revision info
    let mut cmd = Command::new("git");
    cmd.arg("log")
        .arg("-1")
        .arg("--format=%H%n%an%n%ae%n%at%n%cn%n%ce%n%ct%n%s%n%b")
        .arg(&params.rev)
        .current_dir(&repo_path)
        .kill_on_drop(true);

    let output = tokio::time::timeout(
        std::time::Duration::from_secs(state.config.git_timeout),
        cmd.output(),
    )
    .await
    .map_err(|_| GitStoreError::Timeout)??;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!("git log failed: {}", stderr);
        return Err(GitStoreError::GitError(git2::Error::from_str(&stderr)));
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = output_str.lines().collect();

    if lines.len() < 8 {
        return Err(GitStoreError::GitError(git2::Error::from_str(
            "Invalid git log output",
        )));
    }

    let info = serde_json::json!({
        "sha": lines[0],
        "author": {
            "name": lines[1],
            "email": lines[2],
            "timestamp": lines[3].parse::<i64>().unwrap_or(0),
        },
        "committer": {
            "name": lines[4],
            "email": lines[5],
            "timestamp": lines[6].parse::<i64>().unwrap_or(0),
        },
        "message": lines[7..].join("\n"),
    });

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&info).map_err(|e| GitStoreError::Other(e.into()))?))
        .unwrap())
}

/// Git HTTP backend using git http-backend subprocess
pub async fn git_backend(
    State(state): State<crate::web::AppState>,
    req: Request,
) -> Result<Response> {
    let uri = req.uri().clone();
    let method = req.method().clone();
    let headers = req.headers().clone();
    let body = req.into_body();

    // Extract codebase from path
    let path = uri.path();
    let path_segments: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    
    if path_segments.is_empty() {
        return Err(GitStoreError::HttpError("Missing codebase in path".to_string()));
    }
    
    let codebase = path_segments[0];
    let subpath = if path_segments.len() > 1 {
        path_segments[1..].join("/")
    } else {
        String::new()
    };

    debug!("Git HTTP backend request for codebase: {}, subpath: {}, method: {}", 
           codebase, subpath, method);

    // Check if repository exists and get path
    let repo_path = state.repo_manager.repo_path(codebase);
    if !repo_path.exists() {
        return Err(GitStoreError::RepositoryNotFound(codebase.to_string()));
    }

    // Extract request information
    let method_str = method.as_str();
    
    let content_type = headers.get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    
    let query_string = uri.query().unwrap_or("");

    // Parse query for service parameter
    let query_params: HashMap<String, String> = serde_urlencoded::from_str(query_string)
        .unwrap_or_default();
    
    let service = query_params.get("service");

    // For now, allow writes for admin interface, deny for public
    // TODO: Implement proper worker authentication
    let allow_writes = true; // This should be determined by authentication

    // Validate Git service if specified
    if let Some(service) = service {
        validate_git_service(service, allow_writes)?;
    }

    // Setup Git HTTP backend process
    let mut cmd = Command::new("git");
    if allow_writes {
        cmd.args(["-c", "http.receivepack=1"]);
    }
    cmd.arg("http-backend");

    // Setup environment variables for Git HTTP backend
    let mut env_vars = HashMap::new();
    env_vars.insert("GIT_HTTP_EXPORT_ALL".to_string(), "true".to_string());
    env_vars.insert("REQUEST_METHOD".to_string(), method_str.to_string());
    env_vars.insert("CONTENT_TYPE".to_string(), content_type.to_string());
    env_vars.insert("QUERY_STRING".to_string(), query_string.to_string());
    
    // Set the repository path
    let full_path = repo_path.join(subpath.trim_start_matches('/'));
    env_vars.insert("PATH_TRANSLATED".to_string(), full_path.display().to_string());
    env_vars.insert("GIT_PROJECT_ROOT".to_string(), repo_path.display().to_string());

    // Add HTTP headers as environment variables
    for (name, value) in headers.iter() {
        if let Ok(value_str) = value.to_str() {
            let env_name = format!("HTTP_{}", name.as_str().replace('-', "_").to_uppercase());
            env_vars.insert(env_name, value_str.to_string());
        }
    }

    // Setup process with environment
    for (key, value) in env_vars {
        cmd.env(key, value);
    }

    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    debug!("Starting git http-backend process");
    let mut process = cmd.spawn()
        .map_err(|e| GitStoreError::Other(anyhow::anyhow!("Failed to spawn git process: {}", e)))?;

    // Handle request body (stdin to git process)
    if let Some(mut stdin) = process.stdin.take() {
        let body_stream = body.into_data_stream();
        tokio::spawn(async move {
            let mut stdin_writer = StreamReader::new(body_stream.map_err(std::io::Error::other));
            if let Err(e) = tokio::io::copy(&mut stdin_writer, &mut stdin).await {
                warn!("Error writing to git process stdin: {}", e);
            }
        });
    }

    // Handle stderr (logging)
    if let Some(stderr) = process.stderr.take() {
        tokio::spawn(async move {
            let mut lines = tokio::io::BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                warn!("git http-backend stderr: {}", line);
            }
        });
    }

    // Handle stdout (response to client)
    let stdout = process.stdout.take()
        .ok_or_else(|| GitStoreError::Other(anyhow::anyhow!("Failed to capture git process stdout")))?;

    // Parse HTTP response from git http-backend
    let mut reader = tokio::io::BufReader::new(stdout);
    
    // Read headers until empty line
    let mut response_headers = HeaderMap::new();
    let mut status_code = StatusCode::OK;
    let mut content_length: Option<usize> = None;
    
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).await
            .map_err(|e| GitStoreError::Other(anyhow::anyhow!("Failed to read git response: {}", e)))?;
        
        if line.trim().is_empty() {
            break; // End of headers
        }
        
        if let Some((key, value)) = line.trim().split_once(':') {
            let key = key.trim().to_string();
            let value = value.trim().to_string();
            
            if key.eq_ignore_ascii_case("status") {
                // Parse status line: "200 OK" or "404 Not Found"
                if let Some(code_str) = value.split_whitespace().next() {
                    if let Ok(code) = code_str.parse::<u16>() {
                        status_code = StatusCode::from_u16(code).unwrap_or(StatusCode::OK);
                    }
                }
            } else if key.eq_ignore_ascii_case("content-length") {
                content_length = value.parse().ok();
                if let Ok(header_value) = value.parse::<http::HeaderValue>() {
                    response_headers.insert(header::CONTENT_LENGTH, header_value);
                }
            } else {
                if let Ok(header_name) = key.parse::<http::HeaderName>() {
                    if let Ok(header_value) = value.parse::<http::HeaderValue>() {
                        response_headers.insert(header_name, header_value);
                    }
                }
            }
        }
    }

    debug!("Git response status: {}, content-length: {:?}", status_code, content_length);

    // Create response based on whether we have content-length
    if let Some(length) = content_length {
        // Fixed-length response
        let mut body_data = vec![0u8; length];
        reader.read_exact(&mut body_data).await
            .map_err(|e| GitStoreError::Other(anyhow::anyhow!("Failed to read git response body: {}", e)))?;
        
        let mut response = Response::builder().status(status_code);
        for (name, value) in response_headers.iter() {
            response = response.header(name, value);
        }
        
        Ok(response.body(Body::from(body_data))?)
    } else {
        // Streaming response - for now, read all data into memory
        // TODO: Implement proper streaming when axum supports it better
        let mut body_data = Vec::new();
        reader.read_to_end(&mut body_data).await
            .map_err(|e| GitStoreError::Other(anyhow::anyhow!("Failed to read git response body: {}", e)))?;
        
        let mut response = Response::builder().status(status_code);
        for (name, value) in response_headers.iter() {
            response = response.header(name, value);
        }
        
        Ok(response.body(Body::from(body_data))?)
    }
}

/// Validate Git service parameter
fn validate_git_service(service: &str, allow_writes: bool) -> Result<()> {
    match service {
        "git-upload-pack" => {
            // This is a read operation (clone, fetch)
            debug!("Git upload-pack service requested");
            Ok(())
        }
        "git-receive-pack" => {
            // This is a write operation (push)
            if allow_writes {
                debug!("Git receive-pack service requested (writes allowed)");
                Ok(())
            } else {
                warn!("Git receive-pack service denied (writes not allowed)");
                Err(GitStoreError::PermissionDenied)
            }
        }
        _ => {
            warn!("Unknown Git service requested: {}", service);
            Err(GitStoreError::HttpError(format!("Unknown Git service: {}", service)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_query_parsing() {
        let query = "old=abc123&new=def456&path=src/main.rs";
        let params: DiffQuery = serde_urlencoded::from_str(query).unwrap();
        assert_eq!(params.old, "abc123");
        assert_eq!(params.new, "def456");
        assert_eq!(params.path, Some("src/main.rs".to_string()));
    }

    #[test]
    fn test_revision_query_parsing() {
        let query = "rev=abc123";
        let params: RevisionQuery = serde_urlencoded::from_str(query).unwrap();
        assert_eq!(params.rev, "abc123");
    }
}