# Phase 6E: Rust Aggregator HTTP Service Architecture

## 1. Executive Summary & Objective

Phase 6E establishes the first independently addressable HTTP service layer for Blindmark's cryptographic aggregators (`experiments/phase_6e/`).

Building upon the four-component architecture in `experiments/phase_6c/` and the corrected integration contract in Phase 6D, this crate wraps the Leader and Helper aggregators in separate HTTP service daemons.

> **CRITICAL ARCHITECTURAL BOUNDARY**:
> - The Leader Aggregator Service exposes an HTTP server (default port `8001`).
> - The Helper Aggregator Service exposes an independent HTTP server (default port `8002`).
> - Each service maintains its own distinct internal memory space, verification key, task status, and `AggregateShare` state.
> - The Collector is defined as an unshared result retrieval client interface.

---

## 2. Service Architecture Diagram

```
                            +--------------------------+
                            |   PARTICIPANT / CLIENT   |
                            +--------------------------+
                               /                    \
              POST /tasks/1/reports/0                POST /tasks/1/reports/1
             (Target: Leader ONLY)                  (Target: Helper ONLY)
                             /                        \
                            v                          v
    +-------------------------------+          +-------------------------------+
    |        LEADER SERVICE         |          |        HELPER SERVICE         |
    |      (HTTP Port: 8001)        |          |      (HTTP Port: 8002)        |
    |                               |          |                               |
    |  - Routes:                    |          |  - Routes:                    |
    |    GET  /health               |          |    GET  /health               |
    |    GET  /tasks/{id}/status    |  mTLS    |    GET  /tasks/{id}/status    |
    |    POST /tasks/{id}/reports/0 |<========>|    POST /tasks/{id}/reports/1 |
    |                               |  Prep    |                               |
    |  - State: LeaderAggregator    |  Loop    |  - State: HelperAggregator    |
    |  - Owns: AggregateShare_0     |          |  - Owns: AggregateShare_1     |
    +-------------------------------+          +-------------------------------+
                            \                          /
                             \                        /
                              v                      v
                            +--------------------------+
                            |    COLLECTOR SERVICE     |
                            |   (Reconstruction API)   |
                            |  - Verifies Report Count |
                            |  - Calls vdaf.unshard()  |
                            +--------------------------+
```

---

## 3. Component Details & Responsibilities

### 3.1 Leader Service (`leader_service.rs`)
- **Role**: Coordinates the VDAF verification protocol and acts as Aggregator 0.
- **Port**: `8001` (configurable via `ServiceConfig::default_leader()`).
- **Enforces**:
  - `GET /health`: Public health probe returning `{"status":"healthy","role":"leader","aggregator_id":0}`.
  - `GET /tasks/{task_id}/status`: Authenticated endpoint requiring `X-Service-Key` header.
  - `POST /tasks/{task_id}/reports/0`: Ingestion endpoint for Leader's share.
  - **Wrong Aggregator Protection**: Immediately rejects any request containing `/reports/1` with HTTP `400 Bad Request` (`WrongAggregatorTarget`).

### 3.2 Helper Service (`helper_service.rs`)
- **Role**: Independent verifier and accumulator acting as Aggregator 1.
- **Port**: `8002` (configurable via `ServiceConfig::default_helper()`).
- **Enforces**:
  - `GET /health`: Public health probe returning `{"status":"healthy","role":"helper","aggregator_id":1}`.
  - `GET /tasks/{task_id}/status`: Authenticated endpoint requiring `X-Service-Key` header.
  - `POST /tasks/{task_id}/reports/1`: Ingestion endpoint for Helper's share.
  - **Wrong Aggregator Protection**: Immediately rejects any request containing `/reports/0` with HTTP `400 Bad Request` (`WrongAggregatorTarget`).

### 3.3 Collector Service (`collector_service.rs`)
- **Role**: Combines additive aggregate shares obtained from both aggregators.
- **Privacy Enforcement**: The collector interface never exposes raw unverified data and rejects attempts to unshard mismatched report counts.

---

## 4. How to Run the Services

### Build
From `experiments/phase_6e/`:
```powershell
cargo check --offline --all-targets
cargo test --offline --no-fail-fast
```

### Starting the Leader Service
```powershell
cargo run --bin blindmark-phase-6e -- leader
# Output: Starting Leader Service on 127.0.0.1:8001
```

### Starting the Helper Service
```powershell
cargo run --bin blindmark-phase-6e -- helper
# Output: Starting Helper Service on 127.0.0.1:8002
```

