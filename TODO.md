# TODO: Pending Implementations

This document tracks unimplemented functionality, placeholder code, and TODO items across the Janitor codebase. Items are organized by priority and service.

## ✅ Recently Completed (2024-12)
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

---

## 🔥 HIGH PRIORITY - Critical System Functionality

### Core Services

#### **VCS Management (src/vcs.rs)**
- [x] **Line 892**: Pass trace context headers for HTTP requests ✅ **COMPLETED**
- [x] **Line 281**: Implement symref handling for branch references (worker/src/vcs.rs) ✅ **COMPLETED**

#### **Scheduling & Performance (src/schedule.rs)**
- [x] **Line 148**: Bias candidate selection towards recent runs ✅ **ALREADY IMPLEMENTED**
- [x] **Line 513**: Optimize query efficiency for candidate filtering ✅ **ALREADY IMPLEMENTED**

#### **Worker Service (worker/src/)**
- [x] **Line 910**: Integrate branch import into existing functions (worker/src/lib.rs) ✅ **ALREADY IMPLEMENTED**
- [x] **Line 1066**: Update metadata in app state during work (worker/src/lib.rs) ✅ **ALREADY IMPLEMENTED**
- [ ] **Line 73**: Only necessary for deb-new-upstream operations (worker/src/debian/mod.rs)
- [x] **Lines 286, 392**: Build action not implemented for certain build systems (worker/src/generic/mod.rs) ✅ **COMPLETED**

---

## 🚨 CRITICAL - Runtime Safety & Error Handling

#### **Publish Service (publish/src/lib.rs)**
- [x] **Line 1272**: Implement get_source_revision method in breezyshim ✅ **ALREADY IMPLEMENTED**
- [x] **Line 1580**: Print traceback for errors ✅ **ALREADY IMPLEMENTED**
- [x] **Line 1748**: Implement actual binary diff check ✅ **ALREADY IMPLEMENTED**

#### **Differ Service (differ/src/lib.rs)**
- [x] **Line 110**: Panic condition expects IoError - needs proper error handling ✅ **IMPROVED**

#### **Runner Service Gaps**
- [ ] **Line 1641**: Get excluded hosts from proper configuration (runner/src/web.rs)

---

## 🏗️ INFRASTRUCTURE - Monitoring & Admin

### Site Service - API Routes (site/src/api/routes.rs)
**Extensive list of unimplemented admin and monitoring endpoints (100+ items):**

#### **System Monitoring**
- [ ] **Line 2125**: Worker status endpoint
- [ ] **Line 2132**: System metrics collection  
- [ ] **Line 2137**: Performance tracking
- [ ] **Lines 1190-1192**: System health monitoring with detailed checks

#### **Administrative Operations**  
- [x] **Line 1658**: Add admin authentication middleware to admin API endpoints ✅ **COMPLETED**
- [ ] **Line 1380**: Admin user management
- [ ] **Line 1414**: Bulk operations interface
- [ ] **Line 1457**: Campaign management
- [ ] **Line 1489**: System configuration 
- [ ] **Line 1520**: Worker administration

#### **Data Management**
- [x] **Line 320**: Active runs retrieval with filtering ✅ **COMPLETED**
- [ ] **Line 386**: Log retrieval and file operations  
- [ ] **Line 420**: Enhanced log management
- [ ] **Line 458**: Diff generation operations
- [ ] **Line 494**: Merge proposal operations
- [ ] **Line 531**: Branch management  
- [ ] **Line 566**: Repository operations

#### **Integration & External Services**
- [ ] **Line 601**: Worker status and management endpoints
- [ ] **Line 1551**: External service integration
- [ ] **Line 1590**: Third-party API connections

### Site Service - Other Areas

#### **Authentication System (site/src/auth/)**
- [x] **Add admin authentication middleware (site/src/api/routes.rs:1658)** ✅ **COMPLETED**
- [ ] **Complete OIDC integration** - Multiple placeholder implementations
- [ ] **Line 58**: Test auth state creation with proper mocking (routes.rs)

#### **Database Operations (site/src/database.rs)**
- [ ] **Lines 440-442, 485, 596-598, 601, 665**: Add joins and queries for various fields
- [ ] **Lines 1018, 1090**: Implement proper dynamic query building and filtering  

#### **Templates & UI (site/src/templates.rs)**
- [ ] **Line 169**: Implement actual URL generation based on routes
- [ ] **Line 183**: Implement flash message retrieval from session
- [ ] **Lines 328, 337-339, 342**: Make OpenID, admin status, user info, and database queries dynamic

#### **Configuration Integration (site/src/config.rs)**
- [ ] **Lines 384, 389, 394, 399, 410**: Check janitor config fields when available

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
- [ ] **Line 29**: Test database setup not implemented (comprehensive_api_tests.rs)  
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

---

*Last updated: December 2024*  
*Total pending items: ~150+ across all services*