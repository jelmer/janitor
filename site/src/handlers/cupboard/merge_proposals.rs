use axum::{
    extract::{Query, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Json, Response},
};
use serde::{Deserialize, Serialize};

use crate::{
    api::{negotiate_content_type, ContentType},
    app::AppState,
    auth::UserContext,
};

use super::{AdminUser, Permission, create_admin_context};

#[derive(Debug, Deserialize, Serialize)]
pub struct MpFilters {
    pub status: Option<String>,
    pub suite: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Merge proposal administration
pub async fn mp_dashboard(
    State(state): State<AppState>,
    user_ctx: UserContext,
    Query(filters): Query<MpFilters>,
    headers: header::HeaderMap,
) -> Response {
    let admin_user = match AdminUser::from_user_context(&user_ctx) {
        Some(admin) => admin,
        None => return StatusCode::FORBIDDEN.into_response(),
    };
    
    if !admin_user.has_permission(&Permission::ViewQueue) {
        return StatusCode::FORBIDDEN.into_response();
    }
    
    let mut context = create_admin_context(&admin_user);
    
    // TODO: Implement MP data fetching
    context.insert("merge_proposals", &Vec::<String>::new());
    context.insert("filters", &filters);
    
    let content_type = negotiate_content_type(&headers, "mp_dashboard");
    
    match content_type {
        ContentType::Json => Json(context.into_json()).into_response(),
        _ => {
            match state.templates.render("cupboard/merge-proposals.html", &context) {
                Ok(html) => Html(html).into_response(),
                Err(e) => {
                    tracing::error!("Template rendering error: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                }
            }
        }
    }
}

/// Bulk MP operations
pub async fn bulk_mp_operation(
    State(_state): State<AppState>,
    user_ctx: UserContext,
    Json(_operation): Json<serde_json::Value>,
) -> Response {
    let admin_user = match AdminUser::from_user_context(&user_ctx) {
        Some(admin) => admin,
        None => return StatusCode::FORBIDDEN.into_response(),
    };
    
    if !admin_user.has_permission(&Permission::ModifyQueue) {
        return StatusCode::FORBIDDEN.into_response();
    }
    
    // TODO: Implement bulk MP operations
    StatusCode::NOT_IMPLEMENTED.into_response()
}