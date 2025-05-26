# Publish Service Python to Rust Porting Plan

> **Status**: ✅ **COMPLETED** - This service has been fully ported to Rust with enhanced functionality.
> 
> 📋 **Master Plan**: See [`../porting-plan.md`](../porting-plan.md) for overall project coordination and dependencies.

## Overview

This document outlines the plan for porting the remaining functionality from `py/janitor/publish.py` to Rust in the `publish/` crate. The Python file is ~3700 lines and contains comprehensive publishing functionality that needs to be systematically migrated.

**FINAL STATUS**: All planned functionality has been successfully implemented with significant performance improvements.

## Current State Analysis

### Existing Rust Implementation

The `publish/` crate already has several core components implemented:

- **Core types and errors**: `PublishOneRequest`, `PublishOneResult`, `PublishError`, etc.
- **Rate limiting**: `RateLimiter` trait with `NonRateLimiter`, `FixedRateLimiter`, `SlowStartRateLimiter`
- **Single publish operation**: `publish_one()` function and supporting logic
- **State management**: Database operations, publish tracking
- **Web interface**: Partial implementation with some endpoints
- **Application state**: `AppState` structure with necessary dependencies

### Python Functionality to Port

Based on analysis of the Python code, the following major functionality needs to be ported:

1. **Main publish queue processing loop**
2. **Existing merge proposal scanning and management**
3. **Proposal info management system**
4. **Complete web API implementation**
5. **Redis integration and pub/sub**
6. **Background tasks and workers**
7. **Comprehensive error handling and metrics**

## Porting Plan by Priority

### Phase 1: Core Queue Processing (HIGH PRIORITY)

#### 1.1 Main Queue Processing Loop
- **Status**: ❌ Not implemented
- **Python**: `process_queue_loop()`, `publish_pending_ready()` 
- **Target**: Implement `process_queue_loop()` and `publish_pending_ready()` functions
- **Dependencies**: Database queries, rate limiting, worker management
- **Estimated effort**: 2-3 days

#### 1.2 Publish Decision Logic
- **Status**: ❌ Not implemented  
- **Python**: `consider_publish_run()`, `publish_from_policy()`
- **Target**: Complete implementation of `consider_publish_run()` function (currently returns `unimplemented!()`)
- **Dependencies**: Policy evaluation, branch checking, rate limiting
- **Estimated effort**: 3-4 days

#### 1.3 Redis Integration
- **Status**: ⚠️ Partial (Redis connection exists but no pub/sub)
- **Python**: `pubsub_publish()`, `listen_to_runner()`
- **Target**: Implement Redis pub/sub for merge proposal notifications and runner communication
- **Dependencies**: Redis client, message serialization
- **Estimated effort**: 1-2 days

### Phase 2: Merge Proposal Management (HIGH PRIORITY)

#### 2.1 Existing Proposal Scanning
- **Status**: ❌ Not implemented
- **Python**: `check_existing()`, `check_existing_mp()`, `iter_all_mps()`
- **Target**: Implement functions declared but not implemented: `check_existing_mp()`, `iter_all_mps()`
- **Dependencies**: Forge API integration, database updates
- **Estimated effort**: 3-4 days

#### 2.2 Proposal Info Management
- **Status**: ❌ Not implemented
- **Python**: `ProposalInfoManager` class with caching and metadata
- **Target**: Create `ProposalInfoManager` struct with async methods
- **Dependencies**: Database operations, caching logic
- **Estimated effort**: 2-3 days

#### 2.3 Merge Proposal Status Updates
- **Status**: ❌ Not implemented
- **Python**: `get_mp_status()`, `abandon_mp()`, `close_applied_mp()`
- **Target**: Implement proposal status management functions
- **Dependencies**: Forge API, database updates, notifications
- **Estimated effort**: 2-3 days

### Phase 3: Web API Completion (MEDIUM PRIORITY)

#### 3.1 Missing Web Endpoints
- **Status**: ❌ Most endpoints return `unimplemented!()`
- **Python**: 20+ endpoint handlers in the web server
- **Target**: Implement all missing web endpoints:
  - `/merge-proposals` (GET/POST)
  - `/absorbed` 
  - `/policy/*` endpoints
  - `/autopublish`
  - Additional utility endpoints
- **Dependencies**: Request/response serialization, database queries
- **Estimated effort**: 4-5 days

#### 3.2 Authentication and Authorization
- **Status**: ❌ Not implemented
- **Python**: OpenID integration, credential management
- **Target**: Implement authentication middleware and credential handling
- **Dependencies**: OpenID libraries, session management
- **Estimated effort**: 2-3 days

### Phase 4: Background Tasks and Monitoring (MEDIUM PRIORITY)

#### 4.1 Straggler Checking
- **Status**: ❌ Not implemented
- **Python**: `check_stragglers()`, `check_straggler()`
- **Target**: Implement background task to check outdated merge proposals
- **Dependencies**: HTTP client, proposal info management
- **Estimated effort**: 1-2 days

#### 4.2 Metrics and Monitoring
- **Status**: ⚠️ Partial (some metrics defined but not used)
- **Python**: Comprehensive Prometheus metrics throughout
- **Target**: Implement all missing metrics collection points
- **Dependencies**: Prometheus integration
- **Estimated effort**: 2-3 days

#### 4.3 Bucket Rate Limit Refresh
- **Status**: ❌ Not implemented
- **Python**: `refresh_bucket_mp_counts()`
- **Target**: Implement `refresh_bucket_mp_counts()` function (currently returns `unimplemented!()`)
- **Dependencies**: Database queries, rate limiter updates
- **Estimated effort**: 1 day

### Phase 5: Advanced Features (LOW PRIORITY)

#### 5.1 Error Recovery and Resilience
- **Status**: ❌ Limited implementation
- **Python**: Comprehensive error handling, exponential backoff, transient error detection
- **Target**: Enhance error handling throughout the system
- **Dependencies**: Error classification, retry logic
- **Estimated effort**: 2-3 days

#### 5.2 Policy Management
- **Status**: ❌ Not implemented
- **Python**: Dynamic policy loading, policy-based publishing decisions
- **Target**: Implement policy management system
- **Dependencies**: Configuration management, database integration
- **Estimated effort**: 3-4 days

#### 5.3 Advanced Queue Management
- **Status**: ❌ Not implemented
- **Python**: Sophisticated queue prioritization, batch processing
- **Target**: Implement advanced queue management features
- **Dependencies**: Database optimization, scheduling algorithms
- **Estimated effort**: 2-3 days

## Implementation Strategy

### 1. Module Organization

Maintain the current module structure but add new modules as needed:

```rust
// Current modules
pub mod proposal_info;     // NEW - port ProposalInfoManager
pub mod publish_one;       // EXISTS - already ported
pub mod rate_limiter;      // EXISTS - already ported
pub mod state;             // EXISTS - needs expansion
pub mod web;               // EXISTS - needs completion

// New modules to add
pub mod queue;             // Queue processing logic
pub mod scanner;           // Merge proposal scanning
pub mod metrics;           // Prometheus metrics
pub mod background;        // Background tasks
```

### 2. Database Layer

Expand the existing database operations in `state.rs`:

- Add missing query functions for merge proposal management
- Implement proposal info storage and retrieval
- Add policy management queries
- Enhance transaction handling

### 3. Error Handling

Expand the current `PublishError` enum to cover all error cases from Python:

```rust
pub enum PublishError {
    // Existing variants...
    
    // New variants needed
    RateLimited { retry_after: Option<Duration> },
    ForgeError(BrzError),
    DatabaseError(sqlx::Error),
    InvalidConfiguration(String),
    NetworkError(reqwest::Error),
}
```

### 4. Testing Strategy

- Port existing Python tests to Rust
- Add integration tests for new functionality
- Create mock implementations for external dependencies
- Ensure API compatibility with existing clients

### 5. Migration Path

1. **Incremental deployment**: Implement features behind feature flags
2. **API compatibility**: Maintain HTTP API compatibility during transition
3. **Data migration**: Ensure database schema compatibility
4. **Monitoring**: Add extensive logging and metrics during transition

## Dependencies and Infrastructure

### New Rust Dependencies

The following crates may need to be added to `Cargo.toml`:

```toml
# Already included
redis = { workspace = true, features = ["tokio-comp", "json", "connection-manager"] }
reqwest = { workspace = true }
prometheus = "0.14.0"

# May need to add
tokio-cron-scheduler = "0.9"  # For background tasks
openid = "0.14"               # For authentication  
tracing-subscriber = "0.3"    # For enhanced logging
```

### Configuration

Update the configuration system to support:
- Background task scheduling
- Rate limiting configuration
- Authentication settings
- Monitoring endpoints

## Risk Assessment

### High Risk
- **Queue processing logic**: Complex state management and error handling
- **Database transaction handling**: Need to maintain data consistency
- **Forge API integration**: External dependencies and rate limiting

### Medium Risk  
- **Web API compatibility**: Maintaining exact API contract
- **Performance implications**: Ensuring Rust implementation performs as well as Python
- **Configuration migration**: Ensuring all Python configuration options are supported

### Low Risk
- **Metrics collection**: Well-defined interfaces
- **Background tasks**: Independent of core functionality
- **Static content serving**: Straightforward HTTP handling

## Success Criteria

1. **Functional parity**: All Python functionality replicated in Rust
2. **Performance improvement**: At least equivalent performance, ideally better
3. **API compatibility**: Existing clients work without changes
4. **Test coverage**: Comprehensive test suite with >90% coverage
5. **Documentation**: Complete API documentation and deployment guides
6. **Monitoring**: Full observability with metrics and logging

## Timeline Estimate

- **Phase 1 (Core Queue Processing)**: 6-9 days
- **Phase 2 (Merge Proposal Management)**: 7-10 days  
- **Phase 3 (Web API Completion)**: 6-8 days
- **Phase 4 (Background Tasks)**: 4-6 days
- **Phase 5 (Advanced Features)**: 7-10 days

**Total estimated effort**: 30-43 days (6-8.5 weeks)

This can be parallelized across multiple developers or done incrementally alongside other development work.