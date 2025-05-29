use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::env;
use std::net::SocketAddr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub listen_address: SocketAddr,
    pub database_url: String,
    pub redis_url: Option<String>,
    pub template_dir: Option<String>,
    pub static_dir: Option<String>,
    pub debug: bool,
    
    // Authentication
    pub oidc_client_id: Option<String>,
    pub oidc_client_secret: Option<String>,
    pub oidc_issuer_url: Option<String>,
    pub session_secret: String,
    
    // External services
    pub differ_url: Option<String>,
    pub worker_url: Option<String>,
    pub git_store_url: Option<String>,
    pub bzr_store_url: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let listen_address = env::var("LISTEN_ADDRESS")
            .unwrap_or_else(|_| "127.0.0.1:8000".to_string())
            .parse()?;

        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://localhost/janitor".to_string());

        let session_secret = env::var("SESSION_SECRET")
            .unwrap_or_else(|_| {
                if cfg!(debug_assertions) {
                    "debug-secret-key-not-for-production".to_string()
                } else {
                    panic!("SESSION_SECRET environment variable is required in production")
                }
            });

        let debug = env::var("DEBUG")
            .map(|v| v.parse().unwrap_or(false))
            .unwrap_or(cfg!(debug_assertions));

        Ok(Config {
            listen_address,
            database_url,
            redis_url: env::var("REDIS_URL").ok(),
            template_dir: env::var("TEMPLATE_DIR").ok(),
            static_dir: env::var("STATIC_DIR").ok(),
            debug,
            
            // Authentication
            oidc_client_id: env::var("OIDC_CLIENT_ID").ok(),
            oidc_client_secret: env::var("OIDC_CLIENT_SECRET").ok(),
            oidc_issuer_url: env::var("OIDC_ISSUER_URL").ok(),
            session_secret,
            
            // External services
            differ_url: env::var("DIFFER_URL").ok(),
            worker_url: env::var("WORKER_URL").ok(),
            git_store_url: env::var("GIT_STORE_URL").ok(),
            bzr_store_url: env::var("BZR_STORE_URL").ok(),
        })
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listen_address: "127.0.0.1:8000".parse().unwrap(),
            database_url: "postgresql://localhost/janitor".to_string(),
            redis_url: None,
            template_dir: None,
            static_dir: None,
            debug: true,
            
            oidc_client_id: None,
            oidc_client_secret: None,
            oidc_issuer_url: None,
            session_secret: "debug-secret-key".to_string(),
            
            differ_url: None,
            worker_url: None,
            git_store_url: None,
            bzr_store_url: None,
        }
    }
}