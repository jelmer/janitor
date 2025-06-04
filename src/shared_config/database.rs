//! Shared database configuration for all Janitor services

use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;

use crate::shared_config::{defaults::*, env::EnvParser, ConfigError, FromEnv, ValidationError};

/// Database configuration used across all Janitor services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// PostgreSQL connection URL
    pub url: String,
    
    /// Maximum number of connections in the pool
    #[serde(default = "default_db_max_connections")]
    pub max_connections: u32,
    
    /// Connection timeout in seconds
    #[serde(default = "default_db_connection_timeout")]
    pub connection_timeout_seconds: u64,
    
    /// Query timeout in seconds
    #[serde(default = "default_db_query_timeout")]
    pub query_timeout_seconds: u64,
    
    /// Enable SQL query logging (for debugging)
    #[serde(default = "default_false")]
    pub enable_sql_logging: bool,
    
    /// Minimum idle connections to maintain
    #[serde(default)]
    pub min_connections: Option<u32>,
    
    /// Maximum lifetime of a connection in seconds
    #[serde(default)]
    pub max_lifetime_seconds: Option<u64>,
    
    /// Idle timeout for connections in seconds
    #[serde(default)]
    pub idle_timeout_seconds: Option<u64>,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgresql://localhost/janitor".to_string(),
            max_connections: default_db_max_connections(),
            connection_timeout_seconds: default_db_connection_timeout(),
            query_timeout_seconds: default_db_query_timeout(),
            enable_sql_logging: default_false(),
            min_connections: None,
            max_lifetime_seconds: None,
            idle_timeout_seconds: None,
        }
    }
}

impl FromEnv for DatabaseConfig {
    fn from_env() -> Result<Self, ConfigError> {
        Self::from_env_with_prefix("")
    }
    
    fn from_env_with_prefix(prefix: &str) -> Result<Self, ConfigError> {
        let parser = EnvParser::with_prefix(prefix);
        
        let url = parser.get_string("DATABASE_URL")
            .ok_or_else(|| ConfigError::MissingRequired("DATABASE_URL".to_string()))?;
            
        Ok(Self {
            url,
            max_connections: parser.get_u32("DATABASE_MAX_CONNECTIONS")?
                .unwrap_or_else(default_db_max_connections),
            connection_timeout_seconds: parser.get_u64("DATABASE_CONNECTION_TIMEOUT_SECONDS")?
                .unwrap_or_else(default_db_connection_timeout),
            query_timeout_seconds: parser.get_u64("DATABASE_QUERY_TIMEOUT_SECONDS")?
                .unwrap_or_else(default_db_query_timeout),
            enable_sql_logging: parser.get_bool("DATABASE_ENABLE_SQL_LOGGING")?
                .unwrap_or_else(default_false),
            min_connections: parser.get_u32("DATABASE_MIN_CONNECTIONS")?,
            max_lifetime_seconds: parser.get_u64("DATABASE_MAX_LIFETIME_SECONDS")?,
            idle_timeout_seconds: parser.get_u64("DATABASE_IDLE_TIMEOUT_SECONDS")?,
        })
    }
}

impl DatabaseConfig {
    /// Validate the database configuration
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.url.is_empty() {
            return Err(ValidationError::InvalidValue {
                field: "url".to_string(),
                message: "Database URL cannot be empty".to_string(),
            });
        }
        
        if !self.url.starts_with("postgresql://") && !self.url.starts_with("postgres://") {
            return Err(ValidationError::InvalidValue {
                field: "url".to_string(),
                message: "Database URL must be a PostgreSQL connection string".to_string(),
            });
        }
        
        if self.max_connections == 0 {
            return Err(ValidationError::InvalidValue {
                field: "max_connections".to_string(),
                message: "Maximum connections must be greater than 0".to_string(),
            });
        }
        
        if let Some(min_conn) = self.min_connections {
            if min_conn > self.max_connections {
                return Err(ValidationError::InvalidValue {
                    field: "min_connections".to_string(),
                    message: "Minimum connections cannot exceed maximum connections".to_string(),
                });
            }
        }
        
        Ok(())
    }
    
    /// Create a PostgreSQL connection pool from this configuration
    pub async fn create_pool(&self) -> Result<PgPool, sqlx::Error> {
        let mut options = PgPoolOptions::new()
            .max_connections(self.max_connections)
            .acquire_timeout(Duration::from_secs(self.connection_timeout_seconds));
            
        if let Some(min_conn) = self.min_connections {
            options = options.min_connections(min_conn);
        }
        
        if let Some(max_lifetime) = self.max_lifetime_seconds {
            options = options.max_lifetime(Duration::from_secs(max_lifetime));
        }
        
        if let Some(idle_timeout) = self.idle_timeout_seconds {
            options = options.idle_timeout(Duration::from_secs(idle_timeout));
        }
        
        options.connect(&self.url).await
    }
    
    /// Get connection timeout as Duration
    pub fn connection_timeout(&self) -> Duration {
        Duration::from_secs(self.connection_timeout_seconds)
    }
    
    /// Get query timeout as Duration
    pub fn query_timeout(&self) -> Duration {
        Duration::from_secs(self.query_timeout_seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_database_config_validation() {
        let mut config = DatabaseConfig::default();
        assert!(config.validate().is_ok());
        
        // Test empty URL
        config.url = "".to_string();
        assert!(config.validate().is_err());
        
        // Test invalid URL
        config.url = "mysql://localhost/test".to_string();
        assert!(config.validate().is_err());
        
        // Test zero max connections
        config.url = "postgresql://localhost/test".to_string();
        config.max_connections = 0;
        assert!(config.validate().is_err());
        
        // Test min > max connections
        config.max_connections = 10;
        config.min_connections = Some(15);
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_database_config_default() {
        let config = DatabaseConfig::default();
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.connection_timeout_seconds, 30);
        assert_eq!(config.query_timeout_seconds, 60);
        assert!(!config.enable_sql_logging);
    }
}