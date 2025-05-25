# Runner API Comparison: Python vs Rust

This document compares the API endpoints between the Python implementation (`py/janitor/runner.py`) and the Rust implementation (`runner/src/web.rs`).

## Core Queue Management Endpoints

| Endpoint | Python | Rust | Status | Notes |
|----------|---------|------|--------|-------|
| `GET /queue/position` | ✅ | ✅ | ✅ PARITY | Query queue position for codebase/campaign |
| `POST /schedule-control` | ✅ | ✅ | ✅ PARITY | Schedule control runs |
| `POST /schedule` | ✅ | ✅ | ✅ PARITY | Schedule new runs |
| `GET /status` | ✅ | ✅ | ✅ PARITY | Get runner status |
| `GET /queue` | ✅ | ✅ | ✅ PARITY | List queue items |

## Active Run Management 

| Endpoint | Python | Rust | Status | Notes |
|----------|---------|------|--------|-------|
| `GET /active-runs` | ✅ | ✅ | ✅ PARITY | List all active runs |
| `GET /active-runs/{id}` | ✅ | ✅ | ✅ PARITY | Get specific active run |
| `POST /active-runs` | ✅ | ✅ | ✅ PARITY | Assign work (named `assign` in Python) |
| `GET /active-runs/+peek` | ✅ | ✅ | ✅ PARITY | Peek at next assignment |
| `POST /active-runs/{id}/finish` | ✅ | ✅ | ✅ PARITY | Finish run |

## Enhanced Rust-only Endpoints

| Endpoint | Python | Rust | Status | Notes |
|----------|---------|------|--------|-------|
| `POST /active-runs/{id}/finish-multipart` | ❌ | ✅ | 🆕 NEW | Multipart form upload for results |
| `POST /runner/active-runs` | ❌ | ✅ | 🆕 NEW | Worker-authenticated assignment |
| `POST /runner/active-runs/{id}/finish` | ❌ | ✅ | 🆕 NEW | Worker-authenticated finish |
| `POST /runner/active-runs/{id}/finish-multipart` | ❌ | ✅ | 🆕 NEW | Worker-authenticated multipart finish |
| `GET /runner/active-runs/{id}` | ❌ | ✅ | 🆕 NEW | Worker-authenticated run get |

## Log Management

| Endpoint | Python | Rust | Status | Notes |
|----------|---------|------|--------|-------|
| `GET /log/{run_id}` | ✅ | ✅ | ✅ PARITY | List log files for run |
| `GET /log/{run_id}/{filename}` | ✅ | ✅ | ✅ PARITY | Get specific log file |
| `POST /kill/{run_id}` | ✅ | ✅ | ✅ PARITY | Kill active run |

## Codebase/Candidate Management

| Endpoint | Python | Rust | Status | Notes |
|----------|---------|------|--------|-------|
| `GET /codebases` | ✅ | ✅ | ✅ PARITY | Download codebases |
| `POST /codebases` | ✅ | ✅ | ✅ PARITY | Upload codebases |
| `GET /candidates` | ✅ | ✅ | ✅ PARITY | Download candidates |
| `POST /candidates` | ✅ | ✅ | ✅ PARITY | Upload candidates |
| `DELETE /candidates/{id}` | ✅ | ✅ | ✅ PARITY | Delete candidate |

## Run Management

| Endpoint | Python | Rust | Status | Notes |
|----------|---------|------|--------|-------|
| `GET /runs/{id}` | ✅ | ✅ | ✅ PARITY | Get run details |
| `POST /runs/{id}` | ✅ | ✅ | ✅ PARITY | Update run |

## Health/Status Endpoints

| Endpoint | Python | Rust | Status | Notes |
|----------|---------|------|--------|-------|
| `GET /health` | ✅ | ✅ | ✅ PARITY | Health check |
| `GET /ready` | ✅ | ✅ | ✅ PARITY | Readiness check |

## Enhanced Rust-only Features

| Endpoint | Python | Rust | Status | Notes |
|----------|---------|------|--------|-------|
| `GET /metrics` | ❌ | ✅ | 🆕 NEW | Prometheus metrics |
| `GET /queue/stats` | ❌ | ✅ | 🆕 NEW | Queue statistics |
| `GET /watchdog/health` | ❌ | ✅ | 🆕 NEW | Watchdog health status |

## Admin Endpoints (Rust-only)

| Endpoint | Python | Rust | Status | Notes |
|----------|---------|------|--------|-------|
| `GET /admin/workers` | ❌ | ✅ | 🆕 NEW | List workers |
| `POST /admin/workers` | ❌ | ✅ | 🆕 NEW | Create worker |
| `DELETE /admin/workers/{name}` | ❌ | ✅ | 🆕 NEW | Delete worker |
| `GET /admin/security/stats` | ❌ | ✅ | 🆕 NEW | Security statistics |

## Resume System (Rust-only)

| Endpoint | Python | Rust | Status | Notes |
|----------|---------|------|--------|-------|
| `POST /resume/check` | ❌ | ✅ | 🆕 NEW | Check resume information |
| `GET /resume/chain/{run_id}` | ❌ | ✅ | 🆕 NEW | Get resume chain |
| `GET /resume/descendants/{run_id}` | ❌ | ✅ | 🆕 NEW | Get resume descendants |
| `GET /resume/validate` | ❌ | ✅ | 🆕 NEW | Validate resume consistency |

## Summary

- **Full Parity**: 17 endpoints have complete parity between Python and Rust
- **Enhanced Features**: 16 additional endpoints in Rust providing enhanced functionality
- **Missing Features**: 0 Python endpoints are missing from Rust
- **Backward Compatibility**: ✅ Complete - all Python endpoints are supported
- **Enhanced Security**: Worker authentication layer in Rust
- **Enhanced Monitoring**: Metrics, health checks, admin endpoints
- **Enhanced Resume System**: Comprehensive resume functionality