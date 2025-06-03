# BZR Store PyO3 Integration Strategy

## Overview

This document outlines the strategy for implementing the BZR Store service in Rust using PyO3 to integrate with the Python Breezy library. This approach allows us to leverage the mature Breezy/Bazaar implementation while providing a Rust-based service infrastructure.

## Architecture Decision

### Why PyO3?

1. **No Rust Bazaar Implementation**: Unlike Git, there is no mature Rust implementation of the Bazaar protocol
2. **Complex Protocol**: The Bazaar smart protocol is complex and would require significant effort to reimplement
3. **Breezy Maturity**: The Breezy library (Python) is the maintained fork of Bazaar with years of development
4. **Incremental Migration**: PyO3 allows gradual migration from Python to Rust

### Alternative Approaches Considered

1. **Pure Subprocess Approach**: Using `brz` commands via subprocess
   - ✅ Pros: Simple, no Python embedding needed
   - ❌ Cons: Performance overhead, limited functionality, difficult error handling

2. **Full Rust Implementation**: Implementing Bazaar protocol in Rust
   - ✅ Pros: Best performance, no Python dependency
   - ❌ Cons: Massive effort (months/years), risk of incompatibility

3. **PyO3 Integration** (Chosen)
   - ✅ Pros: Full Breezy functionality, good performance, maintainable
   - ❌ Cons: Python runtime dependency, some complexity in integration

## Implementation Architecture

### Component Layers

```
┌─────────────────────────────────────────┐
│          Rust Web Layer (Axum)          │
│  - HTTP routing                         │
│  - Authentication                       │
│  - Request/Response handling            │
└─────────────────┬───────────────────────┘
                  │
┌─────────────────▼───────────────────────┐
│      Rust Business Logic Layer          │
│  - Repository management                │
│  - Path validation                      │
│  - Database operations                  │
└─────────────────┬───────────────────────┘
                  │
┌─────────────────▼───────────────────────┐
│         PyO3 Bridge Layer               │
│  - Python interpreter management        │
│  - Breezy API wrapper                   │
│  - Type conversions                     │
└─────────────────┬───────────────────────┘
                  │
┌─────────────────▼───────────────────────┐
│      Python Breezy Library              │
│  - Bazaar protocol implementation       │
│  - Repository operations                │
│  - Smart server handling                │
└─────────────────────────────────────────┘
```

### Key Components to Implement

#### 1. PyO3 Bridge Module (`src/breezy_bridge.rs`)

```rust
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList};
use std::path::Path;

pub struct BreezyBridge {
    py: Python<'static>,
}

impl BreezyBridge {
    pub fn new() -> PyResult<Self> {
        // Initialize Python interpreter
        pyo3::prepare_freethreaded_python();
        let gil = Python::acquire_gil();
        let py = gil.python();
        
        // Import required modules
        py.run("import sys", None, None)?;
        py.run("from breezy.branch import Branch", None, None)?;
        py.run("from breezy.repository import Repository", None, None)?;
        py.run("from breezy.controldir import ControlDir", None, None)?;
        py.run("from breezy.bzr.smart import medium", None, None)?;
        
        Ok(Self { py })
    }
    
    pub fn open_repository(&self, path: &Path) -> PyResult<PyObject> {
        // Open or create repository
    }
    
    pub fn handle_smart_request(&self, request_data: &[u8], 
                               transport: PyObject) -> PyResult<Vec<u8>> {
        // Handle Bazaar smart protocol request
    }
}
```

#### 2. Repository Operations (`src/repository.rs` - enhanced)

```rust
use crate::breezy_bridge::BreezyBridge;

pub struct RepositoryManager {
    bridge: BreezyBridge,
    local_path: PathBuf,
}

impl RepositoryManager {
    pub async fn open_or_create(&self, codebase: &str) -> Result<Repository> {
        // Use PyO3 bridge to open/create repository
    }
    
    pub async fn get_revision_info(&self, repo: &Repository, 
                                  revid: &str) -> Result<RevisionInfo> {
        // Use PyO3 to extract revision information
    }
}
```

#### 3. Smart Protocol Handler (`src/smart_protocol.rs`)

```rust
pub struct SmartProtocolHandler {
    bridge: BreezyBridge,
}

impl SmartProtocolHandler {
    pub async fn handle_request(&self, 
                               request_data: Vec<u8>,
                               repo_path: &Path,
                               allow_writes: bool) -> Result<Vec<u8>> {
        // Use PyO3 to handle smart protocol request
    }
}
```

## Implementation Phases

### Phase 1: PyO3 Foundation (1 week)

1. **Setup PyO3 Integration**
   - Add PyO3 dependencies to Cargo.toml
   - Create Python interpreter initialization
   - Implement basic Breezy module imports
   - Handle Python exceptions in Rust

2. **Basic Repository Operations**
   - Open/create repository via PyO3
   - Check repository existence
   - Get repository information

### Phase 2: Core Functionality (2-3 weeks)

1. **Smart Protocol Handler**
   - Implement request/response bridge
   - Handle transport abstraction
   - Manage readonly vs read-write access
   - Protocol error handling

2. **Diff and Revision APIs**
   - Implement diff generation via Breezy
   - Extract revision information
   - Format responses appropriately

3. **Repository Management**
   - Campaign and role-based paths
   - Shared repository support
   - Remote configuration

### Phase 3: Web Integration (1-2 weeks)

1. **HTTP Endpoints**
   - Smart protocol endpoints
   - Diff and revision info APIs
   - Repository listing
   - Administrative APIs

2. **Authentication & Authorization**
   - Worker authentication
   - Permission checking
   - Access control

### Phase 4: Optimization & Testing (1 week)

1. **Performance Optimization**
   - Python GIL management
   - Request pooling
   - Caching strategies

2. **Testing**
   - Unit tests with mocked Python
   - Integration tests with real repositories
   - Protocol compatibility tests

## Technical Considerations

### Python Runtime Management

1. **Interpreter Lifecycle**
   - Initialize once at startup
   - Manage GIL appropriately
   - Handle cleanup on shutdown

2. **Thread Safety**
   - PyO3 handles GIL automatically
   - Use tokio::task::spawn_blocking for Python calls
   - Avoid holding GIL during async operations

3. **Error Handling**
   - Convert Python exceptions to Rust errors
   - Provide meaningful error messages
   - Handle Breezy-specific exceptions

### Type Conversions

1. **Common Conversions**
   ```rust
   // RevisionId: bytes in Python, String in Rust
   let revid_bytes = PyBytes::new(py, revid.as_bytes());
   
   // Repository paths
   let path_str = path.to_str().ok_or_else(|| /* error */)?;
   
   // JSON responses
   let py_dict = /* Python dict */;
   let json_str: String = py_dict.call_method0("__str__")?.extract()?;
   ```

2. **Complex Objects**
   - Use PyDict for configuration
   - Extract specific fields as needed
   - Avoid holding Python references long-term

### Subprocess Fallback

For some operations, subprocess might still be simpler:

```rust
// For operations like diff that output to stdout
pub async fn bzr_diff_subprocess(repo_path: &Path, 
                                old_revid: &str, 
                                new_revid: &str) -> Result<String> {
    let output = Command::new("brz")
        .args(&["diff", "-r", &format!("revid:{}..revid:{}", old_revid, new_revid)])
        .current_dir(repo_path)
        .output()
        .await?;
    
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
```

## Cargo.toml Dependencies

```toml
[dependencies]
# Web framework
axum = "0.7"
tokio = { version = "1", features = ["full"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["trace", "cors"] }

# PyO3 for Python integration
pyo3 = { version = "0.20", features = ["auto-initialize"] }
pyo3-asyncio = { version = "0.20", features = ["tokio-runtime"] }

# Database
sqlx = { version = "0.7", features = ["postgres", "runtime-tokio-native-tls"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Other utilities
uuid = { version = "1", features = ["v4"] }
base64 = "0.21"
percent-encoding = "2.3"
```

## Example Implementation

### Basic Repository Open

```rust
use pyo3::prelude::*;
use pyo3::types::PyDict;

pub fn open_repository(py: Python, path: &str) -> PyResult<PyObject> {
    let locals = PyDict::new(py);
    locals.set_item("path", path)?;
    
    py.run(
        r#"
from breezy.repository import Repository
from breezy.controldir import ControlDir
from breezy.errors import NotBranchError

try:
    repo = Repository.open(path)
except NotBranchError:
    controldir = ControlDir.create(path)
    repo = controldir.create_repository(shared=True)
        "#,
        None,
        Some(locals),
    )?;
    
    locals.get_item("repo").unwrap().extract()
}
```

### Smart Protocol Request Handler

```rust
pub async fn handle_smart_request(
    request_data: Vec<u8>,
    repo_path: PathBuf,
    allow_writes: bool,
) -> Result<Vec<u8>> {
    // Move to blocking task for Python operations
    tokio::task::spawn_blocking(move || {
        Python::with_gil(|py| -> PyResult<Vec<u8>> {
            let locals = PyDict::new(py);
            locals.set_item("request_data", PyBytes::new(py, &request_data))?;
            locals.set_item("repo_path", repo_path.to_str().unwrap())?;
            locals.set_item("allow_writes", allow_writes)?;
            
            py.run(
                r#"
from breezy.repository import Repository
from breezy.bzr.smart import medium
from breezy.transport import get_transport_from_url
from io import BytesIO

repo = Repository.open(repo_path)
transport = repo.user_transport

if not allow_writes:
    transport = get_transport_from_url("readonly+" + transport.base)

out_buffer = BytesIO()
protocol_factory, unused_bytes = medium._get_protocol_factory_for_bytes(request_data)
smart_request = protocol_factory(transport, out_buffer.write, ".", jail_root=repo.user_transport)
smart_request.accept_bytes(unused_bytes)

response_data = out_buffer.getvalue()
                "#,
                None,
                Some(locals),
            )?;
            
            let response_bytes = locals
                .get_item("response_data")
                .unwrap()
                .extract::<&PyBytes>()?;
            Ok(response_bytes.as_bytes().to_vec())
        })
    })
    .await?
    .map_err(|e| anyhow::anyhow!("Python error: {}", e))
}
```

## Migration Path

1. **Start with Subprocess** (Quick MVP)
   - Implement basic operations using `brz` commands
   - Get service structure working
   - Validate approach

2. **Add PyO3 for Complex Operations** (Incremental)
   - Smart protocol handling
   - Repository management
   - Keep subprocess for simple operations

3. **Optimize Critical Paths** (Performance)
   - Profile to find bottlenecks
   - Cache Python objects where appropriate
   - Consider native Rust for hot paths

## Risks and Mitigations

### Risk: Python GIL Performance
- **Mitigation**: Use spawn_blocking for Python operations
- **Mitigation**: Batch operations where possible
- **Mitigation**: Cache frequently accessed data

### Risk: Python Dependency Management
- **Mitigation**: Pin Breezy version
- **Mitigation**: Include in Docker image
- **Mitigation**: Document installation requirements

### Risk: Debugging Complexity
- **Mitigation**: Comprehensive logging
- **Mitigation**: Clear error propagation
- **Mitigation**: Integration test suite

## Conclusion

Using PyO3 to integrate Breezy provides the best balance of:
- **Functionality**: Full Bazaar protocol support
- **Maintainability**: Leverage existing, tested code
- **Performance**: Better than pure subprocess
- **Migration Path**: Can optimize incrementally

This approach allows us to deliver a working BZR Store service quickly while keeping the door open for future optimizations.