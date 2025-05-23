# Runner Python to Rust Porting Plan

## Overview

This document outlines the plan for porting the remaining functionality from `py/janitor/runner.py` to the Rust `runner/` crate. The Python runner is a comprehensive queue management and build orchestration service that manages work assignments, monitors active runs, and coordinates with workers.

## Current State

### Already Ported (Rust)
- Basic web server framework using Axum
- Stub endpoints for most HTTP routes
- Basic configuration and CLI argument parsing
- Some utility functions (`committer_env`, `find_changes`, `is_log_filename`, etc.)

### Remaining in Python (~3,200 lines)
- Core queue processing logic
- Database interactions
- Active run management
- Builder system
- Result processing
- Full web API implementation
- Background task management

## Major Components to Port

### 1. Data Models and Structures
- `JanitorResult` - Core result structure for completed runs
- `WorkerResult` - Result data from workers  
- `ActiveRun` - Tracking active/running jobs
- `BuilderResult` and subclasses (`DebianResult`, `GenericResult`)
- `Builder` and subclasses (`DebianBuilder`, `GenericBuilder`)

### 2. Queue Processing System
- `QueueProcessor` class - Core orchestration logic
- Queue assignment algorithm
- Run timeout management  
- Active run monitoring/watchdog
- Rate limiting per host

### 3. Backchannel Communication
- `Backchannel` trait and implementations
- `JenkinsBackchannel` - Jenkins integration
- `PollingBackchannel` - Direct worker communication
- Health checking and keepalive logic

### 4. Database Layer
- PostgreSQL connection management
- Run state tracking queries
- Queue item management
- Active run persistence
- Result storage

### 5. Web API Endpoints
All HTTP routes need full implementation for queue management, run management, results/logs, and codebase management.

### 6. Builder System
- Generic builder for non-Debian targets
- Debian builder with package-specific logic
- Build configuration generation
- Environment variable setup
- Result artifact handling

### 7. Integration Systems
- VCS management integration
- Artifact storage integration  
- Log file management
- Redis state management
- Metrics and monitoring (Prometheus)

## Recommended Porting Order

### Phase 1: Foundation
**Dependencies: None**
1. **Data Models First**
   - Port `JanitorResult`, `WorkerResult`, `ActiveRun` structs
   - Port `BuilderResult` enum and variants (`DebianResult`, `GenericResult`)
   - Add serde serialization/deserialization
   - Port JSON conversion methods

2. **Database Connection Layer**
   - Set up sqlx connection pooling
   - Port basic database connection management
   - Add configuration for database URLs

### Phase 2: Read-Only Operations
**Dependencies: Phase 1**
3. **Simple Read Endpoints**
   - `GET /health` and `GET /ready` (already working)
   - `GET /status` - system status information
   - `GET /queue` - queue listing (read-only view)
   - `GET /queue/position` - position lookup

4. **Log and Run Retrieval**
   - `GET /log/{id}` - log file index
   - `GET /log/{id}/{filename}` - individual log files  
   - `GET /runs/{id}` - get run results
   - `GET /active-runs` - list active runs
   - `GET /active-runs/{id}` - get active run details

### Phase 3: Core Queue Logic
**Dependencies: Phase 2**
5. **Queue Assignment Core**
   - Port queue item data structures
   - Basic queue assignment algorithm (without all features)
   - `POST /active-runs` - assign work to workers (minimal version)
   - Database queries for claiming queue items

6. **Result Processing**
   - `POST /runs/{id}` - update run results
   - `POST /active-runs/{id}/finish` - complete run
   - Basic result storage to database

### Phase 4: Builder System
**Dependencies: Phase 3**
7. **Builder Infrastructure**
   - Port `Builder` trait and basic implementations
   - `GenericBuilder` implementation
   - Build configuration generation
   - Environment variable setup

8. **Debian Builder**
   - Port `DebianBuilder` with package-specific logic
   - Debian result processing and artifact handling
   - Integration with existing Debian functionality in other crates

### Phase 5: Backchannel Communication
**Dependencies: Phase 4**
9. **Backchannel System**
   - Define async `Backchannel` trait
   - `PollingBackchannel` implementation for direct worker communication
   - Basic health checking and keepalive logic

10. **Jenkins Integration**
    - `JenkinsBackchannel` implementation
    - Jenkins-specific health checking and log retrieval

### Phase 6: Advanced Queue Features
**Dependencies: Phase 5**
11. **Queue Management**
    - Advanced queue assignment with priorities and scoring
    - Rate limiting per host
    - `GET /active-runs/+peek` - peek next assignment
    - Queue position calculation and ordering

12. **Scheduling System**
    - `POST /schedule` - manual scheduling
    - `POST /schedule-control` - schedule control
    - Integration with existing scheduling logic

### Phase 7: Active Run Monitoring
**Dependencies: Phase 6**
13. **Watchdog System**
    - Background task for monitoring active runs
    - Timeout detection and handling
    - Run abortion and cleanup logic
    - Health checking integration with backchannels

14. **Run Management**
    - `POST /kill/{run_id}` - terminate runs
    - Cleanup and state management for failed/aborted runs
    - Retry and recovery logic

### Phase 8: Codebase Management
**Dependencies: Phase 7**
15. **Codebase Operations**
    - `GET /codebases` - download codebase list
    - `POST /codebases` - upload codebase updates
    - `GET /candidates` - download candidates
    - `POST /candidates` - upload candidates  
    - `DELETE /candidates/{id}` - remove candidate

### Phase 9: Integration and Polish
**Dependencies: Phase 8**
16. **External Integrations**
    - VCS management integration
    - Artifact storage integration (GCS, local filesystem)
    - Log file management integration
    - Redis state management for coordination

17. **Metrics and Monitoring**
    - Prometheus metrics collection
    - Performance monitoring
    - Error tracking and logging

## Migration Strategy

### Gradual Migration Approach
1. **Start with read-only endpoints** - lowest risk, can run alongside Python
2. **Add simple write operations** - run updates, basic assignment
3. **Migrate core queue logic** - most complex but critical functionality  
4. **Add advanced features** - scheduling, monitoring, management
5. **Complete integration** - external systems and monitoring

### Testing Strategy
- Unit tests for each phase before moving to next
- Integration tests against test database
- Canary deployment with traffic splitting
- Performance benchmarks at each phase

### Compatibility Maintenance
- Preserve HTTP API contracts exactly
- Maintain database schema compatibility
- Keep same configuration format
- Ensure seamless transition for clients

This order ensures that each phase builds on the previous one, with clear dependencies and the ability to test incrementally while maintaining a working system throughout the migration.