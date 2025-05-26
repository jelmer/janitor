# Archive Service Porting Plan

> **Status**: 🚧 **IN PROGRESS** - Basic scanner implemented, needs complete archive generation and web service.
> 
> 📋 **Master Plan**: See [`../porting-plan.md`](../porting-plan.md) for overall project coordination and dependencies.

## Overview

This document outlines the detailed plan for porting the Janitor archive service from Python to Rust. The archive service is responsible for generating Debian APT repositories from successful build artifacts, providing HTTP-accessible package indexes, and managing GPG-signed repository metadata.

### Current State Analysis

**Python Implementation (`py/janitor/debian/archive.py`)**: ~1,065 lines
- Complete APT repository generation system
- HTTP web server with multiple endpoints
- GPG signing integration for repository security
- Package and source metadata generation
- On-demand repository refresh for individual runs/changesets
- Redis pub/sub integration for automated triggering
- Disk caching system for performance optimization
- Comprehensive error handling and logging

**Rust Implementation (`archive/`)**: ~185 lines (minimal)
- Basic scanner module for dpkg-scanpackages/dpkg-scansources
- Some Debian package parsing with deb822-lossless
- Missing: web server, repository generation, GPG signing, caching, etc.

## Technical Architecture Analysis

### Current Python Stack
- **Web Framework**: aiohttp with route definitions and middleware
- **Package Scanning**: subprocess calls to dpkg-scanpackages/dpkg-scansources
- **Repository Generation**: Custom Release file creation with hash support
- **GPG Signing**: python-gnupg for Release.gpg and InRelease generation
- **File Compression**: Built-in gzip/bz2 for Packages/Sources files
- **Database**: AsyncPG for querying build results and metadata
- **Caching**: Custom disk-based caching with file management
- **Pub/Sub**: Redis integration for automated republishing
- **Monitoring**: Prometheus metrics and aiozipkin tracing

### Target Rust Architecture
- **Web Framework**: Axum for HTTP server and routing
- **Package Scanning**: tokio::process for dpkg tools (already implemented)
- **Repository Generation**: Custom Release file generation with crypto hashing
- **GPG Signing**: gpgme-rs or pgp for repository signing
- **File Compression**: flate2 for gzip, bzip2 for bz2 compression
- **Database**: sqlx for type-safe PostgreSQL operations
- **Caching**: Custom async file caching with tokio::fs
- **Pub/Sub**: redis-rs with tokio for messaging
- **Monitoring**: tracing ecosystem with prometheus integration

## Key Functionality Analysis

### Core Components to Port

1. **Package Info Providers** (Lines 74-239)
   - `PackageInfoProvider` trait and implementations
   - `GeneratingPackageInfoProvider` - artifact retrieval and scanning
   - `DiskCachingPackageInfoProvider` - performance optimization

2. **Repository File Generation** (Lines 387-490)
   - `write_suite_files` - main repository generation function
   - `HashedFileWriter` - by-hash file management with compression
   - Release file creation with proper metadata and timestamps

3. **Web Server Endpoints** (Lines 496-802)
   - Publishing triggers (`/publish`, `/last-publish`)
   - Health checks (`/health`, `/ready`)
   - Static distribution files serving
   - On-demand repository generation for runs/changesets
   - GPG key serving for client verification

4. **Database Integration** (Lines 271-307)
   - Build result queries by suite, changeset, and run
   - Integration with run publishing status
   - Support for campaign-based filtering

5. **Background Services** (Lines 825-1055)
   - Redis pub/sub listener for automated triggering
   - Periodic repository republishing
   - Job scheduling and management

## Porting Strategy

### Phase 1: Core Infrastructure (2-3 weeks)

#### 1.1 Enhanced Scanner Module (1 week)
- Extend existing scanner.rs with stream-based processing
- Add proper error handling and logging integration
- Implement async generators for package/source data
- Add artifact retrieval integration

**Current Status**: Basic implementation exists
**Effort Estimate**: ~300 lines to add

**Deliverables:**
- Stream-based package/source scanning
- Artifact manager integration
- Enhanced error handling
- Performance optimizations

#### 1.2 Database Integration (0.5 weeks)
- Port database query functions using sqlx
- Implement build result retrieval by various criteria
- Add connection pooling and error handling
- Port campaign and distribution configuration

**Effort Estimate**: ~150 lines
**Complexity**: Medium - straightforward SQL porting

**Deliverables:**
- Database query functions
- Build result retrieval
- Connection management
- Configuration integration

#### 1.3 Configuration and Setup (0.5 weeks)
- Port configuration structs and parsing
- Add GPG context integration
- Implement artifact manager setup
- Add logging and tracing infrastructure

**Effort Estimate**: ~100 lines
**Complexity**: Low - mostly configuration porting

**Deliverables:**
- Configuration management
- GPG integration setup
- Artifact manager integration
- Logging infrastructure

### Phase 2: Repository Generation Engine (3-4 weeks)

#### 2.1 Package Info Providers (1.5 weeks)
- Port `PackageInfoProvider` trait and implementations
- Implement `GeneratingPackageInfoProvider` with artifact retrieval
- Add `DiskCachingPackageInfoProvider` for performance
- Integrate with existing scanner module

**Effort Estimate**: ~400 lines to port from Python
**Complexity**: High - complex async trait implementations

**Deliverables:**
- PackageInfoProvider trait system
- Artifact-based package generation
- Disk caching implementation
- Performance optimization

#### 2.2 Repository File Generation (1.5 weeks)
- Port `write_suite_files` function with compression support
- Implement `HashedFileWriter` for by-hash repository structure
- Add Release file generation with proper metadata
- Integrate GPG signing for Release.gpg and InRelease

**Effort Estimate**: ~350 lines of complex file I/O and crypto
**Complexity**: Very High - file management, compression, cryptography

**Deliverables:**
- Complete repository generation
- By-hash file structure
- Release file creation
- GPG signing integration

#### 2.3 File Management and Compression (1 week)
- Implement multi-format compression (gzip, bz2, uncompressed)
- Add by-hash file cleanup and rotation
- Implement atomic file operations
- Add file integrity verification

**Effort Estimate**: ~250 lines
**Complexity**: Medium - file operations and compression

**Deliverables:**
- Multi-format compression
- File cleanup management
- Atomic operations
- Integrity verification

### Phase 3: Web Service Implementation (3-4 weeks)

#### 3.1 Basic HTTP Server (1 week)
- Set up Axum application with routing
- Implement health check endpoints (`/health`, `/ready`)
- Add basic static file serving
- Implement metrics and tracing integration

**Effort Estimate**: ~200 lines
**Complexity**: Medium - web framework setup

**Deliverables:**
- Axum HTTP server
- Health check endpoints
- Static file serving
- Monitoring integration

#### 3.2 Repository Serving (1.5 weeks)
- Port distribution file serving endpoints
- Implement on-demand repository generation
- Add proper HTTP headers and caching
- Port by-hash file serving

**Effort Estimate**: ~300 lines
**Complexity**: Medium - HTTP handlers and file serving

**Deliverables:**
- Distribution file endpoints
- On-demand generation
- HTTP caching headers
- By-hash file serving

#### 3.3 Publishing and Management APIs (1.5 weeks)
- Port publishing trigger endpoints (`/publish`)
- Implement last-publish status tracking
- Add GPG key serving endpoint
- Port repository management functions

**Effort Estimate**: ~200 lines
**Complexity**: Medium - API endpoints and state management

**Deliverables:**
- Publishing trigger APIs
- Status tracking endpoints
- GPG key serving
- Management interfaces

### Phase 4: Background Services (2-3 weeks)

#### 4.1 Generator Manager (1.5 weeks)
- Port `GeneratorManager` class functionality
- Implement job scheduling and management
- Add campaign-to-repository mapping
- Integrate with repository generation

**Effort Estimate**: ~250 lines
**Complexity**: High - async job management and coordination

**Deliverables:**
- Job scheduling system
- Campaign management
- Repository coordination
- Background task handling

#### 4.2 Redis Integration (1 week)
- Port Redis pub/sub listener functionality
- Implement automatic republishing triggers
- Add connection management and error handling
- Integrate with generator manager

**Effort Estimate**: ~150 lines
**Complexity**: Medium - async messaging integration

**Deliverables:**
- Redis pub/sub integration
- Automatic triggering
- Connection management
- Event-driven updates

#### 4.3 Periodic Services (0.5 weeks)
- Port periodic repository publishing
- Implement background loop management
- Add graceful shutdown handling
- Integrate with job scheduler

**Effort Estimate**: ~100 lines
**Complexity**: Low - background task coordination

**Deliverables:**
- Periodic publishing
- Background loops
- Shutdown handling
- Service coordination

### Phase 5: Testing and Optimization (2-3 weeks)

#### 5.1 Unit and Integration Tests (1.5 weeks)
- Port existing test data and fixtures
- Implement repository generation tests
- Add HTTP endpoint testing
- Create performance benchmarks

**Effort Estimate**: ~300 lines of test code
**Complexity**: Medium - test infrastructure and validation

**Deliverables:**
- Comprehensive test suite
- Test data fixtures
- HTTP endpoint tests
- Performance benchmarks

#### 5.2 Performance Optimization (1 week)
- Profile repository generation performance
- Optimize file I/O and compression
- Tune caching strategies
- Implement streaming optimizations

**Deliverables:**
- Performance profiling
- I/O optimizations
- Caching improvements
- Streaming enhancements

#### 5.3 Production Readiness (0.5 weeks)
- Add comprehensive error handling
- Implement proper logging and metrics
- Add monitoring and alerting
- Create deployment documentation

**Deliverables:**
- Error handling
- Monitoring integration
- Production documentation
- Deployment guides

## Implementation Details

### Key Dependencies

**Rust Crates:**
```toml
[dependencies]
axum = "0.7"                    # Web framework
tokio = { version = "1.0", features = ["full"] }
sqlx = "0.7"                    # Database toolkit
serde = "1.0"                   # Serialization
tracing = "0.1"                 # Logging/tracing
tower = "0.4"                   # Service middleware
tower-http = "0.5"              # HTTP utilities
redis = "0.24"                  # Redis client
flate2 = "1.0"                  # Gzip compression
bzip2 = "0.4"                   # Bzip2 compression
sha2 = "0.10"                   # SHA hashing
md5 = "0.7"                     # MD5 hashing
chrono = "0.4"                  # Date/time handling
uuid = "1.6"                    # UUID generation
gpgme = "0.11"                  # GPG integration
tempfile = "3.8"                # Temporary files
```

### Critical Migration Patterns

1. **Package Scanning**:
   ```python
   # Python (subprocess + parsing)
   async def scan_packages(td, arch=None):
       proc = await asyncio.create_subprocess_exec(
           "dpkg-scanpackages", td, *args,
           stdout=asyncio.subprocess.PIPE
       )
       stdout, stderr = await proc.communicate()
       for para in Packages.iter_paragraphs(stdout):
           yield para
   ```
   
   ```rust
   // Rust (tokio::process + deb822 parsing)
   async fn scan_packages(td: &str, arch: Option<&str>) -> Result<Vec<Package>, String> {
       let mut proc = Command::new("dpkg-scanpackages")
           .arg(td)
           .stdout(Stdio::piped())
           .spawn()?;
       let stdout = proc.stdout.take().unwrap();
       // Parse with deb822-lossless and return structured data
   }
   ```

2. **Repository Generation**:
   ```python
   # Python (file writing with compression)
   with ExitStack() as es:
       for suffix, fn in SUFFIXES.items():
           fs.append(es.enter_context(
               HashedFileWriter(r, base_path, packages_path + suffix, fn)
           ))
       async for chunk in get_packages(suite_name, component, arch):
           for f in fs:
               f.write(chunk)
   ```
   
   ```rust
   // Rust (async file I/O with compression)
   let mut writers = Vec::new();
   for (suffix, compression) in &[("", None), (".gz", Some(Gzip)), (".bz2", Some(Bz2))] {
       writers.push(HashedFileWriter::new(release, base_path, &format!("{}{}", path, suffix), *compression));
   }
   let mut stream = get_packages(suite_name, component, arch);
   while let Some(chunk) = stream.next().await {
       for writer in &mut writers {
           writer.write(&chunk).await?;
       }
   }
   ```

3. **GPG Signing**:
   ```python
   # Python (python-gnupg)
   data = gpg.Data(r.dump())
   signature, result = gpg_context.sign(data, mode=gpg_mode.DETACH)
   f.write(signature)
   ```
   
   ```rust
   // Rust (gpgme-rs)
   let mut output = Vec::new();
   let signature = ctx.sign_detached(release_data)?;
   signature.write_to(&mut output)?;
   tokio::fs::write(path, output).await?;
   ```

### Risk Mitigation

1. **GPG Integration**: Use well-tested gpgme-rs crate with comprehensive error handling
2. **File I/O Performance**: Implement async streaming with proper buffering
3. **Database Compatibility**: Maintain exact SQL query compatibility
4. **HTTP Compatibility**: Ensure identical response formats and headers

## Timeline and Effort Estimates

### Total Effort: 10-13 weeks (2.5-3.25 months)

| Phase | Duration | Effort Level | Risk Level |
|-------|----------|--------------|------------|
| 1. Core Infrastructure | 2-3 weeks | Medium | Low |
| 2. Repository Generation Engine | 3-4 weeks | Very High | High |
| 3. Web Service Implementation | 3-4 weeks | High | Medium |
| 4. Background Services | 2-3 weeks | High | Medium |
| 5. Testing and Optimization | 2-3 weeks | Medium | Low |

### Critical Dependencies

- **Database Schema**: Must be stable before Phase 1.2
- **Artifact Management**: Required for Phase 2.1
- **Configuration System**: Needed for all phases
- **GPG Infrastructure**: Critical for Phase 2.2

### Success Metrics

1. **Functional Parity**: 100% feature compatibility with Python implementation
2. **Performance**: Repository generation time ≤ Python implementation
3. **Reliability**: Zero data corruption or signing failures
4. **HTTP Compatibility**: All endpoints return identical responses
5. **Resource Usage**: Memory and CPU usage ≤ Python implementation

## Integration Considerations

### Service Dependencies
- **Runner Service**: Triggers archive generation via pub/sub
- **Publisher Service**: Consumes generated repository metadata
- **Site Service**: Links to package files and repository status
- **Database**: Requires read access to run and build tables

### API Compatibility
- All HTTP endpoints must maintain identical URL patterns
- Response formats must be byte-for-byte identical
- HTTP headers and status codes must match exactly
- GPG signatures must validate with existing keys

## Related Porting Plans

- 📋 **Master Plan**: [`../porting-plan.md`](../porting-plan.md) - Overall project coordination
- ✅ **Runner**: [`../runner/porting-plan.md`](../runner/porting-plan.md) - Already completed (provides triggers)
- ✅ **Publisher**: [`../publish/porting-plan.md`](../publish/porting-plan.md) - Already completed (consumes archives)
- 🚧 **Differ**: [`../differ/porting-plan.md`](../differ/porting-plan.md) - In progress
- 🚧 **Site**: [`../site/porting-plan.md`](../site/porting-plan.md) - In progress (links to archives)

---

*This plan will be updated as implementation progresses and requirements evolve.*