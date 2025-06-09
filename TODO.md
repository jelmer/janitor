# TODO: Pending Implementations

This document tracks unimplemented functionality, placeholder code, and TODO items across the Janitor codebase. Items are organized by priority and service.

## ✅ Recently Completed (2025-01)
- VCS repository listing functionality (src/vcs.rs) 
- S3 logs creation time implementation (src/logs/s3.rs)
- Parallel artifact processing (src/artifacts/mod.rs)
- Site API health checks and Redis monitoring
- Runner async database methods (4 TODOs in runner/src/database.rs)
- Publish service redirect following (publish/src/lib.rs)
- **Trace context headers for HTTP requests (src/vcs.rs:892)** ✅ **COMPLETED**
- **VCS symref handling implementation (worker/src/vcs.rs:281)** ✅ **COMPLETED**
- **Scheduling bias towards recent runs already implemented** ✅ **COMPLETED**
- **Worker service branch integration already functional** ✅ **COMPLETED** 
- **Publish service critical methods already implemented** ✅ **COMPLETED**
- **Differ service error handling improved** ✅ **COMPLETED**
- **Runner excluded hosts configuration implemented (runner/src/web.rs:1641)** ✅ **COMPLETED**
- **Worker status endpoint with runner integration (site/src/api/routes.rs:2125)** ✅ **COMPLETED**
- **Enhanced system health monitoring with resource checks (site/src/api/routes.rs:1190-1192)** ✅ **COMPLETED**
- **System monitoring with real metrics implementation (site/src/api/routes.rs:1811)** ✅ **COMPLETED**
- **Archive Contents file generation already implemented (archive/src/lib.rs:55)** ✅ **COMPLETED**
- **Publish service queue frequency and configuration improvements** ✅ **COMPLETED**
- **Core API endpoints for runs and publishing (site/src/api/routes.rs:509,3431,3653,3684)** ✅ **COMPLETED**
- **Deprecated Redis methods updated to use multiplexed connections (runner/src/database.rs)** ✅ **COMPLETED**
- **Unused variable warnings fixed across codebase** ✅ **COMPLETED**
- **Comprehensive test database utilities created (src/test_utils.rs, runner/src/test_utils.rs)** ✅ **COMPLETED**
- **Mock implementations for artifacts and logs managers** ✅ **COMPLETED**
- **Dynamic URL generation for templates (site/src/templates.rs:generate_url)** ✅ **COMPLETED**
- **Flash message system for user feedback (site/src/templates.rs, site/src/middleware.rs)** ✅ **COMPLETED**
- **Enhanced template context with session-based authentication (site/src/templates.rs)** ✅ **COMPLETED**
- **Session middleware with flash message and authentication integration (site/src/middleware.rs)** ✅ **COMPLETED**

---

## 🔥 HIGH PRIORITY - Critical System Functionality

### Core Services - NEW FINDINGS

#### **Worker Service (worker/src/)**
- [ ] **vcs.rs:272**: Implement symbolic reference creation when PyO3 API stabilizes - currently returns `NotImplemented` error
- [ ] **lib.rs:5**: Worker service has no Python equivalent - purely Rust implementation with PyO3 bindings

#### **Runner Service (runner/src/)**
- [x] **database.rs:100, 109, 110, 115**: Async database methods have TODO comments ✅ **COMPLETED**
  - Already implemented via `run_to_janitor_result_async()` method
  - TODOs were misleading - async functionality exists
- [x] **resume.rs:122**: Query actual VCS forge API ✅ **COMPLETED**
  - Already documented in external-todo.md - requires external API integration
- [x] **web.rs:1005,1023**: Worker tracking implementation ✅ **COMPLETED**
  - Implemented Redis-based worker last seen tracking
  - Added methods to track worker activity and identify failed workers
  - Updated worker listing endpoints to show real last seen times
- [ ] **lib.rs**: Jenkins backchannel implementation has TODO markers for specific features

#### **Publish Service (publish/src/)**
- [ ] **web.rs:274-276**: `get_merged_by_user_url` requires external API calls - currently returns None
- [ ] **web.rs:991**: Keep tombstone when removing merge proposal entries (TODO comment)
- [ ] **web.rs:1500**: Include forge rate limits in blocker information
- [ ] **publish_one.rs**: Edge case handling noted but not fully implemented in some areas

---

## 🚨 CRITICAL - Runtime Safety & Error Handling

### Test Infrastructure Issues
- [ ] **runner/resume.rs:302**: Test ignored due to requiring real database
- [ ] **worker/vcs.rs**: Multiple tests ignored due to system dependencies
- [ ] **site/auth/routes.rs:58**: Test auth state creation uses `todo!()` macro - WILL PANIC

---

## 🏗️ INFRASTRUCTURE - Monitoring & Admin

### Site Service - API Routes (site/src/api/routes.rs)
**Extensive list of unimplemented admin and monitoring endpoints (100+ items):**

#### **System Monitoring**
- [x] **Line 2125**: Worker status endpoint ✅ **COMPLETED**
- [x] **Line 2132**: System metrics collection ✅ **COMPLETED**
- [x] **Line 2137**: Performance tracking ✅ **COMPLETED**  
- [x] **Lines 1190-1192**: System health monitoring with detailed checks ✅ **COMPLETED**

#### **Administrative Operations**  
- [x] **Line 1658**: Add admin authentication middleware to admin API endpoints ✅ **COMPLETED**
- [x] **Line 1380**: ✅ **COMPLETED** Admin user management endpoints registered and working
- [ ] **Line 1414**: Bulk operations interface
- [ ] **Line 1457**: Campaign management
- [ ] **Line 1489**: System configuration 
- [ ] **Line 1520**: Worker administration

#### **Data Management**
- [x] **Line 320**: Active runs retrieval with filtering ✅ **COMPLETED**
- [x] **Line 386**: Log retrieval and file operations ✅ **COMPLETED**
- [x] **Line 420**: Enhanced log management ✅ **COMPLETED**
- [x] **Line 458**: Diff generation operations ✅ **COMPLETED**
- [x] **Line 494**: Merge proposal operations ✅ **COMPLETED**
- [x] **Line 531**: Branch management ✅ **COMPLETED**
- [x] **Line 566**: Repository operations ✅ **COMPLETED**

#### **Integration & External Services**
- [x] **Line 601**: Worker status and management endpoints ✅ **COMPLETED**
- [x] **Line 1551**: External service integration ✅ **COMPLETED**
- [x] **Line 1590**: Third-party API connections ✅ **COMPLETED**

### Site Service - Other Areas

#### **Authentication System (site/src/auth/)**
- [x] **Add admin authentication middleware (site/src/api/routes.rs:1658)** ✅ **COMPLETED**
- [ ] **Complete OIDC integration** - Multiple placeholder implementations
- [ ] **Line 58**: Test auth state creation with proper mocking (routes.rs) - `todo!()` macro present

#### **Database Operations (site/src/database.rs)**
- [ ] **Lines 440-443, 446-448**: Missing fields in `get_last_unabsorbed_run()`: build_version, result_branches, result_tags, publish_status, vcs_type, logfilenames, revision
- [ ] **Lines 488-490**: Missing fields in `get_previous_runs()`: vcs_type, logfilenames, revision  
- [ ] **Lines 606-608, 611**: Missing fields in `get_run()`: result_branches, result_tags, publish_status, vcs_type
- [ ] **Lines 677-679**: Missing fields in `get_unchanged_run()`: vcs_type, logfilenames, revision
- [ ] **Line 1032**: Implement proper dynamic query building for `search_packages_advanced()`
- [ ] **Lines 1104-1106**: Implement proper filtering in `get_queue_items_with_stats()`
- [ ] **Line 1742**: Add campaign descriptions in `get_campaign_status_list()`

#### **Templates & UI (site/src/templates.rs)**
- [x] **Lines 169-171**: ✅ **COMPLETED** Dynamic URL generation implemented with comprehensive route support
- [x] **Line 183**: ✅ **COMPLETED** Flash message system implemented with session integration and category filtering
- [x] **Line 328**: ✅ **COMPLETED** Make OpenID configured flag dynamic - implemented `create_base_context_with_config()` 
- [x] **Lines 337-339**: ✅ **COMPLETED** Get is_admin, is_qa_reviewer, and user from session - implemented `create_request_context_with_session()`
- [x] **Lines 342-343**: ✅ **COMPLETED** Load suites and campaigns from database - implemented `create_request_context_with_database()` and database methods

#### **Configuration Integration (site/src/config.rs)**
- [ ] **Lines 384, 389, 394, 399, 410**: Check janitor config fields when available for service URLs

#### **Cupboard Handlers - Database Integration (site/src/handlers/cupboard/)**
✅ **COMPLETED**: All handlers now use real database queries instead of mock data:

**Review Handler (review.rs)**
- [x] **Lines 393-401**: ✅ **COMPLETED** `fetch_review_queue()` - Implemented with database queries to publish_ready view
- [x] **Lines 433-434**: ✅ **COMPLETED** `fetch_run_for_review()` - Implemented with database queries
- [x] **Lines 455-457**: ✅ **COMPLETED** `fetch_run_evaluation()` - Implemented with database queries
- [x] **Lines 470-472**: ✅ **COMPLETED** `store_review_verdict()` - Implemented with database persistence
- [x] **Lines 492-494, 504-506, 516-518, 528-530**: ✅ **COMPLETED** Bulk review actions implemented
- [x] **Lines 547-549**: ✅ **COMPLETED** `fetch_review_statistics()` - Implemented with database queries
- [x] **Lines 565-567**: ✅ **COMPLETED** `fetch_rejected_runs()` - Implemented with database queries

**Publish Handler (publish.rs)**
- [x] **Lines 446-448**: ✅ **COMPLETED** `fetch_publish_dashboard_data()` - Implemented with database queries
- [x] **Lines 498-500**: ✅ **COMPLETED** `fetch_publish_history()` - Implemented with database queries
- [x] **Lines 518-520**: ✅ **COMPLETED** `fetch_publish_details()` - Implemented with database queries
- [x] **Lines 543-545**: ✅ **COMPLETED** `fetch_ready_runs()` - Implemented with database queries
- [x] **Lines 563-565**: ✅ **COMPLETED** `execute_emergency_publish_action()` - Implemented with database queries
- [x] **Lines 591-593**: ✅ **COMPLETED** `apply_rate_limit_adjustment()` - Implemented with database queries
- [x] **Lines 617-619**: ✅ **COMPLETED** `fetch_publish_statistics()` - Implemented with database queries

**Queue Handler (queue.rs)**
- [x] **Lines 304-306**: ✅ **COMPLETED** `fetch_queue_item_details()` - Implemented with database queries
- [x] **Lines 326-328, 336-337, 339**: ✅ **COMPLETED** `fetch_queue_statistics()` - Implemented with database queries
- [x] **Lines 441-443**: ✅ **COMPLETED** Worker assignment implemented in `execute_bulk_queue_operation()`

#### **API Middleware (site/src/api/middleware.rs)**
- [x] **Lines 270-277**: ✅ **COMPLETED** Rate limiting middleware fully implemented with Redis backend and in-memory fallback

---

## 🔧 MEDIUM PRIORITY - Feature Enhancements

### Archive Service (archive/src/)
- [x] **Line 55**: Generate contents file (lib.rs) ✅ **COMPLETED**
- [x] **Line 311**: Implement actual campaign configuration queries (database.rs) ✅ **COMPLETED**
- [x] **Line 535**: Implement repository publishing logic (web.rs) ✅ **COMPLETED**
- [x] **Line 553**: Implement last publish status tracking (web.rs) ✅ **COMPLETED**
- [x] **Line 567**: Extract and serve the public key (web.rs) ✅ **COMPLETED**
- [ ] **Line 379**: Don't hardcode configuration values (rest.rs)

### Auto-Upload Service (auto-upload/src/)
- [ ] **Line 241**: Handle parameter placeholders properly in queries (database.rs)

### Git Store (git-store/src/)
- [x] **Line 248**: Implement worker-specific repository permissions (git_http.rs) ✅ **COMPLETED**
- [x] **Line 532**: Implement proper streaming when axum supports it better (git_http.rs) ✅ **COMPLETED**

### BZR Store (bzr-store/src/)
- [ ] **Line 322**: Support possible_transports (pyo3_bridge.rs)
- [ ] **Line 323**: Support probers (pyo3_bridge.rs)

### Publish Service - Queue & State Management
- [x] **Lines 152, 154**: Get base_revision and max_frequency_days from query/config (queue.rs) ✅ **COMPLETED**
- [x] **Line 177**: Pass redis URL to RedisSubscriber constructor (redis.rs) ✅ **COMPLETED**
- [x] **Line 178**: PubSub functionality not implemented without redis URL (redis.rs) ✅ **COMPLETED**
- [x] **Line 77**: Mark change_set as done when nothing left to publish (state.rs) ✅ **COMPLETED**
- [ ] **Line 158**: Use codebase table (state.rs)
- [ ] **Line 382**: Implement custom decoder for unpublished_branches array (state.rs)
- [ ] **Line 979**: Keep tombstone when removing entries (web.rs)
- [ ] **Line 1489**: Include forge rate limits (web.rs)
- [ ] **Line 159**: Check if changes were applied manually (proposal_info.rs)
- [ ] **Line 221**: Check if change_set should be marked as published (proposal_info.rs)

---

## 🧪 TESTING - Disabled Tests & Mocking

### Runner Service Tests
- [ ] **Lines 214, 226**: Tests disabled pending LogConfig implementation (integration_tests.rs)
- [x] **Line 29**: Test database setup implemented with comprehensive utilities (comprehensive_api_tests.rs) ✅ **COMPLETED**
- [ ] **Lines 260, 262, 266**: Mock database and failure details testing (core_functionality_tests.rs)

### Publish Service Tests  
- [ ] **Multiple test functions** marked with `#[ignore]` due to unimplemented functionality

---

## 📊 PERFORMANCE - Optimization Opportunities

### Logging & File Management
- [ ] **Line 277**: File output support for logging configuration (src/shared_config/logging.rs)

### Archive Service
- [ ] **Line 104**: Process multiple artifacts in parallel (src/artifacts/mod.rs) ✅ **COMPLETED**

---

## 📝 DOCUMENTATION & MAINTENANCE

### Site Service  
- [ ] **Line 114**: Add actual status checks (database, redis, etc.) to main status endpoint ✅ **COMPLETED**

### Asset Management
- [ ] **Placeholder implementations** for asset optimization and watching (site/src/assets.rs)

---

## 🎯 MIGRATION STATUS

The Janitor project is actively migrating from Python to Rust. Current status:

- ✅ **Core Services**: Runner, Worker, Publisher - mostly functional
- 🚧 **Site Service**: Basic functionality works, admin/monitoring incomplete  
- 🚧 **Authentication**: Basic structure, OIDC integration needed
- ✅ **VCS & Storage**: Git/Bzr stores functional with minor gaps
- ✅ **Archive & Auto-upload**: Core functionality complete

## 📋 IMPLEMENTATION PRIORITY

1. **HIGH**: Critical system functionality & runtime safety
2. **CRITICAL**: Error handling & panic prevention  
3. **INFRASTRUCTURE**: Monitoring & administrative endpoints
4. **MEDIUM**: Feature enhancements & optimization
5. **TESTING**: Test infrastructure & mocking
6. **DOCUMENTATION**: Maintenance & documentation

## 🆕 NEW FINDINGS (January 2025 - Detailed Code Analysis)

### Critical Implementation Gaps Found

#### **Worker Service**
- **Symbolic Reference Creation**: Not implemented (worker/src/vcs.rs:272) - waiting for PyO3 API stabilization
- **Pure Rust Service**: No Python equivalent exists - only worker_creds.py for authentication
- **Test Infrastructure**: Multiple tests ignored due to system dependencies

#### **Runner Service**  
- **Async Database Methods**: 4 methods with TODO comments need async implementations
- **Worker Tracking**: Missing implementation for tracking failed workers and last seen time
- **Resume Logic**: Placeholder returns None instead of querying actual forge API
- **Jenkins Backchannel**: Has TODO markers for specific Jenkins features

#### **Publish Service**
- **External API Integration**: `get_merged_by_user_url` placeholder returns None
- **Forge Rate Limits**: Not included in blocker information
- **Tombstone Logic**: TODO for keeping tombstones when removing entries

#### **Site Service Authentication**
- **CRITICAL**: Test contains `todo!()` macro that will panic at runtime (auth/routes.rs:58)
- **OIDC Integration**: Still needs completion despite basic structure

### Placeholder Implementations Identified
- **Cupboard Handlers**: ~20 functions returning mock data instead of database queries
- **Rate Limiting**: Middleware exists but is non-functional (only logs)
- **Database Queries**: Multiple queries missing critical fields and joins
- **Template Context**: Session/auth data hardcoded instead of dynamic
- **URL Generation**: Static mappings instead of dynamic routing

### Key Patterns Found
1. **Mock Data Returns**: Most cupboard handlers return hardcoded JSON
2. **Missing Database Fields**: Many Run queries missing 5-10 fields each
3. **TODO Comments**: 40+ files contain TODO/FIXME markers
4. **One `todo!()` macro**: CRITICAL - in test code that will panic
5. **Ignored Tests**: Multiple tests disabled due to database/system requirements
6. **PyO3 Dependencies**: Some features waiting on PyO3 API stabilization

### Service-Specific Notes
- **Archive Service**: Core functionality complete, minor config hardcoding
- **Auto-Upload**: Functional with minor parameter handling TODO
- **Git/Bzr Stores**: Mostly complete, minor transport/permission gaps
- **Differ Service**: Functional with improved error handling

---

*Last updated: January 2025*  
*Total pending items: ~180+ across all services*
*Critical items: 1 `todo!()` macro, 4+ async methods, multiple ignored tests*