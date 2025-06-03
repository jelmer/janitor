# Git Store Service

The Git Store service provides HTTP-accessible Git repositories with administrative and public interfaces. It's part of the Janitor platform's VCS management infrastructure.

## Features

- **Dual HTTP interfaces**: Administrative (full access) and public (read-only) endpoints
- **Git diff API**: Generate diffs between commits via HTTP API
- **Revision info API**: Get commit metadata and history
- **Repository management**: Auto-creation and management of bare Git repositories
- **Database integration**: Worker authentication and codebase validation
- **Web interface**: Basic repository browsing and listing
- **Health checks**: Health and readiness endpoints for monitoring

## Current Implementation Status

✅ **Phase 1 Complete**: Core Infrastructure
- Project setup with all dependencies
- Repository management with git2-rs
- Database integration with PostgreSQL
- Basic HTTP server with dual applications
- Configuration management
- Error handling and logging

🚧 **Phase 2 Planned**: Git Protocol Implementation
- Git HTTP backend integration (`git http-backend`)
- Request/response streaming
- Authentication and authorization
- Git command validation

🚧 **Phase 3 Planned**: API Endpoints
- Enhanced diff and revision APIs
- Repository management endpoints
- Content negotiation

🚧 **Phase 4 Planned**: Web Repository Browser
- File browsing and viewing
- Commit history navigation
- Template system

## Configuration

Copy `config.example.toml` to create your configuration:

```toml
# Basic configuration
local_path = "/srv/git"
public_url = "http://localhost:9422"
database_url = "postgresql://janitor:password@localhost/janitor"
admin_port = 9421
public_port = 9422
```

## Usage

### Running the service

```bash
# With configuration file
cargo run --bin git-store -- config.toml

# With environment variables
GIT_STORE_LOCAL_PATH=/srv/git \
GIT_STORE_DATABASE_URL=postgresql://localhost/janitor \
cargo run --bin git-store
```

### Environment Variables

All configuration options can be set via environment variables with the `GIT_STORE_` prefix:

- `GIT_STORE_LOCAL_PATH`: Path to git repositories
- `GIT_STORE_DATABASE_URL`: PostgreSQL connection string
- `GIT_STORE_ADMIN_PORT`: Admin interface port
- `GIT_STORE_PUBLIC_PORT`: Public interface port

### API Endpoints

#### Admin Interface (port 9421)
- `GET /health` - Health check
- `GET /ready` - Readiness check (includes database)
- `GET /` - List repositories
- `GET /:codebase` - Repository information
- `POST /:codebase/remote/:name` - Set repository remote
- `GET /:codebase/diff?old=SHA&new=SHA` - Generate diff
- `GET /:codebase/revision?rev=SHA` - Get revision info

#### Public Interface (port 9422)
- `GET /health` - Health check
- `GET /` - List repositories
- `GET /:codebase` - Repository information
- `GET /:codebase/diff?old=SHA&new=SHA` - Generate diff (read-only)
- `GET /:codebase/revision?rev=SHA` - Get revision info

## Development

### Testing

```bash
# Run tests
cargo test -p janitor-git-store

# Run with logging
RUST_LOG=debug cargo test -p janitor-git-store
```

### Building

```bash
# Development build
cargo build -p janitor-git-store

# Release build
cargo build --release -p janitor-git-store
```

## Architecture

The service consists of several modules:

- **config**: Configuration management with TOML and environment support
- **database**: PostgreSQL integration for worker auth and codebase validation
- **error**: Centralized error handling with HTTP status mapping
- **git_http**: Git HTTP protocol handlers and diff/revision APIs
- **repository**: Git repository management using git2-rs
- **web**: HTTP server setup with Axum framework

## Dependencies

- **Axum**: Modern async web framework
- **git2**: Git operations and repository management
- **sqlx**: Type-safe PostgreSQL integration
- **Tera**: Template engine for web interface
- **tokio**: Async runtime
- **tracing**: Structured logging

## Related Services

The Git Store integrates with other Janitor services:

- **Runner**: Uses git-store for repository access during builds
- **Worker**: Authenticates against git-store for repository operations
- **Site**: Links to git-store for repository browsing
- **Publisher**: May interact with git-store for publishing workflows

## TODO

This is Phase 1 implementation. Future phases will add:

- Git HTTP backend integration for full Git protocol support
- Enhanced web repository browser
- Performance optimizations
- Additional authentication methods
- Metrics and monitoring