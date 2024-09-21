use crate::rate_limiter::RateLimiter;
use axum::routing::{delete, get, post, put};
use axum::Router;
use breezyshim::forge::Forge;
use janitor::vcs::{VcsManager, VcsType};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

async fn get_merge_proposals_by_campaign() {
    unimplemented!()
}

async fn get_merge_proposals_by_codebase() {
    unimplemented!()
}

async fn post_merge_proposal() {
    unimplemented!()
}

async fn absorbed() {
    unimplemented!()
}

async fn get_policy() {
    unimplemented!()
}

async fn get_policies() {
    unimplemented!()
}

async fn put_policy() {
    unimplemented!()
}

async fn put_policies() {
    unimplemented!()
}

async fn update_merge_proposal() {
    unimplemented!()
}

async fn delete_policy() {
    unimplemented!()
}

async fn consider() {
    unimplemented!()
}

async fn get_publish_by_id() {
    unimplemented!()
}

async fn publish() {
    unimplemented!()
}

async fn get_credentials() {
    unimplemented!()
}

async fn health() -> &'static str {
    "OK"
}

async fn ready() -> &'static str {
    "OK"
}

async fn scan() {
    unimplemented!()
}

async fn check_stragglers() {
    unimplemented!()
}

async fn refresh_status() {
    unimplemented!()
}

async fn autopublish() {
    unimplemented!()
}

async fn get_rate_limit() {
    unimplemented!()
}

async fn get_all_rate_limits() {
    unimplemented!()
}

async fn get_blockers() {
    unimplemented!()
}

pub fn app(
    worker: Arc<Mutex<crate::PublishWorker>>,
    bucket_rate_limiter: Arc<Mutex<Box<dyn RateLimiter>>>,
    forge_rate_limiter: Arc<Mutex<HashMap<Forge, chrono::DateTime<chrono::Utc>>>>,
    vcs_managers: &HashMap<VcsType, Box<dyn VcsManager>>,
    db: PgPool,
    require_binary_diff: bool,
    modify_mp_limit: Option<i32>,
    push_limit: Option<i32>,
    redis: Option<redis::aio::ConnectionManager>,
    config: &janitor::config::Config,
) -> Router {
    Router::new()
        .route(
            "/:campaign/merge-proposals",
            get(get_merge_proposals_by_campaign),
        )
        .route(
            "/c/:codebase/merge-proposals",
            get(get_merge_proposals_by_codebase),
        )
        .route("/merge-proposals", get(post_merge_proposal))
        .route("/absorbed", get(absorbed))
        .route("/policy/:name", get(get_policy))
        .route("/policy", get(get_policies))
        .route("/policy/:name", put(put_policy))
        .route("/policy", put(put_policies))
        .route("/merge-proposal", post(update_merge_proposal))
        .route("/policy/:name", delete(delete_policy))
        .route("/merge-proposal", post(update_merge_proposal))
        .route("/consider:id", post(consider))
        .route("/publish/:id", get(get_publish_by_id))
        .route("/:campaign/:codebase/publish", post(publish))
        .route("/credentials", get(get_credentials))
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/scan", post(scan))
        .route("/check-stragglers", post(check_stragglers))
        .route("/refresh-status", post(refresh_status))
        .route("/autopublish", post(autopublish))
        .route("/rate-limits/:bucket", get(get_rate_limit))
        .route("/rate-limits", get(get_all_rate_limits))
        .route("/blockers/:id", get(get_blockers))
}
