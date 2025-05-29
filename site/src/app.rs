use anyhow::Result;
use sqlx::{Pool, Postgres, PgPool};
use std::sync::Arc;
use std::time::Instant;
use tera::Tera;

use crate::config::Config;
use crate::templates::setup_templates;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: Pool<Postgres>,
    pub templates: Arc<Tera>,
    pub redis: Option<redis::Client>,
    pub start_time: Instant,
}

impl AppState {
    pub async fn new(config: Config) -> Result<Self> {
        // Initialize database connection
        let db = PgPool::connect(&config.database_url).await?;
        
        // Run migrations if needed
        sqlx::migrate!().run(&db).await?;
        
        // Initialize template engine
        let templates = Arc::new(setup_templates(&config)?);
        
        // Initialize Redis client if configured
        let redis = if let Some(redis_url) = &config.redis_url {
            Some(redis::Client::open(redis_url.as_str())?)
        } else {
            None
        };
        
        Ok(Self {
            config: Arc::new(config),
            db,
            templates,
            redis,
            start_time: Instant::now(),
        })
    }
    
    pub async fn health_check(&self) -> Result<()> {
        // Check database connection
        sqlx::query("SELECT 1").execute(&self.db).await?;
        
        // Check Redis connection if configured
        if let Some(redis_client) = &self.redis {
            let mut conn = redis_client.get_async_connection().await?;
            redis::cmd("PING").query_async::<String>(&mut conn).await?;
        }
        
        Ok(())
    }
}