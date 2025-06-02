pub mod cupboard;
pub mod pkg;
pub mod simple;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, Result as AxumResult},
};
use serde::Deserialize;
use tera::Context;

use crate::app::AppState;
use crate::database::DatabaseError;
use crate::templates::create_base_context;

#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct PackageQuery {
    pub search: Option<String>,
    pub suite: Option<String>,
    pub campaign: Option<String>,
}

pub async fn index(State(state): State<AppState>) -> AxumResult<Html<String>> {
    let mut context = create_base_context();

    // Fetch actual statistics from database
    match state.database.get_stats().await {
        Ok(stats) => {
            context.insert(
                "total_packages",
                &stats.get("total_codebases").unwrap_or(&0),
            );
            context.insert("active_runs", &stats.get("active_runs").unwrap_or(&0));
            context.insert("queue_size", &stats.get("queue_size").unwrap_or(&0));
            context.insert(
                "recent_successful_runs",
                &stats.get("recent_successful_runs").unwrap_or(&0),
            );
        }
        Err(e) => {
            tracing::error!("Failed to fetch stats: {}", e);
            // Use fallback values
            context.insert("total_packages", &0);
            context.insert("active_runs", &0);
            context.insert("queue_size", &0);
            context.insert("recent_successful_runs", &0);
        }
    }

    let html = state
        .templates
        .render("index.html", &context)
        .map_err(|e| {
            tracing::error!("Template rendering error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Html(html))
}

pub async fn about(State(state): State<AppState>) -> AxumResult<Html<String>> {
    let context = create_base_context();

    let html = state
        .templates
        .render("about.html", &context)
        .map_err(|e| {
            tracing::error!("Template rendering error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Html(html))
}

pub async fn package_list(
    State(state): State<AppState>,
    Query(query): Query<PackageQuery>,
    Query(pagination): Query<PaginationQuery>,
) -> AxumResult<Html<String>> {
    let mut context = create_base_context();

    let page = pagination.page.unwrap_or(1) as i64;
    let per_page = pagination.per_page.unwrap_or(50) as i64;
    let offset = (page - 1) * per_page;

    // Fetch packages from database
    match state
        .database
        .get_codebases(Some(per_page), Some(offset), query.search.as_deref())
        .await
    {
        Ok(packages) => {
            context.insert("packages", &packages);

            // Get total count for pagination
            match state
                .database
                .count_codebases(query.search.as_deref())
                .await
            {
                Ok(total_count) => {
                    let total_pages = (total_count + per_page - 1) / per_page;
                    context.insert("total_pages", &total_pages);
                    context.insert("total_count", &total_count);
                }
                Err(e) => {
                    tracing::error!("Failed to count codebases: {}", e);
                    context.insert("total_pages", &1);
                    context.insert("total_count", &0);
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to fetch codebases: {}", e);
            return Err(e.to_status_code().into());
        }
    }

    context.insert("page", &page);
    context.insert("per_page", &per_page);
    context.insert("search_query", &query.search.unwrap_or_default());

    let html = state
        .templates
        .render("pkg-list.html", &context)
        .map_err(|e| {
            tracing::error!("Template rendering error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Html(html))
}

pub async fn package_detail(
    State(state): State<AppState>,
    Path(package_name): Path<String>,
) -> AxumResult<Html<String>> {
    let mut context = create_base_context();

    // Fetch package data from database
    let codebase = match state.database.get_codebase(&package_name).await {
        Ok(codebase) => codebase,
        Err(DatabaseError::NotFound(_)) => {
            return Err(StatusCode::NOT_FOUND.into());
        }
        Err(e) => {
            tracing::error!("Failed to fetch codebase {}: {}", package_name, e);
            return Err(e.to_status_code().into());
        }
    };

    // Fetch recent runs for this codebase
    let runs = match state
        .database
        .get_runs_for_codebase(&package_name, Some(20), Some(0))
        .await
    {
        Ok(runs) => runs,
        Err(e) => {
            tracing::error!("Failed to fetch runs for {}: {}", package_name, e);
            Vec::new() // Continue with empty runs list
        }
    };

    context.insert("package_name", &package_name);
    context.insert("codebase", &codebase);
    context.insert("runs", &runs);

    let html = state.templates.render("pkg.html", &context).map_err(|e| {
        tracing::error!("Template rendering error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Html(html))
}
