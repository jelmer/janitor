//! Source package file parsing and generation for APT repositories.

use crate::{AptRepositoryError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// A Debian source package entry in a Sources file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    /// Package name.
    pub package: String,
    /// Binary package names produced by this source.
    pub binary: Option<String>,
    /// Package version.
    pub version: String,
    /// Maintainer.
    pub maintainer: Option<String>,
    /// Uploaders.
    pub uploaders: Option<String>,
    /// Architecture.
    pub architecture: String,
    /// Standards version.
    pub standards_version: Option<String>,
    /// Format version.
    pub format: Option<String>,
    /// Build dependencies.
    pub build_depends: Option<String>,
    /// Build dependencies (architecture-independent).
    pub build_depends_indep: Option<String>,
    /// Build dependencies (architecture-dependent).
    pub build_depends_arch: Option<String>,
    /// Build conflicts.
    pub build_conflicts: Option<String>,
    /// Build conflicts (architecture-independent).
    pub build_conflicts_indep: Option<String>,
    /// Build conflicts (architecture-dependent).
    pub build_conflicts_arch: Option<String>,
    /// Package section.
    pub section: Option<String>,
    /// Package priority.
    pub priority: Option<String>,
    /// Package homepage.
    pub homepage: Option<String>,
    /// Version control system information.
    pub vcs_browser: Option<String>,
    /// VCS Git repository.
    pub vcs_git: Option<String>,
    /// VCS Bazaar repository.
    pub vcs_bzr: Option<String>,
    /// VCS Subversion repository.
    pub vcs_svn: Option<String>,
    /// VCS Mercurial repository.
    pub vcs_hg: Option<String>,
    /// VCS CVS repository.
    pub vcs_cvs: Option<String>,
    /// VCS Arch repository.
    pub vcs_arch: Option<String>,
    /// VCS Darcs repository.
    pub vcs_darcs: Option<String>,
    /// Directory (relative to repository root).
    pub directory: String,
    /// Files that make up this source package.
    pub files: Vec<SourceFileEntry>,
    /// Checksums for SHA1.
    pub checksums_sha1: Vec<SourceFileEntry>,
    /// Checksums for SHA256.
    pub checksums_sha256: Vec<SourceFileEntry>,
    /// Checksums for SHA512.
    pub checksums_sha512: Vec<SourceFileEntry>,
    /// Additional fields not covered by standard fields.
    pub additional_fields: HashMap<String, String>,
}

/// A file entry in a source package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceFileEntry {
    /// Hash value.
    pub hash: String,
    /// File size in bytes.
    pub size: u64,
    /// Filename.
    pub name: String,
}

impl SourceFileEntry {
    /// Create a new source file entry.
    pub fn new<S: Into<String>>(hash: S, size: u64, name: S) -> Self {
        Self {
            hash: hash.into(),
            size,
            name: name.into(),
        }
    }

    /// Parse a file entry from a checksum line.
    pub fn from_checksum_line(line: &str) -> Result<Self> {
        let parts: Vec<&str> = line.trim().split_whitespace().collect();
        if parts.len() != 3 {
            return Err(AptRepositoryError::invalid_source(
                format!("Invalid checksum line format: {}", line)
            ));
        }

        let hash = parts[0].to_string();
        let size = parts[1].parse::<u64>()
            .map_err(|_| AptRepositoryError::invalid_source(
                format!("Invalid size in checksum line: {}", parts[1])
            ))?;
        let name = parts[2].to_string();

        Ok(Self { hash, size, name })
    }

    /// Convert to a checksum line format.
    pub fn to_checksum_line(&self) -> String {
        format!(" {} {} {}", self.hash, self.size, self.name)
    }
}

impl Source {
    /// Create a new source package with required fields.
    pub fn new<S: Into<String>>(
        package: S,
        version: S,
        architecture: S,
        directory: S,
    ) -> Self {
        Self {
            package: package.into(),
            version: version.into(),
            architecture: architecture.into(),
            directory: directory.into(),
            binary: None,
            maintainer: None,
            uploaders: None,
            standards_version: None,
            format: None,
            build_depends: None,
            build_depends_indep: None,
            build_depends_arch: None,
            build_conflicts: None,
            build_conflicts_indep: None,
            build_conflicts_arch: None,
            section: None,
            priority: None,
            homepage: None,
            vcs_browser: None,
            vcs_git: None,
            vcs_bzr: None,
            vcs_svn: None,
            vcs_hg: None,
            vcs_cvs: None,
            vcs_arch: None,
            vcs_darcs: None,
            files: Vec::new(),
            checksums_sha1: Vec::new(),
            checksums_sha256: Vec::new(),
            checksums_sha512: Vec::new(),
            additional_fields: HashMap::new(),
        }
    }

    /// Parse a source package from a control file paragraph.
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
                    return Err(AptRepositoryError::invalid_source(
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
        let directory = fields.remove("directory")
            .ok_or_else(|| AptRepositoryError::missing_field("Directory"))?;

        // Parse file lists
        let files = Self::parse_file_list(&fields.remove("files").unwrap_or_default())?;
        let checksums_sha1 = Self::parse_file_list(&fields.remove("checksums-sha1").unwrap_or_default())?;
        let checksums_sha256 = Self::parse_file_list(&fields.remove("checksums-sha256").unwrap_or_default())?;
        let checksums_sha512 = Self::parse_file_list(&fields.remove("checksums-sha512").unwrap_or_default())?;

        Ok(Self {
            package,
            version,
            architecture,
            directory,
            binary: fields.remove("binary"),
            maintainer: fields.remove("maintainer"),
            uploaders: fields.remove("uploaders"),
            standards_version: fields.remove("standards-version"),
            format: fields.remove("format"),
            build_depends: fields.remove("build-depends"),
            build_depends_indep: fields.remove("build-depends-indep"),
            build_depends_arch: fields.remove("build-depends-arch"),
            build_conflicts: fields.remove("build-conflicts"),
            build_conflicts_indep: fields.remove("build-conflicts-indep"),
            build_conflicts_arch: fields.remove("build-conflicts-arch"),
            section: fields.remove("section"),
            priority: fields.remove("priority"),
            homepage: fields.remove("homepage"),
            vcs_browser: fields.remove("vcs-browser"),
            vcs_git: fields.remove("vcs-git"),
            vcs_bzr: fields.remove("vcs-bzr"),
            vcs_svn: fields.remove("vcs-svn"),
            vcs_hg: fields.remove("vcs-hg"),
            vcs_cvs: fields.remove("vcs-cvs"),
            vcs_arch: fields.remove("vcs-arch"),
            vcs_darcs: fields.remove("vcs-darcs"),
            files,
            checksums_sha1,
            checksums_sha256,
            checksums_sha512,
            additional_fields: fields,
        })
    }

    /// Parse a file list from a multi-line field.
    fn parse_file_list(content: &str) -> Result<Vec<SourceFileEntry>> {
        let mut entries = Vec::new();
        
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            entries.push(SourceFileEntry::from_checksum_line(line)?);
        }

        Ok(entries)
    }

    /// Convert a file list to a multi-line field value.
    fn format_file_list(entries: &[SourceFileEntry]) -> String {
        if entries.is_empty() {
            return String::new();
        }

        let mut result = String::new();
        for entry in entries {
            result.push_str(&entry.to_checksum_line());
            result.push('\n');
        }
        // Remove the trailing newline
        result.pop();
        result
    }

    /// Convert the source package to a control file paragraph.
    pub fn to_paragraph(&self) -> String {
        let mut paragraph = String::new();

        // Required fields
        paragraph.push_str(&format!("Package: {}\n", self.package));
        if let Some(ref binary) = self.binary {
            paragraph.push_str(&format!("Binary: {}\n", binary));
        }
        paragraph.push_str(&format!("Version: {}\n", self.version));

        // Optional fields in order
        if let Some(ref maintainer) = self.maintainer {
            paragraph.push_str(&format!("Maintainer: {}\n", maintainer));
        }
        if let Some(ref uploaders) = self.uploaders {
            paragraph.push_str(&format!("Uploaders: {}\n", uploaders));
        }

        paragraph.push_str(&format!("Architecture: {}\n", self.architecture));

        if let Some(ref standards_version) = self.standards_version {
            paragraph.push_str(&format!("Standards-Version: {}\n", standards_version));
        }
        if let Some(ref format) = self.format {
            paragraph.push_str(&format!("Format: {}\n", format));
        }

        // Build dependencies
        if let Some(ref build_depends) = self.build_depends {
            paragraph.push_str(&format!("Build-Depends: {}\n", build_depends));
        }
        if let Some(ref build_depends_indep) = self.build_depends_indep {
            paragraph.push_str(&format!("Build-Depends-Indep: {}\n", build_depends_indep));
        }
        if let Some(ref build_depends_arch) = self.build_depends_arch {
            paragraph.push_str(&format!("Build-Depends-Arch: {}\n", build_depends_arch));
        }
        if let Some(ref build_conflicts) = self.build_conflicts {
            paragraph.push_str(&format!("Build-Conflicts: {}\n", build_conflicts));
        }
        if let Some(ref build_conflicts_indep) = self.build_conflicts_indep {
            paragraph.push_str(&format!("Build-Conflicts-Indep: {}\n", build_conflicts_indep));
        }
        if let Some(ref build_conflicts_arch) = self.build_conflicts_arch {
            paragraph.push_str(&format!("Build-Conflicts-Arch: {}\n", build_conflicts_arch));
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

        // VCS fields
        if let Some(ref vcs_browser) = self.vcs_browser {
            paragraph.push_str(&format!("Vcs-Browser: {}\n", vcs_browser));
        }
        if let Some(ref vcs_git) = self.vcs_git {
            paragraph.push_str(&format!("Vcs-Git: {}\n", vcs_git));
        }
        if let Some(ref vcs_bzr) = self.vcs_bzr {
            paragraph.push_str(&format!("Vcs-Bzr: {}\n", vcs_bzr));
        }
        if let Some(ref vcs_svn) = self.vcs_svn {
            paragraph.push_str(&format!("Vcs-Svn: {}\n", vcs_svn));
        }
        if let Some(ref vcs_hg) = self.vcs_hg {
            paragraph.push_str(&format!("Vcs-Hg: {}\n", vcs_hg));
        }
        if let Some(ref vcs_cvs) = self.vcs_cvs {
            paragraph.push_str(&format!("Vcs-Cvs: {}\n", vcs_cvs));
        }
        if let Some(ref vcs_arch) = self.vcs_arch {
            paragraph.push_str(&format!("Vcs-Arch: {}\n", vcs_arch));
        }
        if let Some(ref vcs_darcs) = self.vcs_darcs {
            paragraph.push_str(&format!("Vcs-Darcs: {}\n", vcs_darcs));
        }

        // Directory
        paragraph.push_str(&format!("Directory: {}\n", self.directory));

        // File lists
        if !self.files.is_empty() {
            paragraph.push_str(&format!("Files:\n{}\n", Self::format_file_list(&self.files)));
        }
        if !self.checksums_sha1.is_empty() {
            paragraph.push_str(&format!("Checksums-Sha1:\n{}\n", Self::format_file_list(&self.checksums_sha1)));
        }
        if !self.checksums_sha256.is_empty() {
            paragraph.push_str(&format!("Checksums-Sha256:\n{}\n", Self::format_file_list(&self.checksums_sha256)));
        }
        if !self.checksums_sha512.is_empty() {
            paragraph.push_str(&format!("Checksums-Sha512:\n{}\n", Self::format_file_list(&self.checksums_sha512)));
        }

        // Additional fields
        for (key, value) in &self.additional_fields {
            paragraph.push_str(&format!("{}: {}\n", key, value));
        }

        paragraph
    }
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_paragraph())
    }
}

/// A collection of source packages that can be written to a Sources file.
#[derive(Debug, Clone)]
pub struct SourceFile {
    sources: Vec<Source>,
}

impl SourceFile {
    /// Create a new empty source file.
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
        }
    }

    /// Add a source package to the file.
    pub fn add_source(&mut self, source: Source) {
        self.sources.push(source);
    }

    /// Get all source packages.
    pub fn sources(&self) -> &[Source] {
        &self.sources
    }

    /// Get a mutable reference to all source packages.
    pub fn sources_mut(&mut self) -> &mut Vec<Source> {
        &mut self.sources
    }

    /// Parse a Sources file from a string.
    pub fn from_str(content: &str) -> Result<Self> {
        let mut sources = Vec::new();
        let mut current_paragraph = String::new();

        for line in content.lines() {
            if line.trim().is_empty() {
                if !current_paragraph.trim().is_empty() {
                    sources.push(Source::from_paragraph(&current_paragraph)?);
                    current_paragraph.clear();
                }
            } else {
                current_paragraph.push_str(line);
                current_paragraph.push('\n');
            }
        }

        // Don't forget the last paragraph
        if !current_paragraph.trim().is_empty() {
            sources.push(Source::from_paragraph(&current_paragraph)?);
        }

        Ok(Self { sources })
    }

    /// Convert the source file to a string.
    pub fn to_string(&self) -> String {
        let mut content = String::new();
        
        for (i, source) in self.sources.iter().enumerate() {
            if i > 0 {
                content.push('\n');
            }
            content.push_str(&source.to_paragraph());
        }

        content
    }

    /// Get the number of source packages.
    pub fn len(&self) -> usize {
        self.sources.len()
    }

    /// Check if the source file is empty.
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }

    /// Sort source packages by name and version.
    pub fn sort(&mut self) {
        self.sources.sort_by(|a, b| {
            a.package.cmp(&b.package)
                .then_with(|| a.version.cmp(&b.version))
        });
    }
}

impl Default for SourceFile {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SourceFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_file_entry() {
        let entry = SourceFileEntry::new("abc123", 1024, "test.dsc");
        assert_eq!(entry.hash, "abc123");
        assert_eq!(entry.size, 1024);
        assert_eq!(entry.name, "test.dsc");

        let line = entry.to_checksum_line();
        assert_eq!(line, " abc123 1024 test.dsc");

        let parsed = SourceFileEntry::from_checksum_line(&line).unwrap();
        assert_eq!(parsed, entry);
    }

    #[test]
    fn test_source_creation() {
        let source = Source::new("test-package", "1.0.0", "any", "pool/main/t/test");
        
        assert_eq!(source.package, "test-package");
        assert_eq!(source.version, "1.0.0");
        assert_eq!(source.architecture, "any");
        assert_eq!(source.directory, "pool/main/t/test");
    }

    #[test]
    fn test_source_paragraph_roundtrip() {
        let mut source = Source::new("test-package", "1.0.0", "any", "pool/main/t/test");
        source.maintainer = Some("Test Maintainer <test@example.com>".to_string());
        source.build_depends = Some("debhelper (>= 10)".to_string());
        source.files.push(SourceFileEntry::new("abc123", 1024, "test_1.0.0.dsc"));

        let paragraph = source.to_paragraph();
        let parsed = Source::from_paragraph(&paragraph).unwrap();

        assert_eq!(source.package, parsed.package);
        assert_eq!(source.version, parsed.version);
        assert_eq!(source.architecture, parsed.architecture);
        assert_eq!(source.maintainer, parsed.maintainer);
        assert_eq!(source.build_depends, parsed.build_depends);
        assert_eq!(source.files.len(), parsed.files.len());
        if !source.files.is_empty() {
            assert_eq!(source.files[0], parsed.files[0]);
        }
    }

    #[test]
    fn test_source_file() {
        let mut source_file = SourceFile::new();
        assert!(source_file.is_empty());
        assert_eq!(source_file.len(), 0);

        let source1 = Source::new("package-a", "1.0.0", "any", "pool/main/a/package-a");
        let source2 = Source::new("package-b", "2.0.0", "any", "pool/main/b/package-b");

        source_file.add_source(source1);
        source_file.add_source(source2);

        assert!(!source_file.is_empty());
        assert_eq!(source_file.len(), 2);

        let content = source_file.to_string();
        let parsed = SourceFile::from_str(&content).unwrap();

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed.sources()[0].package, "package-a");
        assert_eq!(parsed.sources()[1].package, "package-b");
    }
}