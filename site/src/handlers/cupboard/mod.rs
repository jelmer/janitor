use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Json, Response},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tera::Context;

use crate::{
    api::{negotiate_content_type, ContentType},
    app::AppState,
    auth::{require_admin, require_qa_reviewer, UserContext},
    database::DatabaseError,
    templates::create_base_context,
};

pub mod api;
pub mod merge_proposals;
pub mod publish;
pub mod queue;
pub mod review;

// Common admin types and utilities

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum AdminRole {
    Admin,      // Full administrative access
    QaReviewer, // Review and quality assurance access
    Operator,   // Limited operational access
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, serde::Serialize)]
pub enum Permission {
    // Queue management
    ViewQueue,
    ModifyQueue,
    BulkQueueOperations,

    // Review system
    ViewReviews,
    BulkReviewActions,
    ManageReviewers,

    // Publishing
    ViewPublishQueue,
    ModifyPublishSettings,
    EmergencyPublishControls,

    // Merge proposals
    ViewMergeProposals,
    BulkMpOperations,
    ManageForgeIntegration,

    // System administration
    ViewSystemMetrics,
    ModifySystemSettings,
    ManageUsers,
}

#[derive(Debug, Clone)]
pub struct AdminUser {
    pub user: crate::auth::User,
    pub roles: Vec<AdminRole>,
    pub permissions: std::collections::HashSet<Permission>,
}

impl AdminUser {
    pub fn from_user_context(user_ctx: &UserContext) -> Option<Self> {
        if !user_ctx.is_admin() && !user_ctx.is_qa_reviewer() {
            return None;
        }

        let mut roles = Vec::new();
        let mut permissions = std::collections::HashSet::new();

        if user_ctx.is_admin() {
            roles.push(AdminRole::Admin);
            // Admin has all permissions
            permissions.insert(Permission::ViewQueue);
            permissions.insert(Permission::ModifyQueue);
            permissions.insert(Permission::BulkQueueOperations);
            permissions.insert(Permission::ViewReviews);
            permissions.insert(Permission::BulkReviewActions);
            permissions.insert(Permission::ManageReviewers);
            permissions.insert(Permission::ViewPublishQueue);
            permissions.insert(Permission::ModifyPublishSettings);
            permissions.insert(Permission::EmergencyPublishControls);
            permissions.insert(Permission::ViewMergeProposals);
            permissions.insert(Permission::BulkMpOperations);
            permissions.insert(Permission::ManageForgeIntegration);
            permissions.insert(Permission::ViewSystemMetrics);
            permissions.insert(Permission::ModifySystemSettings);
            permissions.insert(Permission::ManageUsers);
        }

        if user_ctx.is_qa_reviewer() {
            roles.push(AdminRole::QaReviewer);
            // QA Reviewer has limited permissions
            permissions.insert(Permission::ViewQueue);
            permissions.insert(Permission::ViewReviews);
            permissions.insert(Permission::BulkReviewActions);
            permissions.insert(Permission::ViewPublishQueue);
            permissions.insert(Permission::ViewMergeProposals);
            permissions.insert(Permission::ViewSystemMetrics);
        }

        Some(AdminUser {
            user: user_ctx.user().clone(),
            roles,
            permissions,
        })
    }

    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.permissions.contains(permission)
    }

    pub fn is_admin(&self) -> bool {
        self.roles.contains(&AdminRole::Admin)
    }

    pub fn is_qa_reviewer(&self) -> bool {
        self.roles.contains(&AdminRole::QaReviewer)
    }
}

#[derive(Debug, Serialize)]
pub struct AdminDashboardStats {
    pub total_runs: i64,
    pub active_runs: i64,
    pub queued_items: i64,
    pub pending_reviews: i64,
    pub pending_publishes: i64,
    pub recent_failures: i64,
    pub system_health: String,
    pub worker_count: i64,
    pub workers_active: i64,
    pub workers_idle: i64,
}

#[derive(Debug, Serialize)]
pub struct AdminAuditEvent {
    pub timestamp: DateTime<Utc>,
    pub admin_user: String,
    pub action: String,
    pub target: Option<String>,
    pub details: serde_json::Value,
    pub ip_address: String,
    pub user_agent: String,
}

/// Main admin dashboard
pub async fn admin_dashboard(
    State(state): State<AppState>,
    user_ctx: UserContext,
    headers: header::HeaderMap,
) -> Response {
    // Verify admin access
    let admin_user = match AdminUser::from_user_context(&user_ctx) {
        Some(admin) => admin,
        None => return StatusCode::FORBIDDEN.into_response(),
    };

    let mut context = create_base_context();
    context.insert("user", &admin_user.user);
    context.insert("is_admin", &admin_user.is_admin());
    context.insert("is_qa_reviewer", &admin_user.is_qa_reviewer());
    context.insert("admin_permissions", &admin_user.permissions);

    // Fetch dashboard statistics
    match fetch_admin_dashboard_stats(&state).await {
        Ok(stats) => {
            context.insert("stats", &stats);
        }
        Err(e) => {
            tracing::error!("Failed to fetch admin dashboard stats: {}", e);
            // Use default/empty stats
            let empty_stats = AdminDashboardStats {
                total_runs: 0,
                active_runs: 0,
                queued_items: 0,
                pending_reviews: 0,
                pending_publishes: 0,
                recent_failures: 0,
                system_health: "unknown".to_string(),
                worker_count: 0,
                workers_active: 0,
                workers_idle: 0,
            };
            context.insert("stats", &empty_stats);
        }
    }

    // Content negotiation
    let content_type = negotiate_content_type(&headers, "admin_dashboard");

    match content_type {
        ContentType::Json => Json(context.into_json()).into_response(),
        _ => match state.templates.render("cupboard/dashboard.html", &context) {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("Template rendering error: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        },
    }
}

/// Admin sidebar navigation
pub async fn admin_sidebar(State(state): State<AppState>, user_ctx: UserContext) -> Response {
    let admin_user = match AdminUser::from_user_context(&user_ctx) {
        Some(admin) => admin,
        None => return StatusCode::FORBIDDEN.into_response(),
    };

    let mut context = create_base_context();
    context.insert("user", &admin_user.user);
    context.insert("is_admin", &admin_user.is_admin());
    context.insert("is_qa_reviewer", &admin_user.is_qa_reviewer());
    context.insert("admin_permissions", &admin_user.permissions);

    match state.templates.render("cupboard/sidebar.html", &context) {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("Template rendering error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// System status page
pub async fn system_status(
    State(state): State<AppState>,
    user_ctx: UserContext,
    headers: header::HeaderMap,
) -> Response {
    let admin_user = match AdminUser::from_user_context(&user_ctx) {
        Some(admin) => admin,
        None => return StatusCode::FORBIDDEN.into_response(),
    };

    if !admin_user.has_permission(&Permission::ViewSystemMetrics) {
        return StatusCode::FORBIDDEN.into_response();
    }

    let mut context = create_base_context();
    context.insert("user", &admin_user.user);
    context.insert("is_admin", &admin_user.is_admin());
    context.insert("is_qa_reviewer", &admin_user.is_qa_reviewer());

    // Fetch system status information
    match fetch_system_status(&state).await {
        Ok(status) => {
            context.insert("system_status", &status);
        }
        Err(e) => {
            tracing::error!("Failed to fetch system status: {}", e);
            context.insert(
                "error_message",
                &format!("Failed to load system status: {}", e),
            );
        }
    }

    let content_type = negotiate_content_type(&headers, "system_status");

    match content_type {
        ContentType::Json => Json(context.into_json()).into_response(),
        _ => {
            match state
                .templates
                .render("cupboard/system-status.html", &context)
            {
                Ok(html) => Html(html).into_response(),
                Err(e) => {
                    tracing::error!("Template rendering error: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                }
            }
        }
    }
}

// Helper functions

async fn fetch_admin_dashboard_stats(state: &AppState) -> anyhow::Result<AdminDashboardStats> {
    // TODO: Implement comprehensive dashboard statistics gathering
    // This would integrate with runner service, publish service, and database

    // For now, return basic stats from database
    let db_stats = state.database.get_stats().await.unwrap_or_default();

    Ok(AdminDashboardStats {
        total_runs: db_stats.get("total_runs").copied().unwrap_or(0),
        active_runs: db_stats.get("active_runs").copied().unwrap_or(0),
        queued_items: db_stats.get("queue_size").copied().unwrap_or(0),
        pending_reviews: 0,   // TODO: Implement review counting
        pending_publishes: 0, // TODO: Implement publish queue counting
        recent_failures: 0,   // TODO: Implement failure counting
        system_health: "operational".to_string(), // TODO: Implement health checks
        worker_count: 0,      // TODO: Integrate with runner service
        workers_active: 0,
        workers_idle: 0,
    })
}

async fn fetch_system_status(state: &AppState) -> anyhow::Result<serde_json::Value> {
    // TODO: Implement comprehensive system status checks
    // This would include:
    // - Database connectivity and performance
    // - Runner service health
    // - Publisher service health
    // - Redis connectivity
    // - VCS store health
    // - Disk space and system resources

    Ok(serde_json::json!({
        "database": "operational",
        "runner": "unknown",
        "publisher": "unknown",
        "redis": "unknown",
        "vcs_stores": "unknown",
        "timestamp": Utc::now(),
    }))
}

/// Create admin context with user permissions
pub fn create_admin_context(admin_user: &AdminUser) -> Context {
    let mut context = create_base_context();
    context.insert("user", &admin_user.user);
    context.insert("is_admin", &admin_user.is_admin());
    context.insert("is_qa_reviewer", &admin_user.is_qa_reviewer());
    context.insert("admin_permissions", &admin_user.permissions);
    context.insert("admin_roles", &admin_user.roles);
    context
}

/// Log admin action for audit trail
pub async fn log_admin_action(
    state: &AppState,
    admin_user: &AdminUser,
    action: &str,
    target: Option<&str>,
    details: serde_json::Value,
    ip_address: &str,
    user_agent: &str,
) {
    let audit_event = AdminAuditEvent {
        timestamp: Utc::now(),
        admin_user: admin_user
            .user
            .name
            .clone()
            .unwrap_or_else(|| admin_user.user.email.clone()),
        action: action.to_string(),
        target: target.map(|s| s.to_string()),
        details,
        ip_address: ip_address.to_string(),
        user_agent: user_agent.to_string(),
    };

    // TODO: Store audit event in database or dedicated audit log
    tracing::info!("Admin action: {:?}", audit_event);
}
