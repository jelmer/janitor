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
    pub http_client: reqwest::Client,
    pub log_manager: Arc<LogManager>,
    pub start_time: Instant,
}

// Placeholder log manager - will be enhanced when log storage is implemented
#[derive(Debug)]
pub struct LogManager {
    base_path: String,
}

impl LogManager {
    pub fn new(base_path: String) -> Self {
        Self { base_path }
    }
    
    pub async fn log_exists(&self, _run_id: &str, _log_name: &str) -> Result<bool> {
        // TODO: Implement actual log existence check
        Ok(true)
    }
    
    pub async fn get_log_size(&self, _run_id: &str, _log_name: &str) -> Result<i64> {
        // TODO: Implement actual log size retrieval
        Ok(1024)
    }
    
    pub async fn get_log_content(&self, _run_id: &str, _log_name: &str) -> Result<Vec<u8>> {
        // TODO: Implement actual log content retrieval
        Ok(b"Log file content placeholder".to_vec())
    }
}

impl AppState {
    pub async fn new(config: Config) -> Result<Self> {
        // Initialize database manager
        let database = DatabaseManager::new(&config).await?;

        // Initialize template engine
        let templates = Arc::new(setup_templates(config.site())?);

        // Initialize Redis client if configured
        let redis = if let Some(redis_url) = config.redis_url() {
            Some(redis::Client::open(redis_url)?)
        } else {
            None
        };
        
        // Initialize HTTP client for service communication
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
            
        // Initialize log manager
        let log_manager = Arc::new(LogManager::new(
            config.log_base_path().unwrap_or("/var/log/janitor".to_string())
        ));

        Ok(Self {
            config: Arc::new(config),
            database,
            templates,
            redis,
            http_client,
            log_manager,
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
