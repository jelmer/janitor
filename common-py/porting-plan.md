# Common Python Libraries Porting Plan

> **Status**: 📋 **FUTURE PLANNING** - This plan outlines shared infrastructure components migration.
> 
> 📋 **Master Plan**: See [`../porting-plan.md`](../porting-plan.md) for overall project coordination and dependencies.

## Overview

This document outlines the plan for porting shared Python infrastructure components from `py/janitor/` common modules to Rust. These components provide core functionality used across multiple services and must be prioritized for the infrastructure services phase.

## Scope

### Target Modules (~800 lines total)
- `py/janitor/artifacts.py` (47 lines) - Artifact management
- `py/janitor/config.py` (47 lines) - Configuration utilities  
- `py/janitor/logs.py` (455 lines) - Log management (HIGH PRIORITY)
- `py/janitor/queue.py` (288 lines) - Queue operations (HIGH PRIORITY)
- `py/janitor/schedule.py` (635 lines) - Scheduling logic (HIGH PRIORITY)
- `py/janitor/state.py` (268 lines) - State management (HIGH PRIORITY)
- `py/janitor/vcs.py` (133 lines) - VCS abstraction
- `py/janitor/review.py` (67 lines) - Review system

## Migration Strategy

### Phase 1: Core Infrastructure (COMPLETED)
**Status**: ✅ **All core infrastructure services already ported**

The following have been integrated into `src/` modules:
- ✅ **Log Management**: Enhanced `src/logs/` module with full Python parity
- ✅ **Queue Operations**: Enhanced `src/queue.rs` with complete API compatibility  
- ✅ **Scheduling Logic**: Enhanced `src/schedule.rs` and `src/bin/janitor-schedule.rs`
- ✅ **State Management**: Enhanced `src/state.rs` with all Python API functionality

### Phase 2: VCS and Configuration (MEDIUM PRIORITY)
**Estimated effort**: 1-2 weeks

#### 2.1 VCS Abstraction (`vcs.py` - 133 lines)
- **Target**: Port to `src/vcs.rs`
- **Scope**: VCS manager traits, branch operations, diff generation
- **Dependencies**: Git2, breezyshim bindings
- **Implementation Details**:
  - Abstract `VcsManager` trait
  - Git and Bazaar concrete implementations
  - Branch creation and management
  - Diff generation and comparison
  - Error handling for VCS operations

#### 2.2 Configuration Utilities (`config.py` - 47 lines)
- **Target**: Enhance existing `src/config.rs`
- **Scope**: Configuration validation, environment overrides
- **Dependencies**: Existing config system
- **Implementation Details**:
  - Enhanced validation functions
  - Configuration merging utilities
  - Environment variable processing
  - Profile-based configuration loading

### Phase 3: Artifact and Review Systems (LOW PRIORITY)
**Estimated effort**: 1 week

#### 3.1 Artifact Management (`artifacts.py` - 47 lines)
- **Target**: Enhance `src/artifacts/` module
- **Scope**: Artifact storage, retrieval, metadata
- **Dependencies**: Storage backends (GCS, local filesystem)
- **Implementation Details**:
  - Artifact upload/download utilities
  - Metadata extraction and storage
  - Storage backend abstraction
  - Cleanup and lifecycle management

#### 3.2 Review System (`review.py` - 67 lines)
- **Target**: Create `src/review.rs`
- **Scope**: Review submission, verdict processing
- **Dependencies**: Database operations, scheduling
- **Implementation Details**:
  - Review data structures
  - Verdict calculation logic
  - Database integration
  - Notification system

## Dependencies

### Internal Dependencies
- **Database**: PostgreSQL integration via sqlx
- **Storage**: GCS and filesystem backends
- **VCS**: Git2 and breezyshim integration
- **Configuration**: Enhanced config system

### External Dependencies
```toml
# VCS Integration
git2 = "0.18"
breezyshim = ">=0.1.173"

# Storage & Artifacts
google-cloud-storage = "0.22"
tokio-stream = "0.1"

# Utilities
regex = "1.0"
walkdir = "2.0"
```

## Testing Strategy

### Unit Tests
- VCS operations with test repositories
- Configuration parsing and validation
- Artifact storage and retrieval
- Review logic and verdict calculation

### Integration Tests
- Cross-service communication
- Database integration testing
- Storage backend compatibility
- End-to-end review workflows

## Success Criteria

### Functional Requirements
- 100% API compatibility with Python implementations
- All VCS operations work correctly with Git and Bazaar
- Configuration system supports all existing patterns
- Artifact management preserves all metadata
- Review system maintains exact Python behavior

### Performance Requirements
- VCS operations 2-3x faster than Python
- Configuration loading under 50ms
- Artifact operations with minimal memory usage
- Review processing with sub-second response times

## Risk Assessment

### Low Risk
- **Configuration utilities** - Simple, well-defined interfaces
- **Review system** - Straightforward business logic

### Medium Risk  
- **VCS abstraction** - Multiple VCS systems to support
- **Artifact management** - Storage backend integration complexity

## Timeline

| Component | Effort | Dependencies | Priority |
|-----------|--------|--------------|----------|
| VCS Abstraction | 1 week | Git2, breezyshim | Medium |
| Configuration | 2 days | Existing config | Medium |
| Artifact Management | 3 days | Storage backends | Low |
| Review System | 2 days | Database | Low |

**Total Estimated Duration**: 2-3 weeks

## Related Plans

### Dependencies
- [`../porting-plan.md`](../porting-plan.md) - Master coordination plan
- [`../site/porting-plan.md`](../site/porting-plan.md) - Web interface dependencies
- [`../runner/porting-plan.md`](../runner/porting-plan.md) - Core service integration

### Future Integration
- VCS abstraction will be used by git-store and bzr-store services
- Review system will integrate with site and cupboard interfaces
- Artifact management will be used by runner and worker services