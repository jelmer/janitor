use anyhow::Result;
use std::sync::Arc;
use std::time::Instant;
use tera::Tera;

use crate::config::Config;
use crate::database::DatabaseManager;
use crate::templates::setup_templates;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub database: DatabaseManager,
    pub templates: Arc<Tera>,
    pub redis: Option<redis::Client>,
    pub start_time: Instant,
}

impl AppState {
    pub async fn new(config: Config) -> Result<Self> {
        // Initialize database manager
        let database = DatabaseManager::new(&config).await?;
        
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
            database,
            templates,
            redis,
            start_time: Instant::now(),
        })
    }
    
    pub async fn health_check(&self) -> Result<()> {
        // Check database connection
        self.database.health_check().await?;
        
        // Check Redis connection if configured
        if let Some(redis_client) = &self.redis {
            let mut conn = redis_client.get_async_connection().await?;
            redis::cmd("PING").query_async::<String>(&mut conn).await?;
        }
        
        Ok(())
    }
}