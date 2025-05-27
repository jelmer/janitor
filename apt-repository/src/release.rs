//! Release file generation for APT repositories.

use crate::{AptRepositoryError, HashAlgorithm, HashedFile, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// A Release file for an APT repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Release {
    /// Origin of the repository.
    pub origin: Option<String>,
    /// Label for the repository.
    pub label: Option<String>,
    /// Suite name.
    pub suite: Option<String>,
    /// Codename.
    pub codename: Option<String>,
    /// Version.
    pub version: Option<String>,
    /// Date of the release.
    pub date: DateTime<Utc>,
    /// Valid until date.
    pub valid_until: Option<DateTime<Utc>>,
    /// Supported architectures.
    pub architectures: Vec<String>,
    /// Repository components.
    pub components: Vec<String>,
    /// Description.
    pub description: Option<String>,
    /// Whether packages require authentication.
    pub not_automatic: Option<bool>,
    /// Whether automatic upgrades are allowed.
    pub but_automatic_upgrades: Option<bool>,
    /// Whether by-hash is supported.
    pub acquire_by_hash: Option<bool>,
    /// Files in the repository with their hashes.
    pub files: Vec<HashedFile>,
    /// Additional fields not covered by standard fields.
    pub additional_fields: HashMap<String, String>,
}

impl Release {
    /// Create a new Release with the current date.
    pub fn new() -> Self {
        Self {
            origin: None,
            label: None,
            suite: None,
            codename: None,
            version: None,
            date: Utc::now(),
            valid_until: None,
            architectures: Vec::new(),
            components: Vec::new(),
            description: None,
            not_automatic: None,
            but_automatic_upgrades: None,
            acquire_by_hash: None,
            files: Vec::new(),
            additional_fields: HashMap::new(),
        }
    }

    /// Add a file to the release.
    pub fn add_file(&mut self, file: HashedFile) {
        self.files.push(file);
    }

    /// Get files by hash algorithm.
    pub fn get_files_by_hash(&self, algorithm: &HashAlgorithm) -> Vec<(&HashedFile, &str)> {
        self.files
            .iter()
            .filter_map(|file| file.get_hash(algorithm).map(|hash| (file, hash)))
            .collect()
    }

    /// Parse a Release file from a string.
    pub fn from_str(content: &str) -> Result<Self> {
        let mut fields = HashMap::new();
        let mut current_field = None;
        let mut current_value = String::new();

        for line in content.lines() {
            if line.is_empty() {
                continue;
            }

            if line.starts_with(' ') || line.starts_with('\t') {
                // Continuation line
                if current_field.is_some() {
                    current_value.push('\n');
                    current_value.push_str(line);
                }
            } else {
                // New field
                if let Some(field) = current_field.take() {
                    fields.insert(field, current_value);
                    current_value = String::new();
                }

                if let Some((field, value)) = line.split_once(':') {
                    current_field = Some(field.trim().to_lowercase());
                    current_value = value.trim().to_string();
                } else {
                    return Err(AptRepositoryError::invalid_config(format!(
                        "Invalid line format: {}",
                        line
                    )));
                }
            }
        }

        // Don't forget the last field
        if let Some(field) = current_field {
            fields.insert(field, current_value);
        }

        // Parse date (required field)
        let date_str = fields
            .remove("date")
            .ok_or_else(|| AptRepositoryError::missing_field("Date"))?;
        let date = DateTime::parse_from_rfc2822(&date_str)
            .map_err(|_| AptRepositoryError::invalid_field("Date", &date_str))?
            .with_timezone(&Utc);

        // Parse optional date fields
        let valid_until = fields
            .remove("valid-until")
            .and_then(|s| DateTime::parse_from_rfc2822(&s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        // Parse list fields
        let architectures = fields
            .remove("architectures")
            .map(|s| s.split_whitespace().map(|s| s.to_string()).collect())
            .unwrap_or_default();

        let components = fields
            .remove("components")
            .map(|s| s.split_whitespace().map(|s| s.to_string()).collect())
            .unwrap_or_default();

        // Parse boolean fields
        let not_automatic = fields
            .remove("notautomatic")
            .map(|s| s.to_lowercase() == "yes");
        let but_automatic_upgrades = fields
            .remove("butautomaticupgrades")
            .map(|s| s.to_lowercase() == "yes");
        let acquire_by_hash = fields
            .remove("acquire-by-hash")
            .map(|s| s.to_lowercase() == "yes");

        // Parse file lists
        let mut files = Vec::new();
        for algorithm in HashAlgorithm::all() {
            if let Some(file_list) = fields.remove(algorithm.as_str().to_lowercase().as_str()) {
                let parsed_files = Self::parse_file_list(&file_list, *algorithm)?;
                Self::merge_files(&mut files, parsed_files);
            }
        }

        Ok(Self {
            origin: fields.remove("origin"),
            label: fields.remove("label"),
            suite: fields.remove("suite"),
            codename: fields.remove("codename"),
            version: fields.remove("version"),
            date,
            valid_until,
            architectures,
            components,
            description: fields.remove("description"),
            not_automatic,
            but_automatic_upgrades,
            acquire_by_hash,
            files,
            additional_fields: fields,
        })
    }

    /// Parse a file list from a hash field.
    fn parse_file_list(content: &str, algorithm: HashAlgorithm) -> Result<Vec<HashedFile>> {
        let mut files = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() != 3 {
                return Err(AptRepositoryError::invalid_config(format!(
                    "Invalid file list line: {}",
                    line
                )));
            }

            let hash = parts[0].to_string();
            let size = parts[1]
                .parse::<u64>()
                .map_err(|_| AptRepositoryError::invalid_field("size", parts[1]))?;
            let path = parts[2].to_string();

            let mut file = HashedFile::new(path, size);
            file.add_hash(algorithm, hash);
            files.push(file);
        }

        Ok(files)
    }

    /// Merge files with the same path.
    fn merge_files(existing: &mut Vec<HashedFile>, new_files: Vec<HashedFile>) {
        for new_file in new_files {
            if let Some(existing_file) = existing.iter_mut().find(|f| f.path == new_file.path) {
                // Merge hashes
                for (algorithm, hash) in new_file.hashes.iter() {
                    existing_file.add_hash(*algorithm, hash.to_string());
                }
            } else {
                existing.push(new_file);
            }
        }
    }

    /// Convert the Release to a string.
    pub fn to_string(&self) -> String {
        let mut content = String::new();

        // Standard fields
        if let Some(ref origin) = self.origin {
            content.push_str(&format!("Origin: {}\n", origin));
        }
        if let Some(ref label) = self.label {
            content.push_str(&format!("Label: {}\n", label));
        }
        if let Some(ref suite) = self.suite {
            content.push_str(&format!("Suite: {}\n", suite));
        }
        if let Some(ref codename) = self.codename {
            content.push_str(&format!("Codename: {}\n", codename));
        }
        if let Some(ref version) = self.version {
            content.push_str(&format!("Version: {}\n", version));
        }

        content.push_str(&format!(
            "Date: {}\n",
            self.date.format("%a, %d %b %Y %H:%M:%S %z")
        ));

        if let Some(valid_until) = self.valid_until {
            content.push_str(&format!(
                "Valid-Until: {}\n",
                valid_until.format("%a, %d %b %Y %H:%M:%S %z")
            ));
        }

        if !self.architectures.is_empty() {
            content.push_str(&format!(
                "Architectures: {}\n",
                self.architectures.join(" ")
            ));
        }

        if !self.components.is_empty() {
            content.push_str(&format!("Components: {}\n", self.components.join(" ")));
        }

        if let Some(ref description) = self.description {
            content.push_str(&format!("Description: {}\n", description));
        }

        if let Some(not_automatic) = self.not_automatic {
            content.push_str(&format!(
                "NotAutomatic: {}\n",
                if not_automatic { "yes" } else { "no" }
            ));
        }

        if let Some(but_automatic_upgrades) = self.but_automatic_upgrades {
            content.push_str(&format!(
                "ButAutomaticUpgrades: {}\n",
                if but_automatic_upgrades { "yes" } else { "no" }
            ));
        }

        if let Some(acquire_by_hash) = self.acquire_by_hash {
            content.push_str(&format!(
                "Acquire-By-Hash: {}\n",
                if acquire_by_hash { "yes" } else { "no" }
            ));
        }

        // Additional fields
        for (key, value) in &self.additional_fields {
            content.push_str(&format!("{}: {}\n", key, value));
        }

        // File lists for each hash algorithm
        for algorithm in HashAlgorithm::all() {
            let files_with_hash = self.get_files_by_hash(algorithm);
            if !files_with_hash.is_empty() {
                content.push_str(&format!("{}:\n", algorithm.as_str()));
                for (file, hash) in files_with_hash {
                    content.push_str(&format!(" {} {} {}\n", hash, file.size, file.path));
                }
            }
        }

        content
    }
}

impl Default for Release {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Release {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

/// Builder for creating Release files.
#[derive(Debug, Clone)]
pub struct ReleaseBuilder {
    release: Release,
}

impl ReleaseBuilder {
    /// Create a new Release builder.
    pub fn new() -> Self {
        Self {
            release: Release::new(),
        }
    }

    /// Set the origin.
    pub fn origin<S: Into<String>>(mut self, origin: S) -> Self {
        self.release.origin = Some(origin.into());
        self
    }

    /// Set the label.
    pub fn label<S: Into<String>>(mut self, label: S) -> Self {
        self.release.label = Some(label.into());
        self
    }

    /// Set the suite.
    pub fn suite<S: Into<String>>(mut self, suite: S) -> Self {
        self.release.suite = Some(suite.into());
        self
    }

    /// Set the codename.
    pub fn codename<S: Into<String>>(mut self, codename: S) -> Self {
        self.release.codename = Some(codename.into());
        self
    }

    /// Set the version.
    pub fn version<S: Into<String>>(mut self, version: S) -> Self {
        self.release.version = Some(version.into());
        self
    }

    /// Set the date.
    pub fn date(mut self, date: DateTime<Utc>) -> Self {
        self.release.date = date;
        self
    }

    /// Set the valid until date.
    pub fn valid_until(mut self, valid_until: DateTime<Utc>) -> Self {
        self.release.valid_until = Some(valid_until);
        self
    }

    /// Set the architectures.
    pub fn architectures(mut self, architectures: Vec<String>) -> Self {
        self.release.architectures = architectures;
        self
    }

    /// Set the components.
    pub fn components(mut self, components: Vec<String>) -> Self {
        self.release.components = components;
        self
    }

    /// Set the description.
    pub fn description<S: Into<String>>(mut self, description: S) -> Self {
        self.release.description = Some(description.into());
        self
    }

    /// Set not automatic flag.
    pub fn not_automatic(mut self, not_automatic: bool) -> Self {
        self.release.not_automatic = Some(not_automatic);
        self
    }

    /// Set but automatic upgrades flag.
    pub fn but_automatic_upgrades(mut self, but_automatic_upgrades: bool) -> Self {
        self.release.but_automatic_upgrades = Some(but_automatic_upgrades);
        self
    }

    /// Set acquire by hash flag.
    pub fn acquire_by_hash(mut self, acquire_by_hash: bool) -> Self {
        self.release.acquire_by_hash = Some(acquire_by_hash);
        self
    }

    /// Add a file.
    pub fn add_file(mut self, file: HashedFile) -> Self {
        self.release.add_file(file);
        self
    }

    /// Add an additional field.
    pub fn additional_field<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.release
            .additional_fields
            .insert(key.into(), value.into());
        self
    }

    /// Build the Release.
    pub fn build(self) -> Result<Release> {
        Ok(self.release)
    }
}

impl Default for ReleaseBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HashAlgorithm, HashSet};

    #[test]
    fn test_release_builder() {
        let release = ReleaseBuilder::new()
            .origin("Test Origin")
            .label("Test Repository")
            .suite("stable")
            .codename("stable")
            .architectures(vec!["amd64".to_string(), "i386".to_string()])
            .components(vec!["main".to_string()])
            .description("Test repository")
            .not_automatic(true)
            .acquire_by_hash(true)
            .build()
            .unwrap();

        assert_eq!(release.origin, Some("Test Origin".to_string()));
        assert_eq!(release.label, Some("Test Repository".to_string()));
        assert_eq!(release.suite, Some("stable".to_string()));
        assert_eq!(release.codename, Some("stable".to_string()));
        assert_eq!(release.architectures, vec!["amd64", "i386"]);
        assert_eq!(release.components, vec!["main"]);
        assert_eq!(release.description, Some("Test repository".to_string()));
        assert_eq!(release.not_automatic, Some(true));
        assert_eq!(release.acquire_by_hash, Some(true));
    }

    #[test]
    fn test_release_roundtrip() {
        let mut release = Release::new();
        release.origin = Some("Test Origin".to_string());
        release.suite = Some("stable".to_string());
        release.architectures = vec!["amd64".to_string()];
        release.components = vec!["main".to_string()];

        let mut file = HashedFile::new("main/binary-amd64/Packages", 1024);
        file.add_hash(HashAlgorithm::Md5, "abc123".to_string());
        file.add_hash(HashAlgorithm::Sha256, "def456".to_string());
        release.add_file(file);

        let content = release.to_string();
        let parsed = Release::from_str(&content).unwrap();

        assert_eq!(release.origin, parsed.origin);
        assert_eq!(release.suite, parsed.suite);
        assert_eq!(release.architectures, parsed.architectures);
        assert_eq!(release.components, parsed.components);
        assert_eq!(release.files.len(), parsed.files.len());

        if !release.files.is_empty() {
            let orig_file = &release.files[0];
            let parsed_file = &parsed.files[0];
            assert_eq!(orig_file.path, parsed_file.path);
            assert_eq!(orig_file.size, parsed_file.size);
            assert_eq!(
                orig_file.get_hash(&HashAlgorithm::Md5),
                parsed_file.get_hash(&HashAlgorithm::Md5)
            );
        }
    }

    #[test]
    fn test_file_merging() {
        let mut files = Vec::new();

        let mut file1 = HashedFile::new("test.txt", 1024);
        file1.add_hash(HashAlgorithm::Md5, "abc123".to_string());
        files.push(file1);

        let mut new_files = Vec::new();
        let mut file2 = HashedFile::new("test.txt", 1024);
        file2.add_hash(HashAlgorithm::Sha256, "def456".to_string());
        new_files.push(file2);

        Release::merge_files(&mut files, new_files);

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "test.txt");
        assert_eq!(files[0].get_hash(&HashAlgorithm::Md5), Some("abc123"));
        assert_eq!(files[0].get_hash(&HashAlgorithm::Sha256), Some("def456"));
    }
}
