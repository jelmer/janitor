# Site Python Bindings Porting Plan

> **Status**: 📋 **FUTURE PLANNING** - This plan outlines Python bindings for the Rust site service.
> 
> 📋 **Master Plan**: See [`../porting-plan.md`](../porting-plan.md) for overall project coordination and dependencies.

## Overview

This document outlines the plan for creating Python bindings for the Rust site service. These bindings will provide a compatibility layer allowing existing Python code to interact with the new Rust-based web interface while maintaining API compatibility during the migration period.

## Scope and Purpose

### Target Use Cases
- **Migration Bridge**: Smooth transition from Python to Rust site service
- **Testing Integration**: Python-based integration tests for Rust endpoints
- **Legacy Compatibility**: Support for existing Python scripts and tools
- **Development Workflow**: Allow developers to use Python tooling during migration

### Components to Bind
- HTTP client for Rust site service APIs
- Template rendering compatibility layer
- Authentication and session management
- Database query interfaces (via HTTP APIs)
- Configuration and setup utilities

## Implementation Strategy

### Phase 1: Core HTTP Client (HIGH PRIORITY)
**Estimated effort**: 1 week

#### 1.1 API Client Wrapper
```python
# Target implementation
from janitor_site_py import SiteClient

client = SiteClient(base_url="http://localhost:8080")
response = client.get_codebase("debian/package-name")
```

#### Implementation Details
- **HTTP Client**: Using requests or httpx for HTTP communication
- **Response Models**: Pydantic models matching Rust API schemas
- **Error Handling**: Python exceptions matching Rust error types
- **Authentication**: OAuth2/OpenID Connect integration
- **Async Support**: Optional async/await interface

### Phase 2: Template Compatibility (MEDIUM PRIORITY)
**Estimated effort**: 1-2 weeks

#### 2.1 Template Rendering Bridge
```python
# Compatibility layer for existing template usage
from janitor_site_py import TemplateRenderer

renderer = TemplateRenderer(service_url="http://localhost:8080")
html = renderer.render("codebase.html", context={"codebase": "example"})
```

#### Implementation Details
- **Template Proxy**: Forward template rendering to Rust service
- **Context Serialization**: JSON-based context passing
- **Error Mapping**: Template errors from Rust to Python exceptions
- **Caching**: Client-side caching for rendered templates
- **Development Mode**: Direct template file watching for development

### Phase 3: Database Interface (MEDIUM PRIORITY) 
**Estimated effort**: 1 week

#### 3.1 Database Query Proxy
```python
# Database operations via HTTP APIs
from janitor_site_py import DatabaseClient

db = DatabaseClient(site_url="http://localhost:8080")
codebases = db.get_codebases(limit=50, search="python")
```

#### Implementation Details
- **Query Methods**: Python methods for database operations
- **Result Models**: Pydantic models for database entities
- **Pagination**: Transparent pagination handling
- **Filtering**: Query builder interface for filtering
- **Transactions**: Transaction support via HTTP endpoints

### Phase 4: Configuration Integration (LOW PRIORITY)
**Estimated effort**: 2-3 days

#### 4.1 Configuration Bridge
```python
# Configuration compatibility
from janitor_site_py import SiteConfig

config = SiteConfig.from_env()
site_url = config.get_site_url()
```

#### Implementation Details
- **Environment Integration**: Read Rust service configuration
- **Service Discovery**: Automatic service endpoint detection
- **Health Checking**: Built-in health check integration
- **Fallback Handling**: Graceful degradation when service unavailable

## Python Package Structure

### Package Layout
```
site-py/
├── src/
│   └── janitor_site_py/
│       ├── __init__.py
│       ├── client.py          # HTTP client implementation
│       ├── models.py          # Pydantic models
│       ├── templates.py       # Template rendering bridge
│       ├── database.py        # Database interface
│       ├── auth.py           # Authentication helpers
│       ├── config.py         # Configuration integration
│       └── exceptions.py     # Error handling
├── tests/
│   ├── test_client.py
│   ├── test_templates.py
│   ├── test_database.py
│   └── integration/
│       └── test_site_integration.py
├── pyproject.toml
└── README.md
```

### Dependencies
```toml
[dependencies]
requests = "^2.31.0"
pydantic = "^2.0.0"
typing-extensions = "^4.0.0"

[optional-dependencies]
async = ["httpx[async]", "aiofiles"]
dev = ["pytest", "pytest-asyncio", "black", "mypy"]
```

## API Design

### Core Client Interface
```python
class SiteClient:
    def __init__(self, base_url: str, auth_token: Optional[str] = None):
        """Initialize client with Rust site service."""
        
    # Codebase operations
    def get_codebase(self, name: str) -> CodebaseModel:
        """Get codebase details."""
        
    def list_codebases(self, limit: int = 50, offset: int = 0, 
                      search: Optional[str] = None) -> CodebaseListModel:
        """List codebases with pagination."""
        
    # Run operations
    def get_run(self, run_id: str) -> RunModel:
        """Get run details."""
        
    def list_runs(self, codebase: Optional[str] = None,
                 suite: Optional[str] = None) -> RunListModel:
        """List runs with filtering."""
        
    # Template operations
    def render_template(self, template: str, context: Dict[str, Any]) -> str:
        """Render template via Rust service."""
        
    # Admin operations (requires auth)
    def get_queue_status(self) -> QueueStatusModel:
        """Get queue status (admin only)."""
```

### Data Models
```python
from pydantic import BaseModel
from datetime import datetime
from typing import Optional, List, Dict, Any

class CodebaseModel(BaseModel):
    name: str
    vcs_url: Optional[str]
    vcs_type: Optional[str]
    branch: Optional[str]
    suite: Optional[str]
    maintainer: Optional[str]
    
class RunModel(BaseModel):
    id: str
    codebase: str
    suite: str
    start_time: datetime
    finish_time: Optional[datetime]
    result_code: Optional[str]
    failure_stage: Optional[str]
    worker: Optional[str]
```

## Testing Strategy

### Unit Tests
- HTTP client functionality
- Response model validation
- Error handling and retries
- Authentication flow
- Template rendering proxy

### Integration Tests
- End-to-end API communication
- Template rendering compatibility
- Database operation equivalence
- Authentication and authorization
- Performance and reliability

### Compatibility Tests
- Side-by-side Python vs Rust comparisons
- Template output validation
- API response equivalence
- Error condition handling

## Migration Strategy

### Transition Plan
1. **Parallel Development**: Develop bindings alongside Rust site service
2. **Gradual Adoption**: Migrate Python code to use bindings incrementally
3. **Feature Parity**: Ensure bindings support all required functionality
4. **Testing Integration**: Use bindings for Python-based testing
5. **Sunset Planning**: Plan eventual removal of bindings post-migration

### Compatibility Guarantees
- **API Stability**: Maintain stable interface during migration period
- **Error Handling**: Consistent error behavior between Python and Rust
- **Performance**: Acceptable performance for migration use cases
- **Documentation**: Clear migration path and usage examples

## Performance Considerations

### Expected Performance
- **HTTP Overhead**: Additional latency due to HTTP communication
- **Serialization**: JSON serialization/deserialization overhead
- **Caching**: Client-side caching to reduce API calls
- **Connection Pooling**: Efficient HTTP connection management

### Optimization Strategies
- **Batch Operations**: Group multiple operations where possible
- **Async Interface**: Optional async support for high-concurrency use cases
- **Response Caching**: Cache stable responses (templates, configuration)
- **Connection Reuse**: HTTP/2 and connection pooling

## Maintenance and Lifecycle

### Support Timeline
- **Active Development**: During site service migration (Phase 3)
- **Maintenance Mode**: 6 months post-migration for stabilization
- **Deprecation Warning**: 6 months before sunset
- **End of Life**: 18 months post-migration completion

### Version Strategy
- **Semantic Versioning**: Clear version compatibility with Rust service
- **API Versioning**: Support for multiple Rust API versions
- **Deprecation Policy**: Clear deprecation timeline and migration path

## Success Criteria

### Functional Requirements
- 100% API coverage for migration use cases
- Template rendering produces identical output
- Authentication works seamlessly
- Database operations maintain consistency
- Error handling matches Rust service behavior

### Quality Requirements
- Comprehensive test coverage (>95%)
- Type safety with full mypy compatibility
- Clear documentation and examples
- Performance within acceptable limits
- Reliable error handling and recovery

## Related Plans

### Dependencies
- [`../porting-plan.md`](../porting-plan.md) - Master coordination plan
- [`../site/porting-plan.md`](../site/porting-plan.md) - Main site service implementation

### Integration Points
- **Site Service**: Primary dependency on Rust site implementation
- **Testing Framework**: Integration with existing Python test suites
- **CI/CD**: Compatibility testing in deployment pipelines
- **Documentation**: Usage examples and migration guides