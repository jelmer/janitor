# Cross-Service Integration Testing Strategy

> **Status**: 📋 **PLANNING** - Comprehensive testing strategy for hybrid Python/Rust service environments.
> 
> 📋 **Master Plan**: See [`porting-plan.md`](porting-plan.md) for overall project coordination and dependencies.

## Overview

This document outlines a comprehensive strategy for testing the integration between Python and Rust services during the Janitor platform migration. As services are migrated individually, the platform will operate in a hybrid state requiring sophisticated testing approaches to ensure compatibility, data consistency, and functional correctness across service boundaries.

### Testing Challenges

#### Multi-Language Service Mesh
- **Rust Services**: New implementations with modern architectures
- **Python Services**: Existing implementations with complex business logic
- **API Compatibility**: HTTP/gRPC interfaces between mixed services
- **Data Consistency**: Shared database state across language boundaries
- **Event Systems**: Message queue compatibility and ordering

#### Migration Complexity
- **Phased Rollouts**: Services migrate at different times
- **Feature Parity**: Ensuring identical behavior across implementations
- **Performance Variations**: Different response times affecting integration
- **Error Handling**: Consistent error propagation across service boundaries

## Testing Architecture

### Testing Environment Topology

```mermaid
graph TB
    subgraph "Testing Infrastructure"
        TH[Test Harness]
        TDB[(Test Database)]
        TRedis[(Test Redis)]
        TMQ[Test Message Queue]
    end
    
    subgraph "Service Under Test"
        RS[Rust Service]
        PS[Python Service]
    end
    
    subgraph "Companion Services"
        CS1[Companion Service 1]
        CS2[Companion Service 2]
        CSN[Companion Service N]
    end
    
    TH --> RS
    TH --> PS
    TH --> CS1
    TH --> CS2
    TH --> CSN
    
    RS --> TDB
    PS --> TDB
    CS1 --> TDB
    
    RS --> TRedis
    PS --> TRedis
    
    RS --> TMQ
    PS --> TMQ
```

### Test Harness Architecture

```rust
pub struct IntegrationTestHarness {
    services: ServiceRegistry,
    database: TestDatabase,
    message_queue: TestMessageQueue,
    http_client: TestHttpClient,
    config: TestConfig,
    metrics: TestMetrics,
}

pub struct ServiceRegistry {
    rust_services: HashMap<String, RustServiceHandle>,
    python_services: HashMap<String, PythonServiceHandle>,
    service_topology: ServiceGraph,
}

impl IntegrationTestHarness {
    pub async fn new(config: TestConfig) -> Result<Self, TestError> {
        let database = TestDatabase::create_isolated().await?;
        let message_queue = TestMessageQueue::new().await?;
        let http_client = TestHttpClient::with_tracing();
        
        let services = ServiceRegistry::from_config(&config).await?;
        
        Ok(Self {
            services,
            database,
            message_queue,
            http_client,
            config,
            metrics: TestMetrics::new(),
        })
    }
    
    pub async fn run_scenario<T>(&self, scenario: T) -> TestResult
    where
        T: IntegrationTestScenario + Send + Sync,
    {
        let span = tracing::info_span!("integration_test", scenario = %scenario.name());
        let _guard = span.enter();
        
        // Setup test environment
        self.setup_scenario(&scenario).await?;
        
        // Execute test steps
        let result = scenario.execute(self).await;
        
        // Cleanup and collect metrics
        self.cleanup_scenario(&scenario).await?;
        
        result
    }
}
```

## Testing Categories

### 1. API Compatibility Testing

#### HTTP Interface Validation
```rust
#[async_trait]
pub trait ApiCompatibilityTest {
    async fn test_request_format_compatibility(&self) -> TestResult;
    async fn test_response_format_compatibility(&self) -> TestResult;
    async fn test_error_response_compatibility(&self) -> TestResult;
    async fn test_content_type_compatibility(&self) -> TestResult;
}

pub struct RunnerApiCompatibilityTest {
    rust_runner: ServiceHandle,
    python_runner: ServiceHandle,
    test_data: ApiTestData,
}

impl ApiCompatibilityTest for RunnerApiCompatibilityTest {
    async fn test_request_format_compatibility(&self) -> TestResult {
        let test_cases = vec![
            // Queue assignment request
            ApiTestCase {
                endpoint: "/api/queue/next",
                method: "GET",
                headers: hashmap! {
                    "Content-Type" => "application/json",
                    "X-Worker-Name" => "test-worker"
                },
                body: None,
                expected_status: 200,
            },
            // Run result submission
            ApiTestCase {
                endpoint: "/api/runs/{run_id}/result",
                method: "POST",
                headers: hashmap! {
                    "Content-Type" => "application/json"
                },
                body: Some(json!({
                    "result_code": "success",
                    "result": {"changes": []},
                    "logfilenames": ["build.log"]
                })),
                expected_status: 201,
            },
        ];
        
        for test_case in test_cases {
            // Test against Rust service
            let rust_response = self.rust_runner
                .send_request(&test_case)
                .await?;
                
            // Test against Python service
            let python_response = self.python_runner
                .send_request(&test_case)
                .await?;
                
            // Compare responses
            self.compare_api_responses(&rust_response, &python_response, &test_case)?;
        }
        
        TestResult::Success
    }
    
    async fn test_response_format_compatibility(&self) -> TestResult {
        let queue_item_tests = vec![
            QueueItemTestCase {
                codebase: "test-codebase",
                campaign: "test-campaign",
                expected_fields: vec![
                    "id", "codebase", "campaign", "command", 
                    "estimated_duration", "bucket"
                ],
            },
        ];
        
        for test_case in queue_item_tests {
            let rust_item = self.rust_runner
                .get_queue_item(&test_case.codebase, &test_case.campaign)
                .await?;
                
            let python_item = self.python_runner
                .get_queue_item(&test_case.codebase, &test_case.campaign)
                .await?;
                
            // Validate JSON structure compatibility
            self.validate_json_structure_match(&rust_item, &python_item)?;
            
            // Validate field types and values
            self.validate_field_compatibility(&rust_item, &python_item, &test_case)?;
        }
        
        TestResult::Success
    }
}
```

#### Schema Validation
```rust
pub struct SchemaCompatibilityValidator {
    rust_schemas: HashMap<String, JsonSchema>,
    python_schemas: HashMap<String, JsonSchema>,
}

impl SchemaCompatibilityValidator {
    pub async fn validate_api_schemas(&self) -> Result<CompatibilityReport, ValidationError> {
        let mut report = CompatibilityReport::new();
        
        for (endpoint, rust_schema) in &self.rust_schemas {
            if let Some(python_schema) = self.python_schemas.get(endpoint) {
                let compatibility = self.compare_schemas(rust_schema, python_schema)?;
                report.add_endpoint_result(endpoint.clone(), compatibility);
            } else {
                report.add_missing_endpoint(endpoint.clone());
            }
        }
        
        Ok(report)
    }
    
    fn compare_schemas(
        &self,
        rust_schema: &JsonSchema,
        python_schema: &JsonSchema,
    ) -> Result<SchemaCompatibility, ValidationError> {
        let mut compatibility = SchemaCompatibility::new();
        
        // Check required fields
        let rust_required = rust_schema.required_fields();
        let python_required = python_schema.required_fields();
        
        for field in &rust_required {
            if !python_required.contains(field) {
                compatibility.add_missing_required_field(field.clone());
            }
        }
        
        // Check field types
        for (field_name, rust_type) in rust_schema.field_types() {
            if let Some(python_type) = python_schema.field_types().get(&field_name) {
                if !self.types_compatible(&rust_type, python_type) {
                    compatibility.add_type_mismatch(field_name, rust_type, python_type.clone());
                }
            }
        }
        
        Ok(compatibility)
    }
}
```

### 2. Data Consistency Testing

#### Database State Validation
```rust
pub struct DatabaseConsistencyTester {
    database: TestDatabase,
    rust_services: Vec<ServiceHandle>,
    python_services: Vec<ServiceHandle>,
}

impl DatabaseConsistencyTester {
    pub async fn test_crud_operations_consistency(&self) -> TestResult {
        let test_scenarios = vec![
            CrudTestScenario {
                entity: "Run",
                operations: vec![
                    CrudOperation::Create(create_test_run()),
                    CrudOperation::Read(run_id),
                    CrudOperation::Update(run_id, update_data),
                    CrudOperation::Delete(run_id),
                ],
            },
            CrudTestScenario {
                entity: "Codebase",
                operations: vec![
                    CrudOperation::Create(create_test_codebase()),
                    CrudOperation::List(ListFilter::default()),
                    CrudOperation::Update(codebase_name, update_data),
                ],
            },
        ];
        
        for scenario in test_scenarios {
            self.execute_crud_scenario(scenario).await?;
        }
        
        TestResult::Success
    }
    
    async fn execute_crud_scenario(&self, scenario: CrudTestScenario) -> Result<(), TestError> {
        // Execute operations against Rust service
        let mut rust_results = Vec::new();
        for operation in &scenario.operations {
            let result = self.execute_operation_rust(&scenario.entity, operation).await?;
            rust_results.push(result);
        }
        
        // Reset database state
        self.database.reset_to_snapshot().await?;
        
        // Execute same operations against Python service
        let mut python_results = Vec::new();
        for operation in &scenario.operations {
            let result = self.execute_operation_python(&scenario.entity, operation).await?;
            python_results.push(result);
        }
        
        // Compare results
        for (rust_result, python_result) in rust_results.iter().zip(python_results.iter()) {
            self.compare_operation_results(rust_result, python_result)?;
        }
        
        Ok(())
    }
    
    async fn test_transaction_consistency(&self) -> TestResult {
        let transaction_scenarios = vec![
            TransactionScenario {
                name: "Run Creation with Artifacts",
                steps: vec![
                    TransactionStep::CreateRun(test_run_data()),
                    TransactionStep::UploadArtifacts(artifact_data()),
                    TransactionStep::UpdateRunStatus("completed".to_string()),
                ],
                expected_final_state: ExpectedState::RunWithArtifacts,
            },
            TransactionScenario {
                name: "Queue Item Processing",
                steps: vec![
                    TransactionStep::AssignQueueItem(worker_id()),
                    TransactionStep::UpdateItemStatus("in_progress".to_string()),
                    TransactionStep::CompleteItem(result_data()),
                ],
                expected_final_state: ExpectedState::CompletedQueueItem,
            },
        ];
        
        for scenario in transaction_scenarios {
            self.test_transaction_scenario(scenario).await?;
        }
        
        TestResult::Success
    }
}
```

#### Message Queue Integration
```rust
pub struct MessageQueueConsistencyTester {
    queue: TestMessageQueue,
    rust_publishers: Vec<ServiceHandle>,
    python_publishers: Vec<ServiceHandle>,
    rust_consumers: Vec<ServiceHandle>,
    python_consumers: Vec<ServiceHandle>,
}

impl MessageQueueConsistencyTester {
    pub async fn test_message_format_compatibility(&self) -> TestResult {
        let message_scenarios = vec![
            MessageScenario {
                topic: "run.completed",
                rust_message: json!({
                    "run_id": "550e8400-e29b-41d4-a716-446655440000",
                    "codebase": "test-codebase",
                    "campaign": "test-campaign",
                    "result_code": "success",
                    "timestamp": "2023-01-01T00:00:00Z"
                }),
                python_equivalent: json!({
                    "run_id": "550e8400-e29b-41d4-a716-446655440000",
                    "codebase": "test-codebase",
                    "campaign": "test-campaign",
                    "result_code": "success",
                    "timestamp": "2023-01-01T00:00:00Z"
                }),
            },
        ];
        
        for scenario in message_scenarios {
            // Test Rust -> Python message flow
            self.test_cross_language_messaging(
                &self.rust_publishers[0],
                &self.python_consumers[0],
                &scenario,
            ).await?;
            
            // Test Python -> Rust message flow
            self.test_cross_language_messaging(
                &self.python_publishers[0],
                &self.rust_consumers[0],
                &scenario,
            ).await?;
        }
        
        TestResult::Success
    }
    
    async fn test_message_ordering(&self) -> TestResult {
        let ordered_messages = vec![
            create_message("run.started", run_id_1()),
            create_message("run.progress", run_id_1()),
            create_message("run.completed", run_id_1()),
        ];
        
        // Send messages from Rust service
        for message in &ordered_messages {
            self.rust_publishers[0].send_message(message).await?;
        }
        
        // Verify Python service receives them in order
        let received_messages = self.python_consumers[0]
            .receive_messages(ordered_messages.len())
            .await?;
            
        for (sent, received) in ordered_messages.iter().zip(received_messages.iter()) {
            assert_eq!(sent.id, received.id);
            assert_eq!(sent.topic, received.topic);
        }
        
        TestResult::Success
    }
}
```

### 3. End-to-End Workflow Testing

#### Complete User Journey Testing
```rust
pub struct WorkflowTester {
    services: ServiceRegistry,
    test_data: WorkflowTestData,
}

impl WorkflowTester {
    pub async fn test_complete_job_lifecycle(&self) -> TestResult {
        let workflow = JobLifecycleWorkflow {
            steps: vec![
                WorkflowStep::SubmitCandidate {
                    service: "site",
                    implementation: ServiceImplementation::Python,
                    data: candidate_data(),
                },
                WorkflowStep::ScheduleJob {
                    service: "runner", 
                    implementation: ServiceImplementation::Rust,
                    expected_queue_item: expected_queue_item(),
                },
                WorkflowStep::AssignWorker {
                    service: "runner",
                    implementation: ServiceImplementation::Rust,
                    worker_id: test_worker_id(),
                },
                WorkflowStep::ExecuteJob {
                    service: "worker",
                    implementation: ServiceImplementation::Rust,
                    expected_artifacts: expected_artifacts(),
                },
                WorkflowStep::ProcessResults {
                    service: "runner",
                    implementation: ServiceImplementation::Rust,
                    result_data: result_data(),
                },
                WorkflowStep::PublishChanges {
                    service: "publisher",
                    implementation: ServiceImplementation::Rust,
                    publish_policy: publish_policy(),
                },
            ],
        };
        
        self.execute_workflow(workflow).await
    }
    
    async fn execute_workflow(&self, workflow: JobLifecycleWorkflow) -> TestResult {
        let mut workflow_state = WorkflowState::new();
        
        for (step_index, step) in workflow.steps.iter().enumerate() {
            let step_span = tracing::info_span!(
                "workflow_step",
                step = step_index,
                service = %step.service(),
                implementation = ?step.implementation()
            );
            let _guard = step_span.enter();
            
            let step_result = self.execute_workflow_step(step, &mut workflow_state).await?;
            
            // Validate step result
            self.validate_step_result(step, &step_result, &workflow_state)?;
            
            // Update workflow state
            workflow_state.apply_step_result(step, step_result);
            
            // Wait for async propagation
            self.wait_for_propagation(step).await?;
        }
        
        // Validate final workflow state
        self.validate_final_workflow_state(&workflow, &workflow_state)?;
        
        TestResult::Success
    }
    
    async fn validate_step_result(
        &self,
        step: &WorkflowStep,
        result: &StepResult,
        state: &WorkflowState,
    ) -> Result<(), TestError> {
        match step {
            WorkflowStep::ScheduleJob { expected_queue_item, .. } => {
                let actual_queue_item = result.extract_queue_item()?;
                assert_eq!(actual_queue_item.codebase, expected_queue_item.codebase);
                assert_eq!(actual_queue_item.campaign, expected_queue_item.campaign);
            }
            WorkflowStep::ExecuteJob { expected_artifacts, .. } => {
                let actual_artifacts = result.extract_artifacts()?;
                assert_eq!(actual_artifacts.len(), expected_artifacts.len());
            }
            // ... other step validations
        }
        Ok(())
    }
}
```

#### Cross-Service Data Flow Validation
```rust
pub struct DataFlowValidator {
    services: ServiceRegistry,
    tracing: DistributedTracing,
}

impl DataFlowValidator {
    pub async fn trace_data_flow(&self, workflow: &Workflow) -> Result<DataFlowTrace, TestError> {
        let trace_id = Uuid::new_v4();
        let mut trace = DataFlowTrace::new(trace_id);
        
        for step in &workflow.steps {
            let service = self.services.get_service(&step.service_name)?;
            
            // Inject trace context
            let request = step.create_request_with_trace(trace_id);
            
            // Execute step with tracing
            let response = service.execute_with_tracing(request).await?;
            
            // Extract trace data
            let step_trace = self.extract_step_trace(&response)?;
            trace.add_step(step_trace);
            
            // Validate data transformations
            self.validate_data_transformation(step, &step_trace)?;
        }
        
        Ok(trace)
    }
    
    fn validate_data_transformation(
        &self,
        step: &WorkflowStep,
        trace: &StepTrace,
    ) -> Result<(), TestError> {
        // Validate that data format remains consistent across service boundaries
        for transformation in &trace.data_transformations {
            self.validate_transformation_correctness(transformation)?;
        }
        
        // Validate that no data is lost or corrupted
        self.validate_data_integrity(step, trace)?;
        
        Ok(())
    }
}
```

### 4. Performance and Load Testing

#### Service Interaction Performance
```rust
pub struct PerformanceTester {
    load_generator: LoadGenerator,
    metrics_collector: MetricsCollector,
    services: ServiceRegistry,
}

impl PerformanceTester {
    pub async fn test_service_interaction_performance(&self) -> TestResult {
        let load_scenarios = vec![
            LoadScenario {
                name: "Queue Processing",
                duration: Duration::from_secs(300), // 5 minutes
                rps: 50, // requests per second
                interaction: ServiceInteraction {
                    from: "runner",
                    to: "worker",
                    operation: "assign_queue_item",
                },
            },
            LoadScenario {
                name: "Result Submission",
                duration: Duration::from_secs(300),
                rps: 25,
                interaction: ServiceInteraction {
                    from: "worker",
                    to: "runner", 
                    operation: "submit_result",
                },
            },
        ];
        
        for scenario in load_scenarios {
            self.execute_load_scenario(scenario).await?;
        }
        
        TestResult::Success
    }
    
    async fn execute_load_scenario(&self, scenario: LoadScenario) -> Result<(), TestError> {
        let start_time = Instant::now();
        let mut tasks = Vec::new();
        
        // Start load generation
        for worker_id in 0..scenario.concurrent_workers() {
            let task = self.spawn_load_worker(worker_id, &scenario);
            tasks.push(task);
        }
        
        // Run for specified duration
        tokio::time::sleep(scenario.duration).await;
        
        // Stop load generation
        for task in tasks {
            task.abort();
        }
        
        // Collect and analyze metrics
        let metrics = self.metrics_collector.collect_scenario_metrics(&scenario).await?;
        self.analyze_performance_metrics(&scenario, &metrics)?;
        
        Ok(())
    }
    
    fn analyze_performance_metrics(
        &self,
        scenario: &LoadScenario,
        metrics: &ScenarioMetrics,
    ) -> Result<(), TestError> {
        // Check response time percentiles
        assert!(
            metrics.response_time_p95 < Duration::from_millis(1000),
            "P95 response time exceeded threshold: {:?}", metrics.response_time_p95
        );
        
        // Check error rate
        assert!(
            metrics.error_rate < 0.01, // 1% error rate threshold
            "Error rate exceeded threshold: {}", metrics.error_rate
        );
        
        // Check throughput
        assert!(
            metrics.actual_rps >= scenario.rps as f64 * 0.95, // 95% of target RPS
            "Throughput below threshold: {} < {}", metrics.actual_rps, scenario.rps
        );
        
        Ok(())
    }
}
```

#### Resource Usage Validation
```rust
pub struct ResourceUsageValidator {
    metrics: SystemMetrics,
    thresholds: ResourceThresholds,
}

impl ResourceUsageValidator {
    pub async fn validate_resource_usage_patterns(&self) -> TestResult {
        let test_duration = Duration::from_secs(600); // 10 minutes
        let sampling_interval = Duration::from_secs(10);
        
        let mut samples = Vec::new();
        let start_time = Instant::now();
        
        while start_time.elapsed() < test_duration {
            let sample = self.collect_resource_sample().await?;
            samples.push(sample);
            
            tokio::time::sleep(sampling_interval).await;
        }
        
        self.analyze_resource_patterns(&samples)?;
        self.check_resource_thresholds(&samples)?;
        
        TestResult::Success
    }
    
    async fn collect_resource_sample(&self) -> Result<ResourceSample, TestError> {
        Ok(ResourceSample {
            timestamp: Utc::now(),
            cpu_usage: self.metrics.get_cpu_usage().await?,
            memory_usage: self.metrics.get_memory_usage().await?,
            network_io: self.metrics.get_network_io().await?,
            disk_io: self.metrics.get_disk_io().await?,
            database_connections: self.metrics.get_db_connections().await?,
            queue_depth: self.metrics.get_queue_depth().await?,
        })
    }
    
    fn analyze_resource_patterns(&self, samples: &[ResourceSample]) -> Result<(), TestError> {
        // Check for resource leaks
        let memory_trend = self.calculate_trend(&samples, |s| s.memory_usage as f64)?;
        if memory_trend > self.thresholds.memory_leak_threshold {
            return Err(TestError::ResourceLeak("Memory leak detected".to_string()));
        }
        
        // Check for connection leaks
        let connection_trend = self.calculate_trend(&samples, |s| s.database_connections as f64)?;
        if connection_trend > self.thresholds.connection_leak_threshold {
            return Err(TestError::ResourceLeak("Database connection leak detected".to_string()));
        }
        
        Ok(())
    }
}
```

## Test Data Management

### Test Data Generation
```rust
pub struct TestDataGenerator {
    database: TestDatabase,
    data_templates: DataTemplates,
    generators: DataGenerators,
}

impl TestDataGenerator {
    pub async fn generate_realistic_dataset(&self) -> Result<TestDataset, TestError> {
        let dataset = TestDataset::new();
        
        // Generate codebases with realistic patterns
        let codebases = self.generate_codebases(100).await?;
        dataset.add_codebases(codebases);
        
        // Generate campaigns with proper relationships
        let campaigns = self.generate_campaigns(10).await?;
        dataset.add_campaigns(campaigns);
        
        // Generate realistic run history
        let runs = self.generate_runs_history(1000, &dataset.codebases, &dataset.campaigns).await?;
        dataset.add_runs(runs);
        
        // Generate queue items with proper distribution
        let queue_items = self.generate_queue_items(50, &dataset.codebases, &dataset.campaigns).await?;
        dataset.add_queue_items(queue_items);
        
        Ok(dataset)
    }
    
    async fn generate_runs_history(
        &self,
        count: usize,
        codebases: &[Codebase],
        campaigns: &[Campaign],
    ) -> Result<Vec<Run>, TestError> {
        let mut runs = Vec::new();
        let mut rng = StdRng::seed_from_u64(42); // Deterministic for reproducible tests
        
        for _ in 0..count {
            let codebase = codebases.choose(&mut rng).unwrap();
            let campaign = campaigns.choose(&mut rng).unwrap();
            
            let run = self.generate_realistic_run(codebase, campaign, &mut rng)?;
            runs.push(run);
        }
        
        // Sort by start time for realistic temporal patterns
        runs.sort_by_key(|r| r.start_time);
        
        Ok(runs)
    }
    
    fn generate_realistic_run(
        &self,
        codebase: &Codebase,
        campaign: &Campaign,
        rng: &mut StdRng,
    ) -> Result<Run, TestError> {
        // Generate realistic run characteristics based on campaign type
        let (result_code, duration) = match campaign.name.as_str() {
            "lintian-fixes" => {
                // Lintian fixes tend to be quick and successful
                let success_rate = 0.8;
                if rng.gen_bool(success_rate) {
                    ("success".to_string(), Duration::from_secs(rng.gen_range(300..1800)))
                } else {
                    ("build-failed".to_string(), Duration::from_secs(rng.gen_range(60..300)))
                }
            }
            "new-upstream" => {
                // New upstream tends to be slower and more error-prone
                let success_rate = 0.6;
                if rng.gen_bool(success_rate) {
                    ("success".to_string(), Duration::from_secs(rng.gen_range(1800..7200)))
                } else {
                    ("upstream-not-found".to_string(), Duration::from_secs(rng.gen_range(300..900)))
                }
            }
            _ => {
                ("success".to_string(), Duration::from_secs(rng.gen_range(300..3600)))
            }
        };
        
        let start_time = self.generate_realistic_timestamp(rng);
        let finish_time = start_time + duration;
        
        Ok(Run {
            id: Uuid::new_v4(),
            codebase: codebase.name.clone(),
            campaign: campaign.name.clone(),
            start_time,
            finish_time: Some(finish_time),
            result_code: Some(result_code),
            description: format!("Test run for {} - {}", codebase.name, campaign.name),
            context: None,
            value: Some(rng.gen_range(0.0..1000.0)),
            result: self.generate_realistic_result(campaign, rng),
            logfilenames: vec![format!("{}.log", Uuid::new_v4())],
            result_branches: self.generate_realistic_branches(campaign, rng),
            instigated_context: None,
        })
    }
}
```

### Test Environment Management
```rust
pub struct TestEnvironmentManager {
    containers: ContainerManager,
    databases: DatabaseManager,
    configs: ConfigManager,
}

impl TestEnvironmentManager {
    pub async fn setup_hybrid_environment(&self, config: TestConfig) -> Result<TestEnvironment, TestError> {
        // Start database containers
        let postgres = self.containers.start_postgres().await?;
        let redis = self.containers.start_redis().await?;
        
        // Setup test databases
        let test_db = self.databases.create_test_database(&postgres).await?;
        self.databases.run_migrations(&test_db).await?;
        self.databases.seed_test_data(&test_db, &config.test_data).await?;
        
        // Start Rust services
        let rust_services = self.start_rust_services(&config.rust_services, &test_db, &redis).await?;
        
        // Start Python services
        let python_services = self.start_python_services(&config.python_services, &test_db, &redis).await?;
        
        // Wait for all services to be ready
        self.wait_for_services_ready(&rust_services, &python_services).await?;
        
        Ok(TestEnvironment {
            postgres,
            redis,
            test_db,
            rust_services,
            python_services,
        })
    }
    
    async fn start_rust_services(
        &self,
        configs: &[RustServiceConfig],
        database: &TestDatabase,
        redis: &RedisContainer,
    ) -> Result<Vec<RustServiceHandle>, TestError> {
        let mut services = Vec::new();
        
        for config in configs {
            let service_config = self.configs.generate_rust_config(config, database, redis)?;
            let service = self.containers.start_rust_service(config, service_config).await?;
            services.push(service);
        }
        
        Ok(services)
    }
    
    async fn wait_for_services_ready(
        &self,
        rust_services: &[RustServiceHandle],
        python_services: &[PythonServiceHandle],
    ) -> Result<(), TestError> {
        let timeout = Duration::from_secs(60);
        let start_time = Instant::now();
        
        loop {
            if start_time.elapsed() > timeout {
                return Err(TestError::ServiceStartupTimeout);
            }
            
            let all_ready = self.check_all_services_ready(rust_services, python_services).await?;
            if all_ready {
                break;
            }
            
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        
        Ok(())
    }
}
```

## Continuous Integration Integration

### Test Pipeline Configuration
```yaml
# .github/workflows/integration-tests.yml
name: Cross-Service Integration Tests

on:
  pull_request:
    paths:
      - 'runner/**'
      - 'publish/**'
      - 'py/janitor/**'
      - 'tests/integration/**'

jobs:
  integration-tests:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        test-scenario:
          - api-compatibility
          - data-consistency
          - workflow-complete
          - performance-baseline
    
    services:
      postgres:
        image: postgres:14
        env:
          POSTGRES_PASSWORD: test
          POSTGRES_DB: janitor_test
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
      
      redis:
        image: redis:7
        options: >-
          --health-cmd "redis-cli ping"
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5

    steps:
      - uses: actions/checkout@v3
      
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          
      - name: Setup Python
        uses: actions/setup-python@v4
        with:
          python-version: '3.11'
          
      - name: Install dependencies
        run: |
          cargo build --workspace
          pip install -e .[dev]
          
      - name: Run integration tests
        run: |
          cargo test --package integration-tests --test ${{ matrix.test-scenario }}
        env:
          DATABASE_URL: postgresql://postgres:test@localhost:5432/janitor_test
          REDIS_URL: redis://localhost:6379
          RUST_LOG: info
          
      - name: Upload test results
        uses: actions/upload-artifact@v3
        if: always()
        with:
          name: integration-test-results-${{ matrix.test-scenario }}
          path: |
            target/integration-test-results/
            target/performance-reports/
```

### Test Reporting
```rust
pub struct IntegrationTestReporter {
    results: TestResults,
    metrics: TestMetrics,
    config: ReportConfig,
}

impl IntegrationTestReporter {
    pub async fn generate_report(&self) -> Result<TestReport, ReportError> {
        let mut report = TestReport::new();
        
        // Compatibility test results
        report.add_section(self.generate_compatibility_section().await?);
        
        // Performance comparison
        report.add_section(self.generate_performance_section().await?);
        
        // Data consistency results
        report.add_section(self.generate_consistency_section().await?);
        
        // Workflow test results
        report.add_section(self.generate_workflow_section().await?);
        
        // Generate executive summary
        report.add_summary(self.generate_executive_summary().await?);
        
        Ok(report)
    }
    
    async fn generate_performance_section(&self) -> Result<ReportSection, ReportError> {
        let mut section = ReportSection::new("Performance Comparison");
        
        // Response time comparisons
        let response_times = self.metrics.get_response_time_comparison().await?;
        section.add_chart(Chart::new("Response Times")
            .add_series("Rust Services", response_times.rust_percentiles)
            .add_series("Python Services", response_times.python_percentiles)
        );
        
        // Throughput comparisons
        let throughput = self.metrics.get_throughput_comparison().await?;
        section.add_chart(Chart::new("Throughput")
            .add_series("Rust Services", throughput.rust_rps)
            .add_series("Python Services", throughput.python_rps)
        );
        
        // Resource usage
        let resources = self.metrics.get_resource_usage_comparison().await?;
        section.add_table(Table::new("Resource Usage")
            .add_row(vec!["Service Type", "CPU %", "Memory MB", "DB Connections"])
            .add_row(vec!["Rust", &resources.rust_cpu.to_string(), &resources.rust_memory.to_string(), &resources.rust_db_conns.to_string()])
            .add_row(vec!["Python", &resources.python_cpu.to_string(), &resources.python_memory.to_string(), &resources.python_db_conns.to_string()])
        );
        
        Ok(section)
    }
}
```

## Success Metrics

### Functional Correctness
- **API Compatibility**: 100% request/response format compatibility
- **Data Consistency**: Zero data inconsistencies between services
- **Workflow Completeness**: All end-to-end workflows pass successfully
- **Error Handling**: Consistent error propagation across service boundaries

### Performance Requirements
- **Response Time Parity**: Rust services ≤ Python service response times
- **Throughput Improvement**: Rust services ≥ 2x Python service throughput
- **Resource Efficiency**: Rust services use ≤ 50% memory of Python equivalents
- **Error Rates**: < 0.1% error rate under normal load

### Reliability Requirements
- **Test Coverage**: > 90% of service interactions covered by integration tests
- **Test Stability**: < 1% flaky test rate
- **Environment Consistency**: Identical test results across environments
- **Migration Safety**: Zero production incidents during service migrations

## Conclusion

The cross-service integration testing strategy provides comprehensive coverage for the complex hybrid environment during the Janitor platform migration. This approach ensures:

1. **Service Compatibility**: Thorough validation of API and data format compatibility
2. **Data Integrity**: Comprehensive testing of database state consistency
3. **Workflow Continuity**: End-to-end validation of business processes
4. **Performance Validation**: Benchmarking and comparison of service performance
5. **Migration Safety**: Risk mitigation through extensive testing

**Implementation Priority:**
1. **Phase 1**: API compatibility testing framework
2. **Phase 2**: Database consistency validation
3. **Phase 3**: End-to-end workflow testing
4. **Phase 4**: Performance and load testing integration

The testing strategy supports the overall migration goals by providing confidence in service migrations, enabling gradual rollouts, and ensuring production stability throughout the transition period.