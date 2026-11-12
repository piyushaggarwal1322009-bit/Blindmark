# Phase 6D: Step-by-Step Development Plan for Phase 6E

## 1. Roadmap Overview

Phase 6D established the integration architecture and API contract. The subsequent phases will implement this contract incrementally without destabilizing the working Python or Rust codebases:

```
[Phase 6C: Modular Rust Prototype] (Completed)
               │
               ▼
[Phase 6D: Integration Architecture & Contract] (Completed)
               │
               ▼
[Phase 6E: Rust Aggregator Microservice (axum)]
  ├── Task 1: Expose Phase 6C via lightweight HTTP daemon
  ├── Task 2: Implement Leader & Helper HTTP endpoints
  └── Task 3: Local dual-service integration tests
               │
               ▼
[Phase 6F: FastAPI Client Ingestion & Dispatcher]
  ├── Task 1: Add client submission endpoint in FastAPI
  ├── Task 2: Forward shares to Leader & Helper services
  └── Task 3: Connect Collector output to ResultReleasePolicy
               │
               ▼
[Phase 6G: Differential Privacy & Hardening]
```

---

## 2. Phase 6E Detailed Execution Steps

### Step 1: Microservice Harness (`experiments/phase_6e_service`)
- Create a new Rust crate `experiments/phase_6e_service` depending on `blindmark-phase-6c` and `axum = "0.7"`, `tokio = { version = "1", features = ["full"] }`.
- Provide binary flags:
  - `--role leader --port 8001`
  - `--role helper --port 8002`
  - `--role collector --port 8003`

### Step 2: Implement Endpoints Matching Phase 6D Contract
- `POST /tasks`: Configures campaign parameters.
- `POST /tasks/{id}/reports/{agg_id}`: Receives single input share.
- `POST /tasks/{id}/prepare/share`: Exchanges verifier shares between Helper and Leader.
- `POST /tasks/{id}/prepare/broadcast`: Distributes prep message to Helper.
- `GET /tasks/{id}/aggregate-share`: Returns additive share to Collector.
- `POST /tasks/{id}/collect`: Collector unshards and reports back to FastAPI.

### Step 3: FastAPI Dispatcher Integration
- Update `app/services/vdaf_interface.py` to implement a concrete `HttpVdafAggregatorService` using `httpx`.
- Forward participant reports from FastAPI to Leader (`http://localhost:8001`) and Helper (`http://localhost:8002`).
- Call Collector service upon campaign expiration or admin release request.

### Step 4: Verification & Acceptance Criteria
- Integration test running FastAPI + Leader daemon + Helper daemon via pytest.
- End-to-end participant submission via token issuance -> report submission -> collection -> result evaluation.
- All Phase 6B and Phase 6C tests must continue passing.

