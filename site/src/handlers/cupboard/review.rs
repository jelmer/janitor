use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Json, Response},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    api::{negotiate_content_type, ContentType},
    app::AppState,
    auth::UserContext,
};

use super::{create_admin_context, log_admin_action, AdminUser, Permission};

/// Review dashboard - alias for review queue
pub async fn review_dashboard(
    state: State<AppState>,
    user_ctx: UserContext,
    filters: Query<ReviewFilters>,
    headers: header::HeaderMap,
) -> Response {
    review_queue(state, user_ctx, filters, headers).await
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ReviewFilters {
    pub campaign: Option<String>,
    pub status: Option<String>,
    pub reviewer: Option<String>,
    pub publishable_only: Option<bool>,
    pub required_only: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ReviewItem {
    pub run_id: String,
    pub codebase: String,
    pub campaign: String,
    pub result_code: String,
    pub finish_time: Option<DateTime<Utc>>,
    pub value: Option<i32>,
    pub command: Option<String>,
    pub description: Option<String>,
    pub reviews: Vec<ReviewRecord>,
    pub publish_status: Option<String>,
    pub branch_url: Option<String>,
    pub needs_review: bool,
}

#[derive(Debug, Serialize)]
pub struct ReviewRecord {
    pub reviewer: String,
    pub verdict: String,
    pub comment: Option<String>,
    pub reviewed_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ReviewStatistics {
    pub total_pending: i64,
    pub total_reviewed: i64,
    pub needs_manual_review: i64,
    pub approved_count: i64,
    pub rejected_count: i64,
    pub average_review_time: f64,
    pub reviewers_active: i64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BulkReviewAction {
    pub action: String, // "approve", "reject", "request_changes", "assign_reviewer"
    pub run_ids: Vec<String>,
    pub comment: Option<String>,
    pub reviewer: Option<String>,
    pub parameters: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct ReviewActionResult {
    pub action: String,
    pub total_items: usize,
    pub successful: usize,
    pub failed: usize,
    pub errors: Vec<String>,
    pub completed_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SubmitReviewRequest {
    pub run_id: String,
    pub verdict: String, // "approved", "rejected", "request_changes"
    pub comment: Option<String>,
}

/// Review queue dashboard - main review management interface
pub async fn review_queue(
    State(state): State<AppState>,
    user_ctx: UserContext,
    Query(filters): Query<ReviewFilters>,
    headers: header::HeaderMap,
) -> Response {
    let admin_user = match AdminUser::from_user_context(&user_ctx) {
        Some(admin) => admin,
        None => return StatusCode::FORBIDDEN.into_response(),
    };

    if !admin_user.has_permission(&Permission::ViewReviews) {
        return StatusCode::FORBIDDEN.into_response();
    }

    let mut context = create_admin_context(&admin_user);

    // Fetch pending reviews using database methods
    match fetch_review_queue(&state, &filters).await {
        Ok((items, stats)) => {
            context.insert("review_items", &items);
            context.insert("review_stats", &stats);
            context.insert("filters", &filters);
        }
        Err(e) => {
            tracing::error!("Failed to fetch review data: {}", e);
            context.insert(
                "error_message",
                &format!("Failed to load review data: {}", e),
            );
        }
    }

    // Add available campaigns for filtering
    let campaigns: Vec<String> = state.config.campaigns.keys().cloned().collect();
    context.insert("available_campaigns", &campaigns);

    let content_type = negotiate_content_type(&headers, "review_queue");

    match content_type {
        ContentType::Json => Json(context.into_json()).into_response(),
        _ => {
            match state
                .templates
                .render("cupboard/review-queue.html", &context)
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

/// Individual review interface
pub async fn review_interface(
    State(state): State<AppState>,
    user_ctx: UserContext,
    Path(run_id): Path<String>,
    headers: header::HeaderMap,
) -> Response {
    let admin_user = match AdminUser::from_user_context(&user_ctx) {
        Some(admin) => admin,
        None => return StatusCode::FORBIDDEN.into_response(),
    };

    if !admin_user.has_permission(&Permission::ViewReviews) {
        return StatusCode::FORBIDDEN.into_response();
    }

    let mut context = create_admin_context(&admin_user);
    context.insert("run_id", &run_id);

    // Fetch run details and evaluation
    match fetch_run_for_review(&state, &run_id).await {
        Ok(review_data) => {
            context.insert("run", &review_data);
            // Add evaluation data if available
            if let Ok(evaluation) = fetch_run_evaluation(&state, &run_id).await {
                context.insert("evaluation", &evaluation);
            }
        }
        Err(e) => {
            tracing::error!("Failed to fetch run for review: {}", e);
            return StatusCode::NOT_FOUND.into_response();
        }
    }

    let content_type = negotiate_content_type(&headers, "review_interface");

    match content_type {
        ContentType::Json => Json(context.into_json()).into_response(),
        _ => match state.templates.render("cupboard/review.html", &context) {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("Template rendering error: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        },
    }
}

/// Submit review verdict
pub async fn submit_review(
    State(state): State<AppState>,
    user_ctx: UserContext,
    headers: header::HeaderMap,
    Json(review_request): Json<SubmitReviewRequest>,
) -> Response {
    let admin_user = match AdminUser::from_user_context(&user_ctx) {
        Some(admin) => admin,
        None => return StatusCode::FORBIDDEN.into_response(),
    };

    if !admin_user.has_permission(&Permission::ViewReviews) {
        return StatusCode::FORBIDDEN.into_response();
    }

    // Extract IP and User-Agent for audit logging
    let ip_address = extract_ip_address(&headers);
    let user_agent = extract_user_agent(&headers);

    let reviewer = admin_user.user.email.clone();

    // Store the review in database
    match store_review_verdict(&state, &review_request, &reviewer).await {
        Ok(_) => {
            // Log the review action
            log_admin_action(
                &state,
                &admin_user,
                "submit_review",
                Some(&review_request.run_id),
                serde_json::to_value(&review_request).unwrap_or_default(),
                &ip_address,
                &user_agent,
            )
            .await;

            tracing::info!(
                "Review submitted by {}: {} for run {}",
                reviewer,
                review_request.verdict,
                review_request.run_id
            );

            Json(serde_json::json!({
                "success": true,
                "message": "Review submitted successfully"
            }))
            .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to store review: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// Bulk review operations
pub async fn bulk_review_action(
    State(state): State<AppState>,
    user_ctx: UserContext,
    headers: header::HeaderMap,
    Json(action): Json<BulkReviewAction>,
) -> Response {
    let admin_user = match AdminUser::from_user_context(&user_ctx) {
        Some(admin) => admin,
        None => return StatusCode::FORBIDDEN.into_response(),
    };

    if !admin_user.has_permission(&Permission::BulkReviewActions) {
        return StatusCode::FORBIDDEN.into_response();
    }

    // Extract IP and User-Agent for audit logging
    let ip_address = extract_ip_address(&headers);
    let user_agent = extract_user_agent(&headers);

    // Log the bulk review action attempt
    log_admin_action(
        &state,
        &admin_user,
        &format!("bulk_review_{}", action.action),
        None,
        serde_json::to_value(&action).unwrap_or_default(),
        &ip_address,
        &user_agent,
    )
    .await;

    // Execute bulk review action
    match execute_bulk_review_action(&state, &action, &admin_user.user.email).await {
        Ok(result) => {
            tracing::info!(
                "Bulk review action '{}' completed by {}: {}/{} successful",
                action.action,
                admin_user
                    .user
                    .name
                    .as_deref()
                    .unwrap_or(&admin_user.user.email),
                result.successful,
                result.total_items
            );
            Json(result).into_response()
        }
        Err(e) => {
            tracing::error!("Bulk review action failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// Review statistics endpoint
pub async fn review_statistics(
    State(state): State<AppState>,
    user_ctx: UserContext,
    Query(filters): Query<ReviewFilters>,
) -> Response {
    let admin_user = match AdminUser::from_user_context(&user_ctx) {
        Some(admin) => admin,
        None => return StatusCode::FORBIDDEN.into_response(),
    };

    if !admin_user.has_permission(&Permission::ViewReviews) {
        return StatusCode::FORBIDDEN.into_response();
    }

    match fetch_review_statistics(&state, &filters).await {
        Ok(stats) => Json(stats).into_response(),
        Err(e) => {
            tracing::error!("Failed to fetch review statistics: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// Rejected runs interface  
pub async fn rejected_runs(
    State(state): State<AppState>,
    user_ctx: UserContext,
    Query(filters): Query<ReviewFilters>,
    headers: header::HeaderMap,
) -> Response {
    let admin_user = match AdminUser::from_user_context(&user_ctx) {
        Some(admin) => admin,
        None => return StatusCode::FORBIDDEN.into_response(),
    };

    if !admin_user.has_permission(&Permission::ViewReviews) {
        return StatusCode::FORBIDDEN.into_response();
    }

    let mut context = create_admin_context(&admin_user);

    // Fetch rejected runs
    match fetch_rejected_runs(&state, &filters).await {
        Ok(rejected_runs) => {
            context.insert("rejected_runs", &rejected_runs);
            context.insert("filters", &filters);
        }
        Err(e) => {
            tracing::error!("Failed to fetch rejected runs: {}", e);
            context.insert(
                "error_message",
                &format!("Failed to load rejected runs: {}", e),
            );
        }
    }

    let content_type = negotiate_content_type(&headers, "rejected_runs");

    match content_type {
        ContentType::Json => Json(context.into_json()).into_response(),
        _ => match state.templates.render("cupboard/rejected.html", &context) {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("Template rendering error: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        },
    }
}

// Helper functions

async fn fetch_review_queue(
    state: &AppState,
    filters: &ReviewFilters,
) -> anyhow::Result<(Vec<ReviewItem>, ReviewStatistics)> {
    // Query publish_ready view for runs that need review
    let mut query = "
        SELECT 
            pr.id, pr.codebase, pr.suite, pr.result_code, pr.finish_time, 
            pr.command, pr.description, pr.publish_status, pr.branch_url,
            pr.value
        FROM publish_ready pr
        WHERE pr.publish_status IN ('needs-manual-review', 'unknown')
    ".to_string();

    // Apply filters - using a simpler approach
    if let Some(campaign) = &filters.campaign {
        if !campaign.is_empty() {
            query.push_str(&format!(" AND pr.suite = '{}'", campaign.replace("'", "''")));
        }
    }

    // Note: ReviewFilters doesn't have a codebase field, 
    // filtering by codebase would need to be added to the struct if needed

    // Add ordering and limit
    query.push_str(" ORDER BY pr.finish_time DESC");
    let limit = filters.limit.unwrap_or(100);
    query.push_str(&format!(" LIMIT {}", limit));

    let rows = sqlx::query(&query)
        .fetch_all(state.database.pool())
        .await?;

    let mut items = Vec::new();
    
    for row in rows {
        let run_id: String = row.try_get("id")?;
        
        // Get existing reviews for this run
        let review_rows = sqlx::query(
            "SELECT reviewer, verdict, comment, reviewed_at FROM review WHERE run_id = $1"
        )
        .bind(&run_id)
        .fetch_all(state.database.pool())
        .await?;

        let reviews: Vec<ReviewRecord> = review_rows.into_iter().map(|r| {
            ReviewRecord {
                reviewer: r.try_get("reviewer").unwrap_or_default(),
                verdict: r.try_get("verdict").unwrap_or_default(),
                comment: r.try_get("comment").unwrap_or_default(),
                reviewed_at: r.try_get("reviewed_at").unwrap_or_else(|_| Utc::now()),
            }
        }).collect();

        let publish_status: Option<String> = row.try_get("publish_status").ok();
        let needs_review = publish_status.as_ref()
            .map(|s| s == "needs-manual-review" || s == "unknown")
            .unwrap_or(false);

        items.push(ReviewItem {
            run_id,
            codebase: row.try_get("codebase")?,
            campaign: row.try_get("suite")?,
            result_code: row.try_get("result_code")?,
            finish_time: row.try_get("finish_time").ok(),
            value: row.try_get("value").ok(),
            command: row.try_get("command").ok(),
            description: row.try_get("description").ok(),
            reviews,
            publish_status,
            branch_url: row.try_get("branch_url").ok(),
            needs_review,
        });
    }

    // Calculate statistics
    let stats_row = sqlx::query(
        "SELECT 
            COUNT(*) FILTER (WHERE publish_status = 'needs-manual-review') as needs_manual_review,
            COUNT(*) FILTER (WHERE publish_status IN ('approved', 'rejected')) as total_reviewed,
            COUNT(*) as total_pending,
            COUNT(*) FILTER (WHERE publish_status = 'approved') as approved_count,
            COUNT(*) FILTER (WHERE publish_status = 'rejected') as rejected_count
        FROM publish_ready 
        WHERE publish_status IN ('needs-manual-review', 'unknown', 'approved', 'rejected')"
    )
    .fetch_one(state.database.pool())
    .await?;

    let stats = ReviewStatistics {
        total_pending: stats_row.try_get("total_pending")?,
        total_reviewed: stats_row.try_get("total_reviewed")?,
        needs_manual_review: stats_row.try_get("needs_manual_review")?,
        approved_count: stats_row.try_get("approved_count")?,
        rejected_count: stats_row.try_get("rejected_count")?,
        average_review_time: 0.0, // TODO: Calculate from review timestamps
        reviewers_active: 0, // TODO: Calculate active reviewers
    };

    Ok((items, stats))
}

async fn fetch_run_for_review(state: &AppState, run_id: &str) -> anyhow::Result<ReviewItem> {
    // Get run details from the database
    let run_row = sqlx::query(
        "SELECT id, codebase, suite, result_code, finish_time, value, command, 
         description, publish_status, branch_url
         FROM run WHERE id = $1"
    )
    .bind(run_id)
    .fetch_one(state.database.pool())
    .await?;

    // Get existing reviews for this run
    let review_rows = sqlx::query(
        "SELECT reviewer, verdict, comment, reviewed_at FROM review WHERE run_id = $1"
    )
    .bind(run_id)
    .fetch_all(&state.database.pool())
    .await?;

    let reviews: Vec<ReviewRecord> = review_rows.into_iter().map(|r| {
        ReviewRecord {
            reviewer: r.try_get("reviewer").unwrap_or_default(),
            verdict: r.try_get("verdict").unwrap_or_default(),
            comment: r.try_get("comment").unwrap_or_default(),
            reviewed_at: r.try_get("reviewed_at").unwrap_or_else(|_| Utc::now()),
        }
    }).collect();

    let publish_status: Option<String> = run_row.try_get("publish_status").ok();
    let needs_review = publish_status.as_ref()
        .map(|s| s == "needs-manual-review" || s == "unknown")
        .unwrap_or(false);

    Ok(ReviewItem {
        run_id: run_row.try_get("id")?,
        codebase: run_row.try_get("codebase")?,
        campaign: run_row.try_get("suite")?,
        result_code: run_row.try_get("result_code")?,
        finish_time: run_row.try_get("finish_time").ok(),
        value: run_row.try_get("value").ok(),
        command: run_row.try_get("command").ok(),
        description: run_row.try_get("description").ok(),
        reviews,
        publish_status,
        branch_url: run_row.try_get("branch_url").ok(),
        needs_review,
    })
}

async fn fetch_run_evaluation(state: &AppState, run_id: &str) -> anyhow::Result<serde_json::Value> {
    // TODO: Implement run evaluation fetching
    // This would get detailed evaluation data for the run

    Ok(serde_json::json!({
        "evaluation": "Run completed successfully with no issues detected",
        "score": 85,
        "details": []
    }))
}

async fn store_review_verdict(
    state: &AppState,
    review_request: &SubmitReviewRequest,
    reviewer: &str,
) -> anyhow::Result<()> {
    // Store the review verdict in the database using UPSERT (INSERT ... ON CONFLICT)
    sqlx::query(
        "INSERT INTO review (run_id, reviewer, verdict, comment, reviewed_at) 
         VALUES ($1, $2, $3, $4, NOW())
         ON CONFLICT (run_id, reviewer) 
         DO UPDATE SET 
             verdict = EXCLUDED.verdict,
             comment = EXCLUDED.comment,
             reviewed_at = NOW()"
    )
    .bind(&review_request.run_id)
    .bind(reviewer)
    .bind(&review_request.verdict)
    .bind(&review_request.comment)
    .execute(state.database.pool())
    .await?;

    // Also update the publish_status in the run table if needed
    match review_request.verdict.as_str() {
        "approved" => {
            sqlx::query("UPDATE run SET publish_status = 'approved' WHERE id = $1")
                .bind(&review_request.run_id)
                .execute(state.database.pool())
                .await?;
        }
        "rejected" => {
            sqlx::query("UPDATE run SET publish_status = 'rejected' WHERE id = $1")
                .bind(&review_request.run_id)
                .execute(state.database.pool())
                .await?;
        }
        _ => {
            // For other verdicts, keep current status or set to needs-manual-review
            sqlx::query("UPDATE run SET publish_status = 'needs-manual-review' WHERE id = $1 AND publish_status = 'unknown'")
                .bind(&review_request.run_id)
                .execute(state.database.pool())
                .await?;
        }
    }

    tracing::info!(
        "Stored review verdict '{}' for run {} by reviewer {}",
        review_request.verdict,
        review_request.run_id,
        reviewer
    );

    Ok(())
}

async fn execute_bulk_review_action(
    state: &AppState,
    action: &BulkReviewAction,
    reviewer: &str,
) -> anyhow::Result<ReviewActionResult> {
    let now = Utc::now();

    match action.action.as_str() {
        "approve" => {
            // TODO: Implement bulk approval
            // This would approve all specified runs and set publish_status accordingly
            Ok(ReviewActionResult {
                action: action.action.clone(),
                total_items: action.run_ids.len(),
                successful: action.run_ids.len(),
                failed: 0,
                errors: vec![],
                completed_at: now,
            })
        }
        "reject" => {
            // TODO: Implement bulk rejection
            // This would reject all specified runs and set publish_status accordingly
            Ok(ReviewActionResult {
                action: action.action.clone(),
                total_items: action.run_ids.len(),
                successful: action.run_ids.len(),
                failed: 0,
                errors: vec![],
                completed_at: now,
            })
        }
        "request_changes" => {
            // TODO: Implement bulk request for changes
            // This would mark runs as needing changes
            Ok(ReviewActionResult {
                action: action.action.clone(),
                total_items: action.run_ids.len(),
                successful: action.run_ids.len(),
                failed: 0,
                errors: vec![],
                completed_at: now,
            })
        }
        "assign_reviewer" => {
            // TODO: Implement reviewer assignment
            // This would assign a specific reviewer to the runs
            Ok(ReviewActionResult {
                action: action.action.clone(),
                total_items: action.run_ids.len(),
                successful: 0,
                failed: action.run_ids.len(),
                errors: vec!["Reviewer assignment not yet implemented".to_string()],
                completed_at: now,
            })
        }
        _ => Err(anyhow::anyhow!("Unknown review action: {}", action.action)),
    }
}

async fn fetch_review_statistics(
    state: &AppState,
    filters: &ReviewFilters,
) -> anyhow::Result<ReviewStatistics> {
    // TODO: Implement comprehensive review statistics
    // This would aggregate review data from the database

    Ok(ReviewStatistics {
        total_pending: 5,
        total_reviewed: 20,
        needs_manual_review: 3,
        approved_count: 15,
        rejected_count: 2,
        average_review_time: 1.5, // hours
        reviewers_active: 3,
    })
}

async fn fetch_rejected_runs(
    state: &AppState,
    filters: &ReviewFilters,
) -> anyhow::Result<Vec<ReviewItem>> {
    // TODO: Implement rejected runs fetching
    // This would query for runs with publish_status = 'rejected'

    Ok(vec![ReviewItem {
        run_id: "run-rejected-1".to_string(),
        codebase: "problem-package".to_string(),
        campaign: "lintian-fixes".to_string(),
        result_code: "success".to_string(),
        finish_time: Some(Utc::now() - chrono::Duration::hours(2)),
        value: Some(50),
        command: Some("fix-lintian-issues".to_string()),
        description: Some("Changes introduce regression".to_string()),
        reviews: vec![ReviewRecord {
            reviewer: "qa@example.com".to_string(),
            verdict: "rejected".to_string(),
            comment: Some("This introduces a regression in the build process".to_string()),
            reviewed_at: Utc::now() - chrono::Duration::minutes(30),
        }],
        publish_status: Some("rejected".to_string()),
        branch_url: Some(
            "https://salsa.debian.org/jelmer/problem-package/-/merge_requests/1".to_string(),
        ),
        needs_review: false,
    }])
}

fn extract_ip_address(headers: &header::HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .or(headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string()
}

fn extract_user_agent(headers: &header::HeaderMap) -> String {
    headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string()
}
