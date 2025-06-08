include!(concat!(env!("OUT_DIR"), "/generated/mod.rs"));

use protobuf::text_format;
use std::fs::File;
use std::io::Read;

pub use config::{
    AptRepository, BugTracker, BugTrackerKind, Campaign, Config, DebianBuild, Distribution,
    GenericBuild, MergeProposalConfig, OAuth2Provider, Select,
};

pub fn read_file(file_path: &std::path::Path) -> Result<Config, Box<dyn std::error::Error>> {
    let mut file = File::open(file_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    read_string(&contents)
}

pub fn read_readable<R: Read>(mut readable: R) -> Result<Config, Box<dyn std::error::Error>> {
    let mut contents = String::new();
    readable.read_to_string(&mut contents)?;

    read_string(&contents)
}

pub fn read_string(contents: &str) -> Result<Config, Box<dyn std::error::Error>> {
    Ok(text_format::parse_from_str(contents)?)
}

impl Config {
    pub fn get_distribution(&self, name: &str) -> Option<&Distribution> {
        self.distribution
            .iter()
            .find(|d| d.name.as_ref().unwrap() == name)
    }

    pub fn get_campaign(&self, name: &str) -> Option<&Campaign> {
        self.campaign
            .iter()
            .find(|c| c.name.as_ref().unwrap() == name)
    }

    pub fn find_campaign_by_branch_name(&self, branch_name: &str) -> Option<(&str, &str)> {
        for campaign in &self.campaign {
            if let Some(campaign_branch_name) = &campaign.branch_name {
                if branch_name == campaign_branch_name {
                    return Some((campaign.name.as_ref().unwrap(), "main"));
                }
            }
        }
        None
    }

    pub async fn pg_pool(&self) -> std::result::Result<sqlx::PgPool, sqlx::Error> {
        if let Some(db_location) = self.database_location.as_ref() {
            sqlx::postgres::PgPool::connect(db_location.as_str()).await
        } else {
            sqlx::postgres::PgPool::connect_with(sqlx::postgres::PgConnectOptions::new()).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_read_file() {
        let config = read_file(std::path::Path::new("janitor.conf.example")).unwrap();
        assert_eq!(config.distribution.len(), 1);
        assert_eq!(config.campaign.len(), 8);
        assert_eq!(config.apt_repository.len(), 1);
    }

    #[test]
    fn test_read_string() {
        let contents = r#"distribution { name: "test" }"#;
        let config = read_string(contents).unwrap();
        assert_eq!(config.distribution.len(), 1);
        assert_eq!(config.distribution[0].name, Some("test".to_string()));
    }

    #[test]
    fn test_read_readable() {
        let contents = r#"distribution { name: "test" }"#;
        let cursor = Cursor::new(contents.as_bytes());
        let config = read_readable(cursor).unwrap();
        assert_eq!(config.distribution.len(), 1);
        assert_eq!(config.distribution[0].name, Some("test".to_string()));
    }

    #[test]
    fn test_get_distribution() {
        let config = read_string(r#"distribution { name: "test" }"#).unwrap();
        assert_eq!(
            config.get_distribution("test").unwrap().name,
            Some("test".to_string())
        );
        assert!(config.get_distribution("test2").is_none());
    }

    #[test]
    fn test_get_campaign() {
        let config = read_string(r#"campaign { name: "test-campaign" }"#).unwrap();
        assert_eq!(
            config.get_campaign("test-campaign").unwrap().name,
            Some("test-campaign".to_string())
        );
        assert!(config.get_campaign("nonexistent").is_none());
    }

    #[test]
    fn test_find_campaign_by_branch_name() {
        let config = read_string(
            r#"campaign { 
                name: "test-campaign" 
                branch_name: "feature-branch"
            }"#,
        )
        .unwrap();

        assert_eq!(
            config.find_campaign_by_branch_name("feature-branch"),
            Some(("test-campaign", "main"))
        );
        assert!(config
            .find_campaign_by_branch_name("nonexistent-branch")
            .is_none());
    }

    #[test]
    fn test_empty_config() {
        let config = read_string("").unwrap();
        assert_eq!(config.distribution.len(), 0);
        assert_eq!(config.campaign.len(), 0);
        assert_eq!(config.apt_repository.len(), 0);
    }

    #[test]
    fn test_invalid_config() {
        let result = read_string("invalid { syntax");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_pg_pool_with_database_location() {
        let config =
            read_string(r#"database_location: "postgresql://localhost/nonexistent""#).unwrap();
        // This should fail to connect but not panic
        let result = config.pg_pool().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_pg_pool_without_database_location() {
        let config = read_string("").unwrap();
        // This should try default connection which will likely fail in tests
        let result = config.pg_pool().await;
        assert!(result.is_err());
    }
}
