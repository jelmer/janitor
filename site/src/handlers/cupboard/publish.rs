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
pub struct PublishFilters {
    pub status: Option<String>,
    pub suite: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Publishing dashboard
pub async fn publish_dashboard(
    State(state): State<AppState>,
    user_ctx: UserContext,
    Query(filters): Query<PublishFilters>,
    headers: header::HeaderMap,
) -> Response {
    let admin_user = match AdminUser::from_user_context(&user_ctx) {
        Some(admin) => admin,
        None => return StatusCode::FORBIDDEN.into_response(),
    };
    
    if !admin_user.has_permission(&Permission::ViewPublishQueue) {
        return StatusCode::FORBIDDEN.into_response();
    }
    
    let mut context = create_admin_context(&admin_user);
    
    // TODO: Implement publish data fetching
    context.insert("publish_queue", &Vec::<String>::new());
    context.insert("filters", &filters);
    
    let content_type = negotiate_content_type(&headers, "publish_dashboard");
    
    match content_type {
        ContentType::Json => Json(context.into_json()).into_response(),
        _ => {
            match state.templates.render("cupboard/publish.html", &context) {
                Ok(html) => Html(html).into_response(),
                Err(e) => {
                    tracing::error!("Template rendering error: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                }
            }
        }
    }
}

/// Emergency publish controls
pub async fn emergency_publish_stop(
    State(_state): State<AppState>,
    user_ctx: UserContext,
    Json(_params): Json<serde_json::Value>,
) -> Response {
    let admin_user = match AdminUser::from_user_context(&user_ctx) {
        Some(admin) => admin,
        None => return StatusCode::FORBIDDEN.into_response(),
    };
    
    if !admin_user.has_permission(&Permission::EmergencyPublishControls) {
        return StatusCode::FORBIDDEN.into_response();
    }
    
    // TODO: Implement emergency publish stop
    StatusCode::NOT_IMPLEMENTED.into_response()
}