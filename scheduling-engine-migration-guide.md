# Scheduling Engine Migration Guide

> **Status**: 📋 **PLANNING** - Detailed implementation strategy for complex scheduling algorithms.
> 
> 📋 **Master Plan**: See [`porting-plan.md`](porting-plan.md) for overall project coordination and dependencies.

## Overview

This document provides a comprehensive migration strategy for the Janitor scheduling engine from Python to Rust. The scheduling engine is one of the most complex and critical components of the Janitor platform, responsible for intelligent prioritization of work items, resource allocation, success probability estimation, and sophisticated queue management.

### Current State Analysis

**Python Implementation (`py/janitor/schedule.py`)**: 635 lines
- **Sophisticated Algorithm**: Complex scoring system with multiple factors
- **Success Probability Estimation**: Machine learning-like predictive modeling
- **Duration Estimation**: Historical data analysis for runtime prediction
- **Dependency Management**: Complex dependency resolution and ordering
- **Queue Management**: Integration with Redis-based queue system
- **Database Operations**: Heavy PostgreSQL interaction with complex queries
- **Statistical Analysis**: Success rate calculations and trend analysis

**Rust Implementation**: Currently **scattered across multiple crates**
- `src/schedule.rs` (root crate): Basic scheduling infrastructure (minimal)
- `runner/src/web.rs`: Some queue assignment logic (incomplete)
- Missing: Core scheduling algorithms, success estimation, dependency resolution

## Technical Architecture Analysis

### Current Python Algorithm Components

#### 1. Priority Scoring System
```python
# Core scoring factors
FIRST_RUN_BONUS = 100.0
PUBLISH_MODE_VALUE = {
    "skip": 0,
    "build-only": 0, 
    "push": 500,
    "propose": 400,
    "attempt-push": 450,
    "bts": 100,
}
```

#### 2. Success Probability Estimation
- **Historical Analysis**: Analyzes past runs for patterns
- **Result Code Classification**: Filters transient vs. permanent failures
- **Time-based Decay**: Recent failures weighted more heavily
- **Campaign-specific Learning**: Per-campaign success rate tracking

#### 3. Duration Estimation
- **Median Calculation**: Historical runtime analysis
- **Campaign Defaults**: Per-campaign duration baselines
- **Codebase Specifics**: Per-repository runtime patterns
- **Fallback Mechanisms**: Default estimates when no data available

#### 4. Dependency Resolution
- **Campaign Dependencies**: Inter-campaign ordering requirements
- **Version Dependencies**: Debian version-specific ordering
- **Database Constraints**: Foreign key and constraint validation

### Target Rust Architecture

#### Core Scheduling Engine (`janitor-scheduler/` crate)
```rust
pub struct SchedulingEngine {
    db_pool: PgPool,
    queue: Arc<dyn QueueManager>,
    config: SchedulingConfig,
    metrics: SchedulingMetrics,
}

pub struct CandidateScore {
    base_value: f64,
    publish_bonus: f64,
    first_run_bonus: f64,
    success_probability: f64,
    estimated_duration: Duration,
    priority_factors: Vec<PriorityFactor>,
}

pub struct SchedulingDecision {
    candidate: Candidate,
    score: CandidateScore,
    reasoning: Vec<SchedulingReason>,
    dependencies: Vec<CandidateId>,
}
```

#### Statistical Analysis (`janitor-analytics/` crate)
```rust
pub struct SuccessProbabilityEstimator {
    historical_data: HistoricalRunData,
    campaign_models: HashMap<String, CampaignModel>,
    result_classifiers: Vec<ResultClassifier>,
}

pub struct DurationEstimator {
    median_calculator: MedianCalculator,
    campaign_baselines: HashMap<String, Duration>,
    codebase_patterns: HashMap<String, DurationPattern>,
}
```

## Migration Strategy

### Phase 1: Data Structure Migration (2-3 weeks)

#### 1.1 Core Types and Models
**Target**: Port all Python data structures to Rust with proper serialization

```rust
// Core scheduling types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub codebase: String,
    pub campaign: String,
    pub branch_url: String,
    pub context: Option<String>,
    pub value: f64,
    pub success_chance: Option<f64>,
    pub command: Vec<String>,
    pub publish_policy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItem {
    pub id: Uuid,
    pub codebase: String,
    pub campaign: String,
    pub command: Vec<String>,
    pub estimated_duration: Option<Duration>,
    pub bucket: String,
    pub priority: i32,
    pub created_at: DateTime<Utc>,
}
```

#### 1.2 Database Schema Mapping
**Target**: Create type-safe database operations with sqlx

```rust
#[derive(sqlx::FromRow)]
struct CandidateRow {
    codebase: String,
    branch_url: String,
    campaign: String,
    context: Option<String>,
    value: f64,
    success_chance: Option<f64>,
    publish: sqlx::types::Json<Vec<PublishPolicy>>,
    command: Vec<String>,
}

impl TryFrom<CandidateRow> for Candidate {
    type Error = SchedulingError;
    
    fn try_from(row: CandidateRow) -> Result<Self, Self::Error> {
        // Conversion logic with error handling
    }
}
```

#### 1.3 Configuration Management
**Target**: TOML-based configuration with validation

```toml
[scheduling]
first_run_bonus = 100.0
default_estimated_duration = 15
default_schedule_offset = -1.0

[publish_mode_values]
skip = 0
build_only = 0
push = 500
propose = 400
attempt_push = 450
bts = 100

[success_estimation]
ignore_worker_failures_older_than_days = 1
historical_data_window_days = 30
minimum_data_points = 5
```

### Phase 2: Core Algorithm Implementation (4-5 weeks)

#### 2.1 Priority Scoring Engine
**Target**: Implement the core scoring algorithm with exact Python parity

```rust
impl SchedulingEngine {
    pub async fn calculate_candidate_score(
        &self,
        candidate: &Candidate,
        historical_data: &HistoricalData,
    ) -> Result<CandidateScore, SchedulingError> {
        let mut score = CandidateScore {
            base_value: candidate.value,
            publish_bonus: self.calculate_publish_bonus(&candidate.publish_policy)?,
            first_run_bonus: self.calculate_first_run_bonus(candidate).await?,
            success_probability: self.estimate_success_probability(candidate, historical_data).await?,
            estimated_duration: self.estimate_duration(candidate, historical_data).await?,
            priority_factors: vec![],
        };
        
        score.apply_priority_factors();
        Ok(score)
    }
    
    fn calculate_publish_bonus(&self, publish_policy: &str) -> Result<f64, SchedulingError> {
        let policies: Vec<PublishPolicy> = serde_json::from_str(publish_policy)?;
        let bonus = policies.iter()
            .map(|policy| self.config.publish_mode_values.get(&policy.mode).unwrap_or(&0.0))
            .sum();
        Ok(bonus)
    }
}
```

#### 2.2 Success Probability Estimation
**Target**: Port the sophisticated predictive modeling logic

```rust
pub struct SuccessProbabilityEstimator {
    config: EstimationConfig,
    result_classifiers: HashMap<String, ResultClassifier>,
}

impl SuccessProbabilityEstimator {
    pub async fn estimate_success_probability(
        &self,
        candidate: &Candidate,
        historical_runs: &[HistoricalRun],
    ) -> Result<f64, EstimationError> {
        // Filter relevant historical data
        let relevant_runs = self.filter_relevant_runs(candidate, historical_runs)?;
        
        if relevant_runs.is_empty() {
            return Ok(self.config.default_success_probability);
        }
        
        // Apply time-based weighting
        let weighted_outcomes = self.apply_time_weighting(&relevant_runs)?;
        
        // Apply result code filtering (ignore transient failures)
        let filtered_outcomes = self.filter_transient_failures(&weighted_outcomes)?;
        
        // Calculate probability with confidence intervals
        let probability = self.calculate_weighted_success_rate(&filtered_outcomes)?;
        
        Ok(probability.clamp(0.0, 1.0))
    }
    
    fn filter_transient_failures(
        &self,
        runs: &[WeightedRun],
    ) -> Result<Vec<WeightedRun>, EstimationError> {
        runs.iter()
            .filter(|run| !self.should_ignore_result(run))
            .cloned()
            .collect::<Vec<_>>()
            .into()
    }
    
    fn should_ignore_result(&self, run: &WeightedRun) -> bool {
        if let Some(classifier) = self.result_classifiers.get(&run.result_code) {
            classifier.should_ignore(run)
        } else {
            false
        }
    }
}
```

#### 2.3 Duration Estimation Engine
**Target**: Implement statistical duration prediction

```rust
pub struct DurationEstimator {
    config: DurationConfig,
    statistics: DurationStatistics,
}

impl DurationEstimator {
    pub async fn estimate_duration(
        &self,
        candidate: &Candidate,
        historical_runs: &[HistoricalRun],
    ) -> Result<Duration, EstimationError> {
        // Try campaign-specific median first
        if let Some(duration) = self.estimate_campaign_median(candidate, historical_runs).await? {
            return Ok(duration);
        }
        
        // Fall back to codebase-specific median
        if let Some(duration) = self.estimate_codebase_median(candidate, historical_runs).await? {
            return Ok(duration);
        }
        
        // Use campaign default
        if let Some(duration) = self.get_campaign_default(candidate).await? {
            return Ok(duration);
        }
        
        // Global default
        Ok(Duration::minutes(self.config.default_estimated_duration))
    }
    
    async fn estimate_campaign_median(
        &self,
        candidate: &Candidate,
        historical_runs: &[HistoricalRun],
    ) -> Result<Option<Duration>, EstimationError> {
        let campaign_runs: Vec<_> = historical_runs
            .iter()
            .filter(|run| run.campaign == candidate.campaign)
            .filter(|run| run.duration.is_some())
            .collect();
            
        if campaign_runs.len() < self.config.minimum_data_points {
            return Ok(None);
        }
        
        let mut durations: Vec<Duration> = campaign_runs
            .into_iter()
            .map(|run| run.duration.unwrap())
            .collect();
            
        durations.sort();
        let median_idx = durations.len() / 2;
        Ok(Some(durations[median_idx]))
    }
}
```

### Phase 3: Queue Management Integration (2-3 weeks)

#### 3.1 Redis Queue Integration
**Target**: Seamless integration with existing Redis queue system

```rust
pub struct RedisQueueManager {
    redis_pool: bb8::Pool<RedisConnectionManager>,
    config: QueueConfig,
}

impl QueueManager for RedisQueueManager {
    async fn add_items_bulk(
        &self,
        items: Vec<QueueItem>,
    ) -> Result<Vec<QueueItemId>, QueueError> {
        let mut conn = self.redis_pool.get().await?;
        let mut pipe = redis::pipe();
        
        for item in &items {
            let serialized = serde_json::to_string(item)?;
            pipe.zadd(
                &format!("queue:{}", item.bucket),
                serialized,
                item.priority,
            );
        }
        
        let results: Vec<i32> = pipe.query_async(&mut *conn).await?;
        
        // Convert to QueueItemId results
        Ok(items.into_iter().map(|item| item.id).collect())
    }
    
    async fn get_queue_position(
        &self,
        item_id: &QueueItemId,
    ) -> Result<Option<usize>, QueueError> {
        let mut conn = self.redis_pool.get().await?;
        
        // Search across all queue buckets
        for bucket in &self.config.buckets {
            let rank: Option<isize> = conn
                .zrank(&format!("queue:{}", bucket), item_id.to_string())
                .await?;
                
            if let Some(position) = rank {
                return Ok(Some(position as usize));
            }
        }
        
        Ok(None)
    }
}
```

#### 3.2 Dependency Resolution Engine
**Target**: Complex dependency validation and ordering

```rust
pub struct DependencyResolver {
    db_pool: PgPool,
    cache: Arc<RwLock<HashMap<String, Vec<Dependency>>>>,
}

impl DependencyResolver {
    pub async fn resolve_dependencies(
        &self,
        candidates: &[Candidate],
    ) -> Result<Vec<SchedulingDecision>, DependencyError> {
        let mut graph = DependencyGraph::new();
        
        // Build dependency graph
        for candidate in candidates {
            let deps = self.get_dependencies(candidate).await?;
            graph.add_node(candidate.clone(), deps);
        }
        
        // Topological sort with cycle detection
        let ordered_candidates = graph.topological_sort()?;
        
        // Convert to scheduling decisions
        let decisions = ordered_candidates
            .into_iter()
            .map(|candidate| {
                SchedulingDecision {
                    candidate: candidate.clone(),
                    score: CandidateScore::default(), // Will be calculated later
                    reasoning: vec![SchedulingReason::DependencyOrder],
                    dependencies: graph.get_dependencies(&candidate).unwrap_or_default(),
                }
            })
            .collect();
            
        Ok(decisions)
    }
    
    async fn deps_satisfied(
        &self,
        campaign: &str,
        dependencies: &[Dependency],
    ) -> Result<bool, DependencyError> {
        if dependencies.is_empty() {
            return Ok(true);
        }
        
        for dep in dependencies {
            match dep {
                Dependency::Campaign(dep_campaign) => {
                    if !self.campaign_completed(dep_campaign).await? {
                        return Ok(false);
                    }
                }
                Dependency::Version(version_constraint) => {
                    if !self.version_available(version_constraint).await? {
                        return Ok(false);
                    }
                }
            }
        }
        
        Ok(true)
    }
}
```

### Phase 4: Integration and Testing (3-4 weeks)

#### 4.1 Python Compatibility Layer
**Target**: Gradual migration with Python fallback

```rust
pub struct SchedulingOrchestrator {
    rust_engine: SchedulingEngine,
    python_fallback: Option<PythonScheduler>,
    config: OrchestrationConfig,
}

impl SchedulingOrchestrator {
    pub async fn schedule_candidates(
        &self,
        candidates: Vec<Candidate>,
    ) -> Result<Vec<SchedulingDecision>, SchedulingError> {
        match self.config.mode {
            OrchestrationMode::RustOnly => {
                self.rust_engine.schedule_candidates(candidates).await
            }
            OrchestrationMode::PythonFallback => {
                match self.rust_engine.schedule_candidates(candidates.clone()).await {
                    Ok(decisions) => Ok(decisions),
                    Err(e) => {
                        warn!("Rust scheduling failed, falling back to Python: {}", e);
                        self.python_fallback
                            .as_ref()
                            .unwrap()
                            .schedule_candidates(candidates)
                            .await
                    }
                }
            }
            OrchestrationMode::ShadowMode => {
                let rust_result = self.rust_engine.schedule_candidates(candidates.clone()).await;
                let python_result = self.python_fallback
                    .as_ref()
                    .unwrap()
                    .schedule_candidates(candidates)
                    .await;
                    
                self.compare_results(&rust_result, &python_result).await;
                python_result // Use Python results in shadow mode
            }
        }
    }
}
```

#### 4.2 Algorithm Validation Testing
**Target**: Comprehensive comparison with Python implementation

```rust
#[cfg(test)]
mod compatibility_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_priority_calculation_parity() {
        let test_cases = load_test_cases("scheduling_test_data.json").await.unwrap();
        
        for test_case in test_cases {
            let rust_score = rust_engine
                .calculate_candidate_score(&test_case.candidate, &test_case.historical_data)
                .await
                .unwrap();
                
            let python_score = python_engine
                .calculate_candidate_score(&test_case.candidate, &test_case.historical_data)
                .await
                .unwrap();
                
            assert_eq!(
                rust_score.total_score().round() as i32,
                python_score.total_score().round() as i32,
                "Score mismatch for candidate: {:?}", test_case.candidate
            );
        }
    }
    
    #[tokio::test]
    async fn test_success_probability_estimation_parity() {
        // Test with real historical data
        let historical_data = load_historical_data("success_probability_test_data.json").await.unwrap();
        
        for test_case in historical_data.test_cases {
            let rust_probability = rust_estimator
                .estimate_success_probability(&test_case.candidate, &test_case.runs)
                .await
                .unwrap();
                
            let python_probability = python_estimator
                .estimate_success_probability(&test_case.candidate, &test_case.runs)
                .await
                .unwrap();
                
            let difference = (rust_probability - python_probability).abs();
            assert!(
                difference < 0.01, // 1% tolerance
                "Success probability mismatch: rust={}, python={}, candidate={:?}",
                rust_probability, python_probability, test_case.candidate
            );
        }
    }
}
```

#### 4.3 Performance Benchmarking
**Target**: Document performance improvements and identify bottlenecks

```rust
#[cfg(test)]
mod performance_tests {
    use criterion::{criterion_group, criterion_main, Criterion};
    
    fn benchmark_scheduling_throughput(c: &mut Criterion) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let engine = rt.block_on(create_test_engine()).unwrap();
        let candidates = rt.block_on(load_benchmark_candidates(1000)).unwrap();
        
        c.bench_function("schedule_1000_candidates", |b| {
            b.to_async(&rt).iter(|| async {
                engine.schedule_candidates(candidates.clone()).await.unwrap()
            })
        });
    }
    
    fn benchmark_success_probability_estimation(c: &mut Criterion) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let estimator = rt.block_on(create_test_estimator()).unwrap();
        let test_data = rt.block_on(load_estimation_benchmark_data()).unwrap();
        
        c.bench_function("estimate_success_probability", |b| {
            b.to_async(&rt).iter(|| async {
                for test_case in &test_data {
                    estimator
                        .estimate_success_probability(&test_case.candidate, &test_case.runs)
                        .await
                        .unwrap();
                }
            })
        });
    }
    
    criterion_group!(benches, benchmark_scheduling_throughput, benchmark_success_probability_estimation);
    criterion_main!(benches);
}
```

## Database Migration Strategy

### Schema Compatibility
The Rust implementation must maintain exact compatibility with existing database schema:

```sql
-- Key tables used by scheduling engine
CREATE TABLE candidate (
    codebase TEXT REFERENCES codebase(name),
    suite TEXT NOT NULL,
    context TEXT,
    value DOUBLE PRECISION,
    success_chance DOUBLE PRECISION,
    command TEXT[] NOT NULL,
    publish_policy TEXT NOT NULL
);

CREATE TABLE run (
    id UUID PRIMARY KEY,
    codebase TEXT NOT NULL,
    campaign TEXT NOT NULL,
    start_time TIMESTAMP,
    finish_time TIMESTAMP,
    result_code TEXT,
    duration INTERVAL
);
```

### Query Optimization
Rust implementation should optimize expensive Python queries:

```rust
// Optimized query with proper indexing hints
impl SchedulingEngine {
    async fn load_historical_runs(
        &self,
        candidate: &Candidate,
        time_window: Duration,
    ) -> Result<Vec<HistoricalRun>, DatabaseError> {
        let cutoff_time = Utc::now() - time_window;
        
        sqlx::query_as!(
            HistoricalRun,
            r#"
            SELECT 
                id, codebase, campaign, start_time, finish_time,
                result_code, duration,
                EXTRACT(EPOCH FROM duration) as duration_seconds
            FROM run 
            WHERE 
                codebase = $1 
                AND campaign = $2 
                AND start_time >= $3
                AND duration IS NOT NULL
            ORDER BY start_time DESC
            LIMIT 100
            "#,
            candidate.codebase,
            candidate.campaign,
            cutoff_time
        )
        .fetch_all(&self.db_pool)
        .await
        .map_err(DatabaseError::from)
    }
}
```

## Monitoring and Observability

### Metrics Collection
```rust
pub struct SchedulingMetrics {
    candidates_processed: Counter,
    scheduling_duration: Histogram,
    queue_items_added: Counter,
    success_probability_accuracy: Histogram,
    duration_estimation_accuracy: Histogram,
}

impl SchedulingEngine {
    async fn schedule_with_metrics(
        &self,
        candidates: Vec<Candidate>,
    ) -> Result<Vec<SchedulingDecision>, SchedulingError> {
        let start_time = Instant::now();
        let candidate_count = candidates.len();
        
        let result = self.schedule_candidates_internal(candidates).await;
        
        // Record metrics
        let duration = start_time.elapsed();
        self.metrics.candidates_processed.inc_by(candidate_count as u64);
        self.metrics.scheduling_duration.observe(duration.as_secs_f64());
        
        if let Ok(ref decisions) = result {
            self.metrics.queue_items_added.inc_by(decisions.len() as u64);
        }
        
        result
    }
}
```

### Logging Strategy
```rust
use tracing::{info, warn, error, debug, instrument};

impl SchedulingEngine {
    #[instrument(skip(self, candidates), fields(candidate_count = candidates.len()))]
    pub async fn schedule_candidates(
        &self,
        candidates: Vec<Candidate>,
    ) -> Result<Vec<SchedulingDecision>, SchedulingError> {
        info!("Starting candidate scheduling");
        
        let decisions = self.schedule_candidates_internal(candidates).await?;
        
        info!(
            decision_count = decisions.len(),
            "Scheduling completed successfully"
        );
        
        for decision in &decisions {
            debug!(
                codebase = %decision.candidate.codebase,
                campaign = %decision.candidate.campaign,
                score = %decision.score.total_score(),
                "Scheduled candidate"
            );
        }
        
        Ok(decisions)
    }
}
```

## Risk Mitigation

### High-Risk Areas
1. **Algorithm Accuracy**: Complex mathematical calculations must match Python exactly
2. **Database Performance**: Scheduling queries are expensive and must be optimized
3. **Dependency Resolution**: Complex graph algorithms with cycle detection
4. **Statistical Calculations**: Floating-point precision and edge cases

### Mitigation Strategies
1. **Extensive Testing**: Property-based testing with real data
2. **Shadow Mode**: Run Rust alongside Python for comparison
3. **Gradual Rollout**: Feature flags for individual algorithm components
4. **Monitoring**: Comprehensive metrics and alerting for accuracy

## Success Criteria

### Functional Requirements
- ✅ **Algorithm Parity**: Identical scheduling decisions to Python implementation
- ✅ **Performance**: 5-10x improvement in scheduling throughput
- ✅ **Database Compatibility**: Zero schema changes required
- ✅ **Queue Integration**: Seamless Redis queue operations

### Quality Requirements
- ✅ **Test Coverage**: >95% coverage with property-based tests
- ✅ **Documentation**: Complete algorithm documentation with examples
- ✅ **Monitoring**: Production-ready observability and alerting
- ✅ **Maintainability**: Clean, well-documented Rust code

### Performance Requirements
- ✅ **Throughput**: Process 1000+ candidates in <1 second
- ✅ **Memory Usage**: 50-70% reduction vs. Python
- ✅ **Database Load**: Optimized queries reducing DB pressure
- ✅ **Latency**: Real-time scheduling decisions <100ms

## Implementation Timeline

| Phase | Duration | Dependencies | Risk Level |
|-------|----------|--------------|------------|
| Phase 1 (Data Structures) | 2-3 weeks | Database schema analysis | Low |
| Phase 2 (Core Algorithms) | 4-5 weeks | Phase 1 complete | High |
| Phase 3 (Queue Integration) | 2-3 weeks | Phase 2, Redis setup | Medium |
| Phase 4 (Testing & Migration) | 3-4 weeks | Phase 3, Python comparison | High |

**Total Estimated Duration: 11-15 weeks (3-4 months)**

## Conclusion

The scheduling engine migration represents the most complex algorithmic challenge in the Janitor porting effort. Success requires:

1. **Mathematical Precision**: Exact replication of complex scoring algorithms
2. **Performance Optimization**: Significant improvements in throughput and latency
3. **Statistical Accuracy**: Maintaining predictive model effectiveness
4. **Integration Seamlessness**: Zero disruption to existing workflows

The phased approach allows for incremental validation and reduces risk through extensive testing and shadow mode operation. The resulting Rust implementation will provide the foundation for future enhancements and improved system reliability.

**Next Steps:**
1. Begin Phase 1 with comprehensive Python algorithm analysis
2. Create detailed test datasets from production data
3. Establish performance benchmarking baseline
4. Set up shadow mode infrastructure for parallel testing