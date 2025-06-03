# PyO3 Implementation Plan for BZR Store

## Implementation Roadmap

### Stage 1: Hybrid Subprocess/PyO3 MVP (1-2 weeks)

Start with a hybrid approach that uses subprocess for simple operations and PyO3 for complex protocol handling.

#### 1.1 Basic Infrastructure

**Files to create:**
- `src/main.rs` - Service entry point (similar to git-store)
- `src/config.rs` - Configuration management
- `src/error.rs` - Error types and handling
- `src/web.rs` - Axum web server setup
- `src/repository.rs` - Repository management (subprocess-based initially)
- `src/subprocess.rs` - Subprocess utilities for brz commands

**Initial Cargo.toml:**
```toml
[package]
name = "bzr-store"
version = "0.1.0"
edition.workspace = true

[[bin]]
name = "bzr-store"

[dependencies]
# Web framework
axum = { version = "0.7", features = ["macros"] }
tokio = { version = "1", features = ["full"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["trace", "cors", "normalize-path"] }

# Database
sqlx = { version = "0.7", features = ["postgres", "runtime-tokio-native-tls"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Templates
tera = "1.19"

# HTTP/Auth
base64 = "0.21"
mime = "0.3"
percent-encoding = "2.3"

# PyO3 - commented out initially
# pyo3 = { version = "0.20", features = ["auto-initialize", "abi3-py38"] }
# pyo3-asyncio = { version = "0.20", features = ["tokio-runtime"] }
```

#### 1.2 Subprocess Implementation

Create subprocess wrapper for basic Bazaar operations:

```rust
// src/subprocess.rs
use std::path::Path;
use tokio::process::Command;
use anyhow::Result;

pub struct BrzCommand;

impl BrzCommand {
    pub async fn init_shared_repo(path: &Path) -> Result<()> {
        let output = Command::new("brz")
            .args(&["init-repo", "--no-trees"])
            .current_dir(path)
            .output()
            .await?;
        
        if !output.status.success() {
            anyhow::bail!("Failed to init repository: {}", 
                         String::from_utf8_lossy(&output.stderr));
        }
        Ok(())
    }
    
    pub async fn diff(repo_path: &Path, old_revid: &str, new_revid: &str) -> Result<Vec<u8>> {
        let output = Command::new("brz")
            .args(&["diff", "-r", &format!("revid:{}..revid:{}", old_revid, new_revid)])
            .current_dir(repo_path)
            .output()
            .await?;
        
        if output.status.code() == Some(3) {
            // Exit code 3 means no differences
            Ok(output.stdout)
        } else if output.status.success() {
            Ok(output.stdout)
        } else {
            anyhow::bail!("brz diff failed: {}", String::from_utf8_lossy(&output.stderr))
        }
    }
    
    pub async fn check_repository_exists(path: &Path) -> Result<bool> {
        let output = Command::new("brz")
            .args(&["info"])
            .current_dir(path)
            .output()
            .await?;
        
        Ok(output.status.success())
    }
}
```

### Stage 2: PyO3 Smart Protocol Integration (2-3 weeks)

#### 2.1 PyO3 Bridge Setup

**Create `src/python_bridge/mod.rs`:**
```rust
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct PythonBridge {
    initialized: Arc<Mutex<bool>>,
}

impl PythonBridge {
    pub fn new() -> Result<Self, PyErr> {
        Ok(Self {
            initialized: Arc::new(Mutex::new(false)),
        })
    }
    
    pub async fn ensure_initialized(&self) -> Result<(), PyErr> {
        let mut initialized = self.initialized.lock().await;
        if !*initialized {
            Python::with_gil(|py| {
                // Import required modules
                py.import("breezy.branch")?;
                py.import("breezy.repository")?;
                py.import("breezy.controldir")?;
                py.import("breezy.bzr.smart.medium")?;
                py.import("breezy.transport")?;
                Ok::<(), PyErr>(())
            })?;
            *initialized = true;
        }
        Ok(())
    }
}
```

#### 2.2 Smart Protocol Handler

**Create `src/smart_protocol.rs`:**
```rust
use crate::python_bridge::PythonBridge;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};
use std::path::Path;

pub struct SmartProtocolHandler {
    bridge: Arc<PythonBridge>,
}

impl SmartProtocolHandler {
    pub fn new(bridge: Arc<PythonBridge>) -> Self {
        Self { bridge }
    }
    
    pub async fn handle_request(
        &self,
        request_data: Vec<u8>,
        repo_path: &Path,
        campaign: Option<&str>,
        role: Option<&str>,
        allow_writes: bool,
    ) -> Result<Vec<u8>, anyhow::Error> {
        self.bridge.ensure_initialized().await?;
        
        let repo_path = repo_path.to_owned();
        let campaign = campaign.map(|s| s.to_string());
        let role = role.map(|s| s.to_string());
        
        // Run in blocking task for Python GIL
        tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| -> PyResult<Vec<u8>> {
                let locals = PyDict::new(py);
                
                // Set up variables
                locals.set_item("request_data", PyBytes::new(py, &request_data))?;
                locals.set_item("repo_path", repo_path.to_str().unwrap())?;
                locals.set_item("campaign", campaign)?;
                locals.set_item("role", role)?;
                locals.set_item("allow_writes", allow_writes)?;
                
                // Execute smart protocol handling
                py.run(SMART_PROTOCOL_HANDLER_CODE, None, Some(locals))?;
                
                // Get response
                let response_bytes = locals
                    .get_item("response_data")
                    .unwrap()
                    .extract::<&PyBytes>()?;
                Ok(response_bytes.as_bytes().to_vec())
            })
        })
        .await?
        .map_err(|e: PyErr| anyhow::anyhow!("Python error: {}", e))
    }
}

const SMART_PROTOCOL_HANDLER_CODE: &str = r#"
import os
from io import BytesIO
from breezy.repository import Repository
from breezy.controldir import ControlDir
from breezy.bzr.smart import medium
from breezy.transport import get_transport_from_url
from breezy.errors import NotBranchError

# Open or create repository
try:
    repo = Repository.open(repo_path)
except NotBranchError:
    controldir = ControlDir.create(repo_path)
    repo = controldir.create_repository(shared=True)

# Set up transport with campaign/role paths
transport = repo.user_transport
if campaign:
    transport = transport.clone(campaign)
    if allow_writes:
        transport.ensure_base()
if role:
    transport = transport.clone(role)
    if allow_writes:
        transport.ensure_base()

# Apply write restrictions
if not allow_writes:
    transport = get_transport_from_url("readonly+" + transport.base)

# Handle smart protocol request
out_buffer = BytesIO()
protocol_factory, unused_bytes = medium._get_protocol_factory_for_bytes(request_data)
smart_request = protocol_factory(
    transport, 
    out_buffer.write, 
    ".", 
    jail_root=repo.user_transport
)
smart_request.accept_bytes(unused_bytes)

# Get response
response_data = out_buffer.getvalue()
"#;
```

### Stage 3: Full Integration (1-2 weeks)

#### 3.1 Enhanced Repository Operations

**Update `src/repository.rs`:**
```rust
use crate::python_bridge::PythonBridge;
use crate::subprocess::BrzCommand;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct RepositoryManager {
    local_path: PathBuf,
    bridge: Option<Arc<PythonBridge>>,
}

impl RepositoryManager {
    pub fn new(local_path: PathBuf, bridge: Option<Arc<PythonBridge>>) -> Self {
        Self { local_path, bridge }
    }
    
    pub async fn open_or_create(&self, codebase: &str) -> Result<(), anyhow::Error> {
        let repo_path = self.repo_path(codebase);
        
        if !repo_path.exists() {
            std::fs::create_dir_all(&repo_path)?;
        }
        
        // Check if repository exists
        if !BrzCommand::check_repository_exists(&repo_path).await? {
            // Create shared repository
            BrzCommand::init_shared_repo(&repo_path).await?;
        }
        
        Ok(())
    }
    
    pub fn repo_path(&self, codebase: &str) -> PathBuf {
        self.local_path.join(codebase)
    }
    
    pub async fn get_revision_info(
        &self,
        codebase: &str,
        revid: &str,
    ) -> Result<serde_json::Value, anyhow::Error> {
        if let Some(bridge) = &self.bridge {
            // Use PyO3 for complex operations
            self.get_revision_info_pyo3(codebase, revid).await
        } else {
            // Fallback to subprocess
            self.get_revision_info_subprocess(codebase, revid).await
        }
    }
    
    async fn get_revision_info_subprocess(
        &self,
        codebase: &str,
        revid: &str,
    ) -> Result<serde_json::Value, anyhow::Error> {
        // Implementation using brz log command
        todo!()
    }
    
    async fn get_revision_info_pyo3(
        &self,
        codebase: &str,
        revid: &str,
    ) -> Result<serde_json::Value, anyhow::Error> {
        // Implementation using PyO3
        todo!()
    }
}
```

#### 3.2 Web Endpoints

**Create comprehensive web routes in `src/web.rs`:**
```rust
use axum::{
    Router,
    routing::{get, post},
    extract::{Path, Query, State},
    response::{Response, IntoResponse},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub repo_manager: Arc<RepositoryManager>,
    pub db_manager: Arc<DatabaseManager>,
    pub smart_handler: Option<Arc<SmartProtocolHandler>>,
    pub config: Arc<Config>,
}

pub fn create_admin_app(state: AppState) -> Router {
    Router::new()
        .route("/", get(handle_repo_list))
        .route("/health", get(handle_health))
        .route("/ready", get(handle_ready))
        .route("/:codebase/diff", get(handle_diff))
        .route("/:codebase/revision-info", get(handle_revision_info))
        .route("/:codebase/.bzr/smart", post(handle_smart_protocol))
        .route("/:codebase/:campaign/.bzr/smart", post(handle_smart_protocol))
        .route("/:codebase/:campaign/:role/.bzr/smart", post(handle_smart_protocol))
        .route("/:codebase/remotes/:remote", post(handle_set_remote))
        .with_state(state)
}

pub fn create_public_app(state: AppState) -> Router {
    Router::new()
        .route("/", get(handle_home))
        .route("/bzr/", get(handle_repo_list))
        .route("/bzr/:codebase/.bzr/smart", post(handle_smart_protocol))
        .route("/bzr/:codebase/:campaign/.bzr/smart", post(handle_smart_protocol))
        .route("/bzr/:codebase/:campaign/:role/.bzr/smart", post(handle_smart_protocol))
        .with_state(state)
}

// Endpoint implementations...
```

### Stage 4: Testing and Optimization (1 week)

#### 4.1 Integration Tests

**Create `tests/integration_test.rs`:**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_repository_creation() {
        let temp_dir = TempDir::new().unwrap();
        let repo_manager = RepositoryManager::new(temp_dir.path().to_owned(), None);
        
        repo_manager.open_or_create("test-repo").await.unwrap();
        
        // Verify repository exists
        assert!(repo_manager.repo_path("test-repo").exists());
    }
    
    #[tokio::test]
    async fn test_smart_protocol_read() {
        // Test smart protocol with read-only access
    }
    
    #[tokio::test]
    async fn test_smart_protocol_write() {
        // Test smart protocol with write access
    }
}
```

#### 4.2 Performance Optimization

1. **Connection Pooling**: Reuse Python interpreter instances
2. **Caching**: Cache repository objects in Python memory
3. **Async Optimization**: Properly handle blocking Python calls
4. **Monitoring**: Add performance metrics

## Deployment Considerations

### Docker Image

```dockerfile
FROM rust:1.75 as builder

WORKDIR /app
COPY . .
RUN cargo build --release --bin bzr-store

FROM debian:bookworm-slim

# Install Python and Breezy
RUN apt-get update && apt-get install -y \
    python3 \
    python3-pip \
    python3-dev \
    && pip3 install breezy \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/bzr-store /usr/local/bin/

EXPOSE 9929 9930
CMD ["bzr-store"]
```

### Environment Variables

```bash
# Database connection
DATABASE_URL=postgresql://user:pass@localhost/janitor

# Repository storage
BZR_STORE_PATH=/var/lib/janitor/bzr-repos

# Server configuration
BZR_STORE_HOST=0.0.0.0
BZR_STORE_ADMIN_PORT=9929
BZR_STORE_PUBLIC_PORT=9930

# Python optimization
PYTHONUNBUFFERED=1
PYTHONDONTWRITEBYTECODE=1
```

## Risk Mitigation Strategies

### 1. Gradual Migration
- Start with subprocess for MVP
- Add PyO3 for performance-critical paths
- Keep subprocess as fallback

### 2. Testing Strategy
- Unit tests for Rust components
- Integration tests with real Bazaar repositories
- Protocol compatibility tests with bzr/brz clients
- Performance benchmarks

### 3. Monitoring
- Request latency metrics
- Python GIL contention monitoring
- Repository operation performance
- Error rates and types

## Success Criteria

1. **Functional Parity**: All Python bzr_store.py features work
2. **Performance**: Smart protocol latency < 200ms for typical operations
3. **Compatibility**: Works with standard bzr/brz clients
4. **Reliability**: 99.9% uptime, graceful error handling
5. **Maintainability**: Clear separation between Rust and Python code

## Timeline

- **Week 1-2**: Subprocess MVP with basic functionality
- **Week 3-4**: PyO3 smart protocol integration
- **Week 5**: Enhanced repository operations
- **Week 6**: Testing and optimization
- **Week 7**: Production deployment preparation

This approach provides a working service quickly while building towards a performant, maintainable solution.