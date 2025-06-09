use anyhow::Result;
use std::sync::Arc;
use std::time::Instant;
use tera::Tera;

use crate::assets::{AssetManager, AssetManifest};
use crate::config::Config;
use crate::database::DatabaseManager;
use crate::realtime::{RealtimeConfig, RealtimeManager};
use crate::templates::setup_templates;
use janitor::logs::{get_log_manager, LogFileManager};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub database: DatabaseManager,
    pub templates: Arc<Tera>,
    pub redis: Option<redis::Client>,
    pub http_client: reqwest::Client,
    pub log_manager: Arc<Box<dyn LogFileManager>>,
    pub realtime: Arc<RealtimeManager>,
    pub assets: AssetManifest,
    pub start_time: Instant,
    pub auth_state: Option<Arc<crate::auth::middleware::AuthState>>,
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

        // Initialize log manager using the factory function
        let log_url = config.log_url().unwrap_or_else(|| {
            format!(
                "file://{}",
                config
                    .log_base_path()
                    .unwrap_or("/var/log/janitor".to_string())
            )
        });
        let log_manager = Arc::new(get_log_manager(Some(&log_url)).await?);

        // Initialize real-time manager
        let realtime_config = RealtimeConfig::default();
        let realtime_manager = Arc::new(RealtimeManager::new(redis.clone(), realtime_config));

        // Start real-time manager
        if let Err(e) = realtime_manager.start().await {
            tracing::warn!("Failed to start real-time manager: {}", e);
        }

        // Initialize asset manager
        let mut asset_manager = if cfg!(debug_assertions) {
            AssetManager::development("site/static".to_string())
        } else {
            AssetManager::new("site/static".to_string())
        };

        // Generate asset fingerprints for cache busting
        if let Err(e) = asset_manager.generate_fingerprints() {
            tracing::warn!("Failed to generate asset fingerprints: {}", e);
        }

        let asset_manifest = AssetManifest::new(asset_manager);

        Ok(Self {
            config: Arc::new(config),
            database,
            templates,
            redis,
            http_client,
            log_manager,
            realtime: realtime_manager,
            assets: asset_manifest,
            start_time: Instant::now(),
            auth_state: None, // Will be set later during app creation
        })
    }

    pub fn with_auth_state(mut self, auth_state: Arc<crate::auth::middleware::AuthState>) -> Self {
        self.auth_state = Some(auth_state);
        self
    }

    pub async fn health_check(&self) -> Result<()> {
        // Check database connection
        self.database.health_check().await?;

        // Check Redis connection if configured
        if let Some(redis_client) = &self.redis {
            let mut conn = redis_client.get_multiplexed_async_connection().await?;
            redis::cmd("PING").query_async::<String>(&mut conn).await?;
        }

        // Check real-time manager
        self.realtime.health_check().await?;

        Ok(())
    }
}
