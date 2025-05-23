# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

The Janitor is an automated VCS change management platform that orchestrates large-scale code improvements across repositories. It is built on top of [silver-platter](https://github.com/jelmer/silver-platter) and consists of multiple interconnected services that work together to make, build, and publish automated changes.

## Essential Commands

### Development Setup
```bash
# Install Python dependencies
pip3 install --editable .[dev]

# Build Python extensions in-place
make build-inplace

# Full build (generates CSS and builds extensions)
make core
```

### Testing
```bash
# Run all tests (Python and Rust)
make test

# Run only Python tests
PYTHONPATH=$(pwd)/py:$PYTHONPATH PROTOCOL_BUFFERS_PYTHON_IMPLEMENTATION=python python3 -m pytest -vv tests

# Run only Rust tests
cargo test
```

### Code Quality
```bash
# Run all checks (typing, tests, style, formatting)
make check

# Type checking
make typing

# Linting
make ruff

# Format all code
make reformat

# Check formatting without changes
make check-format
```

### Building Docker Images
```bash
# Build specific service
make build-runner

# Build all services
make build-all
```

## Architecture

### Core Services

The Janitor operates as a distributed system with these main components:

- **Runner** (`runner/`): Queue management, schedules work for candidates across codebases, fetches results from workers
- **Worker** (`worker/`): Executes codemod commands, builds projects, uploads results
- **Publisher** (`publish/`): Creates merge proposals or pushes changes, handles rate limiting
- **Site** (`site/`): Web interface and public API
- **VCS Stores** (`git-store/`, `bzr-store/`): HTTP-accessible Git/Bazaar repositories with web UI
- **Differ** (`differ/`): Generates diffs between runs using diffoscope/debdiff
- **Archive** (`archive/`): Debian-specific APT repository generation
- **Auto-upload** (`auto-upload/`): Debian-specific automated package uploads

### Language Distribution

The codebase is hybrid Rust/Python with ongoing migration from Python to Rust:
- **Rust**: Core services, new development should use Rust
- **Python**: Legacy code, web templates, some utilities (in `py/janitor/`)

### Key Concepts

- **Codebase**: A VCS location where changes can be made (usually a repository branch)
- **Campaign**: An effort to fix a particular issue across multiple codebases
- **Candidate**: A record that a campaign should run against a specific codebase
- **Run**: The result of executing a candidate, with artifacts, logs, and status

### Data Flow

1. Codebases and candidates are uploaded to the system
2. Runner schedules work based on priority scores and success likelihood
3. Workers fetch assignments, execute codemods, build, and upload results
4. Publisher processes successful runs according to publish policies
5. Site provides web interface for monitoring and management

## Workspace Structure

The repository uses Cargo workspaces with these main crates:
- Root crate provides shared libraries and utilities
- Each service has its own crate (e.g., `runner/`, `publish/`)
- Python bindings are in `*-py/` crates (e.g., `runner-py/`)
- Common functionality is in `common-py/`

## Development Guidelines

### Language Choice
New code should be written in Rust unless there's a specific reason to use Python. The project is migrating from Python to Rust.

### Testing
All new code must be covered by tests. Use pytest for Python and Rust's built-in test framework for Rust code.

### Code Style
- Use `rustfmt` for Rust code
- Use `ruff format` for Python code
- Run `make reformat` to format all code

### Dependencies
Check existing usage before adding new dependencies. Look at neighboring files, `Cargo.toml`, or `pyproject.toml` to understand what's already available.