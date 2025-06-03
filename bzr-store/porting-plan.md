# BZR Store Service Porting Plan

> **Status**: 🔄 **IN PROGRESS** - Phase 1 COMPLETE ✅ | Phase 2 starting (PyO3 integration)
> 
> 📋 **Master Plan**: See [`../porting-plan.md`](../porting-plan.md) for overall project coordination and dependencies.

## Overview

This document outlines the detailed plan for porting the Janitor bzr-store service from Python to Rust using PyO3 for Bazaar library integration. The bzr-store service provides HTTP-accessible Bazaar repositories with administrative and public interfaces, Bazaar smart protocol support, and integration with the Janitor platform's VCS management.

### Current State Analysis

**Python Implementation (`py/janitor/bzr_store.py`)**: ~455 lines
- Complete Bazaar hosting service with dual HTTP interfaces (admin + public)
- Bazaar smart protocol support via Breezy library
- Repository auto-creation with shared repository support
- Campaign and role-based repository organization
- Bazaar diff and revision info APIs
- Worker authentication and permission management
- Database integration for codebase validation

**Rust Implementation (`bzr-store/`)**: ~5 lines (minimal skeleton)
- Only contains basic library structure
- Missing: Bazaar protocol, HTTP server, repository management, PyO3 integration

## Technical Architecture Strategy

### PyO3 Hybrid Approach

Given the complexity of the Bazaar protocol and the mature Python Breezy library, we'll use a hybrid approach:

```
┌─────────────────────────┐
│   Axum Web Server       │  ← Pure Rust (HTTP, routing, middleware)
│  (Pure Rust)            │
└───────────┬─────────────┘
            │
┌───────────▼─────────────┐
│  Business Logic         │  ← Mixed (auth, config in Rust, 
│  (Pure Rust + PyO3)     │    Bazaar ops via PyO3)
└───────────┬─────────────┘
            │
┌───────────▼─────────────┐
│  PyO3 Bridge Layer      │  ← Rust-Python interop
│  (Rust-Python interop)  │
└───────────┬─────────────┘
            │
┌───────────▼─────────────┐
│  Breezy Library         │  ← Python (mature Bazaar implementation)
│  (Python - Bazaar impl) │
└─────────────────────────┘
```

### Component Distribution

1. **Pure Rust Components** (no Python dependencies):
   - Web server infrastructure (Axum)
   - Authentication and database operations
   - Repository path management and validation
   - Configuration management and logging
   - Health/ready endpoints
   - Repository listing and basic file operations

2. **PyO3 Integration Required** (Python Breezy library):
   - Bazaar smart protocol handling (bzr://, bzr+ssh://)
   - Repository creation with shared repository support
   - Complex revision operations and diff generation
   - Branch and transport management
   - Bazaar-specific metadata operations

3. **Subprocess Fallback** (for simple operations):
   - Basic diff generation (`brz diff`)
   - Repository information queries
   - Remote configuration setup

## Implementation Phases

### Phase 1: Foundation and Subprocess MVP (1-2 weeks) ✅ **COMPLETE**

#### 1.1 Project Setup and Core Infrastructure (3-4 days)
**Target**: Basic Rust project with PyO3 and web server setup

- **Dependencies Setup**:
  ```toml
  [dependencies]
  axum = "0.7"
  tokio = { version = "1.0", features = ["full"] }
  sqlx = { version = "0.7", features = ["postgres", "runtime-tokio-rustls"] }
  pyo3 = { version = "0.22", features = ["extension-module", "abi3-py38"] }
  tera = "1.19"
  serde = { version = "1.0", features = ["derive"] }
  tracing = "0.1"
  url = "2.4"
  ```

- **Project Structure**:
  ```
  bzr-store/
  ├── Cargo.toml
  ├── src/
  │   ├── main.rs          # Service entry point
  │   ├── lib.rs           # Library exports
  │   ├── config.rs        # Configuration management
  │   ├── database.rs      # PostgreSQL integration
  │   ├── error.rs         # Error handling
  │   ├── repository.rs    # Repository management
  │   ├── pyo3_bridge.rs   # PyO3 integration layer
  │   ├── subprocess.rs    # Subprocess fallback operations
  │   └── web.rs           # HTTP server and routes
  ```

- **Basic Configuration**:
  ```rust
  #[derive(Debug, Deserialize)]
  pub struct Config {
      pub database_url: String,
      pub repository_path: PathBuf,
      pub admin_bind: SocketAddr,  // e.g., 127.0.0.1:9929
      pub public_bind: SocketAddr, // e.g., 127.0.0.1:9930
      pub python_path: Option<String>,
  }
  ```

#### 1.2 Subprocess-based Basic Operations (3-4 days)
**Target**: Basic repository operations using `brz` subprocess

- **Repository Management**:
  ```rust
  pub struct BzrRepositoryManager {
      base_path: PathBuf,
      database: DatabaseManager,
  }
  
  impl BzrRepositoryManager {
      pub async fn ensure_repository(&self, codebase: &str, campaign: &str, role: &str) -> Result<PathBuf> {
          // Create shared repository structure if needed
          // Campaign-based organization: base_path/campaign/codebase/role
      }
      
      pub async fn get_diff_subprocess(&self, old_revid: &str, new_revid: &str) -> Result<Vec<u8>> {
          // Use `brz diff -r old..new` subprocess
      }
  }
  ```

- **Basic HTTP Endpoints**:
  ```rust
  // Health and readiness
  GET /health
  GET /ready
  
  // Basic repository operations
  GET /{codebase}/diff?old={old}&new={new}
  GET /{codebase}/revision-info?old={old}&new={new}
  GET /repositories
  ```

#### 1.3 Database Integration and Authentication (2-3 days)
**Target**: Worker authentication and codebase validation

- **Database Operations**:
  ```rust
  impl DatabaseManager {
      pub async fn validate_codebase(&self, codebase: &str) -> Result<bool>;
      pub async fn authenticate_worker(&self, username: &str, password: &str) -> Result<bool>;
      pub async fn get_worker_permissions(&self, username: &str) -> Result<WorkerPermissions>;
  }
  ```

- **Authentication Middleware**:
  ```rust
  pub async fn auth_middleware(
      req: Request<Body>,
      next: Next<Body>,
  ) -> Result<Response<Body>, AuthError> {
      // HTTP Basic Auth for workers
      // Admin vs public interface detection
  }
  ```

### Phase 2: PyO3 Integration and Smart Protocol (2-3 weeks)

#### 2.1 PyO3 Bridge Layer Setup (1 week)
**Target**: Basic Python-Rust interop for Breezy operations

- **PyO3 Bridge Module**:
  ```rust
  use pyo3::prelude::*;
  
  pub struct BreezyBridge {
      python: Python<'static>,
      breezy_module: PyObject,
  }
  
  impl BreezyBridge {
      pub fn new() -> PyResult<Self> {
          // Initialize Python interpreter
          // Import breezy modules
      }
      
      pub async fn create_shared_repository(&self, path: &Path) -> PyResult<()> {
          // Call breezy.repository.Repository.init_shared()
      }
      
      pub async fn get_transport(&self, url: &str) -> PyResult<PyObject> {
          // Get breezy transport for URL
      }
  }
  ```

- **Python Environment Setup**:
  - Embed Python interpreter in Rust binary
  - Import required Breezy modules on startup
  - Handle Python GIL management for async operations

#### 2.2 Repository Operations via PyO3 (1-2 weeks)
**Target**: Core repository management using Breezy library

- **Repository Creation**:
  ```rust
  impl BzrRepositoryManager {
      pub async fn create_repository_pyo3(&self, path: &Path) -> Result<()> {
          self.breezy_bridge.create_shared_repository(path).await
      }
      
      pub async fn init_branch_pyo3(&self, branch_path: &Path, repository_path: &Path) -> Result<()> {
          // Use breezy to create branch in shared repository
      }
  }
  ```

- **Diff and Revision Operations**:
  ```rust
  impl BzrRepositoryManager {
      pub async fn get_diff_pyo3(&self, repo_path: &Path, old_revid: &str, new_revid: &str) -> Result<Vec<u8>> {
          // Use breezy diff functionality
      }
      
      pub async fn get_revision_info_pyo3(&self, repo_path: &Path, old_revid: &str, new_revid: &str) -> Result<Vec<RevisionInfo>> {
          // Walk revision history using breezy
      }
  }
  ```

#### 2.3 Smart Protocol Integration (1 week)
**Target**: Bazaar smart protocol support for clone/pull/push operations

- **Smart Server Setup**:
  ```rust
  pub async fn handle_smart_protocol(
      path: Path<String>,
      headers: HeaderMap,
      body: Bytes,
  ) -> Result<Response<Body>, BzrError> {
      // Parse bzr smart protocol requests
      // Delegate to breezy smart server implementation
      // Stream responses back to client
  }
  ```

- **Protocol Endpoints**:
  ```
  POST /{codebase}/.bzr/smart  # Smart protocol endpoint
  GET /{codebase}/.bzr/*       # Repository metadata
  ```

### Phase 3: Full Feature Parity (1-2 weeks)

#### 3.1 Advanced Repository Features (1 week)
**Target**: Complete repository management functionality

- **Campaign and Role Support**:
  ```rust
  pub struct RepositoryPath {
      pub campaign: String,
      pub codebase: String,
      pub role: String,
  }
  
  impl BzrRepositoryManager {
      pub fn get_repository_path(&self, campaign: &str, codebase: &str, role: &str) -> PathBuf {
          self.base_path.join(campaign).join(codebase).join(role)
      }
      
      pub async fn ensure_campaign_structure(&self, campaign: &str) -> Result<()> {
          // Create campaign directory and shared repository
      }
  }
  ```

- **Repository Listing and Management**:
  ```rust
  impl BzrRepositoryManager {
      pub async fn list_repositories(&self) -> Result<Vec<RepositoryInfo>>;
      pub async fn get_repository_info(&self, path: &RepositoryPath) -> Result<RepositoryInfo>;
      pub async fn configure_remote(&self, path: &RepositoryPath, remote_url: &str) -> Result<()>;
  }
  ```

#### 3.2 Complete API Implementation (1 week)
**Target**: All HTTP endpoints with full functionality

- **Admin Interface** (port 9929):
  ```rust
  // Repository management
  GET /repositories
  POST /repositories/{campaign}/{codebase}/{role}
  GET /{campaign}/{codebase}/{role}/info
  
  // Remote configuration
  POST /{campaign}/{codebase}/{role}/remotes
  GET /{campaign}/{codebase}/{role}/remotes
  
  // Health and admin operations
  GET /health
  GET /ready
  ```

- **Public Interface** (port 9930):
  ```rust
  // Read-only repository access
  GET /{campaign}/{codebase}/{role}/diff
  GET /{campaign}/{codebase}/{role}/revision-info
  POST /{campaign}/{codebase}/{role}/.bzr/smart  # Smart protocol
  GET /{campaign}/{codebase}/{role}/.bzr/*       # Repository files
  ```

### Phase 4: Testing and Optimization (1 week)

#### 4.1 Integration Testing (3-4 days)
**Target**: Comprehensive test suite

- **Bazaar Client Testing**:
  ```rust
  #[tokio::test]
  async fn test_bzr_clone() {
      // Test bzr clone from service
  }
  
  #[tokio::test]
  async fn test_bzr_push() {
      // Test bzr push to service
  }
  ```

- **API Compatibility Testing**:
  ```rust
  #[tokio::test]
  async fn test_python_compatibility() {
      // Compare outputs with Python implementation
  }
  ```

#### 4.2 Performance Optimization (2-3 days)
**Target**: Optimize PyO3 integration

- **GIL Management**: Optimize Python GIL usage for concurrent operations
- **Connection Pooling**: Reuse Python objects where possible
- **Memory Management**: Proper cleanup of PyO3 objects
- **Caching**: Cache frequently accessed Python objects

#### 4.3 Production Readiness (1-2 days)
**Target**: Deployment preparation

- **Configuration**: Complete configuration options
- **Logging**: Comprehensive logging and tracing
- **Monitoring**: Health checks and metrics
- **Documentation**: Deployment and operation guides

## Error Handling Strategy

### Error Types
```rust
#[derive(Debug, thiserror::Error)]
pub enum BzrError {
    #[error("Python error: {0}")]
    Python(#[from] pyo3::PyErr),
    
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Repository error: {message}")]
    Repository { message: String },
    
    #[error("Authentication failed")]
    AuthenticationFailed,
    
    #[error("Subprocess error: {0}")]
    Subprocess(String),
}
```

### Error Conversion
```rust
impl From<BzrError> for Response<Body> {
    fn from(error: BzrError) -> Self {
        match error {
            BzrError::AuthenticationFailed => (StatusCode::UNAUTHORIZED, "Authentication required").into_response(),
            BzrError::Repository { .. } => (StatusCode::NOT_FOUND, "Repository not found").into_response(),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response(),
        }
    }
}
```

## Performance Considerations

### PyO3 Optimization
1. **GIL Management**: Use `Python::allow_threads()` for I/O operations
2. **Object Caching**: Cache frequently used Python objects
3. **Batch Operations**: Group multiple Python calls when possible
4. **Memory Management**: Explicit cleanup of PyO3 objects

### Subprocess Fallback
- Use subprocess for operations that don't require complex Python integration
- Implement timeout and resource limits for subprocess calls
- Cache subprocess results where appropriate

## Testing Strategy

### Unit Tests
- Pure Rust components (config, auth, path management)
- PyO3 bridge layer with mocked Python objects
- Error handling and edge cases

### Integration Tests
- Full bzr client operations (clone, pull, push)
- HTTP API endpoints with real bzr client
- Database integration and authentication
- Performance benchmarks vs Python implementation

### Compatibility Tests
- Compare output with Python implementation
- Verify protocol compatibility with various bzr client versions
- Test edge cases and error conditions

## Security Considerations

### Python Integration Security
- Limit Python code execution to Breezy library only
- Validate all inputs before passing to Python
- Handle Python exceptions safely
- Resource limits for Python operations

### Repository Security
- Validate repository paths to prevent directory traversal
- Secure worker authentication
- Audit logging for administrative operations
- Rate limiting for resource-intensive operations

## Migration Strategy

### Deployment Phases
1. **Development**: PyO3-based implementation with fallback to Python service
2. **Staging**: Side-by-side testing with Python implementation
3. **Production**: Gradual rollout with monitoring and rollback capability

### Compatibility
- Maintain HTTP API compatibility with Python implementation
- Ensure bzr client compatibility across versions
- Preserve repository structure and metadata

## Success Criteria

### Functional Requirements
- ✅ 100% bzr client compatibility (clone, pull, push operations)
- ✅ All HTTP API endpoints functional with identical responses
- ✅ Repository management preserves campaign/role structure
- ✅ Worker authentication and permission system working
- ✅ Database integration maintains data consistency

### Performance Requirements
- 🎯 HTTP response times ≤ Python implementation + 20%
- 🎯 bzr operations performance within 30% of Python implementation
- 🎯 Memory usage ≤ Python implementation
- 🎯 Concurrent connections support ≥ Python implementation

### Quality Requirements
- ✅ Comprehensive test coverage (>90% for critical paths)
- ✅ Integration tests verify bzr client compatibility
- ✅ Error handling provides clear feedback
- ✅ Logging and monitoring for production operations
- ✅ Documentation for deployment and operations

## Implementation Timeline

| Phase | Duration | Focus Area | Risk Level |
|-------|----------|------------|------------|
| 1 | 1-2 weeks | Foundation and subprocess MVP | Low |
| 2 | 2-3 weeks | PyO3 integration and smart protocol | High |
| 3 | 1-2 weeks | Full feature parity | Medium |
| 4 | 1 week | Testing and optimization | Low |

**Total Estimated Duration: 5-8 weeks**

## Risk Assessment

### High Risk Areas
1. **PyO3 Integration Complexity**: First major PyO3 integration in the project
2. **Bazaar Smart Protocol**: Complex protocol with many edge cases
3. **Python-Rust Async Integration**: Managing async operations across language boundary
4. **Performance Impact**: PyO3 overhead compared to pure Python

### Mitigation Strategies
1. **Incremental Implementation**: Start with subprocess fallback for core functionality
2. **Extensive Testing**: Comprehensive test suite with real bzr clients
3. **Performance Monitoring**: Continuous benchmarking during development
4. **Fallback Options**: Maintain subprocess operations as backup

## Dependencies and Integration

### External Dependencies
- **Breezy Library**: Python library for Bazaar operations
- **PyO3**: Rust-Python integration
- **Database**: PostgreSQL for worker authentication and codebase validation

### Service Integration
- **Runner/Worker Services**: Use bzr-store for Bazaar repository access
- **Site Service**: Links to repository browsing interfaces
- **Database**: Shared tables for workers and codebases

## Related Plans

- 📋 **Master Plan**: [`../porting-plan.md`](../porting-plan.md) - Overall project coordination
- ✅ **Git Store**: [`../git-store/porting-plan.md`](../git-store/porting-plan.md) - Parallel VCS service
- ✅ **Site**: [`../site/porting-plan.md`](../site/porting-plan.md) - Web interface integration
- ✅ **Runner**: [`../runner/porting-plan.md`](../runner/porting-plan.md) - VCS operations integration

---

*This plan leverages PyO3 to provide a practical migration path that maintains Bazaar functionality while achieving Rust migration goals. The hybrid approach balances implementation complexity with performance requirements.*