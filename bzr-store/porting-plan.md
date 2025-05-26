# BZR Store Service Porting Plan

> **Status**: 🚧 **IN PROGRESS** - Minimal Rust implementation exists, needs complete Bazaar hosting service implementation.
> 
> 📋 **Master Plan**: See [`../porting-plan.md`](../porting-plan.md) for overall project coordination and dependencies.

## Overview

This document outlines the detailed plan for porting the Janitor bzr-store service from Python to Rust. The bzr-store service provides HTTP-accessible Bazaar repositories with both administrative and public interfaces, Bazaar smart protocol support, and integration with the Janitor platform's VCS management.

### Current State Analysis

**Python Implementation (`py/janitor/bzr_store.py`)**: ~455 lines
- Complete Bazaar hosting service with dual HTTP interfaces (admin + public)
- Bazaar smart protocol support via Breezy library integration
- Repository auto-creation and management with shared repositories
- Bzr diff and revision info APIs
- Campaign and role-based branch organization
- Worker authentication and permission management
- Database integration for codebase validation
- Comprehensive error handling and monitoring

**Rust Implementation (`bzr-store/`)**: ~5 lines (minimal)
- Only contains basic library structure
- Missing: Bazaar protocol, HTTP server, repository management, APIs

## Technical Architecture Analysis

### Current Python Stack
- **Web Framework**: aiohttp with dual applications (admin + public)
- **Bazaar Protocol**: Breezy library for Bazaar smart protocol implementation
- **Repository Management**: Breezy for Bazaar operations and shared repositories
- **Database**: AsyncPG for codebase validation and worker authentication
- **Process Management**: asyncio subprocess for `bzr` command operations
- **Templates**: Jinja2 for HTML rendering
- **Monitoring**: aiozipkin tracing and Prometheus metrics

### Target Rust Architecture
- **Web Framework**: Axum for HTTP server with dual applications
- **Bazaar Protocol**: 
  - Subprocess integration with `brz` (Breezy) commands
  - Alternative: Research pure Rust Bazaar implementation (limited availability)
- **Repository Management**: Subprocess-based Bazaar operations
- **Database**: sqlx for PostgreSQL operations
- **Process Management**: tokio::process for Bazaar subprocess operations
- **Templates**: Tera for HTML templating
- **Monitoring**: tracing ecosystem with opentelemetry

## Key Functionality Analysis

### Core Components to Port

1. **Bzr Diff and Revision Info APIs** (Lines 51-133)
   - Bzr diff generation via subprocess using `brz diff`
   - Revision walking and commit information extraction
   - RevisionId validation and error handling
   - JSON response formatting for revision info

2. **Repository Management** (Lines 154-171)
   - Auto-creation of missing repositories with shared repositories
   - Repository path management with campaign and role organization
   - Database validation for codebase existence
   - Shared repository initialization using Breezy ControlDir

3. **Bazaar Smart Protocol Handler** (Lines 173-232)
   - Breezy smart protocol implementation
   - Transport-based repository access with readonly wrapper
   - Campaign and role-based directory structure
   - Protocol factory and request handling

4. **Administrative APIs** (Lines 136-152, 235-270)
   - Bzr remote configuration management (parent branch setting)
   - Repository listing with content negotiation
   - Health and readiness endpoints
   - Repository existence validation

5. **Dual Web Applications** (Lines 272-348)
   - Admin interface with full write access
   - Public interface with controlled access via worker authentication
   - Route configuration for smart protocol endpoints
   - Middleware setup and database integration

## Porting Strategy

### Phase 1: Core Infrastructure (2-3 weeks)

#### 1.1 Project Setup and Dependencies (0.5 weeks)
- Configure Cargo.toml with web and subprocess dependencies
- Set up basic Axum application structure
- Add logging, tracing, and error handling
- Implement configuration management

**Effort Estimate**: ~100 lines
**Complexity**: Low - project setup and basic structure

**Deliverables:**
- Project structure with dependencies
- Basic Axum application
- Configuration management
- Logging infrastructure

#### 1.2 Repository Management Core (1 week)
- Implement repository path management with campaign/role structure
- Add repository auto-creation logic using subprocess calls
- Create shared repository initialization
- Add validation utilities

**Effort Estimate**: ~150 lines
**Complexity**: Medium - subprocess operations and file management

**Deliverables:**
- Repository management functions
- Auto-creation with shared repositories
- Path utilities with campaign/role support
- Subprocess wrapper functions

#### 1.3 Database Integration (0.5 weeks)
- Set up sqlx with PostgreSQL connection pooling
- Port codebase existence validation
- Add worker authentication queries
- Implement connection error handling

**Effort Estimate**: ~80 lines
**Complexity**: Low - basic database operations

**Deliverables:**
- Database connection setup
- Codebase validation functions
- Authentication queries
- Error handling

#### 1.4 Basic HTTP Server (1 week)
- Set up dual Axum applications (admin + public)
- Implement basic routing structure
- Add health and readiness endpoints
- Configure middleware and error handling

**Effort Estimate**: ~150 lines
**Complexity**: Medium - dual application setup

**Deliverables:**
- Dual HTTP applications
- Basic routing
- Health endpoints
- Middleware configuration

### Phase 2: Bazaar Protocol Implementation (4-5 weeks)

#### 2.1 Subprocess Integration Foundation (1 week)
- Implement Breezy/bzr command subprocess wrapper
- Add process management and error handling
- Create environment setup for Bazaar operations
- Implement timeout and cancellation handling

**Effort Estimate**: ~200 lines
**Complexity**: Medium - subprocess management

**Deliverables:**
- Breezy command wrapper
- Process lifecycle management
- Environment configuration
- Error handling and timeouts

#### 2.2 Bazaar Smart Protocol Handler (2-3 weeks)
- Research and implement Bazaar smart protocol in Rust
- Create protocol message parsing and handling
- Implement transport abstraction for repository access
- Add readonly transport wrapper for public access

**Effort Estimate**: ~300-400 lines (major component)
**Complexity**: Very High - custom protocol implementation

**Note**: This is the most challenging part as there's no existing Rust Bazaar library.
**Options**:
1. Implement minimal smart protocol subset
2. Use subprocess calls to `brz serve` with protocol bridging
3. Research existing protocol implementations

**Deliverables:**
- Bazaar smart protocol implementation
- Transport abstraction
- Protocol message handling
- Repository access control

#### 2.3 Repository Access and Security (1 week)
- Implement worker authentication integration
- Add repository permission checking
- Create readonly transport enforcement
- Add security validation and access control

**Effort Estimate**: ~150 lines
**Complexity**: Medium - security and access control

**Deliverables:**
- Authentication integration
- Permission checking
- Access control enforcement
- Security validation

### Phase 3: API Endpoints (2-3 weeks)

#### 3.1 Bzr Diff and Revision APIs (1.5 weeks)
- Port bzr diff generation using subprocess
- Implement revision walking and commit extraction
- Add RevisionId validation and parsing
- Create JSON response formatting

**Effort Estimate**: ~200 lines
**Complexity**: High - Bazaar operations and JSON APIs

**Deliverables:**
- Bzr diff API endpoint
- Revision info API
- JSON response handling
- RevisionId validation

#### 3.2 Repository Management APIs (1 week)
- Port remote configuration management
- Implement repository listing with content negotiation
- Add repository creation and validation
- Create administrative endpoints

**Effort Estimate**: ~150 lines
**Complexity**: Medium - CRUD operations and content negotiation

**Deliverables:**
- Remote management API
- Repository listing
- Content negotiation
- Admin endpoints

#### 3.3 Campaign and Role Support (0.5 weeks)
- Implement campaign-based repository organization
- Add role-based branch management
- Create directory structure validation
- Add configuration validation

**Effort Estimate**: ~80 lines
**Complexity**: Low - directory management

**Deliverables:**
- Campaign organization
- Role-based branches
- Directory validation
- Configuration support

### Phase 4: Testing and Integration (2-3 weeks)

#### 4.1 Protocol Testing (1.5 weeks)
- Create comprehensive test suite for Bazaar operations
- Test smart protocol compatibility with Bazaar clients
- Add performance benchmarks
- Create mock repository testing

**Effort Estimate**: ~250 lines of test code
**Complexity**: Medium - protocol testing

**Deliverables:**
- Bazaar protocol tests
- Client compatibility tests
- Performance benchmarks
- Mock testing utilities

#### 4.2 Integration Testing (1 week)
- Test dual application setup
- Validate authentication and authorization
- Add end-to-end workflow testing
- Create error scenario testing

**Deliverables:**
- Integration test suite
- Authentication testing
- Workflow validation
- Error handling tests

#### 4.3 Production Readiness (0.5 weeks)
- Add comprehensive monitoring and logging
- Create deployment documentation
- Add operational runbooks
- Implement graceful shutdown

**Deliverables:**
- Production monitoring
- Deployment guides
- Operational documentation
- Service management

## Implementation Details

### Key Dependencies

**Rust Crates:**
```toml
[dependencies]
axum = "0.7"                    # Web framework
tokio = { version = "1.0", features = ["full"] }
serde = "1.0"                   # Serialization
sqlx = "0.7"                    # Database toolkit
tera = "1.19"                   # Template engine
tower = "0.4"                   # Service middleware
tower-http = "0.5"              # HTTP utilities
tracing = "0.1"                 # Logging/tracing
uuid = "1.6"                    # ID generation
mime = "0.3"                    # MIME type handling
percent-encoding = "2.3"        # URL encoding
chrono = "0.4"                  # Date/time handling
bytes = "1.5"                   # Byte manipulation
futures = "0.3"                 # Async utilities
```

### Critical Migration Patterns

1. **Bazaar Subprocess Operations**:
   ```python
   # Python (asyncio subprocess)
   args = [sys.executable, "-m", "breezy", "diff", f"-rrevid:{old_revid}..revid:{new_revid}"]
   p = await asyncio.create_subprocess_exec(*args, stdout=PIPE, stderr=PIPE)
   stdout, stderr = await p.communicate()
   ```
   
   ```rust
   // Rust (tokio process)
   let mut cmd = Command::new("brz")
       .args(["diff", &format!("-rrevid:{}..revid:{}", old_revid, new_revid)])
       .stdout(Stdio::piped())
       .stderr(Stdio::piped())
       .spawn()?;
   
   let output = cmd.wait_with_output().await?;
   ```

2. **Bazaar Smart Protocol Handling**:
   ```python
   # Python (Breezy library)
   protocol_factory, unused_bytes = medium._get_protocol_factory_for_bytes(request_data)
   smart_protocol_request = protocol_factory(backing_transport, out_buffer.write, ".")
   smart_protocol_request.accept_bytes(unused_bytes)
   ```
   
   ```rust
   // Rust (custom implementation or subprocess bridge)
   let protocol_handler = BazaarProtocolHandler::new(transport, jail_root);
   let response = protocol_handler.handle_request(&request_data).await?;
   ```

3. **Repository Management**:
   ```python
   # Python (Breezy)
   try:
       repo = Repository.open(repo_path)
   except NotBranchError:
       controldir = ControlDir.create(repo_path)
       repo = controldir.create_repository(shared=True)
   ```
   
   ```rust
   // Rust (subprocess)
   match check_repository_exists(&repo_path).await {
       Ok(repo) => repo,
       Err(_) => {
           std::fs::create_dir_all(&repo_path)?;
           create_shared_repository(&repo_path).await?
       }
   }
   ```

### Risk Mitigation

1. **Bazaar Protocol Complexity**: Consider subprocess bridge to `brz serve` as fallback
2. **Limited Rust Ecosystem**: Bazaar has minimal Rust support compared to Git
3. **Performance**: Profile subprocess vs potential native implementation
4. **Compatibility**: Ensure compatibility with existing Bazaar clients

## Timeline and Effort Estimates

### Total Effort: 10-14 weeks (2.5-3.5 months)

| Phase | Duration | Effort Level | Risk Level |
|-------|----------|--------------|------------|
| 1. Core Infrastructure | 2-3 weeks | Medium | Low |
| 2. Bazaar Protocol Implementation | 4-5 weeks | Very High | Very High |
| 3. API Endpoints | 2-3 weeks | High | Medium |
| 4. Testing and Integration | 2-3 weeks | Medium | Medium |

### Critical Dependencies

- **Breezy/Bazaar**: Must be available for subprocess operations
- **Database Schema**: Requires stable codebase and worker tables
- **Authentication System**: Worker credential validation
- **File System**: Local repository storage with proper permissions

### Success Metrics

1. **Protocol Compatibility**: 100% Bazaar client compatibility
2. **Performance**: Repository operations ≤ Python implementation
3. **Feature Parity**: All administrative and smart protocol features working
4. **Security**: Proper authentication and access control
5. **Reliability**: Stable subprocess handling and error recovery

## Major Technical Challenges

### Bazaar Smart Protocol Implementation
The biggest challenge is implementing the Bazaar smart protocol in Rust. Unlike Git, Bazaar has very limited Rust ecosystem support.

**Options**:
1. **Minimal Implementation**: Implement only the smart protocol subset needed by Janitor
2. **Subprocess Bridge**: Use `brz serve` with protocol bridging (lower performance, higher compatibility)
3. **Research Alternative**: Investigate if any Rust Bazaar libraries exist or are in development

**Recommendation**: Start with subprocess bridge approach for compatibility, then evaluate native implementation.

## Integration Considerations

### Service Dependencies
- **Runner/Worker Services**: Use bzr-store for repository access
- **Site Service**: Links to repository browsing (if implemented)
- **Database**: Requires read access to codebase and worker tables
- **File System**: Shared repository storage with proper permissions

### Bazaar Client Compatibility
- Standard Bazaar clients must work without modification
- HTTP(S) branch, pull, and push operations
- Authentication via HTTP basic auth for workers
- Proper Bazaar protocol error handling and responses

## Related Porting Plans

- 📋 **Master Plan**: [`../porting-plan.md`](../porting-plan.md) - Overall project coordination
- ✅ **Runner**: [`../runner/porting-plan.md`](../runner/porting-plan.md) - Already completed (uses bzr-store)
- ✅ **Publisher**: [`../publish/porting-plan.md`](../publish/porting-plan.md) - Already completed
- 🚧 **Git Store**: [`../git-store/porting-plan.md`](../git-store/porting-plan.md) - In progress (similar VCS hosting)
- 🚧 **Site**: [`../site/porting-plan.md`](../site/porting-plan.md) - In progress

---

*This plan will be updated as implementation progresses and requirements evolve.*