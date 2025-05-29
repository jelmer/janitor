use axum::{
    extract::{Path, Query, State},
    response::{Html, Json, Result as AxumResult},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tera::Context;

use crate::app::AppState;
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
    
    // TODO: Fetch actual statistics from database
    context.insert("total_packages", &12345);
    context.insert("active_runs", &42);
    context.insert("recent_runs", &Vec::<Value>::new());
    
    let html = state.templates
        .render("index.html", &context)
        .map_err(|e| {
            tracing::error!("Template rendering error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    Ok(Html(html))
}

pub async fn about(State(state): State<AppState>) -> AxumResult<Html<String>> {
    let context = create_base_context();
    
    let html = state.templates
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
    
    let page = pagination.page.unwrap_or(1);
    let per_page = pagination.per_page.unwrap_or(50);
    
    // TODO: Implement actual package querying from database
    context.insert("packages", &Vec::<Value>::new());
    context.insert("page", &page);
    context.insert("per_page", &per_page);
    context.insert("total_pages", &1);
    context.insert("search_query", &query.search.unwrap_or_default());
    
    let html = state.templates
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
    
    // TODO: Fetch actual package data from database
    context.insert("package_name", &package_name);
    context.insert("package", &json!({
        "name": package_name,
        "description": "Package description here",
        "runs": []
    }));
    
    let html = state.templates
        .render("pkg.html", &context)
        .map_err(|e| {
            tracing::error!("Template rendering error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    Ok(Html(html))
}