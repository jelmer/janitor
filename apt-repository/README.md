# APT Repository Library

A Rust library for creating and parsing APT repositories. This library provides
functionality to generate repository metadata files (Release, Packages,
                                                     Sources) with proper
cryptographic hashing and compression support.

## Features

- Generate APT repository metadata files
- Support for multiple compression formats (gzip, bzip2, uncompressed)
- Cryptographic hashing (MD5, SHA1, SHA256, SHA512)
- By-hash repository structure support
- Release file generation with GPG signing preparation
- Async support for file operations
- Comprehensive parsing of existing repository files

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
apt-repository = "0.1"
```

### Basic Usage

```rust
use apt_repository::*;

// Create a repository
let repo = RepositoryBuilder::new()
    .origin("My Repository")
    .label("My APT Repository")
    .suite("stable")
    .codename("stable")
    .architectures(vec!["amd64".to_string(), "i386".to_string()])
    .components(vec!["main".to_string()])
    .build()?;

// Create package providers (implement PackageProvider and SourceProvider traits)
let package_provider = MyPackageProvider::new();
let source_provider = MySourceProvider::new();

// Generate repository metadata
let release = repo.generate_repository("/path/to/repo", &package_provider, &source_provider)?;

println!("Generated repository with {} files", release.files.len());
```

### Async Usage

```rust
use apt_repository::*;

// Create an async repository
let repo = RepositoryBuilder::new()
    .origin("My Async Repository")
    .suite("testing")
    .build()?;

let async_repo = AsyncRepository::new(repo);

// Use async providers
let package_provider = MyAsyncPackageProvider::new();
let source_provider = MyAsyncSourceProvider::new();

// Generate repository metadata asynchronously
let release = async_repo.generate_repository("/path/to/repo", &package_provider, &source_provider).await?;
```

### Working with Packages

```rust
use apt_repository::*;

// Create a package
let mut package = Package::new(
    "my-package",
    "1.0.0",
    "amd64", 
    "pool/main/m/my-package_1.0.0_amd64.deb",
    1024
);
package.maintainer = Some("John Doe <john@example.com>".to_string());
package.description = Some("A sample package".to_string());
package.depends = Some("libc6 (>= 2.17)".to_string());

// Add to a package file
let mut packages = PackageFile::new();
packages.add_package(package);

// Convert to Packages file format
let packages_content = packages.to_string();

// Parse from existing content
let parsed_packages = PackageFile::from_str(&packages_content)?;
```

### Working with Sources

```rust
use apt_repository::*;

// Create a source package
let mut source = Source::new(
    "my-package",
    "1.0.0",
    "any",
    "pool/main/m/my-package"
);
source.maintainer = Some("John Doe <john@example.com>".to_string());
source.build_depends = Some("debhelper (>= 10)".to_string());

// Add files
source.files.push(SourceFileEntry::new("abc123", 1024, "my-package_1.0.0.dsc"));
source.checksums_sha256.push(SourceFileEntry::new("def456", 1024, "my-package_1.0.0.dsc"));

// Add to a source file
let mut sources = SourceFile::new();
sources.add_source(source);

// Convert to Sources file format
let sources_content = sources.to_string();
```

### Custom Providers

Implement the `PackageProvider` and `SourceProvider` traits to provide package data:

```rust
use apt_repository::*;

struct MyPackageProvider {
    // Your data source
}

impl PackageProvider for MyPackageProvider {
    fn get_packages(&self, suite: &str, component: &str, architecture: &str) -> Result<PackageFile> {
        // Load packages from your data source
        let mut packages = PackageFile::new();
        // ... add packages
        Ok(packages)
    }
}

struct MySourceProvider {
    // Your data source  
}

impl SourceProvider for MySourceProvider {
    fn get_sources(&self, suite: &str, component: &str) -> Result<SourceFile> {
        // Load sources from your data source
        let mut sources = SourceFile::new();
        // ... add sources
        Ok(sources)
    }
}
```

### Async Providers

For async support, implement the async traits:

```rust
use apt_repository::*;

struct MyAsyncPackageProvider {
    // Your async data source
}

#[async_trait::async_trait]
impl AsyncPackageProvider for MyAsyncPackageProvider {
    async fn get_packages(&self, suite: &str, component: &str, architecture: &str) -> Result<PackageFile> {
        // Load packages asynchronously
        let mut packages = PackageFile::new();
        // ... add packages
        Ok(packages)
    }
}
```

## Repository Structure

The library generates standard APT repository structure:

```
repository/
├── Release                    # Main release file
├── Release.gpg               # GPG signature (when signed)
├── InRelease                 # Inline GPG signature (when signed)
└── main/                     # Component directory
    ├── binary-amd64/         # Architecture-specific packages
    │   ├── Packages          # Package index
    │   ├── Packages.gz       # Compressed package index  
    │   ├── Packages.bz2      # Compressed package index
    │   └── by-hash/          # By-hash structure (optional)
    │       ├── MD5Sum/
    │       ├── SHA1/
    │       ├── SHA256/
    │       └── SHA512/
    └── source/               # Source packages
        ├── Sources           # Source index
        ├── Sources.gz        # Compressed source index
        ├── Sources.bz2       # Compressed source index
        └── by-hash/          # By-hash structure (optional)
```

## Configuration Options

### Repository Builder Options

- `origin()` - Set repository origin
- `label()` - Set repository label  
- `suite()` - Set suite name (required)
- `codename()` - Set codename
- `version()` - Set version
- `architectures()` - Set supported architectures (required)
- `components()` - Set repository components (required)
- `description()` - Set repository description
- `not_automatic()` - Set NotAutomatic flag
- `but_automatic_upgrades()` - Set ButAutomaticUpgrades flag
- `acquire_by_hash()` - Enable by-hash support
- `compressions()` - Set compression formats
- `hash_algorithms()` - Set hash algorithms

### Compression Formats

- `Compression::None` - No compression
- `Compression::Gzip` - Gzip compression (.gz)
- `Compression::Bzip2` - Bzip2 compression (.bz2)

### Hash Algorithms

- `HashAlgorithm::Md5` - MD5 hashing
- `HashAlgorithm::Sha1` - SHA-1 hashing
- `HashAlgorithm::Sha256` - SHA-256 hashing
- `HashAlgorithm::Sha512` - SHA-512 hashing

## Features

- `default` - Includes async support
- `async` - Enables async functionality with tokio

## Integration with Janitor

This library is designed to be used by the [Janitor](https://github.com/jelmer/janitor) archive service:

```rust
use apt_repository::*;

// Integrate with janitor-archive crate
let repo = RepositoryBuilder::new()
    .origin("Janitor")
    .suite("experimental")
    .build()?;

// Use with janitor's package scanning
let package_provider = JanitorPackageProvider::new(artifact_manager);
let release = repo.generate_repository(dists_dir, &package_provider, &source_provider)?;
```

## Error Handling

The library uses a comprehensive error type that covers:

- I/O errors during file operations
- Invalid repository configuration
- Package/source parsing errors
- Compression failures
- Hash calculation errors

```rust
use apt_repository::*;

match repo.generate_repository(path, &pkg_provider, &src_provider) {
    Ok(release) => println!("Repository generated successfully"),
    Err(AptRepositoryError::Io(e)) => eprintln!("I/O error: {}", e),
    Err(AptRepositoryError::InvalidConfiguration(msg)) => eprintln!("Config error: {}", msg),
    Err(e) => eprintln!("Other error: {}", e),
}
```

## License

This project is licensed under the GPL-3.0+ license.
