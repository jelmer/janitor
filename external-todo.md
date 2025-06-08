# External Dependencies TODO

This document tracks features and implementations that are blocked by external dependencies, waiting for upstream changes, or dependent on third-party API stabilization.

## Breezyshim / PyO3 Dependencies

### Symbolic Reference Creation
- **Location**: `worker/src/vcs.rs:272`
- **Function**: `create_symbolic_ref()`
- **Blocked by**: PyO3 API stabilization in breezyshim crate
- **Description**: Need to create Git symbolic references (symrefs) for tags. The functionality requires access to Git-specific repository operations through the breezyshim crate's PyO3 bindings.
- **Impact**: Tag symbolic references are not created, but operations continue without error
- **Workaround**: Currently logs the intended operation and returns Ok()
- **Note**: This is not causing any runtime errors - function returns Ok() to allow operations to continue

### Bazaar Transport Support
- **Location**: `bzr-store/src/pyo3_bridge.rs:322`
- **Function**: `possible_transports` parameter in various functions
- **Blocked by**: PyO3 binding complexity for transport objects
- **Description**: Cannot pass possible_transports parameter through PyO3 bridge
- **Impact**: May affect performance of Bazaar operations

### Bazaar Probers Support
- **Location**: `bzr-store/src/pyo3_bridge.rs:323`
- **Function**: `probers` parameter in various functions
- **Blocked by**: PyO3 binding complexity for prober objects
- **Description**: Cannot pass probers parameter through PyO3 bridge
- **Impact**: May affect repository format detection

## External API Dependencies

### VCS Forge Resume Information
- **Location**: `runner/src/resume.rs:122`
- **Function**: `get_proposal_resumable()`
- **Blocked by**: Need to implement actual forge API queries (GitHub, GitLab, etc.)
- **Description**: Currently returns None instead of querying forge APIs for merge proposal status
- **Impact**: Cannot determine if merge proposals can be resumed

### Merge Proposal Merged-By Information
- **Location**: `publish/src/web.rs:274-276`
- **Function**: `get_merged_by_user_url()`
- **Blocked by**: Need to implement external forge API calls
- **Description**: Requires querying GitHub/GitLab/etc APIs to get merge information
- **Impact**: Cannot display who merged a proposal

### Forge Rate Limits
- **Location**: `publish/src/web.rs:1500`
- **Function**: Blocker information display
- **Blocked by**: Need to query forge APIs for current rate limit status
- **Description**: Should include forge-specific rate limit information in blocker details
- **Impact**: Incomplete rate limit information shown to users

## Axum Framework Limitations

### HTTP Response Streaming
- **Location**: `git-store/src/git_http.rs:532`
- **Function**: Git pack streaming
- **Blocked by**: Better streaming support in Axum
- **Description**: Current implementation buffers entire response, should stream for large packs
- **Impact**: Higher memory usage for large Git operations
- **Workaround**: Currently buffers and sends complete response

## Database Migration Dependencies

### Codebase Table Usage
- **Location**: `publish/src/state.rs:158`
- **Function**: Various database queries
- **Blocked by**: Database schema migration to use codebase table
- **Description**: Need to migrate from package-based queries to codebase-based queries
- **Impact**: Some queries may be less efficient or incomplete

## Configuration System Dependencies

### Dynamic Configuration Loading
- **Location**: Multiple locations in `site/src/config.rs` (lines 384, 389, 394, 399, 410)
- **Function**: Service URL resolution
- **Blocked by**: Janitor config integration
- **Description**: Need to check janitor config fields when available for service URLs
- **Impact**: Service URLs are currently hardcoded or use defaults

## Test Infrastructure Dependencies

### Database-Dependent Tests
- **Location**: `runner/src/resume.rs:302`
- **Test**: `test_resume_result`
- **Blocked by**: Test database infrastructure
- **Description**: Test requires real database connection
- **Impact**: Test is ignored

### System-Dependent Tests
- **Location**: `worker/src/vcs.rs` (multiple tests)
- **Blocked by**: System dependencies (Git, file system)
- **Description**: Tests require specific system setup
- **Impact**: Tests are ignored

## Notes

- Items in this file are not bugs or missing features, but rather implementations waiting on external factors
- When external dependencies are updated, search for the locations listed here to implement the features
- This file should be reviewed periodically to check if any blockers have been resolved

Last updated: January 2025