# Cupboard Admin Interface Porting Plan

> **Status**: 🏗️ **IN PROGRESS** - Phase 4.1 (Admin Infrastructure) completed, continuing with Queue Management.
> 
> **Progress**: 2/6 phases complete (~770 lines ported of 1,629 total)
> 
> 📋 **Master Plan**: See [`../porting-plan.md`](../porting-plan.md) for overall project coordination and dependencies.

## Overview

This document outlines the plan for porting the Cupboard admin interface from `py/janitor/site/cupboard/` (1,629 lines) to Rust as part of the site service. The Cupboard interface provides administrative controls for queue management, review systems, and publishing oversight.

## Scope Analysis

### Target Modules (1,629 lines total)
- `__init__.py` (771 lines) - Core admin interface and navigation
- `api.py` (411 lines) - Admin API endpoints and operations  
- `merge_proposals.py` (123 lines) - Merge proposal management
- `publish.py` (133 lines) - Publishing controls and oversight
- `queue.py` (98 lines) - Queue management interface
- `review.py` (93 lines) - Review system administration

### Complexity Assessment
- **High Complexity**: Admin authentication, queue manipulation, bulk operations
- **Medium Complexity**: Review workflows, merge proposal management
- **Low Complexity**: Status displays, configuration interfaces

## Migration Strategy

### Phase 1: Admin Infrastructure ✅ **COMPLETED**
**Estimated effort**: 2 weeks | **Actual effort**: 1 day

#### 1.1 Core Admin Framework (`__init__.py` - 771 lines) ✅ **COMPLETED**
**Target**: `src/handlers/cupboard/mod.rs`

##### Implementation Details
- **Admin Authentication**: Role-based access control (admin, qa_reviewer)
- **Navigation System**: Admin interface routing and menu structure
- **Common Utilities**: Shared admin functionality and helpers
- **Template Integration**: Admin-specific template rendering
- **Permission System**: Fine-grained permission checking

##### Key Components
```rust
// Admin-specific middleware and utilities
pub struct AdminContext {
    pub user: User,
    pub is_admin: bool,
    pub is_qa_reviewer: bool,
    pub permissions: Vec<Permission>,
}

// Admin route handlers
pub async fn admin_dashboard(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
) -> Response {
    // Admin dashboard implementation
}
```

#### 1.2 Admin Template System ✅ **COMPLETED**
**Target**: `templates/cupboard/` directory

##### Template Structure
- **Layout**: Admin-specific layout with navigation
- **Dashboard**: Overview of system status and metrics
- **Navigation**: Sidebar and breadcrumb navigation
- **Common Components**: Shared admin UI components
- **Responsive Design**: Mobile-friendly admin interface

### Phase 2: Admin API Layer ✅ **COMPLETED**
**Estimated effort**: 2 weeks | **Actual effort**: 1 day

#### 2.1 Admin API Endpoints (`api.py` - 411 lines) ✅ **COMPLETED**
**Target**: `src/handlers/cupboard/api.rs`

##### Core API Groups
- **System Management**: Service status, configuration, health checks
- **User Management**: User roles, permissions, session management
- **Data Operations**: Bulk operations, imports, exports
- **Monitoring**: Real-time metrics, log access, debugging tools

##### Implementation Structure
```rust
// Admin API route definitions
pub fn admin_api_routes() -> Router<AppState> {
    Router::new()
        .route("/api/admin/status", get(admin_system_status))
        .route("/api/admin/users", get(admin_list_users).post(admin_create_user))
        .route("/api/admin/users/:id", get(admin_get_user).put(admin_update_user))
        .route("/api/admin/operations", post(admin_bulk_operation))
        .route("/api/admin/metrics", get(admin_metrics))
        .layer(require_admin_middleware())
}
```

##### Security Features
- **CSRF Protection**: Token-based CSRF prevention
- **Rate Limiting**: Admin-specific rate limiting
- **Audit Logging**: Comprehensive admin action logging
- **Input Validation**: Strict validation for admin operations
- **Permission Checking**: Method-level permission verification

### Phase 3: Queue Management (HIGH PRIORITY)
**Estimated effort**: 1 week

#### 3.1 Queue Administration (`queue.py` - 98 lines)
**Target**: `src/handlers/cupboard/queue.rs`

##### Features to Implement
- **Queue Visualization**: Real-time queue status and statistics
- **Priority Management**: Manual priority adjustments for candidates
- **Bulk Operations**: Bulk reschedule, cancel, or requeue operations
- **Queue Filtering**: Advanced filtering and search capabilities
- **Worker Management**: Worker assignment and load balancing

##### Implementation Components
```rust
// Queue management interface
pub async fn queue_dashboard(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    Query(params): Query<QueueFilters>,
) -> Response {
    // Queue dashboard with filtering and pagination
}

pub async fn bulk_queue_operation(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    Json(operation): Json<BulkQueueOperation>,
) -> Response {
    // Bulk queue manipulation (reschedule, cancel, etc.)
}
```

##### Queue Operations
- **Reschedule**: Bulk rescheduling of failed runs
- **Priority Boost**: Temporary priority increases
- **Worker Assignment**: Manual worker assignment
- **Queue Cleanup**: Remove stuck or obsolete items
- **Statistics**: Detailed queue metrics and analytics

### Phase 4: Review System Administration (MEDIUM PRIORITY)
**Estimated effort**: 1 week

#### 4.1 Review Management (`review.py` - 93 lines)
**Target**: `src/handlers/cupboard/review.rs`

##### Review Administration Features
- **Review Queue**: List of pending reviews with filtering
- **Bulk Review Actions**: Approve, reject, or request changes in bulk
- **Review Statistics**: Analytics on review patterns and performance
- **Reviewer Management**: Assign and manage reviewer roles
- **Review Templates**: Standardized review response templates

##### Implementation Structure
```rust
// Review administration
pub async fn review_queue(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    Query(filters): Query<ReviewFilters>,
) -> Response {
    // Admin view of pending reviews
}

pub async fn bulk_review_action(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    Json(action): Json<BulkReviewAction>,
) -> Response {
    // Bulk approve/reject/modify reviews
}
```

### Phase 5: Publishing Controls (MEDIUM PRIORITY)
**Estimated effort**: 1 week

#### 5.1 Publish Administration (`publish.py` - 133 lines)
**Target**: `src/handlers/cupboard/publish.rs`

##### Publishing Management Features
- **Publish Queue**: Overview of pending publishing operations
- **Rate Limit Controls**: Adjust per-repository rate limits
- **Batch Publishing**: Coordinate large-scale publishing operations
- **Publishing Statistics**: Analytics on publishing success rates
- **Emergency Controls**: Emergency stop and rollback capabilities

##### Implementation Components
```rust
// Publishing administration
pub async fn publish_dashboard(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
) -> Response {
    // Publishing status and controls
}

pub async fn emergency_publish_stop(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    Json(params): Json<EmergencyStopParams>,
) -> Response {
    // Emergency publishing controls
}
```

### Phase 6: Merge Proposal Management (MEDIUM PRIORITY)
**Estimated effort**: 1 week

#### 6.1 MP Administration (`merge_proposals.py` - 123 lines)
**Target**: `src/handlers/cupboard/merge_proposals.rs`

##### MP Management Features
- **MP Overview**: System-wide merge proposal status
- **Bulk MP Operations**: Close, reopen, or update multiple MPs
- **MP Analytics**: Success rates, merge times, failure analysis
- **Integration Health**: Monitor forge API health and issues
- **Manual Intervention**: Tools for manual MP management

## Technical Implementation

### Authentication and Authorization

#### Role-Based Access Control
```rust
#[derive(Debug, Clone)]
pub enum AdminRole {
    Admin,        // Full administrative access
    QaReviewer,   // Review and quality assurance access  
    Operator,     // Limited operational access
}

pub struct AdminUser {
    pub user: User,
    pub roles: Vec<AdminRole>,
    pub permissions: HashSet<Permission>,
}
```

#### Permission System
```rust
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Permission {
    // Queue management
    ViewQueue,
    ModifyQueue,
    BulkQueueOperations,
    
    // Review system
    ViewReviews,
    BulkReviewActions,
    ManageReviewers,
    
    // Publishing
    ViewPublishQueue,
    ModifyPublishSettings,
    EmergencyPublishControls,
    
    // System administration
    ViewSystemMetrics,
    ModifySystemSettings,
    ManageUsers,
}
```

### Database Integration

#### Admin-Specific Queries
```rust
impl DatabaseManager {
    // Admin dashboard statistics
    pub async fn get_admin_dashboard_stats(&self) -> Result<AdminDashboardStats, DatabaseError> {
        // Comprehensive system statistics for admin dashboard
    }
    
    // Bulk operations
    pub async fn bulk_reschedule_runs(
        &self, 
        filters: &QueueFilters,
        admin_user: &AdminUser,
    ) -> Result<BulkOperationResult, DatabaseError> {
        // Bulk rescheduling with audit logging
    }
}
```

### Template Integration

#### Admin Template Structure
```
templates/cupboard/
├── layout.html              # Admin layout with navigation
├── dashboard.html           # Main admin dashboard
├── queue/
│   ├── index.html          # Queue overview
│   ├── details.html        # Queue item details
│   └── bulk_operations.html # Bulk operation forms
├── reviews/
│   ├── queue.html          # Review queue
│   ├── statistics.html     # Review analytics
│   └── templates.html      # Review response templates
├── publish/
│   ├── dashboard.html      # Publishing overview
│   ├── controls.html       # Rate limit and emergency controls
│   └── history.html        # Publishing history
└── merge_proposals/
    ├── overview.html       # MP system overview
    ├── management.html     # MP bulk operations
    └── analytics.html      # MP analytics and health
```

## Security Considerations

### Access Control
- **Multi-Factor Authentication**: Optional MFA for admin users
- **Session Management**: Enhanced session security for admin accounts
- **IP Restrictions**: Optional IP-based access restrictions
- **Activity Monitoring**: Real-time monitoring of admin activities

### Audit Logging
```rust
#[derive(Debug, Serialize)]
pub struct AdminAuditEvent {
    pub timestamp: DateTime<Utc>,
    pub admin_user: String,
    pub action: String,
    pub target: Option<String>,
    pub details: serde_json::Value,
    pub ip_address: IpAddr,
    pub user_agent: String,
}
```

### Data Protection
- **Sensitive Data Masking**: Mask sensitive information in logs
- **Bulk Operation Limits**: Prevent accidental mass operations
- **Confirmation Requirements**: Require confirmation for destructive actions
- **Rollback Capabilities**: Ability to undo certain admin operations

## Testing Strategy

### Unit Tests
- Admin authentication and authorization
- Permission checking logic
- Bulk operation safety and rollback
- Template rendering with admin context
- API endpoint security and validation

### Integration Tests
- End-to-end admin workflows
- Cross-service administrative operations
- Security boundary testing
- Performance under admin load
- Error handling and recovery

### Security Tests
- Authentication bypass attempts
- Authorization escalation tests
- CSRF and XSS protection validation
- Session security verification
- Audit log integrity checks

## Performance Requirements

### Response Time Targets
- **Dashboard Load**: < 500ms for admin dashboard
- **Queue Operations**: < 2s for bulk queue operations
- **Review Actions**: < 1s for individual review actions
- **Publishing Controls**: < 100ms for emergency stops
- **Analytics Queries**: < 5s for complex analytics

### Scalability Requirements
- **Concurrent Admins**: Support 10+ concurrent admin users
- **Bulk Operations**: Handle 1000+ items in bulk operations
- **Real-time Updates**: WebSocket updates for live monitoring
- **Data Volume**: Efficient handling of large datasets

## Migration Timeline

| Phase | Component | Effort | Dependencies | Priority | Status |
|-------|-----------|--------|--------------|----------|---------|
| 1 | Admin Infrastructure | 2 weeks | Site core | High | ✅ **COMPLETED** |
| 2 | Admin API Layer | 2 weeks | Phase 1 | High | ✅ **COMPLETED** |
| 3 | Queue Management | 1 week | Phases 1-2 | High | 🏗️ **IN PROGRESS** |
| 4 | Review Administration | 1 week | Phases 1-2 | Medium | 📋 **TODO** |
| 5 | Publishing Controls | 1 week | Phases 1-2 | Medium | 📋 **TODO** |
| 6 | MP Management | 1 week | Phases 1-2 | Medium | 📋 **TODO** |

**Total Estimated Duration**: 8 weeks

## Success Criteria

### Functional Requirements
- 100% feature parity with Python Cupboard interface
- All admin operations work correctly
- Proper authentication and authorization
- Comprehensive audit logging
- Real-time monitoring and controls

### Quality Requirements
- Comprehensive test coverage (>95%)
- Security testing passes all checks
- Performance meets or exceeds targets
- Clean, maintainable admin interface code
- Complete documentation for admin features

### Security Requirements
- No privilege escalation vulnerabilities
- Comprehensive audit trail for all actions
- Proper session and authentication security
- Input validation prevents all injection attacks
- Rate limiting prevents abuse

## Related Plans

### Dependencies
- [`../porting-plan.md`](../porting-plan.md) - Master coordination plan
- [`porting-plan.md`](porting-plan.md) - Main site service implementation
- [`../runner/porting-plan.md`](../runner/porting-plan.md) - Queue system integration
- [`../publish/porting-plan.md`](../publish/porting-plan.md) - Publishing system integration

### Integration Points
- **Site Service**: Core web framework and authentication
- **Runner Service**: Queue management and job control
- **Publisher Service**: Publishing controls and monitoring
- **Database**: Admin-specific queries and operations