include!(concat!(env!("OUT_DIR"), "/generated/mod.rs"));

use protobuf::text_format;
use std::fs::File;
use std::io::Read;

pub use config::{AptRepository, Campaign, Config, Distribution};

pub fn read_file(file_path: &std::path::Path) -> Result<Config, Box<dyn std::error::Error>> {
    let mut file = File::open(file_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_file() {
        let config = read_file(std::path::Path::new("janitor.conf.example")).unwrap();
        assert_eq!(config.distribution.len(), 1);
        assert_eq!(config.campaign.len(), 8);
        assert_eq!(config.apt_repository.len(), 1);
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
}
