# Database State Management Strategy Guide

> **Status**: 📋 **PLANNING** - Comprehensive strategy for shared database state management across services.
> 
> 📋 **Master Plan**: See [`porting-plan.md`](porting-plan.md) for overall project coordination and dependencies.

## Overview

This document provides a comprehensive strategy for migrating the Janitor platform's database state management from Python to Rust while maintaining consistency, performance, and reliability across all services. The state management layer is critical infrastructure that all services depend on for database operations, type safety, and data consistency.

### Current State Analysis

**Python Implementation (`py/janitor/state.py`)**: 268 lines
- **Core Run Model**: Central data structure for job execution tracking
- **Database Connection Management**: asyncpg pool configuration and management
- **Type Codecs**: Custom PostgreSQL type handling (JSON, debversion)
- **Result Branch Logic**: Complex branch result management
- **Query Utilities**: Common database operations and utilities
- **Error Handling**: Database-specific error middleware and metrics

**Rust Implementation**: Currently **scattered and incomplete**
- `src/state.rs` (root crate): Basic types and some functionality (~200 lines)
- Individual crates have their own state management (inconsistent)
- Missing: Unified state management, type safety, connection pooling

## Technical Architecture Analysis

### Current Python State Management

#### 1. Core Data Models
```python
class Run:
    """Central execution tracking model with complex state logic"""
    def __init__(self, id, codebase, campaign, start_time, finish_time, 
                 result_code, description, context, value, result, 
                 logfilenames, result_branches, instigated_context):
        # Complex initialization with validation
        
    def duration(self) -> datetime.timedelta:
        # Duration calculation with edge case handling
        
    def get_result_branch(self, role):
        # Branch resolution logic
```

#### 2. Database Connection Management
```python
def create_pool(uri, *args, **kwargs) -> asyncpg.pool.Pool:
    return asyncpg.create_pool(
        uri, init=init_types, *args, **kwargs
    )

async def init_types(conn):
    # Custom PostgreSQL type registration
    await conn.set_type_codec("json", encoder=json.dumps, decoder=json.loads)
    await conn.set_type_codec("debversion", encoder=str, decoder=Version)
```

#### 3. Query Utilities
```python
async def iter_publishable_suites(conn: asyncpg.Connection, codebase: str) -> list[str]:
    # Complex query with business logic
    
async def has_cotenants(conn: asyncpg.Connection, codebase: str, tenant: str) -> bool:
    # Tenant validation logic
```

### Target Rust Architecture

#### Unified State Management Crate (`janitor-state/`)
```rust
pub struct StateManager {
    db_pool: PgPool,
    config: DatabaseConfig,
    metrics: DatabaseMetrics,
    type_registry: TypeRegistry,
}

pub struct Run {
    pub id: Uuid,
    pub codebase: String,
    pub campaign: String,
    pub start_time: DateTime<Utc>,
    pub finish_time: Option<DateTime<Utc>>,
    pub result_code: Option<String>,
    pub description: String,
    pub context: Option<String>,
    pub value: Option<f64>,
    pub result: Option<serde_json::Value>,
    pub logfilenames: Vec<String>,
    pub result_branches: Option<serde_json::Value>,
    pub instigated_context: Option<String>,
}

impl Run {
    pub fn duration(&self) -> Option<Duration> {
        self.finish_time.map(|finish| {
            (finish - self.start_time).to_std().unwrap_or(Duration::ZERO)
        })
    }
    
    pub fn get_result_branch(&self, role: &str) -> Option<ResultBranch> {
        // Safe branch resolution with proper error handling
    }
}
```

## Migration Strategy

### Phase 1: Core Type System Migration (2-3 weeks)

#### 1.1 Database Model Definitions
**Target**: Create comprehensive, type-safe Rust models

```rust
// Core database models with full sqlx integration
#[derive(sqlx::FromRow, Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub id: Uuid,
    pub codebase: String,
    pub campaign: String,
    pub start_time: DateTime<Utc>,
    pub finish_time: Option<DateTime<Utc>>,
    pub result_code: Option<String>,
    pub description: String,
    pub context: Option<String>,
    pub value: Option<f64>,
    pub result: Option<sqlx::types::Json<serde_json::Value>>,
    pub logfilenames: Vec<String>,
    pub result_branches: Option<sqlx::types::Json<serde_json::Value>>,
    pub instigated_context: Option<String>,
}

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct Codebase {
    pub name: String,
    pub branch_url: String,
    pub vcs_type: VcsType,
    pub description: Option<String>,
    pub last_processed: Option<DateTime<Utc>>,
}

#[derive(sqlx::Type, Debug, Clone, PartialEq, Eq)]
#[sqlx(type_name = "vcs_type", rename_all = "lowercase")]
pub enum VcsType {
    Git,
    Bzr,
    Hg,
}

#[derive(sqlx::Type, Debug, Clone, PartialEq, Eq)]
#[sqlx(type_name = "publish_status", rename_all = "kebab-case")]
pub enum PublishStatus {
    Success,
    Failed,
    NotAttempted,
    InProgress,
}
```

#### 1.2 Custom Type Support
**Target**: Handle PostgreSQL custom types safely

```rust
// Custom Debian version type support
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DebianVersion(pub String);

impl sqlx::Type<sqlx::Postgres> for DebianVersion {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("debversion")
    }
}

impl sqlx::Encode<'_, sqlx::Postgres> for DebianVersion {
    fn encode_by_ref(&self, buf: &mut sqlx::postgres::PgArgumentBuffer) -> sqlx::encode::IsNull {
        <String as sqlx::Encode<sqlx::Postgres>>::encode_by_ref(&self.0, buf)
    }
}

impl sqlx::Decode<'_, sqlx::Postgres> for DebianVersion {
    fn decode(value: sqlx::postgres::PgValueRef<'_>) -> Result<Self, sqlx::error::BoxDynError> {
        let s = <String as sqlx::Decode<sqlx::Postgres>>::decode(value)?;
        Ok(DebianVersion(s))
    }
}

// JSON type handling with validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultBranches(pub HashMap<String, ResultBranch>);

impl ResultBranches {
    pub fn get_branch(&self, role: &str) -> Option<&ResultBranch> {
        self.0.get(role)
    }
    
    pub fn validate(&self) -> Result<(), ValidationError> {
        for (role, branch) in &self.0 {
            branch.validate(role)?;
        }
        Ok(())
    }
}
```

#### 1.3 Database Connection Management
**Target**: Robust, high-performance connection pooling

```rust
pub struct DatabaseManager {
    pool: PgPool,
    config: DatabaseConfig,
    metrics: DatabaseMetrics,
}

impl DatabaseManager {
    pub async fn new(config: DatabaseConfig) -> Result<Self, DatabaseError> {
        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .idle_timeout(Duration::from_secs(config.idle_timeout_seconds))
            .max_lifetime(Duration::from_secs(config.max_lifetime_seconds))
            .connect_with(config.connection_options())
            .await?;
        
        // Initialize custom types
        Self::init_custom_types(&pool).await?;
        
        Ok(Self {
            pool,
            config,
            metrics: DatabaseMetrics::new(),
        })
    }
    
    async fn init_custom_types(pool: &PgPool) -> Result<(), DatabaseError> {
        // Verify custom types exist
        let custom_types = sqlx::query!(
            "SELECT typname FROM pg_type WHERE typname IN ('debversion', 'vcs_type', 'publish_status')"
        )
        .fetch_all(pool)
        .await?;
        
        if custom_types.len() != 3 {
            return Err(DatabaseError::MissingCustomTypes);
        }
        
        Ok(())
    }
    
    pub fn get_pool(&self) -> &PgPool {
        &self.pool
    }
    
    pub async fn health_check(&self) -> Result<DatabaseHealth, DatabaseError> {
        let start = Instant::now();
        
        let result = sqlx::query!("SELECT 1 as health_check")
            .fetch_one(&self.pool)
            .await;
            
        let latency = start.elapsed();
        
        match result {
            Ok(_) => Ok(DatabaseHealth {
                status: HealthStatus::Healthy,
                latency,
                active_connections: self.pool.size() as u32,
                idle_connections: self.pool.num_idle(),
            }),
            Err(e) => Ok(DatabaseHealth {
                status: HealthStatus::Unhealthy(e.to_string()),
                latency,
                active_connections: 0,
                idle_connections: 0,
            }),
        }
    }
}
```

### Phase 2: Query Interface Standardization (2-3 weeks)

#### 2.1 Repository Pattern Implementation
**Target**: Consistent data access patterns across services

```rust
#[async_trait]
pub trait RunRepository: Send + Sync {
    async fn get_run(&self, id: Uuid) -> Result<Option<Run>, DatabaseError>;
    async fn create_run(&self, run: NewRun) -> Result<Run, DatabaseError>;
    async fn update_run(&self, id: Uuid, updates: RunUpdates) -> Result<Run, DatabaseError>;
    async fn delete_run(&self, id: Uuid) -> Result<(), DatabaseError>;
    
    async fn list_runs(
        &self,
        filter: RunFilter,
        pagination: Pagination,
    ) -> Result<Vec<Run>, DatabaseError>;
    
    async fn count_runs(&self, filter: RunFilter) -> Result<i64, DatabaseError>;
}

pub struct PostgresRunRepository {
    pool: PgPool,
    metrics: RepositoryMetrics,
}

impl PostgresRunRepository {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            metrics: RepositoryMetrics::new("run_repository"),
        }
    }
}

#[async_trait]
impl RunRepository for PostgresRunRepository {
    async fn get_run(&self, id: Uuid) -> Result<Option<Run>, DatabaseError> {
        let start = Instant::now();
        
        let result = sqlx::query_as!(
            Run,
            r#"
            SELECT 
                id, codebase, campaign, start_time, finish_time,
                result_code, description, context, value,
                result as "result: Json<serde_json::Value>",
                logfilenames,
                result_branches as "result_branches: Json<serde_json::Value>",
                instigated_context
            FROM run 
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await;
        
        self.metrics.query_duration.observe(start.elapsed().as_secs_f64());
        
        match result {
            Ok(run) => {
                self.metrics.queries_successful.inc();
                Ok(run)
            }
            Err(e) => {
                self.metrics.queries_failed.inc();
                Err(DatabaseError::from(e))
            }
        }
    }
    
    async fn create_run(&self, new_run: NewRun) -> Result<Run, DatabaseError> {
        let start = Instant::now();
        
        let result = sqlx::query_as!(
            Run,
            r#"
            INSERT INTO run (
                id, codebase, campaign, start_time, description,
                context, value, instigated_context
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8
            )
            RETURNING 
                id, codebase, campaign, start_time, finish_time,
                result_code, description, context, value,
                result as "result: Json<serde_json::Value>",
                logfilenames,
                result_branches as "result_branches: Json<serde_json::Value>",
                instigated_context
            "#,
            new_run.id,
            new_run.codebase,
            new_run.campaign,
            new_run.start_time,
            new_run.description,
            new_run.context,
            new_run.value,
            new_run.instigated_context
        )
        .fetch_one(&self.pool)
        .await;
        
        self.metrics.query_duration.observe(start.elapsed().as_secs_f64());
        
        match result {
            Ok(run) => {
                self.metrics.queries_successful.inc();
                info!(run_id = %run.id, "Created new run");
                Ok(run)
            }
            Err(e) => {
                self.metrics.queries_failed.inc();
                error!(error = %e, "Failed to create run");
                Err(DatabaseError::from(e))
            }
        }
    }
}
```

#### 2.2 Business Logic Layer
**Target**: Port complex business logic with proper abstractions

```rust
pub struct StateService {
    run_repo: Arc<dyn RunRepository>,
    codebase_repo: Arc<dyn CodebaseRepository>,
    metrics: StateServiceMetrics,
}

impl StateService {
    pub async fn iter_publishable_suites(
        &self,
        codebase: &str,
    ) -> Result<Vec<String>, StateError> {
        // Port complex business logic from Python
        let runs = self.run_repo
            .list_runs(
                RunFilter::builder()
                    .codebase(codebase)
                    .result_code_not_null()
                    .build(),
                Pagination::all(),
            )
            .await?;
        
        let mut publishable_suites = HashSet::new();
        
        for run in runs {
            if let Some(result_branches) = run.result_branches {
                if let Ok(branches) = serde_json::from_value::<ResultBranches>(result_branches.0) {
                    for (role, branch) in branches.0 {
                        if self.is_branch_publishable(&branch).await? {
                            publishable_suites.insert(role);
                        }
                    }
                }
            }
        }
        
        Ok(publishable_suites.into_iter().collect())
    }
    
    async fn is_branch_publishable(&self, branch: &ResultBranch) -> Result<bool, StateError> {
        // Complex business logic for determining publishability
        match &branch.status {
            BranchStatus::Success => Ok(true),
            BranchStatus::Failed => Ok(false),
            BranchStatus::Pending => {
                // Check if dependencies are satisfied
                self.check_branch_dependencies(branch).await
            }
        }
    }
    
    pub async fn has_cotenants(
        &self,
        codebase: &str,
        tenant: &str,
    ) -> Result<bool, StateError> {
        let count = sqlx::query!(
            "SELECT COUNT(*) as count FROM run WHERE codebase = $1 AND campaign != $2",
            codebase,
            tenant
        )
        .fetch_one(self.run_repo.get_pool())
        .await?;
        
        Ok(count.count.unwrap_or(0) > 0)
    }
}
```

### Phase 3: Error Handling and Middleware (1-2 weeks)

#### 3.1 Comprehensive Error Handling
**Target**: Robust error handling with proper categorization

```rust
#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("Connection error: {0}")]
    Connection(#[from] sqlx::Error),
    
    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),
    
    #[error("Custom type error: {0}")]
    CustomType(String),
    
    #[error("Missing custom types in database")]
    MissingCustomTypes,
    
    #[error("Transaction failed: {0}")]
    Transaction(String),
    
    #[error("Constraint violation: {0}")]
    ConstraintViolation(String),
    
    #[error("Resource exhaustion: {0}")]
    ResourceExhaustion(String),
}

impl DatabaseError {
    pub fn is_retryable(&self) -> bool {
        match self {
            DatabaseError::Connection(_) => true,
            DatabaseError::ResourceExhaustion(_) => true,
            DatabaseError::Transaction(_) => true,
            _ => false,
        }
    }
    
    pub fn should_alert(&self) -> bool {
        match self {
            DatabaseError::MissingCustomTypes => true,
            DatabaseError::ResourceExhaustion(_) => true,
            _ => false,
        }
    }
}

// Middleware for automatic error handling
pub async fn database_error_middleware<T>(
    operation: impl Future<Output = Result<T, DatabaseError>>,
    context: &str,
) -> Result<T, DatabaseError> {
    let start = Instant::now();
    let result = operation.await;
    let duration = start.elapsed();
    
    match &result {
        Ok(_) => {
            debug!(
                context = context,
                duration_ms = duration.as_millis(),
                "Database operation successful"
            );
        }
        Err(e) => {
            if e.should_alert() {
                error!(
                    context = context,
                    error = %e,
                    duration_ms = duration.as_millis(),
                    "Critical database error"
                );
            } else if e.is_retryable() {
                warn!(
                    context = context,
                    error = %e,
                    duration_ms = duration.as_millis(),
                    "Retryable database error"
                );
            } else {
                info!(
                    context = context,
                    error = %e,
                    duration_ms = duration.as_millis(),
                    "Database operation failed"
                );
            }
        }
    }
    
    result
}
```

#### 3.2 Metrics and Monitoring
**Target**: Comprehensive database operation monitoring

```rust
pub struct DatabaseMetrics {
    pub connection_pool_size: Gauge,
    pub active_connections: Gauge,
    pub idle_connections: Gauge,
    pub query_duration: Histogram,
    pub queries_total: Counter,
    pub queries_failed: Counter,
    pub transactions_total: Counter,
    pub transactions_failed: Counter,
    pub custom_type_operations: Counter,
}

impl DatabaseMetrics {
    pub fn new() -> Self {
        Self {
            connection_pool_size: register_gauge!(
                "janitor_db_connection_pool_size",
                "Current connection pool size"
            ).unwrap(),
            active_connections: register_gauge!(
                "janitor_db_active_connections",
                "Number of active database connections"
            ).unwrap(),
            idle_connections: register_gauge!(
                "janitor_db_idle_connections", 
                "Number of idle database connections"
            ).unwrap(),
            query_duration: register_histogram!(
                "janitor_db_query_duration_seconds",
                "Database query execution time"
            ).unwrap(),
            queries_total: register_counter!(
                "janitor_db_queries_total",
                "Total number of database queries"
            ).unwrap(),
            queries_failed: register_counter!(
                "janitor_db_queries_failed",
                "Number of failed database queries"
            ).unwrap(),
            transactions_total: register_counter!(
                "janitor_db_transactions_total",
                "Total number of database transactions"
            ).unwrap(),
            transactions_failed: register_counter!(
                "janitor_db_transactions_failed",
                "Number of failed database transactions"
            ).unwrap(),
            custom_type_operations: register_counter!(
                "janitor_db_custom_type_operations",
                "Custom type encode/decode operations"
            ).unwrap(),
        }
    }
    
    pub fn update_connection_stats(&self, pool: &PgPool) {
        self.connection_pool_size.set(pool.size() as f64);
        self.active_connections.set((pool.size() - pool.num_idle()) as f64);
        self.idle_connections.set(pool.num_idle() as f64);
    }
}
```

### Phase 4: Service Integration (2-3 weeks)

#### 4.1 Shared State Crate Architecture
**Target**: Centralized state management for all services

```rust
// Main state management crate
pub struct JanitorState {
    database: DatabaseManager,
    services: StateServices,
    config: StateConfig,
}

pub struct StateServices {
    pub runs: Arc<dyn RunRepository>,
    pub codebases: Arc<dyn CodebaseRepository>,
    pub candidates: Arc<dyn CandidateRepository>,
    pub queue_items: Arc<dyn QueueItemRepository>,
}

impl JanitorState {
    pub async fn new(config: StateConfig) -> Result<Self, StateError> {
        let database = DatabaseManager::new(config.database.clone()).await?;
        let pool = database.get_pool().clone();
        
        let services = StateServices {
            runs: Arc::new(PostgresRunRepository::new(pool.clone())),
            codebases: Arc::new(PostgresCodebaseRepository::new(pool.clone())),
            candidates: Arc::new(PostgresCandidateRepository::new(pool.clone())),
            queue_items: Arc::new(PostgresQueueItemRepository::new(pool.clone())),
        };
        
        Ok(Self {
            database,
            services,
            config,
        })
    }
    
    pub async fn health_check(&self) -> Result<HealthStatus, StateError> {
        let db_health = self.database.health_check().await?;
        
        // Test each repository
        let mut checks = Vec::new();
        
        checks.push(self.test_repository_health("runs", &self.services.runs).await);
        checks.push(self.test_repository_health("codebases", &self.services.codebases).await);
        checks.push(self.test_repository_health("candidates", &self.services.candidates).await);
        checks.push(self.test_repository_health("queue_items", &self.services.queue_items).await);
        
        let all_healthy = checks.iter().all(|check| check.is_ok());
        
        if all_healthy {
            Ok(HealthStatus::Healthy)
        } else {
            Ok(HealthStatus::Degraded(checks))
        }
    }
}

// Integration with service crates
impl RunnerState {
    pub fn new(janitor_state: Arc<JanitorState>) -> Self {
        Self {
            runs: janitor_state.services.runs.clone(),
            queue_items: janitor_state.services.queue_items.clone(),
            // Service-specific state
        }
    }
}
```

#### 4.2 Migration Compatibility Layer
**Target**: Gradual migration support with Python compatibility

```rust
pub struct StateMigrationManager {
    rust_state: Arc<JanitorState>,
    python_fallback: Option<PythonStateInterface>,
    config: MigrationConfig,
}

impl StateMigrationManager {
    pub async fn get_run(&self, id: Uuid) -> Result<Option<Run>, StateError> {
        match self.config.mode {
            MigrationMode::RustOnly => {
                self.rust_state.services.runs.get_run(id).await
            }
            MigrationMode::PythonFallback => {
                match self.rust_state.services.runs.get_run(id).await {
                    Ok(run) => Ok(run),
                    Err(e) => {
                        warn!("Rust state access failed, falling back to Python: {}", e);
                        self.python_fallback
                            .as_ref()
                            .unwrap()
                            .get_run(id)
                            .await
                    }
                }
            }
            MigrationMode::ShadowMode => {
                let rust_result = self.rust_state.services.runs.get_run(id).await;
                let python_result = self.python_fallback
                    .as_ref()
                    .unwrap()
                    .get_run(id)
                    .await;
                    
                self.compare_run_results(&rust_result, &python_result).await;
                python_result // Use Python results in shadow mode
            }
        }
    }
    
    async fn compare_run_results(
        &self,
        rust_result: &Result<Option<Run>, StateError>,
        python_result: &Result<Option<Run>, StateError>,
    ) {
        match (rust_result, python_result) {
            (Ok(Some(rust_run)), Ok(Some(python_run))) => {
                if rust_run != python_run {
                    warn!(
                        run_id = %rust_run.id,
                        "Run data mismatch between Rust and Python implementations"
                    );
                    self.log_run_differences(rust_run, python_run).await;
                }
            }
            (Ok(None), Ok(None)) => {
                // Both returned None - expected
            }
            (rust_res, python_res) => {
                warn!(
                    rust_result = ?rust_res,
                    python_result = ?python_res,
                    "Result type mismatch between Rust and Python implementations"
                );
            }
        }
    }
}
```

## Testing Strategy

### Unit Testing
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::testing::TestPool;
    
    #[sqlx::test]
    async fn test_run_creation_and_retrieval(pool: TestPool) {
        let repo = PostgresRunRepository::new(pool.clone());
        
        let new_run = NewRun {
            id: Uuid::new_v4(),
            codebase: "test-codebase".to_string(),
            campaign: "test-campaign".to_string(),
            start_time: Utc::now(),
            description: "Test run".to_string(),
            context: None,
            value: Some(100.0),
            instigated_context: None,
        };
        
        let created_run = repo.create_run(new_run.clone()).await.unwrap();
        assert_eq!(created_run.id, new_run.id);
        assert_eq!(created_run.codebase, new_run.codebase);
        
        let retrieved_run = repo.get_run(new_run.id).await.unwrap().unwrap();
        assert_eq!(created_run, retrieved_run);
    }
    
    #[sqlx::test]
    async fn test_custom_types(pool: TestPool) {
        // Test debversion type handling
        let version = DebianVersion("1.0.0-1".to_string());
        
        sqlx::query!(
            "INSERT INTO test_table (version) VALUES ($1)",
            version as DebianVersion
        )
        .execute(&pool)
        .await
        .unwrap();
        
        let result = sqlx::query!(
            "SELECT version FROM test_table LIMIT 1"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        
        assert_eq!(result.version, version);
    }
}
```

### Integration Testing
```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_python_compatibility() {
        let config = StateConfig::test_config();
        let rust_state = JanitorState::new(config).await.unwrap();
        let python_state = PythonStateInterface::new().await.unwrap();
        
        // Test data consistency
        let test_runs = create_test_runs().await;
        
        for test_run in test_runs {
            let rust_result = rust_state.services.runs
                .get_run(test_run.id)
                .await
                .unwrap();
                
            let python_result = python_state
                .get_run(test_run.id)
                .await
                .unwrap();
                
            assert_eq!(rust_result, python_result);
        }
    }
    
    #[tokio::test]
    async fn test_business_logic_parity() {
        let rust_state = create_test_state().await;
        let python_state = create_python_test_state().await;
        
        let test_codebase = "test-codebase";
        
        let rust_suites = rust_state
            .iter_publishable_suites(test_codebase)
            .await
            .unwrap();
            
        let python_suites = python_state
            .iter_publishable_suites(test_codebase)
            .await
            .unwrap();
            
        assert_eq!(rust_suites.sort(), python_suites.sort());
    }
}
```

## Performance Optimization

### Connection Pool Tuning
```rust
pub struct DatabaseConfig {
    pub max_connections: u32,
    pub min_connections: u32,
    pub idle_timeout_seconds: u64,
    pub max_lifetime_seconds: u64,
    pub connection_timeout_seconds: u64,
    pub query_timeout_seconds: u64,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            max_connections: 20,
            min_connections: 5,
            idle_timeout_seconds: 600,  // 10 minutes
            max_lifetime_seconds: 3600, // 1 hour
            connection_timeout_seconds: 30,
            query_timeout_seconds: 30,
        }
    }
}
```

### Query Optimization
```rust
impl PostgresRunRepository {
    // Optimized query for common use cases
    pub async fn get_recent_runs(
        &self,
        codebase: &str,
        limit: i64,
    ) -> Result<Vec<Run>, DatabaseError> {
        sqlx::query_as!(
            Run,
            r#"
            SELECT 
                id, codebase, campaign, start_time, finish_time,
                result_code, description, context, value,
                result as "result: Json<serde_json::Value>",
                logfilenames,
                result_branches as "result_branches: Json<serde_json::Value>",
                instigated_context
            FROM run 
            WHERE codebase = $1 
            ORDER BY start_time DESC 
            LIMIT $2
            "#,
            codebase,
            limit
        )
        .fetch_all(&self.pool)
        .await
        .map_err(DatabaseError::from)
    }
    
    // Bulk operations for performance
    pub async fn create_runs_batch(
        &self,
        new_runs: Vec<NewRun>,
    ) -> Result<Vec<Run>, DatabaseError> {
        let mut transaction = self.pool.begin().await?;
        let mut created_runs = Vec::with_capacity(new_runs.len());
        
        for new_run in new_runs {
            let run = sqlx::query_as!(
                Run,
                "INSERT INTO run (...) VALUES (...) RETURNING *",
                // ... parameters
            )
            .fetch_one(&mut *transaction)
            .await?;
            
            created_runs.push(run);
        }
        
        transaction.commit().await?;
        Ok(created_runs)
    }
}
```

## Migration Timeline

| Phase | Duration | Dependencies | Risk Level |
|-------|----------|--------------|------------|
| Phase 1 (Core Types) | 2-3 weeks | Database schema analysis | Medium |
| Phase 2 (Query Interface) | 2-3 weeks | Phase 1, Repository patterns | Medium |
| Phase 3 (Error Handling) | 1-2 weeks | Phase 2, Monitoring setup | Low |
| Phase 4 (Service Integration) | 2-3 weeks | Phase 3, Service coordination | High |

**Total Estimated Duration: 7-11 weeks (2-3 months)**

## Risk Mitigation

### High-Risk Areas
1. **Data Consistency**: Ensuring Rust and Python see identical data
2. **Custom Types**: PostgreSQL custom type handling complexity
3. **Transaction Management**: Complex business logic transactions
4. **Performance**: Connection pool and query optimization

### Mitigation Strategies
1. **Shadow Mode Testing**: Run Rust alongside Python for validation
2. **Comprehensive Testing**: Property-based testing with real data
3. **Gradual Migration**: Service-by-service migration with rollback
4. **Monitoring**: Extensive metrics and alerting for anomalies

## Success Criteria

### Functional Requirements
- ✅ **Data Consistency**: 100% compatibility with Python state management
- ✅ **Type Safety**: Zero runtime type errors with compile-time guarantees
- ✅ **Business Logic Parity**: Identical results for all business operations
- ✅ **Error Handling**: Comprehensive error categorization and recovery

### Performance Requirements
- ✅ **Query Performance**: 2-5x improvement in query execution time
- ✅ **Memory Usage**: 50-70% reduction vs. Python
- ✅ **Connection Management**: Efficient pool utilization with minimal overhead
- ✅ **Throughput**: Higher concurrent operation capacity

### Quality Requirements
- ✅ **Test Coverage**: >95% coverage with integration tests
- ✅ **Documentation**: Complete API documentation with examples
- ✅ **Monitoring**: Production-ready metrics and health checks
- ✅ **Maintainability**: Clean, well-documented repository patterns

## Conclusion

The database state management migration is foundational to the entire Janitor platform migration. Success in this area enables:

1. **Unified Data Access**: Consistent patterns across all services
2. **Type Safety**: Compile-time guarantees for database operations
3. **Performance**: Significant improvements in database interaction efficiency
4. **Maintainability**: Clear separation of concerns and testable code

The phased approach minimizes risk while enabling incremental validation. The resulting Rust implementation will provide a solid foundation for all other service migrations.

**Next Steps:**
1. Begin comprehensive analysis of current database schema
2. Create detailed type mapping documentation
3. Establish testing infrastructure with real production data
4. Implement shadow mode testing framework