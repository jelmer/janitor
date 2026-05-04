# apt-repository

A Rust library for creating and parsing APT repositories. Generates Release,
Packages, and Sources metadata files with cryptographic hashing and compression.

## Usage

```toml
[dependencies]
apt-repository = "0.1"
```

### Generating a repository

```rust
use apt_repository::*;

let repo = RepositoryBuilder::new()
    .origin("My Repository")
    .suite("stable")
    .architectures(vec!["amd64".to_string()])
    .components(vec!["main".to_string()])
    .build()?;

let package_provider = MyPackageProvider::new();
let source_provider = MySourceProvider::new();

let release = repo.generate_repository("/srv/apt", &package_provider, &source_provider)?;
```

Implement `PackageProvider` and `SourceProvider` to feed package data:

```rust
impl PackageProvider for MyPackageProvider {
    fn get_packages(&self, suite: &str, component: &str, architecture: &str) -> Result<PackageFile> {
        let mut packages = PackageFile::new();
        packages.add_package(new_package("my-pkg", "1.0", "amd64", "pool/main/m/my-pkg_1.0_amd64.deb", 12345));
        Ok(packages)
    }
}
```

Async variants (`AsyncPackageProvider`, `AsyncSourceProvider`, `AsyncRepository`)
are available under the `async` feature (enabled by default).

### Repository layout

```
dists/stable/
├── Release
├── main/
│   ├── binary-amd64/
│   │   ├── Packages
│   │   ├── Packages.gz
│   │   └── Packages.bz2
│   └── source/
│       ├── Sources
│       ├── Sources.gz
│       └── Sources.bz2
```

With `acquire_by_hash(true)`, a `by-hash/` subtree is also written.

## Features

- `async` (default) — enables `AsyncRepository` and async provider traits via tokio

## License

GPL-3.0+
