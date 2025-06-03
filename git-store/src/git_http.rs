//! Git HTTP protocol implementation

use crate::error::{GitStoreError, Result};
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::Response,
};
// use bytes::Bytes; // Will be used in later phases
// use futures_util::StreamExt; // Will be used in later phases
use serde::Deserialize;
use std::collections::HashMap;
// use std::sync::Arc; // Will be used later
use tokio::process::Command;
// use tokio_util::io::ReaderStream; // Will be used in later phases
use tracing::warn;

// State is now unified in web.rs

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

/// Placeholder for git HTTP backend (will be implemented in Phase 2)
pub async fn git_backend(
    State(_state): State<crate::web::AppState>,
    Path(_params): Path<HashMap<String, String>>,
    _headers: HeaderMap,
    _body: axum::body::Body,
) -> Result<Response> {
    // This will be implemented in Phase 2
    Err(GitStoreError::Other(anyhow::anyhow!(
        "Git HTTP backend not yet implemented"
    )))
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