# Auto-Upload Service Porting Plan

> **Status**: 🚧 **IN PROGRESS** - Phase 4 (Database Integration and Backfill) ✅ COMPLETE | Phase 5 ready
> 
> 📋 **Master Plan**: See [`../porting-plan.md`](../porting-plan.md) for overall project coordination and dependencies.

## Overview

This document outlines the detailed plan for porting the Janitor auto-upload service from Python to Rust. The auto-upload service is responsible for automatically uploading successful Debian package builds to configured repositories using `debsign` for signing and `dput` for uploading.

### Current State Analysis

**Python Implementation (`py/janitor/debian/auto_upload.py`)**: ~295 lines
- Complete auto-upload service with Redis pub/sub integration
- Artifact retrieval and processing for Debian packages
- GPG signing integration via `debsign` 
- Package uploading via `dput` to configured hosts
- HTTP web server for health checks and metrics
- Backfill functionality for uploading historical builds
- Distribution filtering and source-only upload options
- Comprehensive error handling and metrics collection

**Rust Implementation (`auto-upload/`)**: ~11 lines (minimal)
- Only contains basic re-exports from silver-platter crate
- Missing: web server, Redis integration, artifact processing, main service logic

## Technical Architecture Analysis

### Current Python Stack
- **Process Management**: asyncio for concurrent operations
- **Package Signing**: silver-platter's `debsign` function
- **Package Uploading**: silver-platter's `dput_changes` function  
- **Web Framework**: aiohttp with basic health endpoint
- **Pub/Sub**: Redis async client for runner integration
- **Database**: AsyncPG for backfill operations
- **Metrics**: Prometheus counters via aiohttp-openmetrics
- **Artifact Management**: Integration with artifact manager for file retrieval
- **Configuration**: Shared configuration system with other services

### Target Rust Architecture
- **Async Runtime**: Tokio for async operations and process management
- **Package Signing**: silver-platter Rust crate (already available)
- **Package Uploading**: silver-platter Rust crate (already available)
- **Web Framework**: Axum for HTTP server and health endpoints
- **Pub/Sub**: redis-rs with tokio for Redis integration
- **Database**: sqlx for PostgreSQL backfill operations
- **Metrics**: prometheus crate with axum integration
- **Artifact Management**: Integration with Rust artifact manager
- **Configuration**: Shared Rust configuration system

## Key Functionality Analysis

### Core Components to Port

1. **Upload Build Result** (Lines 55-132)
   - Artifact retrieval from storage
   - Changes file discovery and validation
   - File permission management for signing
   - GPG signing via `debsign`
   - Package uploading via `dput`
   - Error handling and retry logic

2. **Redis Pub/Sub Listener** (Lines 134-164)
   - Redis message handling for runner results
   - Distribution filtering logic
   - Asynchronous upload triggering
   - Connection management and error handling

3. **Web Server** (Lines 44-52)
   - Basic HTTP server for health checks
   - Metrics endpoint integration
   - Configuration injection

4. **Backfill Functionality** (Lines 167-191)
   - Database query for historical builds
   - Bulk upload processing
   - Distribution and source filtering

5. **Main Service Loop** (Lines 193-295)
   - Command-line argument parsing
   - Service initialization and configuration
   - Task orchestration and error handling
   - Graceful shutdown handling

## Porting Strategy

### Phase 1: Core Service Infrastructure (1-2 weeks) ✅ **COMPLETE**

#### 1.1 Configuration and Setup (0.5 weeks)
- Port configuration structures and parsing
- Add command-line argument handling with clap
- Implement logging and tracing setup
- Add artifact manager integration

**Effort Estimate**: ~100 lines
**Complexity**: Low - straightforward configuration porting

**Deliverables:**
- Configuration management
- CLI argument parsing
- Logging infrastructure
- Artifact manager setup

#### 1.2 Web Server Foundation (0.5 weeks)
- Set up Axum application with basic routing
- Implement health check endpoint
- Add Prometheus metrics integration
- Configure graceful shutdown

**Effort Estimate**: ~80 lines
**Complexity**: Low - basic web server setup

**Deliverables:**
- Basic HTTP server
- Health check endpoint
- Metrics collection
- Shutdown handling

#### 1.3 Error Types and Utilities (0.5 weeks)
- Define error types for upload operations
- Add file permission utilities
- Implement temporary directory management
- Create logging helpers

**Effort Estimate**: ~60 lines
**Complexity**: Low - utility functions and error handling

**Deliverables:**
- Error type definitions
- File utilities
- Temporary file management
- Logging helpers

### Phase 2: Upload Processing Engine (2-3 weeks) ✅ **COMPLETE**

#### 2.1 Artifact Processing (1 week)
- Port artifact retrieval functionality
- Implement changes file discovery
- Add file permission management
- Create validation and filtering logic

**Effort Estimate**: ~150 lines
**Complexity**: Medium - file processing and validation

**Deliverables:**
- Artifact retrieval system
- Changes file processing
- File permission handling
- Validation logic

#### 2.2 Package Signing and Upload (1 week)
- Integrate silver-platter `debsign` functionality
- Implement `dput` upload integration
- Add error handling and retries
- Create progress tracking and logging

**Effort Estimate**: ~120 lines
**Complexity**: Medium - external process integration

**Deliverables:**
- GPG signing integration
- Package upload system
- Error handling and retries
- Progress monitoring

#### 2.3 Upload Orchestration (1 week)
- Port main upload logic from `upload_build_result`
- Implement source-only filtering
- Add distribution-based processing
- Create metrics collection

**Effort Estimate**: ~100 lines
**Complexity**: Medium - orchestration and business logic

**Deliverables:**
- Complete upload workflow
- Filtering capabilities
- Business logic implementation
- Metrics integration

### Phase 3: Redis Integration and Messaging (1-2 weeks) ✅ **COMPLETE**

#### 3.1 Redis Pub/Sub Client (1 week)
- Set up Redis connection management
- Implement pub/sub message handling
- Add JSON message parsing
- Create connection error handling

**Effort Estimate**: ~80 lines
**Complexity**: Medium - async messaging and JSON handling

**Deliverables:**
- Redis connection management
- Pub/sub message handling
- JSON parsing and validation
- Connection error recovery

#### 3.2 Message Processing (0.5 weeks)
- Port result message handling logic
- Implement distribution filtering
- Add target validation (Debian-only)
- Create upload triggering

**Effort Estimate**: ~60 lines
**Complexity**: Low - message routing and filtering

**Deliverables:**
- Message processing pipeline
- Distribution filtering
- Target validation
- Upload triggering

#### 3.3 Integration Testing (0.5 weeks)
- Test Redis integration with mock messages
- Validate upload triggering
- Test error handling and recovery
- Add integration test suite

**Deliverables:**
- Integration test suite
- Mock message testing
- Error scenario validation
- Performance testing

### Phase 4: Database Integration and Backfill (1-2 weeks) ✅ **COMPLETE**

#### 4.1 ✅ Database Query Implementation (COMPLETED)
- **Completed**: Port backfill database queries using sqlx
- **Completed**: Implement distribution filtering with optional parameters
- **Completed**: Add source package deduplication with DISTINCT ON queries
- **Completed**: Create connection pool management with health checks

**Effort**: ~280 lines (database.rs)
**Complexity**: Medium - SQL queries and connection management

**Deliverables:**
- ✅ Database query functions with error handling
- ✅ Connection pool setup with configurable limits  
- ✅ Query optimization with filtered results
- ✅ Result processing with proper type mapping

#### 4.2 ✅ Backfill Functionality (COMPLETED)
- **Completed**: Port backfill main loop with async processing
- **Completed**: Implement batch processing with configurable batch sizes
- **Completed**: Add progress reporting with real-time statistics
- **Completed**: Create error handling for failed uploads with retry logic

**Effort**: ~470 lines (backfill.rs)
**Complexity**: Medium - async coordination and retry logic

**Deliverables:**
- ✅ Backfill processing loop with concurrent task management
- ✅ Batch upload handling with rate limiting
- ✅ Progress reporting with atomic counters
- ✅ Error recovery with exponential backoff

#### 4.3 ✅ CLI Integration (COMPLETED)
- **Completed**: Add backfill command-line options with clap subcommands
- **Completed**: Implement mode switching between serve and backfill
- **Completed**: Add validation and error messages with structured logging
- **Completed**: Test backfill functionality with comprehensive test suite

**Effort**: ~160 lines (main.rs updates)
**Complexity**: Low - CLI argument parsing and command routing

**Deliverables:**
- ✅ CLI backfill options with extensive configuration
- ✅ Mode switching logic with proper error handling
- ✅ Input validation with helpful error messages
- ✅ Functional testing with 19 passing tests

### Phase 5: Service Orchestration and Testing (1-2 weeks)

#### 5.1 Main Service Loop (1 week)
- Port main async service orchestration
- Implement task spawning and management
- Add signal handling for graceful shutdown
- Create service health monitoring

**Effort Estimate**: ~100 lines
**Complexity**: Medium - async task coordination

**Deliverables:**
- Service orchestration
- Task management
- Signal handling
- Health monitoring

#### 5.2 Integration Testing (0.5 weeks)
- Create comprehensive test suite
- Test Redis integration with real messages
- Validate upload workflow end-to-end
- Add performance benchmarks

**Deliverables:**
- Complete test suite
- End-to-end testing
- Performance validation
- Documentation

#### 5.3 Production Readiness (0.5 weeks)
- Add comprehensive error logging
- Implement monitoring and alerting
- Create deployment documentation
- Add operational runbooks

**Deliverables:**
- Production logging
- Monitoring integration
- Deployment guides
- Operational documentation

## Implementation Details

### Key Dependencies

**Rust Crates:**
```toml
[dependencies]
tokio = { version = "1.0", features = ["full"] }
axum = "0.7"                    # Web framework
serde = "1.0"                   # JSON serialization
serde_json = "1.0"              # JSON parsing
sqlx = "0.7"                    # Database toolkit
redis = "0.24"                  # Redis client
clap = "4.4"                    # CLI argument parsing
tracing = "0.1"                 # Logging/tracing
tower = "0.4"                   # Service middleware
prometheus = "0.13"             # Metrics collection
tempfile = "3.8"                # Temporary directories
silver-platter = "0.1"          # Debian package operations
anyhow = "1.0"                  # Error handling
uuid = "1.6"                    # ID generation
```

### Critical Migration Patterns

1. **Upload Processing**:
   ```python
   # Python (subprocess + async)
   async def upload_build_result(log_id, artifact_manager, dput_host, debsign_keyid=None):
       with tempfile.TemporaryDirectory() as td:
           await artifact_manager.retrieve_artifacts(log_id, td)
           for changes_filename in changes_filenames:
               await debsign(td, changes_filename, debsign_keyid)
               await dput_changes(td, changes_filename, dput_host)
   ```
   
   ```rust
   // Rust (tokio + silver-platter)
   async fn upload_build_result(
       log_id: &str,
       artifact_manager: &ArtifactManager,
       dput_host: &str,
       debsign_keyid: Option<&str>,
   ) -> Result<(), UploadError> {
       let temp_dir = tempfile::tempdir()?;
       artifact_manager.retrieve_artifacts(log_id, temp_dir.path()).await?;
       for changes_file in find_changes_files(temp_dir.path()).await? {
           debsign(temp_dir.path(), &changes_file, debsign_keyid).await?;
           dput_changes(temp_dir.path(), &changes_file, dput_host).await?;
       }
       Ok(())
   }
   ```

2. **Redis Message Handling**:
   ```python
   # Python (redis-py + JSON)
   async def handle_result_message(msg):
       result = json.loads(msg["data"])
       if result["target"]["name"] != "debian":
           return
       await upload_build_result(result["log_id"], ...)
   ```
   
   ```rust
   // Rust (redis-rs + serde)
   async fn handle_result_message(msg: &redis::Msg) -> Result<(), MessageError> {
       let result: RunResult = serde_json::from_slice(msg.get_payload_bytes())?;
       if result.target.name != "debian" {
           return Ok(());
       }
       upload_build_result(&result.log_id, ...).await?;
       Ok(())
   }
   ```

3. **Database Backfill**:
   ```python
   # Python (asyncpg)
   async def backfill(db, artifact_manager, dput_host):
       async with db.acquire() as conn:
           query = "SELECT DISTINCT ON (distribution, source) ..."
           for row in await conn.fetch(query):
               await upload_build_result(row["run_id"], ...)
   ```
   
   ```rust
   // Rust (sqlx)
   async fn backfill(
       db: &PgPool,
       artifact_manager: &ArtifactManager,
       dput_host: &str,
   ) -> Result<(), BackfillError> {
       let query = "SELECT DISTINCT ON (distribution, source) ...";
       let rows = sqlx::query!(query).fetch_all(db).await?;
       for row in rows {
           upload_build_result(&row.run_id, artifact_manager, dput_host).await?;
       }
       Ok(())
   }
   ```

### Risk Mitigation

1. **External Process Integration**: Comprehensive error handling for `debsign` and `dput` failures
2. **File Permissions**: Proper umask handling and permission management for signing
3. **Redis Connection Reliability**: Connection pooling and automatic reconnection
4. **Artifact Availability**: Graceful handling of missing artifacts with proper logging

## Timeline and Effort Estimates

### Total Effort: 5-8 weeks (1.25-2 months)

| Phase | Duration | Effort Level | Risk Level |
|-------|----------|--------------|------------|
| 1. Core Service Infrastructure | 1-2 weeks | Low | Low |
| 2. Upload Processing Engine | 2-3 weeks | Medium | Medium |
| 3. Redis Integration and Messaging | 1-2 weeks | Medium | Low |
| 4. Database Integration and Backfill | 1-2 weeks | Medium | Low |
| 5. Service Orchestration and Testing | 1-2 weeks | Medium | Low |

### Critical Dependencies

- **Silver-platter Rust Crate**: Must support `debsign` and `dput` operations
- **Artifact Manager**: Required for file retrieval functionality
- **Configuration System**: Needed for service setup and Redis/DB connections
- **Redis Infrastructure**: Critical for message handling

### Success Metrics

1. **Functional Parity**: 100% feature compatibility with Python implementation
2. **Performance**: Upload processing time ≤ Python implementation
3. **Reliability**: Zero upload failures due to service issues
4. **Integration**: Seamless Redis message handling and database operations
5. **Operational**: Comprehensive logging and monitoring capabilities

## Integration Considerations

### Service Dependencies
- **Runner Service**: Sends upload triggers via Redis pub/sub
- **Artifact Manager**: Provides access to build artifacts
- **Database**: Requires read access to debian_build table for backfill
- **External Services**: Integrates with GPG keyring and dput upload targets

### Security Considerations
- **GPG Key Management**: Secure access to signing keys
- **Upload Credentials**: Proper handling of dput authentication
- **File Permissions**: Correct umask and permission handling
- **Audit Logging**: Comprehensive logging of all upload operations

## Related Porting Plans

- 📋 **Master Plan**: [`../porting-plan.md`](../porting-plan.md) - Overall project coordination
- ✅ **Runner**: [`../runner/porting-plan.md`](../runner/porting-plan.md) - Already completed (provides triggers)
- ✅ **Publisher**: [`../publish/porting-plan.md`](../publish/porting-plan.md) - Already completed
- 🚧 **Archive**: [`../archive/porting-plan.md`](../archive/porting-plan.md) - In progress
- 🚧 **Site**: [`../site/porting-plan.md`](../site/porting-plan.md) - In progress

---

*This plan will be updated as implementation progresses and requirements evolve.*