//! Package file parsing and generation for APT repositories.

use crate::{AptRepositoryError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// A Debian binary package entry in a Packages file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Package {
    /// Package name.
    pub package: String,
    /// Package version.
    pub version: String,
    /// Architecture.
    pub architecture: String,
    /// Maintainer.
    pub maintainer: Option<String>,
    /// Installed size in kilobytes.
    pub installed_size: Option<u64>,
    /// Package dependencies.
    pub depends: Option<String>,
    /// Package pre-dependencies.
    pub pre_depends: Option<String>,
    /// Package recommendations.
    pub recommends: Option<String>,
    /// Package suggestions.
    pub suggests: Option<String>,
    /// Package conflicts.
    pub conflicts: Option<String>,
    /// Package breaks.
    pub breaks: Option<String>,
    /// Package replaces.
    pub replaces: Option<String>,
    /// Package provides.
    pub provides: Option<String>,
    /// Package section.
    pub section: Option<String>,
    /// Package priority.
    pub priority: Option<String>,
    /// Package homepage.
    pub homepage: Option<String>,
    /// Package description (short).
    pub description: Option<String>,
    /// Package description (long).
    pub description_long: Option<String>,
    /// Package tags.
    pub tag: Option<String>,
    /// Filename (relative to repository root).
    pub filename: String,
    /// File size in bytes.
    pub size: u64,
    /// MD5 hash.
    pub md5sum: Option<String>,
    /// SHA1 hash.
    pub sha1: Option<String>,
    /// SHA256 hash.
    pub sha256: Option<String>,
    /// SHA512 hash.
    pub sha512: Option<String>,
    /// Additional fields not covered by standard fields.
    pub additional_fields: HashMap<String, String>,
}

impl Package {
    /// Create a new package with required fields.
    pub fn new<S: Into<String>>(
        package: S,
        version: S,
        architecture: S,
        filename: S,
        size: u64,
    ) -> Self {
        Self {
            package: package.into(),
            version: version.into(),
            architecture: architecture.into(),
            filename: filename.into(),
            size,
            maintainer: None,
            installed_size: None,
            depends: None,
            pre_depends: None,
            recommends: None,
            suggests: None,
            conflicts: None,
            breaks: None,
            replaces: None,
            provides: None,
            section: None,
            priority: None,
            homepage: None,
            description: None,
            description_long: None,
            tag: None,
            md5sum: None,
            sha1: None,
            sha256: None,
            sha512: None,
            additional_fields: HashMap::new(),
        }
    }

    /// Parse a package from a control file paragraph.
    pub fn from_paragraph(paragraph: &str) -> Result<Self> {
        let mut fields = HashMap::new();
        let mut current_field = None;
        let mut current_value = String::new();

        for line in paragraph.lines() {
            if line.is_empty() {
                continue;
            }

            if line.starts_with(' ') || line.starts_with('\t') {
                // Continuation line
                if current_field.is_some() {
                    current_value.push('\n');
                    current_value.push_str(line.trim_start());
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
                    return Err(AptRepositoryError::invalid_package(
                        format!("Invalid line format: {}", line)
                    ));
                }
            }
        }

        // Don't forget the last field
        if let Some(field) = current_field {
            fields.insert(field, current_value);
        }

        // Extract required fields
        let package = fields.remove("package")
            .ok_or_else(|| AptRepositoryError::missing_field("Package"))?;
        let version = fields.remove("version")
            .ok_or_else(|| AptRepositoryError::missing_field("Version"))?;
        let architecture = fields.remove("architecture")
            .ok_or_else(|| AptRepositoryError::missing_field("Architecture"))?;
        let filename = fields.remove("filename")
            .ok_or_else(|| AptRepositoryError::missing_field("Filename"))?;
        
        let size_str = fields.remove("size")
            .ok_or_else(|| AptRepositoryError::missing_field("Size"))?;
        let size = size_str.parse::<u64>()
            .map_err(|_| AptRepositoryError::invalid_field("Size", &size_str))?;

        // Extract optional fields
        let installed_size = fields.remove("installed-size")
            .and_then(|s| s.parse().ok());

        // Handle description specially (split short and long)
        let (description, description_long) = if let Some(desc) = fields.remove("description") {
            if let Some((short, long)) = desc.split_once('\n') {
                (Some(short.to_string()), Some(long.to_string()))
            } else {
                (Some(desc), None)
            }
        } else {
            (None, None)
        };

        Ok(Self {
            package,
            version,
            architecture,
            filename,
            size,
            maintainer: fields.remove("maintainer"),
            installed_size,
            depends: fields.remove("depends"),
            pre_depends: fields.remove("pre-depends"),
            recommends: fields.remove("recommends"),
            suggests: fields.remove("suggests"),
            conflicts: fields.remove("conflicts"),
            breaks: fields.remove("breaks"),
            replaces: fields.remove("replaces"),
            provides: fields.remove("provides"),
            section: fields.remove("section"),
            priority: fields.remove("priority"),
            homepage: fields.remove("homepage"),
            description,
            description_long,
            tag: fields.remove("tag"),
            md5sum: fields.remove("md5sum"),
            sha1: fields.remove("sha1"),
            sha256: fields.remove("sha256"),
            sha512: fields.remove("sha512"),
            additional_fields: fields,
        })
    }

    /// Convert the package to a control file paragraph.
    pub fn to_paragraph(&self) -> String {
        let mut paragraph = String::new();

        // Required fields
        paragraph.push_str(&format!("Package: {}\n", self.package));
        paragraph.push_str(&format!("Version: {}\n", self.version));
        paragraph.push_str(&format!("Architecture: {}\n", self.architecture));

        // Optional fields in order
        if let Some(ref maintainer) = self.maintainer {
            paragraph.push_str(&format!("Maintainer: {}\n", maintainer));
        }
        if let Some(installed_size) = self.installed_size {
            paragraph.push_str(&format!("Installed-Size: {}\n", installed_size));
        }
        if let Some(ref depends) = self.depends {
            paragraph.push_str(&format!("Depends: {}\n", depends));
        }
        if let Some(ref pre_depends) = self.pre_depends {
            paragraph.push_str(&format!("Pre-Depends: {}\n", pre_depends));
        }
        if let Some(ref recommends) = self.recommends {
            paragraph.push_str(&format!("Recommends: {}\n", recommends));
        }
        if let Some(ref suggests) = self.suggests {
            paragraph.push_str(&format!("Suggests: {}\n", suggests));
        }
        if let Some(ref conflicts) = self.conflicts {
            paragraph.push_str(&format!("Conflicts: {}\n", conflicts));
        }
        if let Some(ref breaks) = self.breaks {
            paragraph.push_str(&format!("Breaks: {}\n", breaks));
        }
        if let Some(ref replaces) = self.replaces {
            paragraph.push_str(&format!("Replaces: {}\n", replaces));
        }
        if let Some(ref provides) = self.provides {
            paragraph.push_str(&format!("Provides: {}\n", provides));
        }
        if let Some(ref section) = self.section {
            paragraph.push_str(&format!("Section: {}\n", section));
        }
        if let Some(ref priority) = self.priority {
            paragraph.push_str(&format!("Priority: {}\n", priority));
        }
        if let Some(ref homepage) = self.homepage {
            paragraph.push_str(&format!("Homepage: {}\n", homepage));
        }

        // Description (combine short and long)
        if let Some(ref description) = self.description {
            paragraph.push_str(&format!("Description: {}", description));
            if let Some(ref long_desc) = self.description_long {
                paragraph.push_str(&format!("\n{}", long_desc));
            }
            paragraph.push('\n');
        }

        if let Some(ref tag) = self.tag {
            paragraph.push_str(&format!("Tag: {}\n", tag));
        }

        // File information
        paragraph.push_str(&format!("Filename: {}\n", self.filename));
        paragraph.push_str(&format!("Size: {}\n", self.size));

        // Hashes
        if let Some(ref md5sum) = self.md5sum {
            paragraph.push_str(&format!("MD5sum: {}\n", md5sum));
        }
        if let Some(ref sha1) = self.sha1 {
            paragraph.push_str(&format!("SHA1: {}\n", sha1));
        }
        if let Some(ref sha256) = self.sha256 {
            paragraph.push_str(&format!("SHA256: {}\n", sha256));
        }
        if let Some(ref sha512) = self.sha512 {
            paragraph.push_str(&format!("SHA512: {}\n", sha512));
        }

        // Additional fields
        for (key, value) in &self.additional_fields {
            paragraph.push_str(&format!("{}: {}\n", key, value));
        }

        paragraph
    }
}

impl fmt::Display for Package {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_paragraph())
    }
}

/// A collection of packages that can be written to a Packages file.
#[derive(Debug, Clone)]
pub struct PackageFile {
    packages: Vec<Package>,
}

impl PackageFile {
    /// Create a new empty package file.
    pub fn new() -> Self {
        Self {
            packages: Vec::new(),
        }
    }

    /// Add a package to the file.
    pub fn add_package(&mut self, package: Package) {
        self.packages.push(package);
    }

    /// Get all packages.
    pub fn packages(&self) -> &[Package] {
        &self.packages
    }

    /// Get a mutable reference to all packages.
    pub fn packages_mut(&mut self) -> &mut Vec<Package> {
        &mut self.packages
    }

    /// Parse a Packages file from a string.
    pub fn from_str(content: &str) -> Result<Self> {
        let mut packages = Vec::new();
        let mut current_paragraph = String::new();

        for line in content.lines() {
            if line.trim().is_empty() {
                if !current_paragraph.trim().is_empty() {
                    packages.push(Package::from_paragraph(&current_paragraph)?);
                    current_paragraph.clear();
                }
            } else {
                current_paragraph.push_str(line);
                current_paragraph.push('\n');
            }
        }

        // Don't forget the last paragraph
        if !current_paragraph.trim().is_empty() {
            packages.push(Package::from_paragraph(&current_paragraph)?);
        }

        Ok(Self { packages })
    }

    /// Convert the package file to a string.
    pub fn to_string(&self) -> String {
        let mut content = String::new();
        
        for (i, package) in self.packages.iter().enumerate() {
            if i > 0 {
                content.push('\n');
            }
            content.push_str(&package.to_paragraph());
        }

        content
    }

    /// Get the number of packages.
    pub fn len(&self) -> usize {
        self.packages.len()
    }

    /// Check if the package file is empty.
    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }

    /// Sort packages by name and version.
    pub fn sort(&mut self) {
        self.packages.sort_by(|a, b| {
            a.package.cmp(&b.package)
                .then_with(|| a.version.cmp(&b.version))
        });
    }
}

impl Default for PackageFile {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for PackageFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_creation() {
        let package = Package::new("test-package", "1.0.0", "amd64", "pool/main/t/test_1.0.0_amd64.deb", 1024);
        
        assert_eq!(package.package, "test-package");
        assert_eq!(package.version, "1.0.0");
        assert_eq!(package.architecture, "amd64");
        assert_eq!(package.filename, "pool/main/t/test_1.0.0_amd64.deb");
        assert_eq!(package.size, 1024);
    }

    #[test]
    fn test_package_paragraph_roundtrip() {
        let mut package = Package::new("test-package", "1.0.0", "amd64", "test.deb", 1024);
        package.maintainer = Some("Test Maintainer <test@example.com>".to_string());
        package.description = Some("A test package".to_string());
        package.md5sum = Some("abc123".to_string());

        let paragraph = package.to_paragraph();
        let parsed = Package::from_paragraph(&paragraph).unwrap();

        assert_eq!(package.package, parsed.package);
        assert_eq!(package.version, parsed.version);
        assert_eq!(package.architecture, parsed.architecture);
        assert_eq!(package.maintainer, parsed.maintainer);
        assert_eq!(package.description, parsed.description);
        assert_eq!(package.md5sum, parsed.md5sum);
    }

    #[test]
    fn test_package_file() {
        let mut package_file = PackageFile::new();
        assert!(package_file.is_empty());
        assert_eq!(package_file.len(), 0);

        let package1 = Package::new("package-a", "1.0.0", "amd64", "a.deb", 1024);
        let package2 = Package::new("package-b", "2.0.0", "amd64", "b.deb", 2048);

        package_file.add_package(package1);
        package_file.add_package(package2);

        assert!(!package_file.is_empty());
        assert_eq!(package_file.len(), 2);

        let content = package_file.to_string();
        let parsed = PackageFile::from_str(&content).unwrap();

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed.packages()[0].package, "package-a");
        assert_eq!(parsed.packages()[1].package, "package-b");
    }

    #[test]
    fn test_package_sorting() {
        let mut package_file = PackageFile::new();
        
        let package1 = Package::new("zpackage", "1.0.0", "amd64", "z.deb", 1024);
        let package2 = Package::new("apackage", "2.0.0", "amd64", "a.deb", 2048);
        let package3 = Package::new("apackage", "1.0.0", "amd64", "a1.deb", 1024);

        package_file.add_package(package1);
        package_file.add_package(package2);
        package_file.add_package(package3);

        package_file.sort();

        let packages = package_file.packages();
        assert_eq!(packages[0].package, "apackage");
        assert_eq!(packages[0].version, "1.0.0");
        assert_eq!(packages[1].package, "apackage");
        assert_eq!(packages[1].version, "2.0.0");
        assert_eq!(packages[2].package, "zpackage");
    }
}