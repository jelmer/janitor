# Git Store Service Porting Plan

> **Status**: 🚧 **IN PROGRESS** - Phase 1 (Core Infrastructure) completed. Git hosting service foundation implemented.
> 
> 📋 **Master Plan**: See [`../porting-plan.md`](../porting-plan.md) for overall project coordination and dependencies.

## Overview

This document outlines the detailed plan for porting the Janitor git-store service from Python to Rust. The git-store service provides HTTP-accessible Git repositories with both administrative and public interfaces, Git protocol support, web-based repository browsing, and integration with the Janitor platform's codebase management.

### Current State Analysis

**Python Implementation (`py/janitor/git_store.py`)**: ~752 lines
- Complete Git hosting service with dual HTTP interfaces (admin + public)
- Git protocol support via both CGI backend and Dulwich server
- Repository auto-creation and management
- Web-based repository browsing via Klaus integration
- Git diff and revision info APIs
- Worker authentication and permission management
- Database integration for codebase validation
- Comprehensive error handling and monitoring

**Rust Implementation (`git-store/`)**: ~8 lines (minimal)
- Only contains basic library structure and hello world main
- Missing: Git protocol, HTTP server, repository management, web interface

## Technical Architecture Analysis

### Current Python Stack
- **Web Framework**: aiohttp with dual applications (admin + public)
- **Git Protocol**: 
  - CGI backend via `git http-backend` subprocess
  - Dulwich pure-Python implementation option
- **Repository Management**: Dulwich for Git operations and repository handling
- **Web Interface**: Klaus for repository browsing (Flask-based)
- **Database**: AsyncPG for codebase validation and worker authentication
- **Templates**: Jinja2 for HTML rendering
- **Process Management**: asyncio subprocess for Git operations
- **Monitoring**: aiozipkin tracing and Prometheus metrics

### Target Rust Architecture
- **Web Framework**: Axum for HTTP server with dual applications
- **Git Protocol**: 
  - Subprocess integration with `git http-backend`
  - Alternative: Pure Rust Git implementation (git2-rs or gix)
- **Repository Management**: git2-rs for Git operations
- **Web Interface**: Custom Rust implementation or integration with existing solution
- **Database**: sqlx for PostgreSQL operations
- **Templates**: Tera for HTML templating
- **Process Management**: tokio::process for Git subprocess operations
- **Monitoring**: tracing ecosystem with opentelemetry

## Key Functionality Analysis

### Core Components to Port

1. **Git Diff and Revision Info APIs** (Lines 54-144)
   - Git diff generation via subprocess
   - Revision walking and commit information extraction
   - SHA validation and error handling
   - JSON response formatting

2. **Repository Management** (Lines 147-157)
   - Auto-creation of missing repositories
   - Repository path management
   - Database validation for codebase existence
   - Bare repository initialization

3. **Git Protocol Handlers** (Lines 272-509)
   - CGI backend integration (`git http-backend`)
   - Dulwich pure-Python protocol support
   - HTTP header parsing and response streaming
   - Authentication and permission checking

4. **Web Repository Browser** (Lines 175-246)
   - Klaus integration for Git repository browsing
   - Flask WSGI application wrapping
   - Template customization and branding
   - URL routing and endpoint mapping

5. **Administrative APIs** (Lines 249-270, 512-537)
   - Git remote configuration management
   - Repository listing with content negotiation
   - Health and readiness endpoints
   - Repository existence validation

6. **Dual Web Applications** (Lines 552-640)
   - Admin interface with full write access
   - Public interface with controlled access
   - Worker authentication integration
   - Route configuration and middleware setup

## Porting Strategy

### Phase 1: Core Infrastructure ✅ **COMPLETED** (2-3 weeks)

#### 1.1 Project Setup and Dependencies ✅ **COMPLETED** (0.5 weeks)
- ✅ Configure Cargo.toml with Git and web dependencies
- ✅ Set up basic Axum application structure
- ✅ Add logging, tracing, and error handling
- ✅ Implement configuration management

**Effort Estimate**: ~100 lines ✅ **Actual**: ~150 lines
**Complexity**: Low - project setup and basic structure

**Deliverables:**
- ✅ Project structure with dependencies (Axum, git2, sqlx, tera, etc.)
- ✅ Basic Axum application with dual admin/public servers
- ✅ Configuration management with TOML and environment variables
- ✅ Logging infrastructure with tracing

#### 1.2 Repository Management Core ✅ **COMPLETED** (1 week)
- ✅ Implement Git repository operations using git2-rs
- ✅ Add repository auto-creation logic
- ✅ Create repository path management
- ✅ Add SHA validation utilities

**Effort Estimate**: ~200 lines ✅ **Actual**: ~220 lines
**Complexity**: Medium - Git operations and file management

**Deliverables:**
- ✅ Repository management functions (RepositoryManager)
- ✅ Auto-creation logic (open_or_create method)
- ✅ Path utilities and validation
- ✅ Git operations wrapper with comprehensive error handling

#### 1.3 Database Integration ✅ **COMPLETED** (0.5 weeks)
- ✅ Set up sqlx with PostgreSQL connection pooling
- ✅ Port codebase existence validation
- ✅ Add worker authentication queries
- ✅ Implement connection error handling

**Effort Estimate**: ~80 lines ✅ **Actual**: ~180 lines
**Complexity**: Low - basic database operations

**Deliverables:**
- ✅ Database connection setup with pooling
- ✅ Codebase validation functions (DatabaseManager)
- ✅ Authentication queries with bcrypt password verification
- ✅ Error handling and health checks

#### 1.4 Basic HTTP Server ✅ **COMPLETED** (1 week)
- ✅ Set up dual Axum applications (admin + public)
- ✅ Implement basic routing structure
- ✅ Add health and readiness endpoints
- ✅ Configure middleware and error handling

**Effort Estimate**: ~150 lines ✅ **Actual**: ~250 lines
**Complexity**: Medium - dual application setup

**Deliverables:**
- ✅ Dual HTTP applications (admin on 9421, public on 9422)
- ✅ Basic routing with repository and diff endpoints
- ✅ Health endpoints with database connectivity checks
- ✅ Middleware configuration (compression, tracing, CORS)

#### Phase 1 Summary ✅ **COMPLETED**

**Total Implementation**: ~800 lines of Rust code across 6 modules
- **config.rs**: Configuration management with TOML/env support (120 lines)
- **database.rs**: PostgreSQL integration with worker auth (180 lines)
- **error.rs**: Comprehensive error handling (80 lines)
- **git_http.rs**: Basic git diff and revision APIs (180 lines)
- **repository.rs**: Git repository management (220 lines)
- **web.rs**: HTTP server with dual applications (250 lines)
- **main.rs**: Service entry point (100 lines)

**Key Achievements**:
- ✅ Complete foundation for git hosting service
- ✅ Dual HTTP servers (admin/public) with proper separation
- ✅ Repository auto-creation and management using git2-rs
- ✅ Database integration with worker authentication
- ✅ Basic git diff and revision info APIs
- ✅ Comprehensive error handling and logging
- ✅ Configuration management and example config
- ✅ Full test coverage with 9 passing tests
- ✅ Documentation and README

**Ready for Phase 2**: Git HTTP backend integration and full Git protocol support.

### Phase 2: Git Protocol Implementation (3-4 weeks)

#### 2.1 Git HTTP Backend Integration (2 weeks)
- Port CGI backend functionality using tokio::process
- Implement HTTP header parsing and response streaming
- Add environment variable setup for Git operations
- Create chunked response handling

**Effort Estimate**: ~300 lines of complex subprocess and HTTP handling
**Complexity**: Very High - subprocess integration, HTTP streaming, header parsing

**Deliverables:**
- Git HTTP backend integration
- Request/response streaming
- Environment setup
- Error handling and timeouts

#### 2.2 Git Operations and Validation (1 week)
- Port SHA validation and Git safety checks
- Implement Git service authorization
- Add repository access control
- Create Git command validation

**Effort Estimate**: ~150 lines
**Complexity**: Medium - validation logic and security

**Deliverables:**
- SHA validation functions
- Service authorization
- Access control logic
- Command validation

#### 2.3 Alternative Git Implementation (1 week)
- Research and implement pure Rust Git protocol (optional)
- Consider git2-rs or gix for direct Git operations
- Evaluate performance vs subprocess approach
- Add configuration option for implementation choice

**Effort Estimate**: ~200 lines (research + implementation)
**Complexity**: High - Git protocol implementation

**Deliverables:**
- Pure Rust Git option evaluation
- Implementation choice configuration
- Performance comparison
- Alternative backend

### Phase 3: API Endpoints (2-3 weeks)

#### 3.1 Git Diff and Revision APIs (1.5 weeks)
- Port git diff generation using subprocess or git2-rs
- Implement revision walking and commit extraction
- Add JSON response formatting
- Create timeout and error handling

**Effort Estimate**: ~250 lines
**Complexity**: High - Git operations and JSON APIs

**Deliverables:**
- Git diff API endpoint
- Revision info API
- JSON response handling
- Timeout management

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

#### 3.3 Authentication and Authorization (0.5 weeks)
- Port worker authentication logic
- Implement permission checking middleware
- Add access control for write operations
- Create authentication utilities

**Effort Estimate**: ~100 lines
**Complexity**: Medium - security and middleware

**Deliverables:**
- Authentication middleware
- Permission checking
- Access control logic
- Security utilities

### Phase 4: Web Repository Browser (3-4 weeks)

#### 4.1 Template System Setup (1 week)
- Set up Tera templating engine
- Port repository listing templates
- Create basic layout and styling
- Implement template utilities

**Effort Estimate**: ~150 lines + template files
**Complexity**: Medium - templating and styling

**Deliverables:**
- Tera template system
- Repository listing templates
- Basic styling
- Template utilities

#### 4.2 Repository Browser Implementation (2-3 weeks)
- Research replacement for Klaus functionality
- Implement file browsing and viewing
- Add commit history and diff viewing
- Create blob and tree rendering

**Effort Estimate**: ~400-500 lines (major component)
**Complexity**: Very High - complex Git browsing functionality

**Options:**
1. Port Klaus functionality to Rust
2. Integrate existing Rust Git web interface
3. Build minimal custom implementation

**Deliverables:**
- Repository browsing interface
- File and commit viewing
- Git history navigation
- Blob and diff rendering

#### 4.3 Integration and Polish (1 week)
- Integrate browser with main application
- Add proper URL routing and redirects
- Implement responsive design
- Add error pages and handling

**Effort Estimate**: ~100 lines + styling
**Complexity**: Medium - integration and polish

**Deliverables:**
- Integrated web interface
- URL routing
- Responsive design
- Error handling

### Phase 5: Testing and Production Readiness (2-3 weeks)

#### 5.1 Integration Testing (1.5 weeks)
- Create comprehensive test suite for Git operations
- Test HTTP protocol compatibility
- Add performance benchmarks
- Create mock repository testing

**Effort Estimate**: ~300 lines of test code
**Complexity**: Medium - testing infrastructure

**Deliverables:**
- Git protocol tests
- HTTP compatibility tests
- Performance benchmarks
- Mock testing utilities

#### 5.2 Performance Optimization (1 week)
- Profile Git operations and HTTP serving
- Optimize subprocess handling and streaming
- Tune connection pooling and caching
- Implement request optimization

**Deliverables:**
- Performance profiling
- Optimization implementation
- Caching strategies
- Request tuning

#### 5.3 Production Deployment (0.5 weeks)
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
git2 = "0.18"                   # Git operations
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

1. **Git Subprocess Operations**:
   ```python
   # Python (asyncio subprocess)
   p = await asyncio.create_subprocess_exec(
       "git", "diff", old_sha, new_sha,
       stdout=asyncio.subprocess.PIPE,
       cwd=repo_path
   )
   stdout, stderr = await p.communicate()
   ```
   
   ```rust
   // Rust (tokio process)
   let mut cmd = Command::new("git")
       .args(["diff", &old_sha, &new_sha])
       .current_dir(&repo_path)
       .stdout(Stdio::piped())
       .spawn()?;
   
   let output = cmd.wait_with_output().await?;
   ```

2. **HTTP Response Streaming**:
   ```python
   # Python (aiohttp streaming)
   response = web.StreamResponse(headers=headers)
   await response.prepare(request)
   chunk = await p.stdout.read(CHUNK_SIZE)
   while chunk:
       await response.write(chunk)
       chunk = await p.stdout.read(CHUNK_SIZE)
   ```
   
   ```rust
   // Rust (axum streaming)
   let stream = ReaderStream::new(stdout);
   let body = StreamBody::new(stream.map_ok(Frame::data));
   Response::builder()
       .header("content-type", content_type)
       .body(body)?
   ```

3. **Repository Management**:
   ```python
   # Python (dulwich)
   try:
       repo = Repo(repo_path)
   except NotGitRepository:
       repo = Repo.init_bare(repo_path, mkdir=True)
   ```
   
   ```rust
   // Rust (git2)
   let repo = match Repository::open(&repo_path) {
       Ok(repo) => repo,
       Err(_) => {
           std::fs::create_dir_all(&repo_path)?;
           Repository::init_bare(&repo_path)?
       }
   };
   ```

### Risk Mitigation

1. **Git Protocol Compatibility**: Extensive testing with standard Git clients
2. **Performance**: Profile subprocess vs native Git operations
3. **Web Browser Complexity**: Consider existing Rust solutions or minimal implementation
4. **Subprocess Security**: Proper input validation and environment isolation

## Timeline and Effort Estimates

### Total Effort: 12-17 weeks (3-4.25 months)

| Phase | Duration | Effort Level | Risk Level |
|-------|----------|--------------|------------|
| 1. Core Infrastructure | 2-3 weeks | Medium | Low |
| 2. Git Protocol Implementation | 3-4 weeks | Very High | High |
| 3. API Endpoints | 2-3 weeks | High | Medium |
| 4. Web Repository Browser | 3-4 weeks | Very High | High |
| 5. Testing and Production Readiness | 2-3 weeks | Medium | Low |

### Critical Dependencies

- **Git Infrastructure**: Git binary must be available for subprocess operations
- **Database Schema**: Requires stable codebase and worker tables
- **Authentication System**: Worker credential validation
- **File System**: Local repository storage with proper permissions

### Success Metrics

1. **Protocol Compatibility**: 100% Git client compatibility
2. **Performance**: Repository operations ≤ Python implementation
3. **Feature Parity**: All administrative and browsing features working
4. **Web Interface**: Functional repository browsing and file viewing
5. **Security**: Proper authentication and access control

## Integration Considerations

### Service Dependencies
- **Runner/Worker Services**: Use git-store for repository access
- **Site Service**: Links to repository browsing interfaces
- **Database**: Requires read access to codebase and worker tables
- **File System**: Shared repository storage with proper permissions

### Git Client Compatibility
- Standard Git clients must work without modification
- HTTP(S) clone, fetch, and push operations
- Authentication via HTTP basic auth for workers
- Proper Git protocol error handling and responses

## Related Porting Plans

- 📋 **Master Plan**: [`../porting-plan.md`](../porting-plan.md) - Overall project coordination
- ✅ **Runner**: [`../runner/porting-plan.md`](../runner/porting-plan.md) - Already completed (uses git-store)
- ✅ **Publisher**: [`../publish/porting-plan.md`](../publish/porting-plan.md) - Already completed
- 🚧 **Site**: [`../site/porting-plan.md`](../site/porting-plan.md) - In progress (links to git-store)
- 🔄 **BZR Store**: Similar VCS hosting service for Bazaar repositories

---

*This plan will be updated as implementation progresses and requirements evolve.*