use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum_extra::TypedHeader;
use breezyshim::RevisionId;
use clap::Parser;
use janitor::artifacts::ArtifactManager;
use janitor_differ::{DifferError, DifferResult};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info};

const PRECACHE_RETRIEVE_TIMEOUT: u64 = 300;
const TMP_PREFIX: &str = "janitor-differ";

#[derive(Parser)]
struct Args {
    #[clap(long, default_value = "127.0.0.1")]
    /// The address to listen on
    listen_address: std::net::IpAddr,

    #[clap(long, default_value = "9920")]
    /// The port to listen on
    port: u16,

    #[clap(long, default_value = "janitor.conf")]
    /// The path to the configuration file
    config: PathBuf,

    #[clap(long)]
    /// The path to the cache directory
    cache_path: Option<PathBuf>,

    #[clap(long, default_value = "1500")]
    /// Task memory limit (in MB)
    task_memory_limit: Option<usize>,

    #[clap(long, default_value = "60")]
    /// Task time limit (in seconds)
    task_timeout: Option<usize>,

    #[clap(long, default_value = "diffoscope")]
    diffoscope_command: String,

    #[clap(flatten)]
    logging: janitor::logging::LoggingArgs,
}

// Error handling is now provided by the janitor_differ::DifferError type

struct AppState {
    pool: sqlx::PgPool,
    artifact_manager: Arc<Box<dyn ArtifactManager>>,
    task_memory_limit: Option<usize>,
    task_timeout: Option<usize>,
    diffoscope_cache_path: Option<PathBuf>,
    debdiff_cache_path: Option<PathBuf>,
    diffoscope_command: String,
}

#[cfg(test)]
static_assertions::assert_impl_all!(AppState: Send, Sync);

async fn health() -> &'static str {
    "OK"
}

async fn ready() -> &'static str {
    "OK"
}

#[derive(Debug, sqlx::FromRow)]
struct Run {
    result_code: String,
    build_source: String,
    campaign: String,
    id: String,
    build_version: String,
    main_branch_revision: breezyshim::RevisionId,
}

async fn get_run(conn: &sqlx::PgPool, run_id: &str) -> DifferResult<Option<Run>> {
    let query = sqlx::query_as::<_, Run>(
        r#"SELECT result_code, source AS build_source, suite AS campaign, id, debian_build.version AS build_version, main_branch_revision FROM run LEFT JOIN debian_build ON debian_build.run_id = run.id WHERE id = $1"#)
        .bind(run_id);

    query.fetch_optional(conn).await.map_err(DifferError::Database)
}

async fn get_unchanged_run(
    conn: &sqlx::PgPool,
    codebase: &str,
    main_branch_revision: &RevisionId,
) -> DifferResult<Option<Run>> {
    let query = sqlx::query_as::<_, Run>(
        r#"SELECT result_code, source AS build_source, suite AS campaign, id, debian_build.version AS build_version
FROM
    run
LEFT JOIN
    debian_build ON debian_build.run_id = run.id
WHERE
    revision = $1 AND
    codebase = $2 AND
    result_code = 'success' AND
    run.id = run.change_set
ORDER BY finish_time DESC
"#).bind(main_branch_revision).bind(codebase);

    query.fetch_optional(conn).await.map_err(DifferError::Database)
}

/// Precache the diff between two runs.
async fn precache(
    artifact_manager: Arc<Box<dyn ArtifactManager>>,
    old_id: String,
    new_id: String,
    task_memory_limit: Option<usize>,
    task_timeout: Option<usize>,
    diffoscope_cache_path: Option<PathBuf>,
    debdiff_cache_path: Option<PathBuf>,
    diffoscope_command: Option<String>,
) -> DifferResult<()> {
    let old_dir = tempfile::TempDir::with_prefix(TMP_PREFIX)
        .map_err(|e| DifferError::IoError {
            operation: "create_temp_dir".to_string(),
            path: std::env::temp_dir(),
            source: e,
        })?;
    let new_dir = tempfile::TempDir::with_prefix(TMP_PREFIX)
        .map_err(|e| DifferError::IoError {
            operation: "create_temp_dir".to_string(),
            path: std::env::temp_dir(),
            source: e,
        })?

    let (old_result, new_result) = tokio::join!(
        artifact_manager.retrieve_artifacts(
            &old_id,
            old_dir.path(),
            Some(&janitor_differ::is_binary)
        ),
        artifact_manager.retrieve_artifacts(
            &new_id,
            new_dir.path(),
            Some(&janitor_differ::is_binary)
        ),
    );

    match old_result {
        Ok(()) => {}
        Err(janitor::artifacts::Error::ArtifactsMissing) => {
            return Err(DifferError::ArtifactsMissing {
                run_id: old_id.clone(),
            });
        }
        Err(e) => {
            return Err(DifferError::ArtifactRetrievalFailed {
                run_id: old_id.clone(),
                reason: e.to_string(),
            });
        }
    };

    match new_result {
        Ok(()) => {}
        Err(janitor::artifacts::Error::ArtifactsMissing) => {
            return Err(DifferError::ArtifactsMissing {
                run_id: new_id.clone(),
            });
        }
        Err(e) => {
            return Err(DifferError::ArtifactRetrievalFailed {
                run_id: new_id.clone(),
                reason: e.to_string(),
            });
        }
    };

    let old_binaries = janitor_differ::find_binaries(old_dir.path())?.collect::<Vec<_>>();
    if old_binaries.is_empty() {
        return Err(DifferError::ArtifactsMissing {
            run_id: old_id.clone(),
        });
    }

    let new_binaries = janitor_differ::find_binaries(new_dir.path())?.collect::<Vec<_>>();
    if new_binaries.is_empty() {
        return Err(DifferError::ArtifactsMissing {
            run_id: new_id.clone(),
        });
    }

    let p = if let Some(debdiff_cache_path) = debdiff_cache_path.as_ref() {
        Some(determine_debdiff_cache_path(
            debdiff_cache_path,
            &old_id,
            &new_id,
        )?)
    } else {
        None
    };

    if p.as_ref().and_then(|p| Some(!p.exists())).unwrap_or(false) {
        use std::io::Write;
        let mut f = std::fs::File::create(p.as_ref().unwrap())
            .map_err(|e| DifferError::IoError {
                operation: "create_cache_file".to_string(),
                path: p.as_ref().unwrap().to_path_buf(),
                source: e,
            })?;

        let debdiff_result = janitor::debdiff::run_debdiff(
            old_binaries
                .iter()
                .map(|(_n, p)| p.to_str().unwrap())
                .collect(),
            new_binaries
                .iter()
                .map(|(_n, p)| p.to_str().unwrap())
                .collect(),
        )
        .await
        .map_err(|e| DifferError::DiffCommandError {
            command: "debdiff".to_string(),
            reason: e.to_string(),
        })?;

        f.write_all(debdiff_result.as_slice())
            .map_err(|e| DifferError::IoError {
                operation: "write_cache_file".to_string(),
                path: p.as_ref().unwrap().to_path_buf(),
                source: e,
            })?
        info!(
            old_run_id = old_id,
            new_run_id = new_id,
            "Precached debdiff result for {}/{}",
            old_id,
            new_id,
        );
    }

    let p = if let Some(diffoscope_cache_path) = diffoscope_cache_path.as_ref() {
        Some(determine_diffoscope_cache_path(
            diffoscope_cache_path,
            &old_id,
            &new_id,
        )?)
    } else {
        None
    };

    if p.as_ref().and_then(|p| Some(!p.exists())).unwrap_or(false) {
        let diffoscope_diff = janitor_differ::diffoscope::run_diffoscope(
            old_binaries
                .iter()
                .map(|(n, p)| (n.to_str().unwrap(), p.to_str().unwrap()))
                .collect::<Vec<_>>()
                .as_slice(),
            new_binaries
                .iter()
                .map(|(n, p)| (n.to_str().unwrap(), p.to_str().unwrap()))
                .collect::<Vec<_>>()
                .as_slice(),
            task_timeout.map(|t| t as f64),
            task_memory_limit.map(|m| m as u64),
            diffoscope_command.as_deref(),
        )
        .await
        .map_err(|e| DifferError::DiffCommandError {
            command: "diffoscope".to_string(),
            reason: e.to_string(),
        })?;

        let f = std::fs::File::create(p.unwrap())
            .map_err(|e| DifferError::IoError {
                operation: "create_cache_file".to_string(),
                path: p.unwrap().to_path_buf(),
                source: e,
            })?;

        serde_json::to_writer(f, &diffoscope_diff)
            .map_err(|e| DifferError::JsonError(e))?
        info!(
            old_run_id = old_id,
            new_run_id = new_id,
            "Precached diffoscope result for {}/{}",
            old_id,
            new_id
        );
    }

    Ok(())
}

async fn handle_precache(
    Path((old_id, new_id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let (old_run, new_run) = match get_run_pair(&state.pool, &old_id, &new_id).await {
        Ok(r) => r,
        Err(e) => return e.into_response(),
    };

    tokio::spawn(precache(
        state.artifact_manager.clone(),
        old_run.id.clone(),
        new_run.id.clone(),
        state.task_memory_limit,
        state.task_timeout,
        state.diffoscope_cache_path.clone(),
        state.debdiff_cache_path.clone(),
        Some(state.diffoscope_command.clone()),
    ));

    (StatusCode::ACCEPTED, "Pre-caching started").into_response()
}

async fn get_run_pair(
    pool: &sqlx::PgPool,
    old_id: &str,
    new_id: &str,
) -> DifferResult<(Run, Run)> {
    let new_run = get_run(pool, new_id).await?;
    let old_run = get_run(pool, old_id).await?;

    let old_run = match old_run {
        Some(run) if run.result_code == "success" => run,
        Some(run) => {
            return Err(DifferError::RunNotSuccessful {
                run_id: old_id.to_string(),
                status: run.result_code,
            });
        }
        None => {
            return Err(DifferError::RunNotFound {
                run_id: old_id.to_string(),
            });
        }
    };

    let new_run = match new_run {
        Some(run) if run.result_code == "success" => run,
        Some(run) => {
            return Err(DifferError::RunNotSuccessful {
                run_id: new_id.to_string(),
                status: run.result_code,
            });
        }
        None => {
            return Err(DifferError::RunNotFound {
                run_id: new_id.to_string(),
            });
        }
    };

    Ok((old_run, new_run))
}

#[derive(Debug, serde::Deserialize)]
struct DiffoscopeQuery {
    #[serde(default)]
    filter_boring: bool,

    #[serde(default)]
    css_url: Option<String>,
}

/// Parse and negotiate content type from Accept header
fn negotiate_content_type(headers: &axum::http::HeaderMap) -> DifferResult<mime::Mime> {
    use std::str::FromStr;
    
    let accept_str = headers
        .get("Accept")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json");
    
    let accept = accept_header::Accept::from_str(accept_str)
        .map_err(|_| DifferError::AcceptHeaderError(accept_str.to_string()))?;

    let available = vec![
        mime::Mime::from_str("application/json").unwrap(),
        mime::Mime::from_str("text/html").unwrap(),
        mime::Mime::from_str("text/plain").unwrap(),
        mime::Mime::from_str("text/x-diff").unwrap(),
        mime::Mime::from_str("text/markdown").unwrap(),
    ];

    accept.negotiate(&available)
        .map_err(|_| DifferError::ContentNegotiationFailed {
            available: available.iter().map(|m| m.to_string()).collect(),
            requested: accept_str.to_string(),
        })
}

async fn handle_diffoscope(
    Path((old_id, new_id)): Path<(String, String)>,
    Query(query): Query<DiffoscopeQuery>,
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    // Delegate to inner function for proper error handling
    match handle_diffoscope_inner(old_id, new_id, query, state, headers).await {
        Ok(response) => response,
        Err(e) => e.into_response(),
    }
}

async fn handle_diffoscope_inner(
    old_id: String,
    new_id: String,
    query: DiffoscopeQuery,
    state: Arc<AppState>,
    headers: axum::http::HeaderMap,
) -> DifferResult<Response> {
    let best = negotiate_content_type(&headers)?;
    let (old_run, new_run) = get_run_pair(&state.pool, &old_id, &new_id).await?;

    let cache_path = match state.diffoscope_cache_path.as_ref() {
        Some(p) => Some(determine_diffoscope_cache_path(p, &old_run.id, &new_run.id)?),
        None => None,
    };

    let diffoscope_diff = if let Some(ref cache_path) = cache_path {
        if cache_path.exists() {
            let f = std::fs::File::open(cache_path)
                .map_err(|e| DifferError::IoError {
                    operation: "open_cache_file".to_string(),
                    path: cache_path.clone(),
                    source: e,
                })?;
            let diff: janitor_differ::diffoscope::DiffoscopeOutput =
                serde_json::from_reader(f)
                    .map_err(|e| DifferError::JsonError(e))?;
            Some(diff)
        } else {
            None
        }
    } else {
        None
    };

    let mut diffoscope_diff = if let Some(diffoscope_diff) = diffoscope_diff {
        diffoscope_diff
    } else {
        info!(
            old_run_id = old_run.id,
            new_run_id = new_run.id,
            "Generating diffoscope between {} ({}/{}/{}) and {} ({}/{}/{})",
            old_run.id,
            old_run.build_source,
            old_run.build_version,
            old_run.campaign,
            new_run.id,
            new_run.build_source,
            new_run.build_version,
            new_run.campaign,
        );

        let old_dir = tempfile::TempDir::with_prefix(TMP_PREFIX)
            .map_err(|e| DifferError::IoError {
                operation: "create_temp_dir".to_string(),
                path: std::env::temp_dir(),
                source: e,
            })?;
        let new_dir = tempfile::TempDir::with_prefix(TMP_PREFIX)
            .map_err(|e| DifferError::IoError {
                operation: "create_temp_dir".to_string(),
                path: std::env::temp_dir(),
                source: e,
            })?;

        let old_clone_id = old_run.id.clone();
        let new_clone_id = new_run.id.clone();

        let old_artifact_manager = state.artifact_manager.clone();
        let new_artifact_manager = state.artifact_manager.clone();

        let (old_result, new_result) = tokio::join! {
            old_artifact_manager.retrieve_artifacts(
                &old_clone_id, old_dir.path(), Some(&janitor_differ::is_binary)
            ),
            new_artifact_manager.retrieve_artifacts(
                &new_clone_id, new_dir.path(), Some(&janitor_differ::is_binary)
            )
        };

        match old_result {
            Ok(()) => {}
            Err(janitor::artifacts::Error::ArtifactsMissing) => {
                return Err(DifferError::ArtifactsMissing {
                    run_id: old_run.id.clone(),
                });
            }
            Err(e) => {
                return Err(DifferError::ArtifactRetrievalFailed {
                    run_id: old_run.id.clone(),
                    reason: e.to_string(),
                });
            }
        }

        match new_result {
            Ok(()) => {}
            Err(janitor::artifacts::Error::ArtifactsMissing) => {
                return Err(DifferError::ArtifactsMissing {
                    run_id: new_run.id.clone(),
                });
            }
            Err(e) => {
                return Err(DifferError::ArtifactRetrievalFailed {
                    run_id: new_run.id.clone(),
                    reason: e.to_string(),
                });
            }
        }

        let old_binaries = janitor_differ::find_binaries(old_dir.path())?.collect::<Vec<_>>();
        if old_binaries.is_empty() {
            return Err(DifferError::ArtifactsMissing {
                run_id: old_run.id.clone(),
            });
        }

        let new_binaries = janitor_differ::find_binaries(new_dir.path())?.collect::<Vec<_>>();
        if new_binaries.is_empty() {
            return Err(DifferError::ArtifactsMissing {
                run_id: new_run.id.clone(),
            });
        }

        let diffoscope_diff = janitor_differ::diffoscope::run_diffoscope(
            old_binaries
                .iter()
                .map(|(n, p)| (n.to_str().unwrap(), p.to_str().unwrap()))
                .collect::<Vec<_>>()
                .as_slice(),
            new_binaries
                .iter()
                .map(|(n, p)| (n.to_str().unwrap(), p.to_str().unwrap()))
                .collect::<Vec<_>>()
                .as_slice(),
            state.task_timeout.map(|t| t as f64),
            state.task_memory_limit.map(|m| m as u64),
            Some(state.diffoscope_command.as_str()),
        )
        .await
        .map_err(|e| DifferError::DiffCommandError {
            command: "diffoscope".to_string(),
            reason: e.to_string(),
        })?;

        if let Some(cache_path) = cache_path.as_ref() {
            let f = std::fs::File::create(cache_path)
                .map_err(|e| DifferError::IoError {
                    operation: "create_cache_file".to_string(),
                    path: cache_path.clone(),
                    source: e,
                })?;
            serde_json::to_writer(f, &diffoscope_diff)
                .map_err(|e| DifferError::JsonError(e))?;
        }

        diffoscope_diff
    };

    diffoscope_diff.source1 = format!(
        "{} version {} ({})",
        old_run.build_source, old_run.build_version, old_run.campaign
    )
    .into();
    diffoscope_diff.source2 = format!(
        "{} version {} ({})",
        new_run.build_source, new_run.build_version, new_run.campaign
    )
    .into();

    janitor_differ::diffoscope::filter_irrelevant(&mut diffoscope_diff);

    let mut title = format!(
        "diffoscope for {} applied to {}",
        new_run.campaign, new_run.build_source
    );

    if query.filter_boring {
        janitor_differ::diffoscope::filter_boring(
            &mut diffoscope_diff,
            &old_run.build_version,
            &new_run.build_version,
            &old_run.campaign,
            &new_run.campaign,
        );
        title.push_str(" (filtered)");
    }

    let formatted = janitor_differ::diffoscope::format_diffoscope(
        &diffoscope_diff,
        best.essence_str(),
        &title,
        query.css_url.as_deref(),
    )
    .map_err(|e| DifferError::DiffCommandError {
        command: "format_diffoscope".to_string(),
        reason: e.to_string(),
    })?;

    Ok((StatusCode::OK, formatted).into_response())
}

#[derive(Debug, serde::Deserialize)]
struct DebdiffQuery {
    #[serde(default)]
    filter_boring: bool,
}

async fn handle_debdiff(
    Path((old_id, new_id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
    Query(query): Query<DebdiffQuery>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    // Delegate to inner function for proper error handling
    match handle_debdiff_inner(old_id, new_id, query, state, headers).await {
        Ok(response) => response,
        Err(e) => e.into_response(),
    }
}

async fn handle_debdiff_inner(
    old_id: String,
    new_id: String,
    query: DebdiffQuery,
    state: Arc<AppState>,
    headers: axum::http::HeaderMap,
) -> DifferResult<Response> {
    let (old_run, new_run) = get_run_pair(&state.pool, &old_id, &new_id).await?;

    let cache_path = match state.debdiff_cache_path.as_ref() {
        Some(p) => Some(determine_debdiff_cache_path(p, &old_run.id, &new_run.id)?),
        None => None,
    };

    let debdiff = if let Some(cache_path) = cache_path.as_ref() {
        std::fs::read_to_string(cache_path).ok()
    } else {
        None
    };

    let mut debdiff = if let Some(debdiff) = debdiff {
        debdiff
    } else {
        info!(
            "Generating debdiff between {} ({}/{}/{}) and {} ({}/{}/{})",
            old_run.id,
            old_run.build_source,
            old_run.build_version,
            old_run.campaign,
            new_run.id,
            new_run.build_source,
            new_run.build_version,
            new_run.campaign,
        );

        let old_dir = tempfile::TempDir::with_prefix(TMP_PREFIX)
            .map_err(|e| DifferError::IoError {
                operation: "create_temp_dir".to_string(),
                path: std::env::temp_dir(),
                source: e,
            })?;
        let new_dir = tempfile::TempDir::with_prefix(TMP_PREFIX)
            .map_err(|e| DifferError::IoError {
                operation: "create_temp_dir".to_string(),
                path: std::env::temp_dir(),
                source: e,
            })?;

        let (old_result, new_result) = tokio::join!(
            state.artifact_manager.retrieve_artifacts(
                &old_run.id,
                old_dir.path(),
                Some(&janitor_differ::is_binary)
            ),
            state.artifact_manager.retrieve_artifacts(
                &new_run.id,
                new_dir.path(),
                Some(&janitor_differ::is_binary)
            )
        );

        match old_result {
            Ok(()) => {}
            Err(janitor::artifacts::Error::ArtifactsMissing) => {
                return Err(DifferError::ArtifactsMissing {
                    run_id: old_run.id.clone(),
                });
            }
            Err(e) => {
                return Err(DifferError::ArtifactRetrievalFailed {
                    run_id: old_run.id.clone(),
                    reason: e.to_string(),
                });
            }
        }

        match new_result {
            Ok(()) => {}
            Err(janitor::artifacts::Error::ArtifactsMissing) => {
                return Err(DifferError::ArtifactsMissing {
                    run_id: new_run.id.clone(),
                });
            }
            Err(e) => {
                return Err(DifferError::ArtifactRetrievalFailed {
                    run_id: new_run.id.clone(),
                    reason: e.to_string(),
                });
            }
        }

        let old_binaries = janitor_differ::find_binaries(old_dir.path())?.collect::<Vec<_>>();
        if old_binaries.is_empty() {
            return Err(DifferError::ArtifactsMissing {
                run_id: old_run.id.clone(),
            });
        }

        let new_binaries = janitor_differ::find_binaries(new_dir.path())?.collect::<Vec<_>>();
        if new_binaries.is_empty() {
            return Err(DifferError::ArtifactsMissing {
                run_id: new_run.id.clone(),
            });
        }

        let debdiff = janitor::debdiff::run_debdiff(
            old_binaries
                .iter()
                .map(|(_n, p)| p.to_str().unwrap())
                .collect(),
            new_binaries
                .iter()
                .map(|(_n, p)| p.to_str().unwrap())
                .collect(),
        )
        .await
        .map_err(|e| DifferError::DiffCommandError {
            command: "debdiff".to_string(),
            reason: e.to_string(),
        })?;

        if let Some(cache_path) = cache_path.as_ref() {
            std::fs::write(cache_path, &debdiff)
                .map_err(|e| DifferError::IoError {
                    operation: "write_cache_file".to_string(),
                    path: cache_path.clone(),
                    source: e,
                })?;
        }
        String::from_utf8(debdiff)
            .map_err(|e| DifferError::DiffCommandError {
                command: "debdiff".to_string(),
                reason: format!("Invalid UTF-8 output: {}", e),
            })?
    };

    if query.filter_boring {
        debdiff = janitor::debdiff::filter_boring(
            &debdiff,
            &old_run.build_version,
            &new_run.build_version,
        );
    }

    let best = negotiate_content_type(&headers)?;

    let response_content = match best.essence_str() {
        "text/x-diff" | "text/plain" => debdiff,
        "text/markdown" => janitor::debdiff::markdownify_debdiff(&debdiff),
        "text/html" => janitor::debdiff::htmlize_debdiff(&debdiff),
        _ => return Err(DifferError::ContentNegotiationFailed {
            available: vec!["text/plain".to_string(), "text/html".to_string(), "text/markdown".to_string()],
            requested: best.to_string(),
        }),
    };

    Ok((StatusCode::OK, response_content).into_response())
}

fn determine_diffoscope_cache_path(
    cache_path: &std::path::Path,
    old_id: &str,
    new_id: &str,
) -> DifferResult<PathBuf> {
    let base_path = cache_path.join("diffoscope");
    if !base_path.exists() {
        std::fs::create_dir_all(&base_path)
            .map_err(|e| DifferError::IoError {
                operation: "create_cache_dir".to_string(),
                path: base_path.clone(),
                source: e,
            })?
    }
    Ok(base_path.join(format!("{}_{}.json", old_id, new_id)))
}

fn determine_debdiff_cache_path(
    cache_path: &std::path::Path,
    old_id: &str,
    new_id: &str,
) -> DifferResult<PathBuf> {
    let base_path = cache_path.join("debdiff");
    if !base_path.exists() {
        std::fs::create_dir_all(&base_path)
            .map_err(|e| DifferError::IoError {
                operation: "create_cache_dir".to_string(),
                path: base_path.clone(),
                source: e,
            })?
    }
    Ok(base_path.join(format!("{}_{}", old_id, new_id)))
}

async fn listen_to_runner(redis: redis::aio::ConnectionManager, db: sqlx::PgPool) {
    todo!();
}

#[tokio::main]
pub async fn main() -> Result<(), i8> {
    let args = Args::parse();

    args.logging.init();

    let config = Box::new(janitor::config::read_file(&args.config).map_err(|e| {
        error!("Failed to read config: {}", e);
        1
    })?);

    let config: &'static _ = Box::leak(config);

    let db = janitor::state::create_pool(config).await.map_err(|e| {
        error!("Failed to create database pool: {}", e);
        1
    })?;

    let artifact_manager =
        janitor::artifacts::get_artifact_manager(&config.artifact_location.clone().unwrap())
            .await
            .map_err(|e| {
                error!("Failed to create artifact manager: {}", e);
                1
            })?;

    if let Some(ref cache_path) = args.cache_path {
        if !cache_path.exists() {
            std::fs::create_dir_all(cache_path).map_err(|e| {
                error!("Failed to create cache directory: {}", e);
                1
            })?;
        }
    }

    let state = Arc::new(AppState {
        pool: db.clone(),
        artifact_manager: Arc::new(artifact_manager),
        task_memory_limit: args.task_memory_limit,
        task_timeout: args.task_timeout,
        diffoscope_command: args.diffoscope_command,
        diffoscope_cache_path: args.cache_path.as_ref().map(|p| p.join("diffoscope")),
        debdiff_cache_path: args.cache_path.as_ref().map(|p| p.join("debdiff")),
    });

    let app = axum::Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/precache/:old_id/:new_id", post(handle_precache))
        .route("/diffoscope/:old_id/:new_id", get(handle_diffoscope))
        .route("/debdiff/:old_id/:new_id", get(handle_debdiff))
        .with_state(state);

    // run it
    let addr = std::net::SocketAddr::new(args.listen_address, args.port);
    info!("listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| {
        error!("Failed to bind to address: {}", e);
        1
    })?;

    if let Some(redis_location) = config.redis_location.as_ref() {
        let client = redis::Client::open(redis_location.to_string()).map_err(|e| {
            error!("Failed to create redis client: {}", e);
            1
        })?;

        let connman = redis::aio::ConnectionManager::new(client)
            .await
            .map_err(|e| {
                error!("Failed to create redis async connection: {}", e);
                1
            })?;

        tokio::spawn(listen_to_runner(connman, db));
    }

    axum::serve(listener, app).await.unwrap();

    Ok(())
}
