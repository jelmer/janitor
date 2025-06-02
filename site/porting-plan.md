# Site (Web Interface) Porting Plan

## Overview

This document outlines the detailed migration plan for porting the Janitor web interface from Python to Rust. The site represents the largest and most complex component, with **4,915 lines** of Python code and **101 HTML templates** containing **3,273 lines** of Jinja2 markup.

## Current Implementation Analysis

### Technology Stack (Python)
- **Web Framework**: aiohttp with aiohttp-jinja2 integration
- **Template Engine**: Jinja2 with custom filters and global functions
- **Database**: AsyncPG for PostgreSQL with connection pooling
- **Authentication**: OpenID Connect (OIDC) with custom session management
- **Monitoring**: aiozipkin for tracing, aiohttp-openmetrics for Prometheus
- **Pub/Sub**: Redis-based real-time messaging
- **API Documentation**: aiohttp-apispec with marshmallow schemas

### Code Distribution

| Component | Lines | Complexity | Description |
|-----------|-------|------------|-------------|
| **Main Site** |
| `simple.py` | 769 | ⭐⭐⭐⭐ | Primary web app with main page handlers |
| `api.py` | 695 | ⭐⭐⭐⭐⭐ | Complete REST API implementation |
| `pkg.py` | 414 | ⭐⭐⭐ | Package views and codebase browsing |
| `common.py` | 389 | ⭐⭐⭐⭐ | Shared utilities and template helpers |
| `webhook.py` | 346 | ⭐⭐⭐ | Webhook processing and integrations |
| `openid.py` | 165 | ⭐⭐⭐⭐ | OpenID Connect authentication |
| **Cupboard Admin** |
| `cupboard/__init__.py` | 771 | ⭐⭐⭐⭐⭐ | Main admin interface |
| `cupboard/api.py` | 411 | ⭐⭐⭐⭐ | Administrative REST endpoints |
| `cupboard/review.py` | 194 | ⭐⭐⭐ | Code review system |
| `cupboard/queue.py` | 113 | ⭐⭐ | Queue management interfaces |
| `cupboard/merge_proposals.py` | 91 | ⭐⭐ | Merge proposal management |
| `cupboard/publish.py` | 49 | ⭐⭐ | Publishing status views |
| **Templates** | 3,273 | ⭐⭐⭐⭐ | 101 Jinja2 templates |
| **Static Assets** | ~500 | ⭐⭐ | CSS, JS, images |

**Total: 4,915 Python lines + 3,273 template lines = 8,188 lines**

## Target Architecture (Rust)

### Technology Stack (Rust)
- **Web Framework**: Axum with Tower middleware
- **Template Engine**: Tera (Jinja2-compatible syntax)
- **Database**: sqlx with PostgreSQL async driver
- **Authentication**: oauth2 crate with custom session management
- **Monitoring**: tracing with prometheus metrics
- **Pub/Sub**: redis crate for real-time messaging
- **API Documentation**: utoipa for OpenAPI generation

### Dependencies to Add
```toml
# Web Framework
axum = "0.8"
tower = "0.5"
tower-http = "0.6"
hyper = "1.0"

# Templates & Assets
tera = "1.19"
include_dir = "0.7"

# Authentication & Security
oauth2 = "4.4"
jsonwebtoken = "9.0"
cookie = "0.18"
bcrypt = "0.15"

# Database
sqlx = { version = "0.8", features = ["postgres", "chrono", "uuid"] }

# API & Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
utoipa = "4.0"
utoipa-axum = "0.1"

# Monitoring & Observability
tracing = "0.1"
tracing-subscriber = "0.3"
metrics = "0.22"
metrics-prometheus = "0.7"

# Redis & Pub/Sub
redis = { version = "0.27", features = ["tokio-comp", "connection-manager"] }

# Time & Utils
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.0", features = ["v4", "serde"] }
```

## Phased Implementation Plan

### Phase 3.1: Core Infrastructure (3-4 weeks)
**Target**: Foundation and shared utilities

#### 3.1.1: Project Setup & Web Framework
- [ ] Create Axum-based web server with proper configuration
- [ ] Set up routing structure matching Python paths
- [ ] Implement middleware stack (logging, metrics, CORS)
- [ ] Add health check and basic admin endpoints

#### 3.1.2: Database Integration
- [ ] Port database connection management from asyncpg to sqlx
- [ ] Implement connection pooling and transaction management
- [ ] Add database health monitoring
- [ ] Create shared query utilities and error handling

#### 3.1.3: Template Engine Setup
- [ ] Configure Tera template engine with Jinja2 compatibility
- [ ] Set up template loading and caching
- [ ] Implement custom filters and functions from Python
- [ ] Create template context helpers and globals

#### 3.1.4: Configuration & Environment
- [ ] Port configuration management from Python
- [ ] Add environment-specific settings
- [ ] Implement feature flags and debug modes
- [ ] Set up logging and monitoring infrastructure

### Phase 3.2: Authentication System (2-3 weeks)
**Target**: OpenID Connect and session management

#### 3.2.1: OpenID Connect Integration
- [ ] Port OIDC provider configuration
- [ ] Implement authorization code flow
- [ ] Add token validation and refresh logic
- [ ] Create user info retrieval and mapping

#### 3.2.2: Session Management
- [ ] Implement secure cookie-based sessions
- [ ] Add session storage and persistence
- [ ] Create session middleware and context injection
- [ ] Port user role and permission checking

#### 3.2.3: Authentication Middleware
- [ ] Create auth middleware for protected routes
- [ ] Implement login/logout flows
- [ ] Add user context injection
- [ ] Set up role-based access control

### Phase 3.3: Core API Foundation (2-3 weeks)
**Target**: Basic REST API infrastructure

#### 3.3.1: API Routing & Middleware
- [ ] Set up API route structure matching Python
- [ ] Implement content negotiation (JSON/HTML)
- [ ] Add request/response logging and metrics
- [ ] Create error handling and status code mapping

#### 3.3.2: Request/Response Types
- [ ] Define API request/response structures
- [ ] Implement validation and serialization
- [ ] Port marshmallow schemas to Rust structs
- [ ] Add OpenAPI documentation generation

#### 3.3.3: Common API Utilities
- [ ] Implement pagination helpers
- [ ] Add query parameter parsing
- [ ] Create response formatting utilities
- [ ] Port API error types and handling

### Phase 3.4: Template Migration (4-5 weeks)
**Target**: Convert all Jinja2 templates to Tera

#### 3.4.1: Layout Templates (Week 1)
- [ ] Port `layout.html` base template
- [ ] Convert navigation and sidebar templates
- [ ] Migrate footer and common includes
- [ ] Set up template inheritance structure

#### 3.4.2: Main Site Templates (Week 2)
- [ ] Port index page and about templates
- [ ] Convert FAQ and help page templates
- [ ] Migrate package listing and search templates
- [ ] Port authentication-related templates

#### 3.4.3: Package/Codebase Templates (Week 2)
- [ ] Convert codebase browsing templates
- [ ] Port run history and result templates
- [ ] Migrate log viewing and diff templates
- [ ] Convert merge proposal templates

#### 3.4.4: Error & Status Templates (Week 1)
- [ ] Port 40+ result code templates
- [ ] Convert error pages (404, 500, etc.)
- [ ] Migrate status and loading templates
- [ ] Add template debugging and validation

### Phase 3.5: Main Site Implementation (3-4 weeks)
**Target**: Port `simple.py` and `pkg.py` functionality

#### 3.5.1: Homepage & Navigation
- [ ] Implement main page handler with statistics
- [ ] Add codebase listing and search
- [ ] Port pagination and filtering
- [ ] Create navigation state management

#### 3.5.2: Package Views
- [ ] Port codebase detail views
- [ ] Implement run history browsing
- [ ] Add log file viewing and downloading
- [ ] Create diff generation and viewing

#### 3.5.3: Search & Discovery
- [ ] Implement package search functionality
- [ ] Add filtering and sorting options
- [ ] Port typeahead search integration
- [ ] Create result ranking and relevance

### Phase 3.6: REST API Implementation (4-5 weeks)
**Target**: Port complete `api.py` functionality (695 lines)

#### 3.6.1: Core API Endpoints
- [ ] Port codebase and campaign endpoints
- [ ] Implement run and result endpoints
- [ ] Add queue and worker status APIs
- [ ] Create health and metrics endpoints

#### 3.6.2: Query & Filtering APIs
- [ ] Implement complex query parameters
- [ ] Add result filtering and pagination
- [ ] Port search and discovery APIs
- [ ] Create export and reporting endpoints

#### 3.6.3: Administrative APIs
- [ ] Port admin-only endpoints
- [ ] Implement configuration management APIs
- [ ] Add system status and monitoring APIs
- [ ] Create bulk operation endpoints

### Phase 3.7: Cupboard Admin Interface (3-4 weeks)
**Target**: Port all cupboard functionality (1,629 lines)

#### 3.7.1: Admin Dashboard
- [ ] Port main cupboard interface
- [ ] Implement worker monitoring views
- [ ] Add system status dashboard
- [ ] Create metrics and reporting views

#### 3.7.2: Queue Management
- [ ] Port queue browsing and management
- [ ] Implement job control (pause, resume, cancel)
- [ ] Add priority adjustment and reordering
- [ ] Create bulk queue operations

#### 3.7.3: Review System
- [ ] Port code review interfaces
- [ ] Implement verdict submission and tracking
- [ ] Add reviewer assignment and workflows
- [ ] Create review history and analytics

#### 3.7.4: Publishing Controls
- [ ] Port publish configuration management
- [ ] Implement merge proposal oversight
- [ ] Add rate limiting controls
- [ ] Create publishing analytics

### Phase 3.8: Real-time Features (2 weeks)
**Target**: Redis pub/sub and webhooks

#### 3.8.1: Redis Integration
- [ ] Port Redis pub/sub functionality
- [ ] Implement real-time status updates
- [ ] Add live queue monitoring
- [ ] Create event streaming for long operations

#### 3.8.2: Webhook Processing
- [ ] Port webhook endpoint handlers
- [ ] Implement signature verification
- [ ] Add event processing and routing
- [ ] Create webhook registration management

### Phase 3.9: Static Assets & Frontend (1-2 weeks)
**Target**: Asset serving and JavaScript integration

#### 3.9.1: Asset Pipeline
- [ ] Set up static file serving
- [ ] Implement CSS/JS compression and caching
- [ ] Add fingerprinting for cache busting
- [ ] Create asset optimization pipeline

#### 3.9.2: JavaScript Integration
- [ ] Port custom JavaScript functionality
- [ ] Integrate third-party libraries (jQuery, DataTables)
- [ ] Add client-side form validation
- [ ] Implement AJAX interactions

### Phase 3.10: Testing & Validation (2-3 weeks)
**Target**: Comprehensive testing and Python parity verification

#### 3.10.1: Unit Testing ✅ **COMPLETED**
- [x] Create comprehensive unit tests
- [x] Test all API endpoints for parity  
- [x] Validate template rendering
- [x] Test authentication and session flows

#### 3.10.2: Integration Testing ✅ **COMPLETED**
- [x] Test full user workflows
- [x] Validate database operations
- [x] Test real-time features
- [x] Perform load and performance testing

#### 3.10.3: Python Parity Verification ✅ **COMPLETED**
- [x] Compare all API responses with Python implementation
- [x] Validate HTML output matching with advanced comparison algorithms
- [x] Test edge cases and error handling including boundary conditions
- [x] Verify performance improvements with comprehensive benchmarking

## Implementation Strategy

### Development Approach
1. **Incremental Replacement**: Implement services one by one with feature flags
2. **Template-First**: Focus on template migration early to enable rapid iteration
3. **API Compatibility**: Maintain exact HTTP API compatibility during migration
4. **Performance Monitoring**: Benchmark against Python implementation continuously

### Technical Considerations

#### Template Migration Strategy
- Use Tera's Jinja2 compatibility mode
- Create automated template validation tools
- Implement side-by-side comparison testing
- Maintain template feature parity

#### Database Migration
- Reuse existing PostgreSQL schema
- Port complex queries incrementally
- Maintain transaction compatibility
- Add query performance monitoring

#### Authentication Migration
- Preserve existing session tokens during transition
- Implement gradual OIDC provider migration
- Maintain user role compatibility
- Add security audit logging

### Risk Mitigation

#### High-Risk Areas
1. **Template Compatibility**: Jinja2→Tera syntax differences
2. **Authentication Flow**: OIDC integration complexity
3. **Database Queries**: Complex asyncpg→sqlx conversion
4. **Real-time Features**: Redis pub/sub integration

#### Migration Strategy
1. **Feature Flags**: Enable gradual rollout of Rust components
2. **Parallel Deployment**: Run both versions side-by-side initially
3. **Automated Testing**: Comprehensive comparison testing
4. **Rollback Plan**: Quick revert to Python if issues arise

## Success Criteria

### Functional Requirements
- [ ] 100% API endpoint compatibility with Python
- [ ] Identical HTML output for all templates
- [ ] Complete authentication and session compatibility
- [ ] All real-time features working (Redis pub/sub)
- [ ] Full admin interface functionality

### Performance Requirements
- [ ] Page load times < 200ms (vs Python's 500ms+)
- [ ] API response times < 100ms for simple queries
- [ ] Template rendering 5x faster than Jinja2
- [ ] Memory usage 50-70% lower than Python
- [ ] Cold start times < 3 seconds

### Quality Requirements
- [ ] Test coverage > 90% for all new Rust code
- [ ] Zero regressions in existing functionality
- [ ] Security audit with no new vulnerabilities
- [ ] Performance benchmarks show significant improvements

## Timeline Summary

| Phase | Duration | Dependencies | Risk Level |
|-------|----------|--------------|------------|
| 3.1 Core Infrastructure | 3-4 weeks | Phase 2 complete | Medium |
| 3.2 Authentication | 2-3 weeks | Phase 3.1 | High |
| 3.3 API Foundation | 2-3 weeks | Phase 3.1, 3.2 | Medium |
| 3.4 Template Migration | 4-5 weeks | Phase 3.1 | High |
| 3.5 Main Site | 3-4 weeks | Phase 3.4 | Medium |
| 3.6 REST API | 4-5 weeks | Phase 3.3, 3.5 | Medium |
| 3.7 Cupboard Admin | 3-4 weeks | Phase 3.6 | Medium |
| 3.8 Real-time Features | 2 weeks | Phase 3.6 | Medium |
| 3.9 Static Assets | 1-2 weeks | Phase 3.5 | Low |
| 3.10 Testing & Validation | 2-3 weeks | All phases | Low |

**Total Estimated Duration: 26-36 weeks (6.5-9 months)**

## Related Plans

This site porting plan coordinates with:
- Main porting plan: [`porting-plan.md`](../porting-plan.md)
- Runner service: [`runner/porting-plan.md`](../runner/porting-plan.md) 
- Publisher service: [`publish/porting-plan.md`](../publish/porting-plan.md)
- Differ service: [`differ/porting-plan.md`](../differ/porting-plan.md)

## Next Steps

1. **Begin Phase 3.1**: Set up core Axum infrastructure
2. **Template Analysis**: Deep dive into Jinja2→Tera conversion requirements
3. **API Endpoint Catalog**: Complete inventory of all 695 lines of API code
4. **Authentication Design**: Detailed OIDC integration architecture
5. **Performance Baseline**: Establish Python performance benchmarks