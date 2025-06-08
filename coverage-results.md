# Cargo Tarpaulin Coverage Results

## Summary

| Crate | Coverage | Lines Covered | Files with 0% Coverage Needing Unit Tests |
|-------|----------|---------------|-------------------------------------------|
| janitor-differ | 3.33% | 136/4088 | `differ/src/test_utils.rs`, `differ/src/lib.rs` |
| janitor-worker | 4.13% | 232/5620 | `worker/src/tee.rs`, `worker/src/generic/mod.rs`, `worker/src/debian/build.rs` |
| janitor-runner | 6.44% | 358/5553 | `runner/src/config_generator.rs`, `runner/src/database.rs`, `runner/src/watchdog.rs`, `runner/src/web.rs` |
| janitor-publish | 0.13% | 8/6068 | `publish/src/proposal_info.rs`, `publish/src/publish_one.rs`, `publish/src/queue.rs`, `publish/src/redis.rs`, `publish/src/state.rs`, `publish/src/web.rs` |
| janitor-mail-filter | 1.46% | 19/1301 | (Most covered, but low overall coverage) |

## Files with 0% Coverage That Need Unit Tests

### janitor-differ
- `differ/src/test_utils.rs` - Test utilities module that could benefit from tests
- `differ/src/lib.rs` - Main library interface needs comprehensive testing

### janitor-worker  
- `worker/src/tee.rs` - Tee functionality for output streaming
- `worker/src/generic/mod.rs` - Generic worker implementation (250 lines uncovered)
- `worker/src/debian/build.rs` - Debian build functionality (123 lines uncovered)

### janitor-runner
- `runner/src/config_generator.rs` - Configuration generation (137 lines uncovered)
- `runner/src/database.rs` - Database operations (838 lines uncovered - critical!)
- `runner/src/watchdog.rs` - Watchdog monitoring functionality (211 lines uncovered)
- `runner/src/web.rs` - Web server endpoints (1057 lines uncovered)

### janitor-publish
- `publish/src/proposal_info.rs` - Proposal information management (131 lines uncovered)
- `publish/src/publish_one.rs` - Single publish operations (355 lines uncovered)
- `publish/src/queue.rs` - Queue management (154 lines uncovered)
- `publish/src/redis.rs` - Redis integration (101 lines uncovered)
- `publish/src/state.rs` - State management (114 lines uncovered)
- `publish/src/web.rs` - Web endpoints (512 lines uncovered)

## Recommendations

1. **Critical Priority**: Focus on `runner/src/database.rs` (838 uncovered lines) as database operations are fundamental to the system.

2. **High Priority**: 
   - `runner/src/web.rs` (1057 lines) - Web endpoints need testing for API stability
   - `publish/src/web.rs` (512 lines) - Another critical web interface
   - `publish/src/publish_one.rs` (355 lines) - Core publishing functionality

3. **Medium Priority**:
   - `worker/src/generic/mod.rs` (250 lines) - Generic worker logic
   - `runner/src/watchdog.rs` (211 lines) - Monitoring functionality
   - All queue/state management modules

4. **Low Priority**:
   - Test utility modules
   - Configuration generation modules

## Notes

- The overall coverage is very low across all crates (0.13% - 6.44%)
- Many core modules have 0% coverage, indicating missing unit tests
- Database, web endpoints, and core business logic modules should be prioritized for testing
- Consider adding integration tests in addition to unit tests for better coverage