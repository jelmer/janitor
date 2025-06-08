//! Environment variable parsing utilities

use std::env;
use std::str::FromStr;

use crate::shared_config::ConfigError;

/// Trait for loading configuration from environment variables
pub trait FromEnv: Sized {
    /// Load configuration from environment variables without prefix
    fn from_env() -> Result<Self, ConfigError>;
    
    /// Load configuration from environment variables with a prefix
    fn from_env_with_prefix(prefix: &str) -> Result<Self, ConfigError>;
}

/// Utility for parsing environment variables with optional prefix
pub struct EnvParser {
    prefix: Option<String>,
}

impl EnvParser {
    /// Create a new environment parser without prefix
    pub fn new() -> Self {
        Self { prefix: None }
    }
    
    /// Create a new environment parser with prefix
    pub fn with_prefix(prefix: &str) -> Self {
        let prefix = if prefix.is_empty() {
            None
        } else {
            let mut p = prefix.to_uppercase();
            if !p.ends_with('_') {
                p.push('_');
            }
            Some(p)
        };
        
        Self { prefix }
    }
    
    /// Get the environment variable key with prefix applied
    fn get_key(&self, key: &str) -> String {
        match &self.prefix {
            Some(prefix) => format!("{}{}", prefix, key),
            None => key.to_string(),
        }
    }
    
    /// Get a string value from environment
    pub fn get_string(&self, key: &str) -> Option<String> {
        env::var(self.get_key(key)).ok()
    }
    
    /// Get a required string value from environment
    pub fn get_required_string(&self, key: &str) -> Result<String, ConfigError> {
        self.get_string(key)
            .ok_or_else(|| ConfigError::MissingRequired(self.get_key(key)))
    }
    
    /// Get a boolean value from environment
    pub fn get_bool(&self, key: &str) -> Result<Option<bool>, ConfigError> {
        match self.get_string(key) {
            Some(value) => {
                let parsed = match value.to_lowercase().as_str() {
                    "true" | "yes" | "1" | "on" => true,
                    "false" | "no" | "0" | "off" => false,
                    _ => return Err(ConfigError::ParseError {
                        field: self.get_key(key),
                        message: format!("Invalid boolean value: '{}'", value),
                    }),
                };
                Ok(Some(parsed))
            }
            None => Ok(None),
        }
    }
    
    /// Get a u16 value from environment
    pub fn get_u16(&self, key: &str) -> Result<Option<u16>, ConfigError> {
        self.parse_numeric(key)
    }
    
    /// Get a u32 value from environment
    pub fn get_u32(&self, key: &str) -> Result<Option<u32>, ConfigError> {
        self.parse_numeric(key)
    }
    
    /// Get a u64 value from environment
    pub fn get_u64(&self, key: &str) -> Result<Option<u64>, ConfigError> {
        self.parse_numeric(key)
    }
    
    /// Get a usize value from environment
    pub fn get_usize(&self, key: &str) -> Result<Option<usize>, ConfigError> {
        self.parse_numeric(key)
    }
    
    /// Get an i32 value from environment
    pub fn get_i32(&self, key: &str) -> Result<Option<i32>, ConfigError> {
        self.parse_numeric(key)
    }
    
    /// Get an i64 value from environment
    pub fn get_i64(&self, key: &str) -> Result<Option<i64>, ConfigError> {
        self.parse_numeric(key)
    }
    
    /// Get an f64 value from environment
    pub fn get_f64(&self, key: &str) -> Result<Option<f64>, ConfigError> {
        self.parse_numeric(key)
    }
    
    /// Parse a numeric value from environment
    fn parse_numeric<T>(&self, key: &str) -> Result<Option<T>, ConfigError>
    where
        T: FromStr,
        T::Err: std::fmt::Display,
    {
        match self.get_string(key) {
            Some(value) => {
                let parsed = value.parse::<T>().map_err(|e| ConfigError::ParseError {
                    field: self.get_key(key),
                    message: format!("Invalid numeric value '{}': {}", value, e),
                })?;
                Ok(Some(parsed))
            }
            None => Ok(None),
        }
    }
    
    /// Get a comma-separated list of strings from environment
    pub fn get_string_list(&self, key: &str) -> Option<Vec<String>> {
        self.get_string(key).map(|value| {
            value
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
    }
    
    /// Get a URL from environment
    pub fn get_url(&self, key: &str) -> Result<Option<url::Url>, ConfigError> {
        match self.get_string(key) {
            Some(value) => {
                let url = value.parse::<url::Url>().map_err(|e| ConfigError::ParseError {
                    field: self.get_key(key),
                    message: format!("Invalid URL '{}': {}", value, e),
                })?;
                Ok(Some(url))
            }
            None => Ok(None),
        }
    }
    
    /// Get a duration in seconds from environment
    pub fn get_duration_seconds(&self, key: &str) -> Result<Option<std::time::Duration>, ConfigError> {
        match self.get_u64(key)? {
            Some(seconds) => Ok(Some(std::time::Duration::from_secs(seconds))),
            None => Ok(None),
        }
    }
}

impl Default for EnvParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper functions for common environment variable patterns
pub mod helpers {
    use super::*;
    
    /// Get database URL from common environment variables
    pub fn get_database_url() -> Result<String, ConfigError> {
        // Try common database URL environment variables
        for key in &["DATABASE_URL", "POSTGRES_URL", "POSTGRESQL_URL"] {
            if let Ok(url) = env::var(key) {
                return Ok(url);
            }
        }
        
        // Try to construct from individual components
        if let (Ok(host), Ok(user), Ok(db)) = (
            env::var("PGHOST").or_else(|_| env::var("DB_HOST")),
            env::var("PGUSER").or_else(|_| env::var("DB_USER")),
            env::var("PGDATABASE").or_else(|_| env::var("DB_NAME")),
        ) {
            let port = env::var("PGPORT")
                .or_else(|_| env::var("DB_PORT"))
                .unwrap_or_else(|_| "5432".to_string());
            
            let password = env::var("PGPASSWORD")
                .or_else(|_| env::var("DB_PASSWORD"))
                .unwrap_or_else(|_| "".to_string());
            
            let auth = if password.is_empty() {
                user
            } else {
                format!("{}:{}", user, password)
            };
            
            return Ok(format!("postgresql://{}@{}:{}/{}", auth, host, port, db));
        }
        
        Err(ConfigError::MissingRequired("DATABASE_URL".to_string()))
    }
    
    /// Get Redis URL from common environment variables
    pub fn get_redis_url() -> Result<String, ConfigError> {
        // Try common Redis URL environment variables
        for key in &["REDIS_URL", "REDIS_URI"] {
            if let Ok(url) = env::var(key) {
                return Ok(url);
            }
        }
        
        // Try to construct from individual components
        if let Ok(host) = env::var("REDIS_HOST") {
            let port = env::var("REDIS_PORT").unwrap_or_else(|_| "6379".to_string());
            let db = env::var("REDIS_DB").unwrap_or_else(|_| "0".to_string());
            
            let auth = if let Ok(password) = env::var("REDIS_PASSWORD") {
                format!(":{}@", password)
            } else {
                "".to_string()
            };
            
            return Ok(format!("redis://{}{}:{}/{}", auth, host, port, db));
        }
        
        // Default to localhost
        Ok("redis://localhost:6379/0".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    
    #[test]
    fn test_env_parser_no_prefix() {
        env::set_var("TEST_VAR", "test_value");
        env::set_var("TEST_BOOL", "true");
        env::set_var("TEST_NUM", "42");
        
        let parser = EnvParser::new();
        
        assert_eq!(parser.get_string("TEST_VAR"), Some("test_value".to_string()));
        assert_eq!(parser.get_bool("TEST_BOOL").unwrap(), Some(true));
        assert_eq!(parser.get_u32("TEST_NUM").unwrap(), Some(42));
        assert_eq!(parser.get_string("NONEXISTENT"), None);
        
        env::remove_var("TEST_VAR");
        env::remove_var("TEST_BOOL");
        env::remove_var("TEST_NUM");
    }
    
    #[test]
    fn test_env_parser_with_prefix() {
        env::set_var("APP_TEST_VAR", "prefixed_value");
        env::set_var("APP_TEST_BOOL", "false");
        
        let parser = EnvParser::with_prefix("APP");
        
        assert_eq!(parser.get_string("TEST_VAR"), Some("prefixed_value".to_string()));
        assert_eq!(parser.get_bool("TEST_BOOL").unwrap(), Some(false));
        
        env::remove_var("APP_TEST_VAR");
        env::remove_var("APP_TEST_BOOL");
    }
    
    #[test]
    fn test_boolean_parsing() {
        env::set_var("BOOL_TRUE_1", "true");
        env::set_var("BOOL_TRUE_2", "yes");
        env::set_var("BOOL_TRUE_3", "1");
        env::set_var("BOOL_FALSE_1", "false");
        env::set_var("BOOL_FALSE_2", "no");
        env::set_var("BOOL_FALSE_3", "0");
        env::set_var("BOOL_INVALID", "maybe");
        
        let parser = EnvParser::new();
        
        assert_eq!(parser.get_bool("BOOL_TRUE_1").unwrap(), Some(true));
        assert_eq!(parser.get_bool("BOOL_TRUE_2").unwrap(), Some(true));
        assert_eq!(parser.get_bool("BOOL_TRUE_3").unwrap(), Some(true));
        assert_eq!(parser.get_bool("BOOL_FALSE_1").unwrap(), Some(false));
        assert_eq!(parser.get_bool("BOOL_FALSE_2").unwrap(), Some(false));
        assert_eq!(parser.get_bool("BOOL_FALSE_3").unwrap(), Some(false));
        assert!(parser.get_bool("BOOL_INVALID").is_err());
        
        env::remove_var("BOOL_TRUE_1");
        env::remove_var("BOOL_TRUE_2");
        env::remove_var("BOOL_TRUE_3");
        env::remove_var("BOOL_FALSE_1");
        env::remove_var("BOOL_FALSE_2");
        env::remove_var("BOOL_FALSE_3");
        env::remove_var("BOOL_INVALID");
    }
    
    #[test]
    fn test_string_list_parsing() {
        env::set_var("LIST_VAR", "item1,item2,item3");
        env::set_var("LIST_SPACES", " item1 , item2 , item3 ");
        env::set_var("LIST_EMPTY", "");
        
        let parser = EnvParser::new();
        
        assert_eq!(
            parser.get_string_list("LIST_VAR"),
            Some(vec!["item1".to_string(), "item2".to_string(), "item3".to_string()])
        );
        assert_eq!(
            parser.get_string_list("LIST_SPACES"),
            Some(vec!["item1".to_string(), "item2".to_string(), "item3".to_string()])
        );
        assert_eq!(parser.get_string_list("LIST_EMPTY"), Some(vec![]));
        assert_eq!(parser.get_string_list("NONEXISTENT"), None);
        
        env::remove_var("LIST_VAR");
        env::remove_var("LIST_SPACES");
        env::remove_var("LIST_EMPTY");
    }
}