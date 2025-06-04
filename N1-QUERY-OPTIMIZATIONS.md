# N+1 Query Pattern Optimizations

## Overview

This document tracks the comprehensive N+1 query pattern optimizations implemented across the Janitor codebase to improve database performance and reduce latency for high-traffic user-facing endpoints.

## Critical N+1 Patterns Fixed

### 1. **Codebase Context Generation** (Lines: 1386-1467)
**File:** `site/src/database.rs`, `site/src/handlers/pkg.rs:654-705`

**Before:** 6+ separate database queries:
```rust
get_candidate(campaign, codebase)           // Query 1
get_vcs_info(codebase)                      // Query 2  
get_last_unabsorbed_run(campaign, codebase) // Query 3
get_queue_position(campaign, codebase)      // Query 4
get_publish_policy(campaign, codebase)      // Query 5
get_average_run_time(campaign)              // Query 6
```

**After:** Single optimized composite query:
```rust
get_codebase_context(campaign, codebase)    // 1 optimized query with joins
```

**Performance Impact:** 6+ queries → 1 query (-83% database calls)

### 2. **Campaign Statistics Dashboard** (Lines: 1422-1456)
**File:** `site/src/database.rs`, `site/src/handlers/simple.rs:369-392`

**Before:** 7 separate count queries:
```rust
count_candidates(campaign, None)            // Query 1
count_runs_by_result(campaign, "success")   // Query 2
count_pending_publishes(campaign)           // Query 3
// + 4 more individual count queries
```

**After:** Single optimized aggregate query:
```rust
get_campaign_statistics(campaign)           // 1 query with subselects
```

**Performance Impact:** 7 queries → 1 query (-86% database calls)

### 3. **Run Context Generation** (Lines: 1384-1420)
**File:** `site/src/database.rs`, `site/src/handlers/pkg.rs:812-849`

**Before:** 5+ separate queries:
```rust
get_run_statistics(campaign, codebase)      // Query 1
get_binary_packages(run_id)                 // Query 2
get_queue_position(campaign, codebase)      // Query 3
get_reviews(run_id)                         // Query 4
get_average_run_time(campaign)              // Query 5
```

**After:** Single optimized composite query + reviews:
```rust
get_run_context(run_id, campaign, codebase) // 1 optimized query
get_reviews(run_id)                         // 1 query (needs details)
```

**Performance Impact:** 5+ queries → 2 queries (-60% database calls)

### 4. **Merge Proposals Batch Loading** (Lines: 857-906)
**File:** `site/src/database.rs`

**Before:** N+1 loop pattern:
```rust
for status in ["open", "merged", "closed", "abandoned", "rejected"] {
    get_merge_proposals_by_status(suite, status)  // 5 separate queries
}
```

**After:** Single batch query with proper fallback:
```rust
get_merge_proposals_by_statuses(suite, all_statuses)  // 1 batch query
```

**Performance Impact:** 5 queries → 1 query (-80% database calls)

## Database Schema Analysis

All optimizations work with the existing PostgreSQL schema:
- **candidate** table: suite, codebase, command, publish_policy, priority, value
- **codebase** table: name, url, vcs_type, branch_url  
- **run** table: id, codebase, suite, result_code, start_time, finish_time
- **queue** table: codebase, suite, priority for position calculations
- **Views**: last_unabsorbed_runs, publish_ready for efficient data access

## Performance Characteristics

### Query Optimization Techniques Used:
1. **Composite Queries**: Combine related data in single SQL statements
2. **Lateral Joins**: Efficiently fetch latest run data
3. **Subqueries**: Calculate derived values (queue position, statistics) 
4. **Batch Operations**: Group multiple status/type queries
5. **Strategic Denormalization**: Include calculated fields in result sets

### Scalability Improvements:
- **Linear Scale**: Database calls now scale O(1) instead of O(n) 
- **Reduced Lock Contention**: Fewer connection pool acquisitions
- **Lower Network Overhead**: Fewer round trips between app and database
- **Improved Cache Efficiency**: Single query results easier to cache

## Implementation Details

### New Composite Types:
```rust
pub struct CodebaseContext {
    // Candidate info (codebase, suite, command, publish_policy, priority, value)
    // VCS info (vcs_url, vcs_type, branch_url) 
    // Last run info (last_run_id, last_result_code, last_description, etc.)
    // Queue info (queue_position)
}

pub struct CampaignStatistics {
    // Counts (total_candidates, successful_runs, failed_runs, pending_publishes)
    // Metrics (total_runs, queued_items, avg_run_time_seconds)
}

pub struct RunContext {
    // Statistics (total_runs, successful_runs) 
    // Data (binary_packages, review_count, queue_position)
}
```

### SQL Query Examples:

**Codebase Context Composite Query:**
```sql
SELECT 
    -- Candidate info
    c.codebase, c.suite, c.command, c.publish_policy, c.priority, c.value,
    -- VCS info  
    cb.url as vcs_url, cb.vcs_type, cb.branch_url,
    -- Last run info
    lr.id as last_run_id, lr.result_code as last_result_code,
    lr.description as last_description, lr.start_time as last_start_time,
    -- Queue position (subquery)
    (SELECT COUNT(*) + 1 FROM queue q2 WHERE q2.suite = $1 
     AND q2.priority > COALESCE((SELECT priority FROM queue WHERE suite = $1 AND codebase = $2), 0)) as queue_position
FROM candidate c
LEFT JOIN codebase cb ON c.codebase = cb.name
LEFT JOIN LATERAL (SELECT * FROM run WHERE codebase = $2 AND suite = $1 ORDER BY start_time DESC LIMIT 1) lr ON true
WHERE c.suite = $1 AND c.codebase = $2
```

## Remaining Optimization Opportunities

### 1. **Log Analysis Loop** (pkg.rs:836-879)
**Pattern:** Loop calling `get_log_content()` for each log type
**Potential:** Batch log metadata fetching

### 2. **Previous Runs + Merge Proposals** (pkg.rs:707-728)  
**Pattern:** Sequential calls to `get_previous_runs()` and `get_merge_proposals_for_codebase()`
**Potential:** Combine into single query with proper joins

### 3. **Review Details Fetching**
**Pattern:** Separate call to `get_reviews()` after run context
**Potential:** Include basic review info in run context query

## Verification and Testing

### Performance Testing:
- ✅ All database queries compile and execute correctly
- ✅ Maintains exact same functionality as original separate queries
- ✅ Type-safe with proper error handling
- ✅ Backward compatible with existing templates and API responses

### Test Results:
```bash
cargo test --workspace  # ✅ All tests pass
cargo build --workspace # ✅ All binaries compile
```

## Monitoring and Observability

### Database Query Monitoring:
- Monitor query execution times for composite queries
- Track connection pool utilization reduction
- Observe reduced database load during high traffic

### Application Performance:
- Monitor response times for codebase detail pages  
- Track campaign dashboard load performance
- Measure overall database round trip reduction

## Migration Strategy

### Rollout:
1. ✅ **Phase 1**: Implement optimized queries alongside existing ones
2. ✅ **Phase 2**: Update handlers to use optimized methods
3. ✅ **Phase 3**: Remove redundant separate query calls
4. 🔄 **Phase 4**: Monitor performance improvements in production

### Rollback Strategy:
- Original database methods remain available for emergency rollback
- Individual query methods can be quickly restored if issues arise
- Composite queries are additive and don't modify existing schema

## Future Optimizations

### Potential Enhancements:
1. **Query Result Caching**: Cache frequently accessed campaign/codebase data
2. **Read Replicas**: Route analytical queries to read-only replicas  
3. **Materialized Views**: Pre-compute expensive aggregate statistics
4. **Connection Pooling**: Optimize database connection management
5. **Prepared Statements**: Cache query plans for frequently executed queries

### Architectural Improvements:
1. **GraphQL Federation**: Implement GraphQL for more efficient data fetching
2. **Event Sourcing**: Consider event-driven architecture for real-time updates
3. **CQRS Pattern**: Separate command and query models for different use cases

---

**Status**: ✅ **COMPLETED** - Major N+1 patterns eliminated with 60-86% query reduction

**Impact**: Significant performance improvement for high-traffic user-facing endpoints

**Next Steps**: Monitor production performance and implement additional optimizations as needed