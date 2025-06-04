# Janitor Refactoring Roadmap

**Objective**: Eliminate technical debt and improve code maintainability  
**Timeline**: 4-6 weeks  
**Priority**: Medium (security issues resolved)

## Phase 1: Database Module Migration (Week 1)

### Goal: Eliminate database code duplication
**Current State**: 6 services with identical database connection code (~400 lines each)  
**Target State**: All services use shared `janitor::database` module

### Services to Migrate:
1. ✅ `site/src/database.rs` - Already partially migrated
2. `runner/src/database.rs` 
3. `archive/src/database.rs`
4. `auto-upload/src/database.rs`
5. `git-store/src/database.rs`
6. `bzr-store/src/database.rs`

### Migration Steps:
```rust
// 1. Replace service-specific Database struct
// Before:
pub struct Database {
    pool: PgPool,
}

// After:
use janitor::database::{Database, DatabaseConfig};
let db = Database::connect_with_config(
    DatabaseConfig::new(&config.database_url)
        .with_max_connections(10)
).await?;
```

### Expected Reduction: ~2,000 lines of duplicate code

## Phase 2: Error Handling Standardization (Week 2)

### Goal: Replace service-specific error types with unified JanitorError
**Current State**: Each service defines its own error enums  
**Target State**: All services use `janitor::error::JanitorError`

### Services to Migrate:
- All 8 services currently define similar error types
- Replace ~500 `unwrap()` calls with proper error propagation

### Migration Steps:
```rust
// 1. Replace service-specific errors
// Before:
#[derive(Debug)]
pub enum ServiceError {
    Database(sqlx::Error),
    Config(String),
}

// After:
use janitor::error::{JanitorError, Result};
// Remove service-specific error type entirely
```

### Priority Areas:
1. **Critical paths**: Error handling in request handlers
2. **Database operations**: Replace panicking queries  
3. **External commands**: Better error context

## Phase 3: Configuration Management (Week 3)

### Goal: Centralized configuration parsing
**Current State**: Each service duplicates config parsing logic  
**Target State**: Shared configuration module

### Create: `src/config.rs`
```rust
pub struct ServiceConfig {
    pub database_url: String,
    pub redis_url: Option<String>,
    pub log_level: String,
    pub bind_address: String,
}

impl ServiceConfig {
    pub async fn load_from_file(path: &str) -> Result<Self> { /* ... */ }
    pub fn from_env() -> Result<Self> { /* ... */ }
}
```

### Expected Reduction: ~1,200 lines of duplicate config code

## Phase 4: Web Server Setup (Week 4)

### Goal: Shared web server utilities
**Current State**: Each service duplicates Axum/HTTP setup  
**Target State**: Shared web utilities

### Create: `src/web.rs`
```rust
pub struct WebServer {
    app: Router,
    config: WebConfig,
}

impl WebServer {
    pub fn new() -> Self { /* ... */ }
    pub fn with_cors(mut self) -> Self { /* ... */ }
    pub fn with_logging(mut self) -> Self { /* ... */ }
    pub fn add_routes(mut self, routes: Router) -> Self { /* ... */ }
    pub async fn serve(self, addr: SocketAddr) -> Result<()> { /* ... */ }
}
```

### Expected Reduction: ~800 lines of duplicate web setup

## Phase 5: Performance Optimizations (Week 5)

### Database Query Optimization
1. **Fix N+1 Queries**: `src/queue.rs:279-302`
   ```rust
   // Before: N+1 pattern
   for item in items {
       let details = get_details(item.id).await?; // N queries
   }
   
   // After: Single query with JOIN
   let items_with_details = sqlx::query!(
       "SELECT i.*, d.* FROM items i JOIN details d ON i.id = d.item_id"
   ).fetch_all(&pool).await?;
   ```

2. **Add Query Pagination**
   ```rust
   pub async fn get_queue_items(
       &self,
       limit: Option<i64>,
       offset: Option<i64>
   ) -> Result<Vec<QueueItem>> {
       // Prevent unbounded result sets
   }
   ```

### Memory Optimization
1. **Streaming File Operations**: `worker/src/client.rs:306-314`
   ```rust
   // Before: Load entire file
   let mut buffer = Vec::new();
   file.read_to_end(&mut buffer).await?;
   
   // After: Stream processing
   let stream = tokio_util::io::ReaderStream::new(file);
   ```

## Phase 6: Architecture Simplification (Week 6)

### Remove Over-Engineering
1. **Simplify Trait Abstractions**
   - `src/artifacts/mod.rs`: Replace complex traits with simple functions
   - `worker/src/vcs.rs`: Reduce unnecessary generic complexity

2. **Consolidate Error Types**
   ```rust
   // Remove redundant wrapper types
   // Before: 
   pub struct ServiceError(String);
   
   // After: Use JanitorError directly
   ```

### Code Metrics Targets
- **Total Lines**: Reduce by 40% (from ~15,000 to ~9,000)
- **Duplication**: Reduce from 85% to <10% similarity
- **Cyclomatic Complexity**: Reduce average complexity by 30%
- **Unwrap Count**: Reduce from 500+ to <50

## Implementation Guidelines

### Migration Strategy
1. **Bottom-up approach**: Start with leaf modules, work toward main services
2. **Backward compatibility**: Maintain APIs during migration
3. **Incremental testing**: Test each module migration independently
4. **Feature flags**: Use conditional compilation for gradual rollout

### Quality Gates
1. **No functionality regression**: All tests must pass
2. **Performance maintained**: No performance degradation  
3. **Security preserved**: All security fixes maintained
4. **Documentation updated**: Update documentation as code changes

### Success Metrics

#### Code Quality
- [ ] Duplication reduced from 85% to <10%
- [ ] Average file size reduced by 50%
- [ ] Cyclomatic complexity reduced by 30%
- [ ] Test coverage increased to >80%

#### Developer Experience  
- [ ] Build time improved by 25%
- [ ] New service creation time reduced by 70%
- [ ] Bug fix propagation time reduced by 60%
- [ ] Onboarding time for new developers reduced by 50%

#### Maintainability
- [ ] Single source of truth for common functionality
- [ ] Consistent error handling across all services
- [ ] Unified configuration management
- [ ] Shared testing utilities

## Risk Mitigation

### Technical Risks
1. **Breaking changes**: Use deprecation warnings and gradual migration
2. **Performance regression**: Benchmark critical paths during refactoring
3. **Integration issues**: Maintain extensive integration test suite

### Timeline Risks  
1. **Scope creep**: Focus only on eliminating duplication, not adding features
2. **Resource constraints**: Prioritize highest-impact changes first
3. **Coordination issues**: Use feature branches for parallel development

## Post-Refactoring Benefits

### Short-term (Immediate)
- Faster development cycle
- Easier bug fixes (fix once, applies everywhere)
- Reduced test maintenance burden
- Smaller binary sizes

### Long-term (6+ months)
- Easier to add new services
- Consistent behavior across platform
- Reduced cognitive load for developers
- Foundation for future scaling

### Metrics Tracking
- **Development velocity**: Track story points per sprint
- **Bug resolution time**: Average time from report to fix
- **Code review efficiency**: Average review time
- **System reliability**: Uptime and error rates

This roadmap provides a structured approach to eliminating the technical debt while maintaining system stability and security.