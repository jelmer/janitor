# Site Service Porting Plan

> **Status**: 🚧 **IN PROGRESS** - Minimal Rust implementation exists, needs complete web interface migration.
> 
> 📋 **Master Plan**: See [`../porting-plan.md`](../porting-plan.md) for overall project coordination and dependencies.

## Overview

This document outlines the detailed plan for porting the Janitor site service from Python to Rust. The site service is the primary web interface and public API for the Janitor platform, providing both user-facing HTML interfaces and REST APIs for external consumption.

### Current State Analysis

**Python Implementation (`py/janitor/site/`)**: ~3,286 lines
- Main site interface: `simple.py` (769 lines) - primary web application
- API endpoints: `api.py` (695 lines) - REST API implementation
- Package management: `pkg.py` (414 lines) - package-specific views
- Common utilities: `common.py` (389 lines) - shared functionality
- Authentication: `openid.py` (165 lines) - OpenID Connect integration
- Other modules: `webhook.py`, `setup.py`, `merge_proposals.py`, `pubsub.py`

**Cupboard Subsystem (`py/janitor/site/cupboard/`)**: ~1,629 lines
- Main cupboard app: `__init__.py` (771 lines) - administrative interface
- Cupboard API: `api.py` (411 lines) - admin REST endpoints
- Review system: `review.py` (194 lines) - code review functionality
- Queue management: `queue.py` (113 lines) - job queue interfaces
- Publishing: `publish.py` (49 lines) - publish status views
- Merge proposals: `merge_proposals.py` (91 lines) - MP management

**Templates**: 50+ Jinja2 HTML templates covering all user interfaces

**Rust Implementation (`site/`)**: ~88 lines (minimal)
- Only contains basic log analysis functionality
- No web server, templates, or API endpoints implemented

## Technical Architecture Analysis

### Current Python Stack
- **Web Framework**: aiohttp with aiohttp-jinja2 for templating
- **Authentication**: OpenID Connect with custom session management
- **Database**: AsyncPG for PostgreSQL connections
- **Templates**: Jinja2 with custom filters and globals
- **Static Assets**: Direct file serving with custom CSS/JS
- **API**: RESTful endpoints with marshmallow schemas
- **Monitoring**: aiozipkin for distributed tracing
- **Pub/Sub**: Redis-based messaging for real-time updates

### Target Rust Architecture
- **Web Framework**: Axum for HTTP server and routing
- **Authentication**: OAuth2/OIDC with axum-extra sessions
- **Database**: sqlx for type-safe PostgreSQL operations
- **Templates**: Tera templating engine (Jinja2-like syntax)
- **Static Assets**: tower-http for static file serving
- **API**: serde for JSON serialization with typed schemas
- **Monitoring**: tracing ecosystem with opentelemetry
- **Pub/Sub**: redis-rs with tokio for async messaging

## Porting Strategy

### Phase 1: Core Infrastructure (4-5 weeks)

#### 1.1 Web Server Foundation (1 week)
- Set up Axum application with basic routing
- Configure logging and tracing infrastructure
- Implement health check and metrics endpoints
- Set up static file serving for CSS/JS/images

**Deliverables:**
- Basic Axum server with routing structure
- Static asset serving functionality
- Health and metrics endpoints
- Configuration management

#### 1.2 Database Integration (1 week)
- Integrate sqlx with PostgreSQL connection pooling
- Port database query functions from common.py
- Implement connection lifecycle management
- Add database health checks

**Deliverables:**
- Database connection pool setup
- Core query functions ported
- Database health monitoring
- Migration from asyncpg patterns to sqlx

#### 1.3 Authentication System (1.5 weeks)
- Port OpenID Connect authentication from openid.py
- Implement session management with secure cookies
- Add role-based access control (admin, qa_reviewer)
- Port user context and permission checking

**Deliverables:**
- OIDC authentication flow
- Session management system
- Role-based access control
- User context middleware

#### 1.4 Template Engine Setup (0.5 weeks)
- Configure Tera templating engine
- Port template globals and filters from TEMPLATE_ENV
- Set up template inheritance structure
- Implement template caching

**Deliverables:**
- Tera configuration with custom functions
- Template loading and inheritance
- Custom filters for date/duration formatting
- Template caching mechanism

### Phase 2: Core Web Interface (6-7 weeks)

#### 2.1 Basic Page Routes (1.5 weeks)
- Port main page handlers from simple.py
- Implement index, about, FAQ pages
- Add basic navigation and layout templates
- Port template utility functions

**Effort Estimate:**
- **Lines to port**: ~200 from simple.py (basic handlers)
- **Templates**: ~10 basic templates (layout, index, about, FAQ)
- **Complexity**: Medium - straightforward page rendering

**Deliverables:**
- Main site navigation structure
- Basic informational pages
- Template layout hierarchy
- Common template utilities

#### 2.2 Package and Codebase Views (2 weeks)
- Port package-specific functionality from pkg.py (414 lines)
- Implement codebase browsing and search
- Add run history and result display
- Port merge proposal integration

**Effort Estimate:**
- **Lines to port**: ~414 from pkg.py
- **Templates**: ~8 package-related templates
- **Complexity**: High - complex database queries and result formatting

**Deliverables:**
- Package browsing interface
- Codebase search and filtering
- Run history display
- Merge proposal views

#### 2.3 Run and Result Display (2 weeks)
- Port run display logic from common.py
- Implement artifact and log viewing
- Add diff visualization and comparison
- Port result code classification

**Effort Estimate:**
- **Lines to port**: ~300 from common.py (run-related)
- **Templates**: ~12 run and result templates
- **Complexity**: High - complex data formatting and external service integration

**Deliverables:**
- Run detail pages
- Artifact and log viewers
- Diff visualization
- Result classification system

#### 2.4 Chart and Statistics (1.5 weeks)
- Port chart data generation from simple.py
- Implement statistics aggregation
- Add dashboard and summary views
- Port JSON API endpoints for charts

**Effort Estimate:**
- **Lines to port**: ~200 from simple.py (chart handlers)
- **Templates**: ~5 dashboard templates
- **Complexity**: Medium - data aggregation and JSON APIs

**Deliverables:**
- Dashboard with statistics
- Chart data APIs
- Progress tracking views
- Summary report generation

### Phase 3: REST API Implementation (3-4 weeks)

#### 3.1 Core API Endpoints (2 weeks)
- Port main API handlers from api.py (695 lines)
- Implement typed request/response schemas
- Add API authentication and rate limiting
- Port OpenAPI/Swagger documentation

**Effort Estimate:**
- **Lines to port**: ~695 from api.py
- **Complexity**: High - comprehensive API with complex schemas

**Deliverables:**
- Complete REST API implementation
- Typed request/response handling
- API documentation generation
- Rate limiting and authentication

#### 3.2 Integration APIs (1.5 weeks)
- Port webhook handling from webhook.py (346 lines)
- Implement pub/sub message handling
- Add external service integration
- Port VCS integration endpoints

**Effort Estimate:**
- **Lines to port**: ~346 from webhook.py + ~78 from pubsub.py
- **Complexity**: Medium - external integrations and async messaging

**Deliverables:**
- Webhook processing system
- Pub/sub message handling
- External service APIs
- VCS integration endpoints

#### 3.3 API Testing and Validation (0.5 weeks)
- Implement comprehensive API tests
- Add request validation and error handling
- Port API schema validation
- Add API performance monitoring

**Deliverables:**
- API test suite
- Request validation system
- Error handling middleware
- Performance monitoring

### Phase 4: Cupboard Administrative Interface (4-5 weeks)

#### 4.1 Core Cupboard Infrastructure (1 week)
- Port cupboard app structure from cupboard/__init__.py
- Implement admin-only routing and middleware
- Add cupboard-specific templates and styling
- Port utility functions and helpers

**Effort Estimate:**
- **Lines to port**: ~200 from cupboard/__init__.py (infrastructure)
- **Templates**: ~5 cupboard layout templates
- **Complexity**: Medium - admin interface setup

**Deliverables:**
- Cupboard application structure
- Admin authentication middleware
- Cupboard template system
- Navigation and utilities

#### 4.2 Queue and Status Management (1.5 weeks)
- Port queue management from cupboard/queue.py (113 lines)
- Implement worker status monitoring
- Add job scheduling and priority views
- Port queue statistics and metrics

**Effort Estimate:**
- **Lines to port**: ~113 from queue.py + related handlers
- **Templates**: ~8 queue management templates
- **Complexity**: Medium - queue visualization and management

**Deliverables:**
- Queue management interface
- Worker status monitoring
- Job scheduling controls
- Queue metrics dashboard

#### 4.3 Review System (2 weeks)
- Port review functionality from cupboard/review.py (194 lines)
- Implement code review interface
- Add approval/rejection workflows
- Port review history and tracking

**Effort Estimate:**
- **Lines to port**: ~194 from review.py + related handlers
- **Templates**: ~10 review interface templates
- **Complexity**: High - complex review workflows and UI

**Deliverables:**
- Code review interface
- Approval/rejection system
- Review history tracking
- Review workflow automation

#### 4.4 Administrative Tools (1 week)
- Port result code analysis tools
- Implement bulk operations interface
- Add system health monitoring
- Port administrative utilities

**Effort Estimate:**
- **Lines to port**: ~400 from cupboard/__init__.py (admin handlers)
- **Templates**: ~12 administrative templates
- **Complexity**: Medium - various admin tools and bulk operations

**Deliverables:**
- Result code analysis tools
- Bulk operation interface
- System health dashboard
- Administrative utilities

### Phase 5: Template Migration (2-3 weeks)

#### 5.1 Template Conversion (2 weeks)
- Convert 50+ Jinja2 templates to Tera format
- Port template macros and includes
- Update template syntax and filters
- Test template rendering and inheritance

**Effort Estimate:**
- **Templates**: 50+ HTML templates
- **Complexity**: Medium - syntax conversion and testing

**Deliverables:**
- All templates converted to Tera
- Template macro system ported
- Template inheritance working
- Template rendering tests

#### 5.2 Static Asset Integration (1 week)
- Port CSS and JavaScript assets
- Update asset pipeline and optimization
- Implement cache busting and versioning
- Test responsive design and compatibility

**Deliverables:**
- Static asset serving
- Asset optimization pipeline
- Cache management
- Cross-browser compatibility

### Phase 6: Testing and Validation (2-3 weeks)

#### 6.1 Unit and Integration Tests (1.5 weeks)
- Port test suites from Python test files
- Implement Rust-specific test patterns
- Add database test fixtures and helpers
- Create API integration tests

**Deliverables:**
- Comprehensive test suite
- Database test utilities
- API test coverage
- Performance benchmarks

#### 6.2 API Parity Validation (1 week)
- Compare API responses between Python and Rust
- Validate JSON schema compatibility
- Test authentication flows
- Verify webhook processing

**Deliverables:**
- API parity test suite
- Response format validation
- Authentication test coverage
- Webhook test automation

#### 6.3 Performance Optimization (0.5 weeks)
- Profile application performance
- Optimize database queries
- Tune template rendering
- Implement caching strategies

**Deliverables:**
- Performance benchmarks
- Query optimization
- Template caching
- Response time monitoring

## Implementation Details

### Key Dependencies

**Rust Crates:**
```toml
[dependencies]
axum = "0.7"           # Web framework
sqlx = "0.7"           # Database toolkit
tera = "1.19"          # Template engine
serde = "1.0"          # Serialization
tokio = "1.0"          # Async runtime
tower = "0.4"          # Service middleware
tower-http = "0.5"     # HTTP utilities
oauth2 = "4.4"         # OAuth2 client
redis = "0.24"         # Redis client
tracing = "0.1"        # Logging/tracing
opentelemetry = "0.21" # Observability
uuid = "1.6"           # UUID generation
chrono = "0.4"         # Date/time handling
```

### Critical Migration Patterns

1. **Async Handler Conversion**:
   ```python
   # Python (aiohttp)
   async def handle_run(request):
       async with request.app["pool"].acquire() as conn:
           run = await get_run(conn, run_id)
       return render_template("run.html", run=run)
   ```
   
   ```rust
   // Rust (axum)
   async fn handle_run(
       State(app): State<AppState>,
       Path(run_id): Path<String>,
   ) -> Result<Html<String>, AppError> {
       let run = get_run(&app.db, &run_id).await?;
       let html = app.templates.render("run.html", &context! { run })?;
       Ok(Html(html))
   }
   ```

2. **Database Query Migration**:
   ```python
   # Python (asyncpg)
   async def get_run(conn, run_id):
       return await conn.fetchrow("SELECT * FROM run WHERE id = $1", run_id)
   ```
   
   ```rust
   // Rust (sqlx)
   async fn get_run(db: &PgPool, run_id: &str) -> Result<Run, sqlx::Error> {
       sqlx::query_as!(Run, "SELECT * FROM run WHERE id = $1", run_id)
           .fetch_one(db)
           .await
   }
   ```

3. **Template Rendering**:
   ```python
   # Python (jinja2)
   return render_template("template.html", data=data)
   ```
   
   ```rust
   // Rust (tera)
   let html = templates.render("template.html", &context! { data })?;
   Ok(Html(html))
   ```

### Risk Mitigation

1. **Template Compatibility**: Use automated tools to convert Jinja2 syntax to Tera
2. **Database Performance**: Implement connection pooling and query optimization early
3. **Authentication Security**: Thoroughly test OIDC integration and session management
4. **API Compatibility**: Maintain strict API compatibility through comprehensive testing

## Timeline and Effort Estimates

### Total Effort: 18-22 weeks (4.5-5.5 months)

| Phase | Duration | Effort Level | Risk Level |
|-------|----------|--------------|------------|
| 1. Core Infrastructure | 4-5 weeks | High | Medium |
| 2. Core Web Interface | 6-7 weeks | Very High | High |
| 3. REST API Implementation | 3-4 weeks | High | Medium |
| 4. Cupboard Admin Interface | 4-5 weeks | High | Medium |
| 5. Template Migration | 2-3 weeks | Medium | Low |
| 6. Testing and Validation | 2-3 weeks | Medium | Low |

### Critical Dependencies

- **Database Schema**: Must be stable before Phase 1.2
- **Authentication System**: Required for all subsequent phases
- **Template Engine**: Must be working before Phase 2
- **API Compatibility**: Critical for external integrations

### Success Metrics

1. **Functional Parity**: 100% feature compatibility with Python implementation
2. **Performance**: Response times ≤ Python implementation
3. **API Compatibility**: All REST endpoints maintain identical schemas
4. **Template Rendering**: All pages render identically to Python version
5. **Test Coverage**: ≥90% code coverage with comprehensive integration tests

## Related Porting Plans

- 📋 **Master Plan**: [`../porting-plan.md`](../porting-plan.md) - Overall project coordination
- ✅ **Runner**: [`../runner/porting-plan.md`](../runner/porting-plan.md) - Already completed
- ✅ **Publisher**: [`../publish/porting-plan.md`](../publish/porting-plan.md) - Already completed
- 🚧 **Differ**: [`../differ/porting-plan.md`](../differ/porting-plan.md) - In progress

---

*This plan will be updated as implementation progresses and requirements evolve.*