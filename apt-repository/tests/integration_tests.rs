use apt_repository::*;
use apt_repository::repository::{MemoryPackageProvider, MemorySourceProvider};
use std::fs;
use tempfile::TempDir;

#[cfg(feature = "async")]
use apt_repository::async_repository::{AsyncRepository, AsyncMemoryPackageProvider, AsyncMemorySourceProvider};

#[test]
fn test_full_repository_generation() {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();

    // Create a repository
    let repo = RepositoryBuilder::new()
        .origin("Test Origin")
        .label("Test Repository")
        .suite("test-suite")
        .codename("test")
        .architectures(vec!["amd64".to_string(), "i386".to_string()])
        .components(vec!["main".to_string(), "contrib".to_string()])
        .description("A test repository")
        .not_automatic(true)
        .but_automatic_upgrades(true)
        .acquire_by_hash(true)
        .build()
        .unwrap();

    // Create test packages
    let mut package_provider = MemoryPackageProvider::new();
    
    // Add packages for main/amd64
    let mut packages_main_amd64 = PackageFile::new();
    let mut pkg1 = Package::new("test-package-1", "1.0.0", "amd64", "pool/main/t/test-package-1_1.0.0_amd64.deb", 1024);
    pkg1.maintainer = Some("Test Maintainer <test@example.com>".to_string());
    pkg1.description = Some("A test package".to_string());
    pkg1.md5sum = Some("abc123".to_string());
    packages_main_amd64.add_package(pkg1);

    let mut pkg2 = Package::new("another-package", "2.1.0", "amd64", "pool/main/a/another-package_2.1.0_amd64.deb", 2048);
    pkg2.maintainer = Some("Another Maintainer <another@example.com>".to_string());
    pkg2.description = Some("Another test package".to_string());
    pkg2.section = Some("utils".to_string());
    packages_main_amd64.add_package(pkg2);

    package_provider.add_packages("test-suite", "main", "amd64", packages_main_amd64);

    // Add packages for main/i386
    let mut packages_main_i386 = PackageFile::new();
    let pkg3 = Package::new("test-package-1", "1.0.0", "i386", "pool/main/t/test-package-1_1.0.0_i386.deb", 900);
    packages_main_i386.add_package(pkg3);
    package_provider.add_packages("test-suite", "main", "i386", packages_main_i386);

    // Add packages for contrib/amd64
    let mut packages_contrib_amd64 = PackageFile::new();
    let pkg4 = Package::new("contrib-package", "0.5.0", "amd64", "pool/contrib/c/contrib-package_0.5.0_amd64.deb", 512);
    packages_contrib_amd64.add_package(pkg4);
    package_provider.add_packages("test-suite", "contrib", "amd64", packages_contrib_amd64);

    // Create test sources
    let mut source_provider = MemorySourceProvider::new();
    
    let mut sources_main = SourceFile::new();
    let mut src1 = Source::new("test-package-1", "1.0.0", "any", "pool/main/t/test-package-1");
    src1.maintainer = Some("Test Maintainer <test@example.com>".to_string());
    src1.build_depends = Some("debhelper (>= 10)".to_string());
    sources_main.add_source(src1);

    let mut src2 = Source::new("another-package", "2.1.0", "any", "pool/main/a/another-package");
    src2.maintainer = Some("Another Maintainer <another@example.com>".to_string());
    sources_main.add_source(src2);

    source_provider.add_sources("test-suite", "main", sources_main);

    let mut sources_contrib = SourceFile::new();
    let src3 = Source::new("contrib-package", "0.5.0", "any", "pool/contrib/c/contrib-package");
    sources_contrib.add_source(src3);
    source_provider.add_sources("test-suite", "contrib", sources_contrib);

    // Generate the repository
    let release = repo.generate_repository(repo_path, &package_provider, &source_provider).unwrap();

    // Verify the repository structure
    assert!(repo_path.join("Release").exists());
    
    // Check component directories
    assert!(repo_path.join("main").exists());
    assert!(repo_path.join("contrib").exists());
    
    // Check architecture directories
    assert!(repo_path.join("main/binary-amd64").exists());
    assert!(repo_path.join("main/binary-i386").exists());
    assert!(repo_path.join("contrib/binary-amd64").exists());
    assert!(repo_path.join("contrib/binary-i386").exists());
    
    // Check source directories
    assert!(repo_path.join("main/source").exists());
    assert!(repo_path.join("contrib/source").exists());

    // Check Packages files
    assert!(repo_path.join("main/binary-amd64/Packages").exists());
    assert!(repo_path.join("main/binary-amd64/Packages.gz").exists());
    assert!(repo_path.join("main/binary-amd64/Packages.bz2").exists());
    
    // Check Sources files
    assert!(repo_path.join("main/source/Sources").exists());
    assert!(repo_path.join("main/source/Sources.gz").exists());
    assert!(repo_path.join("main/source/Sources.bz2").exists());

    // Check by-hash directories (since acquire_by_hash is enabled)
    assert!(repo_path.join("main/binary-amd64/by-hash").exists());
    assert!(repo_path.join("main/binary-amd64/by-hash/MD5Sum").exists());
    assert!(repo_path.join("main/binary-amd64/by-hash/SHA256").exists());

    // Verify Release file content
    assert_eq!(release.origin, Some("Test Origin".to_string()));
    assert_eq!(release.label, Some("Test Repository".to_string()));
    assert_eq!(release.suite, Some("test-suite".to_string()));
    assert_eq!(release.codename, Some("test".to_string()));
    assert_eq!(release.architectures, vec!["amd64", "i386"]);
    assert_eq!(release.components, vec!["main", "contrib"]);
    assert_eq!(release.description, Some("A test repository".to_string()));
    assert_eq!(release.not_automatic, Some(true));
    assert_eq!(release.but_automatic_upgrades, Some(true));
    assert_eq!(release.acquire_by_hash, Some(true));

    // Verify that files are listed in the Release
    assert!(!release.files.is_empty());
    
    // Check that we have files for different architectures and components
    let file_paths: Vec<&str> = release.files.iter().map(|f| f.path.as_str()).collect();
    assert!(file_paths.iter().any(|p| p.contains("main/binary-amd64")));
    assert!(file_paths.iter().any(|p| p.contains("main/binary-i386")));
    assert!(file_paths.iter().any(|p| p.contains("contrib/binary-amd64")));
    assert!(file_paths.iter().any(|p| p.contains("main/source")));
    assert!(file_paths.iter().any(|p| p.contains("contrib/source")));

    // Verify that compressed files are present
    assert!(file_paths.iter().any(|p| p.ends_with("Packages")));
    assert!(file_paths.iter().any(|p| p.ends_with("Packages.gz")));
    assert!(file_paths.iter().any(|p| p.ends_with("Packages.bz2")));

    // Read and verify a Packages file
    let packages_content = fs::read_to_string(repo_path.join("main/binary-amd64/Packages")).unwrap();
    assert!(packages_content.contains("Package: test-package-1"));
    assert!(packages_content.contains("Package: another-package"));
    assert!(packages_content.contains("Architecture: amd64"));

    // Read and verify a Sources file
    let sources_content = fs::read_to_string(repo_path.join("main/source/Sources")).unwrap();
    assert!(sources_content.contains("Package: test-package-1"));
    assert!(sources_content.contains("Package: another-package"));

    // Parse the Release file and verify it matches
    let release_content = fs::read_to_string(repo_path.join("Release")).unwrap();
    let parsed_release = Release::from_str(&release_content).unwrap();
    assert_eq!(parsed_release.origin, release.origin);
    assert_eq!(parsed_release.suite, release.suite);
    assert_eq!(parsed_release.architectures, release.architectures);
    assert_eq!(parsed_release.components, release.components);
}

#[test]
fn test_package_file_roundtrip() {
    let mut packages = PackageFile::new();
    
    let mut pkg1 = Package::new("test-pkg", "1.0.0", "amd64", "test.deb", 1024);
    pkg1.maintainer = Some("Test <test@example.com>".to_string());
    pkg1.description = Some("Short description".to_string());
    pkg1.description_long = Some(" Long description\n with multiple lines\n and details".to_string());
    pkg1.depends = Some("libc6 (>= 2.17), libssl1.1".to_string());
    pkg1.md5sum = Some("abcdef123456".to_string());
    pkg1.sha256 = Some("fedcba654321".to_string());
    
    let mut pkg2 = Package::new("another-pkg", "2.5.1", "all", "another.deb", 2048);
    pkg2.section = Some("utils".to_string());
    pkg2.priority = Some("optional".to_string());
    
    packages.add_package(pkg1);
    packages.add_package(pkg2);

    let content = packages.to_string();
    let parsed_packages = PackageFile::from_str(&content).unwrap();

    assert_eq!(parsed_packages.len(), 2);
    
    let parsed_pkg1 = &parsed_packages.packages()[0];
    assert_eq!(parsed_pkg1.package, "test-pkg");
    assert_eq!(parsed_pkg1.version, "1.0.0");
    assert_eq!(parsed_pkg1.maintainer, Some("Test <test@example.com>".to_string()));
    assert_eq!(parsed_pkg1.depends, Some("libc6 (>= 2.17), libssl1.1".to_string()));
    assert_eq!(parsed_pkg1.md5sum, Some("abcdef123456".to_string()));
    
    let parsed_pkg2 = &parsed_packages.packages()[1];
    assert_eq!(parsed_pkg2.package, "another-pkg");
    assert_eq!(parsed_pkg2.section, Some("utils".to_string()));
    assert_eq!(parsed_pkg2.priority, Some("optional".to_string()));
}

#[test]
fn test_source_file_roundtrip() {
    let mut sources = SourceFile::new();
    
    let mut src1 = Source::new("test-src", "1.0.0", "any", "pool/main/t/test");
    src1.maintainer = Some("Test Maintainer <test@example.com>".to_string());
    src1.build_depends = Some("debhelper (>= 10), build-essential".to_string());
    src1.standards_version = Some("4.5.0".to_string());
    src1.homepage = Some("https://example.com".to_string());
    src1.vcs_git = Some("https://github.com/example/test.git".to_string());
    
    // Add some files
    src1.files.push(crate::sources::SourceFileEntry::new("abc123", 1024, "test_1.0.0.dsc"));
    src1.files.push(crate::sources::SourceFileEntry::new("def456", 10240, "test_1.0.0.tar.xz"));
    
    src1.checksums_sha256.push(crate::sources::SourceFileEntry::new("fedcba654321", 1024, "test_1.0.0.dsc"));
    src1.checksums_sha256.push(crate::sources::SourceFileEntry::new("123456abcdef", 10240, "test_1.0.0.tar.xz"));
    
    sources.add_source(src1);

    let content = sources.to_string();
    let parsed_sources = SourceFile::from_str(&content).unwrap();

    assert_eq!(parsed_sources.len(), 1);
    
    let parsed_src = &parsed_sources.sources()[0];
    assert_eq!(parsed_src.package, "test-src");
    assert_eq!(parsed_src.version, "1.0.0");
    assert_eq!(parsed_src.maintainer, Some("Test Maintainer <test@example.com>".to_string()));
    assert_eq!(parsed_src.build_depends, Some("debhelper (>= 10), build-essential".to_string()));
    assert_eq!(parsed_src.homepage, Some("https://example.com".to_string()));
    assert_eq!(parsed_src.vcs_git, Some("https://github.com/example/test.git".to_string()));
    
    assert_eq!(parsed_src.files.len(), 2);
    assert_eq!(parsed_src.files[0].name, "test_1.0.0.dsc");
    assert_eq!(parsed_src.files[0].hash, "abc123");
    assert_eq!(parsed_src.files[0].size, 1024);
    
    assert_eq!(parsed_src.checksums_sha256.len(), 2);
    assert_eq!(parsed_src.checksums_sha256[0].name, "test_1.0.0.dsc");
    assert_eq!(parsed_src.checksums_sha256[0].hash, "fedcba654321");
}

#[test]
fn test_compression_and_hashing() {
    let test_data = b"This is test data for compression and hashing";
    
    // Test all compression formats
    for compression in Compression::all() {
        let compressed = compression.compress(test_data).unwrap();
        let decompressed = compression.decompress(&compressed).unwrap();
        assert_eq!(decompressed, test_data);
        
        if *compression != Compression::None {
            assert_ne!(compressed, test_data);
        } else {
            assert_eq!(compressed, test_data);
        }
    }
    
    // Test hashing
    let (size, hashes) = crate::hash::hash_data(test_data, HashAlgorithm::all());
    assert_eq!(size, test_data.len() as u64);
    assert_eq!(hashes.len(), HashAlgorithm::all().len());
    
    for algorithm in HashAlgorithm::all() {
        assert!(hashes.get(algorithm).is_some());
        let hash = hashes.get(algorithm).unwrap();
        assert!(!hash.is_empty());
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }
}

#[cfg(feature = "async")]
#[tokio::test]
async fn test_async_repository_generation() {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();

    let repo = RepositoryBuilder::new()
        .origin("Async Test")
        .suite("async-test")
        .architectures(vec!["amd64".to_string()])
        .components(vec!["main".to_string()])
        .build()
        .unwrap();

    let async_repo = AsyncRepository::new(repo);

    let mut package_provider = AsyncMemoryPackageProvider::new();
    let mut packages = PackageFile::new();
    packages.add_package(Package::new("async-pkg", "1.0.0", "amd64", "async.deb", 1024));
    package_provider.add_packages("async-test", "main", "amd64", packages);

    let source_provider = AsyncMemorySourceProvider::new();

    let release = async_repo.generate_repository(repo_path, &package_provider, &source_provider).await.unwrap();

    // Verify the repository was created
    assert!(tokio::fs::try_exists(repo_path.join("Release")).await.unwrap());
    assert!(tokio::fs::try_exists(repo_path.join("main/binary-amd64/Packages")).await.unwrap());
    
    assert_eq!(release.origin, Some("Async Test".to_string()));
    assert_eq!(release.suite, Some("async-test".to_string()));
}

#[test]
fn test_repository_builder_validation() {
    // Valid repository should build
    let repo = RepositoryBuilder::new()
        .suite("test")
        .architectures(vec!["amd64".to_string()])
        .components(vec!["main".to_string()])
        .build();
    assert!(repo.is_ok());

    // Empty suite should fail
    let repo = RepositoryBuilder::new()
        .suite("")
        .build();
    assert!(repo.is_err());

    // Empty architectures should fail
    let repo = RepositoryBuilder::new()
        .architectures(vec![])
        .build();
    assert!(repo.is_err());

    // Empty components should fail
    let repo = RepositoryBuilder::new()
        .components(vec![])
        .build();
    assert!(repo.is_err());
}