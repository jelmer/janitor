use crate::AppState;
use askama_axum::IntoResponse;
use askama_axum::Template;
use axum::{
    extract::Path, extract::State, http::HeaderMap, http::StatusCode, response::Json,
    response::Response, routing::{get, post}, Router,
};
use janitor::api::worker::{Assignment, Metadata};
use serde_json::json;
use std::sync::{Arc, RwLock};
#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate<'a> {
    pub assignment: Option<&'a Assignment>,
    pub metadata: Option<&'a Metadata>,
    pub lognames: Option<Vec<String>>,
}

#[derive(Template)]
#[template(path = "artifact_index.html")]
pub struct ArtifactIndexTemplate {
    pub names: Vec<String>,
}

#[derive(Template)]
#[template(path = "log_index.html")]
pub struct LogIndexTemplate {
    pub names: Vec<String>,
}

async fn index(State(state): State<Arc<RwLock<AppState>>>) -> Response {
    let state = match state.read() {
        Ok(state) => state,
        Err(_) => {
            log::error!("Worker state lock poisoned");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response();
        }
    };
    let lognames: Option<Vec<String>> =
        if let Some(output_directory) = state.output_directory.as_ref() {
            match output_directory.read_dir() {
                Ok(read_dir) => Some(
                    read_dir
                        .filter_map(|entry| {
                            let entry = entry.ok()?;
                            let filename = entry.file_name();
                            let name = filename.to_str()?;
                            if name.ends_with(".log") {
                                Some(name.to_owned())
                            } else {
                                None
                            }
                        })
                        .collect(),
                ),
                Err(e) => {
                    log::warn!("Failed to read output directory: {}", e);
                    None
                }
            }
        } else {
            None
        };

    IndexTemplate {
        assignment: state.assignment.as_ref(),
        lognames,
        metadata: state.metadata.as_ref(),
    }
    .into_response()
}

async fn health() -> String {
    "ok".to_string()
}

async fn assignment(State(state): State<Arc<RwLock<AppState>>>) -> impl IntoResponse {
    match state.read() {
        Ok(state) => Json(state.assignment.clone()).into_response(),
        Err(_) => {
            log::error!("Worker state lock poisoned in assignment endpoint");
            (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
        }
    }
}

async fn get_logs(State(state): State<Arc<RwLock<AppState>>>, headers: HeaderMap) -> Response {
    let output_directory = match state.read() {
        Ok(state) => state.output_directory.clone(),
        Err(_) => {
            log::error!("Worker state lock poisoned in get_logs");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response();
        }
    };
    if output_directory.is_none() {
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body("Log directory not created yet".into())
            .unwrap();
    }
    let output_directory = output_directory.as_ref().unwrap();
    let names: Vec<String> = match std::fs::read_dir(output_directory) {
        Ok(dir) => dir
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.extension()?.to_str()? == "log" {
                    Some(entry.file_name().to_str()?.to_owned())
                } else {
                    None
                }
            })
            .collect(),
        Err(e) => {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(format!("Error reading log directory: {}", e).into())
                .unwrap();
        }
    };

    match headers
        .get(axum::http::header::ACCEPT)
        .map(|x| x.to_str().unwrap())
    {
        Some("application/json") => Json(names).into_response(),
        _ => LogIndexTemplate { names }.into_response(),
    }
}

async fn get_artifacts(State(state): State<Arc<RwLock<AppState>>>, headers: HeaderMap) -> Response {
    let output_directory = match state.read() {
        Ok(state) => state.output_directory.clone(),
        Err(_) => {
            log::error!("Worker state lock poisoned in get_artifacts");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response();
        }
    };
    if output_directory.is_none() {
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body("Artifact directory not created yet".into())
            .unwrap();
    }
    let output_directory = output_directory.as_ref().unwrap();
    let names: Vec<String> = match std::fs::read_dir(output_directory) {
        Ok(dir) => dir
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.extension()?.to_str()? != "log" {
                    Some(entry.file_name().to_str()?.to_owned())
                } else {
                    None
                }
            })
            .collect(),
        Err(e) => {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(format!("Error reading log directory: {}", e).into())
                .unwrap();
        }
    };

    match headers
        .get(axum::http::header::ACCEPT)
        .map(|x| x.to_str().unwrap())
    {
        Some("application/json") => Json(names).into_response(),
        _ => ArtifactIndexTemplate { names }.into_response(),
    }
}

async fn get_log_id(State(state): State<Arc<RwLock<AppState>>>) -> Json<Option<String>> {
    Json(
        state
            .read()
            .unwrap()
            .assignment
            .as_ref()
            .map(|a| a.id.clone()),
    )
}

async fn get_log_file(
    State(state): State<Arc<RwLock<AppState>>>,
    Path(filename): Path<String>,
) -> Response {
    let output_directory = match state.read() {
        Ok(state) => match state.output_directory.as_ref() {
            Some(dir) => dir.clone(),
            None => {
                return Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body("Log directory not created yet".into())
                    .unwrap();
            }
        },
        Err(_) => {
            log::error!("Worker state lock poisoned in get_log_file");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response();
        }
    };

    let p = match janitor::security::safe_path_join(&output_directory, &filename) {
        Ok(path) => path,
        Err(err) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(format!("Invalid filename: {}", err).into())
                .unwrap();
        }
    };

    let file = match tokio::fs::File::open(&p).await {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body("No such log file".into())
                .unwrap();
        }
        Err(e) => {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(format!("Error opening log file: {}", e).into())
                .unwrap();
        }
    };

    let stream = tokio_util::io::ReaderStream::new(file);

    let body = axum::body::Body::from_stream(stream);

    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        "text/plain".parse().unwrap(),
    );

    (StatusCode::OK, headers, body).into_response()
}

async fn get_artifact_file(
    State(state): State<Arc<RwLock<AppState>>>,
    Path(filename): Path<String>,
) -> Response {
    let output_directory = match state.read() {
        Ok(state) => match state.output_directory.as_ref() {
            Some(dir) => dir.clone(),
            None => {
                return Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body("Log directory not created yet".into())
                    .unwrap();
            }
        },
        Err(_) => {
            log::error!("Worker state lock poisoned in get_log_file");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response();
        }
    };

    let _output_directory_for_match = match Some(&output_directory) {
        Some(dir) => dir.clone(),
        None => {
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body("Artifact directory not created yet".into())
                .unwrap();
        }
    };

    let p = match janitor::security::safe_path_join(&output_directory, &filename) {
        Ok(path) => path,
        Err(err) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(format!("Invalid filename: {}", err).into())
                .unwrap();
        }
    };

    let file = match tokio::fs::File::open(&p).await {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body("No such artifact file".into())
                .unwrap();
        }
        Err(e) => {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(format!("Error opening artifact file: {}", e).into())
                .unwrap();
        }
    };

    let stream = tokio_util::io::ReaderStream::new(file);

    let body = axum::body::Body::from_stream(stream);

    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        "application/octet-stream".parse().unwrap(),
    );

    (StatusCode::OK, headers, body).into_response()
}

/// Kill the worker process
async fn kill() -> Response {
    log::info!("Received kill request via HTTP backchannel");
    std::process::exit(1);
}

/// Terminate the worker gracefully
async fn terminate() -> Response {
    log::info!("Received terminate request via HTTP backchannel");
    // For now, terminate is the same as kill - could be enhanced to flush logs, etc.
    std::process::exit(0);
}

/// Get detailed status information from the worker
async fn status(State(state): State<Arc<RwLock<AppState>>>) -> Response {
    let state = match state.read() {
        Ok(state) => state,
        Err(_) => {
            log::error!("Worker state lock poisoned in status endpoint");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response();
        }
    };

    let current_run_id = state
        .assignment
        .as_ref()
        .map(|a| a.id.clone());

    let alive = true; // Worker is responding if we're here
    let status_str = if state.assignment.is_some() {
        "processing"
    } else {
        "idle"
    };

    // Get uptime (approximate - since process start)
    let uptime_seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let status_info = json!({
        "alive": alive,
        "current_run_id": current_run_id,
        "status": status_str,
        "uptime_seconds": uptime_seconds,
        "metadata": state.metadata
    });

    Json(status_info).into_response()
}

pub fn app(state: Arc<RwLock<AppState>>) -> Router {
    // build our application with a route
    Router::new()
        .route("/", get(index))
        .route("/health", get(health))
        .route("/assignment", get(assignment))
        .route("/logs", get(get_logs))
        .route("/logs/:filename", get(get_log_file))
        .route("/log-id", get(get_log_id))
        .route("/artifacts", get(get_artifacts))
        .route("/artifacts/:filename", get(get_artifact_file))
        .route("/kill", post(kill))
        .route("/terminate", post(terminate))
        .route("/status", get(status))
        .with_state(state.clone().clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use tower::ServiceExt; // for `collect`

    async fn get_body(response: Response<Body>) -> String {
        let body = response.into_body().collect().await.unwrap().to_bytes();
        String::from_utf8(body[..].to_vec()).unwrap()
    }

    #[tokio::test]
    async fn test_health() {
        let app = app(Arc::new(RwLock::new(AppState::default())));
        let request = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), 200);

        let body = get_body(response).await;
        assert_eq!(body, "ok");
    }

    #[tokio::test]
    async fn test_assignment_empty() {
        let state = Arc::new(RwLock::new(AppState::default()));
        let app = app(state.clone());
        let request = Request::builder()
            .uri("/assignment")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), 200);

        let body = get_body(response).await;
        assert_eq!(body, "null");

        if let Ok(mut state) = state.write() {
            state.assignment = Some(Assignment {
                id: "test".to_string(),
                queue_id: 1,
                campaign: "lintian-fixes".to_string(),
                codebase: "foo".to_string(),
                force_build: true,
                branch: janitor::api::worker::Branch {
                    cached_url: None,
                    vcs_type: janitor::vcs::VcsType::Git,
                    url: Some("https://example.com/vcs".parse().unwrap()),
                    subpath: std::path::PathBuf::from(""),
                    additional_colocated_branches: Some(vec![]),
                    default_empty: false,
                },
                resume: None,
                target_repository: janitor::api::worker::TargetRepository {
                    url: "https://example.com/".parse().unwrap(),
                },
                skip_setup_validation: false,
                codemod: janitor::api::worker::Codemod {
                    command: "echo".to_string(),
                    environment: std::collections::HashMap::new(),
                },
                env: std::collections::HashMap::new(),
                build: janitor::api::worker::Build {
                    config: serde_json::Value::Null,
                    environment: None,
                    target: "test".to_string(),
                },
            });
        };

        let request = Request::builder()
            .uri("/assignment")
            .body(Body::empty())
            .unwrap();

        let app = super::app(state.clone());
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), 200);

        let body: Assignment = serde_json::from_str(&get_body(response).await).unwrap();
        assert_eq!(body.id, "test");
        assert_eq!(body.queue_id, 1);
    }

    #[tokio::test]
    async fn test_logs_empty() {
        let state = Arc::new(RwLock::new(AppState::default()));
        let app = app(state.clone());
        let request = Request::builder().uri("/logs").body(Body::empty()).unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), 404);

        let body = get_body(response).await;
        assert_eq!(body, "Log directory not created yet");

        let td = tempfile::tempdir().unwrap();

        if let Ok(mut state) = state.write() {
            state.output_directory = Some(td.path().to_path_buf());
        };

        let request = Request::builder()
            .uri("/logs")
            .header("Accept", "application/json")
            .body(Body::empty())
            .unwrap();

        let app = super::app(state.clone());
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), 200);

        let body: Vec<String> = serde_json::from_str(&get_body(response).await).unwrap();
        assert_eq!(body, Vec::<String>::new());

        // create a log file
        std::fs::write(td.path().join("test.log"), "test content").unwrap();

        let request = Request::builder()
            .uri("/logs")
            .header("Accept", "application/json")
            .body(Body::empty())
            .unwrap();

        let app = super::app(state.clone());

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), 200);

        let body: Vec<String> = serde_json::from_str(&get_body(response).await).unwrap();

        assert_eq!(body, vec!["test.log"]);

        // get the log file
        let request = Request::builder()
            .uri("/logs/test.log")
            .body(Body::empty())
            .unwrap();

        let app = super::app(state.clone());

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), 200);

        let body = get_body(response).await;

        assert_eq!(body, "test content");
    }

    #[tokio::test]
    async fn test_artifacts_empty() {
        let state = Arc::new(RwLock::new(AppState::default()));
        let app = app(state.clone());
        let request = Request::builder()
            .uri("/artifacts")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), 404);

        let body = get_body(response).await;
        assert_eq!(body, "Artifact directory not created yet");

        let td = tempfile::tempdir().unwrap();

        if let Ok(mut state) = state.write() {
            state.output_directory = Some(td.path().to_path_buf());
        };

        let request = Request::builder()
            .uri("/artifacts")
            .header("Accept", "application/json")
            .body(Body::empty())
            .unwrap();

        let app = super::app(state.clone());
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), 200);

        let body: Vec<String> = serde_json::from_str(&get_body(response).await).unwrap();
        assert_eq!(body, Vec::<String>::new());

        // create a log file
        std::fs::write(td.path().join("test.log"), "test").unwrap();

        let request = Request::builder()
            .uri("/artifacts")
            .header("Accept", "application/json")
            .body(Body::empty())
            .unwrap();

        let app = super::app(state.clone());

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), 200);

        let body: Vec<String> = serde_json::from_str(&get_body(response).await).unwrap();

        assert_eq!(body, Vec::<String>::new());

        // create an artifact file
        std::fs::write(td.path().join("test.artifact"), "test").unwrap();

        let request = Request::builder()
            .uri("/artifacts")
            .header("Accept", "application/json")
            .body(Body::empty())
            .unwrap();

        let app = super::app(state.clone());

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), 200);

        let body: Vec<String> = serde_json::from_str(&get_body(response).await).unwrap();

        assert_eq!(body, vec!["test.artifact"]);

        // get the artifact file

        let request = Request::builder()
            .uri("/artifacts/test.artifact")
            .body(Body::empty())
            .unwrap();

        let app = super::app(state.clone());

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), 200);

        let body = get_body(response).await;

        assert_eq!(body, "test");
    }

    #[tokio::test]
    async fn test_index() {
        let state = Arc::new(RwLock::new(AppState::default()));
        let app = app(state.clone());
        let request = Request::builder().uri("/").body(Body::empty()).unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), 200);

        let body = get_body(response).await;
        assert!(body.contains("Job"));
    }

    #[tokio::test]
    async fn test_log_id() {
        let state = Arc::new(RwLock::new(AppState::default()));
        let app = app(state.clone());
        let request = Request::builder()
            .uri("/log-id")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), 200);

        let body = get_body(response).await;
        assert_eq!(body, "null");

        if let Ok(mut state) = state.write() {
            state.assignment = Some(Assignment {
                id: "test".to_string(),
                queue_id: 1,
                campaign: "lintian-fixes".to_string(),
                codebase: "foo".to_string(),
                force_build: true,
                branch: janitor::api::worker::Branch {
                    cached_url: None,
                    vcs_type: janitor::vcs::VcsType::Git,
                    url: Some("https://example.com/vcs".parse().unwrap()),
                    subpath: std::path::PathBuf::from(""),
                    additional_colocated_branches: Some(vec![]),
                    default_empty: false,
                },
                resume: None,
                target_repository: janitor::api::worker::TargetRepository {
                    url: "https://example.com/".parse().unwrap(),
                },
                skip_setup_validation: false,
                codemod: janitor::api::worker::Codemod {
                    command: "echo".to_string(),
                    environment: std::collections::HashMap::new(),
                },
                env: std::collections::HashMap::new(),
                build: janitor::api::worker::Build {
                    config: serde_json::Value::Null,
                    environment: None,
                    target: "test".to_string(),
                },
            });
        };

        let request = Request::builder()
            .uri("/log-id")
            .body(Body::empty())
            .unwrap();

        let app = super::app(state.clone());
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), 200);

        let body: String = serde_json::from_str(&get_body(response).await).unwrap();
        assert_eq!(body, "test");
    }

    #[tokio::test]
    async fn test_status_idle() {
        let state = Arc::new(RwLock::new(AppState::default()));
        let app = app(state.clone());
        let request = Request::builder()
            .uri("/status")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), 200);

        let body = get_body(response).await;
        let status: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(status["alive"], true);
        assert_eq!(status["current_run_id"], serde_json::Value::Null);
        assert_eq!(status["status"], "idle");
    }

    #[tokio::test]
    async fn test_status_processing() {
        let state = Arc::new(RwLock::new(AppState::default()));
        
        // Set up an assignment to simulate processing state
        if let Ok(mut state) = state.write() {
            state.assignment = Some(Assignment {
                id: "test-run".to_string(),
                queue_id: 1,
                campaign: "test-campaign".to_string(),
                codebase: "test-codebase".to_string(),
                force_build: false,
                branch: janitor::api::worker::Branch {
                    cached_url: None,
                    vcs_type: janitor::vcs::VcsType::Git,
                    url: Some("https://example.com/vcs".parse().unwrap()),
                    subpath: std::path::PathBuf::from(""),
                    additional_colocated_branches: Some(vec![]),
                    default_empty: false,
                },
                resume: None,
                target_repository: janitor::api::worker::TargetRepository {
                    url: "https://example.com/".parse().unwrap(),
                },
                skip_setup_validation: false,
                codemod: janitor::api::worker::Codemod {
                    command: "echo".to_string(),
                    environment: std::collections::HashMap::new(),
                },
                env: std::collections::HashMap::new(),
                build: janitor::api::worker::Build {
                    config: serde_json::Value::Null,
                    environment: None,
                    target: "test".to_string(),
                },
            });
        }

        let app = app(state.clone());
        let request = Request::builder()
            .uri("/status")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), 200);

        let body = get_body(response).await;
        let status: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(status["alive"], true);
        assert_eq!(status["current_run_id"], "test-run");
        assert_eq!(status["status"], "processing");
    }
}
