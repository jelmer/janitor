use crate::rate_limiter::RateLimiter;
use axum::Router;
use breezyshim::forge::Forge;
use janitor::vcs::{VcsManager, VcsType};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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
}
