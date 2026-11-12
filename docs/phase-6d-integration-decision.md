# Phase 6D: Integration Approach Evaluation & Decision

## 1. Candidate Architectural Approaches

To bridge the existing Python/FastAPI control plane with the Rust Prio3 cryptographic aggregator components, we evaluate three potential integration patterns:

---

### Option A: Subprocess Execution (CLI Binary Invoked by FastAPI)

FastAPI spawns a compiled Rust CLI binary (e.g. `blindmark-phase-6c.exe`) via `asyncio.create_subprocess_exec`, passing reports and parameters through command-line flags or `stdin`/`stdout` JSON pipes.

- **Security Boundaries**: Moderate. Process isolation between Python and Rust. However, both still run under the same OS host environment.
- **Operational Complexity**: Low in development; no long-running microservice daemon needed.
- **Performance**: **Extremely Poor**. Spawning an OS process, reloading dynamic libraries, and reinitializing cryptographic verification tables for every single report or batch incurs heavy CPU and latency overhead (tens of milliseconds per report).
- **Error Handling**: Brittle. Diagnosing process exit codes and parsing `stderr` strings is error-prone.
- **Isolation of Leader and Helper**: **None**. Both aggregator roles would be simulated sequentially within a single CLI invocation.

---

### Option B: In-Process Native Extension (PyO3 / Rust C-ABI FFI)

The Rust Prio3 logic is compiled into a shared library (`.pyd` on Windows / `.so` on Linux) using `pyo3` and imported directly into FastAPI (`from blindmark_vdaf import verify_report, unshard`).

- **Security Boundaries**: **Critical Vulnerability**. Python process memory space directly hosts Leader, Helper, and FastAPI control plane state simultaneously. A memory-read bug, debug logging error, or administrator intrusion in Python compromises all secret shares at once.
- **Operational Complexity**: Moderate. Requires cross-language build tooling (`maturin` / `setuptools-rust`), Python ABI matching, and MSVC linkage synchronization.
- **Performance**: High. Zero IPC overhead, direct in-memory buffer passing.
- **Error Handling**: Strong. Rust `Result<T, E>` translates cleanly to Python exceptions.
- **Isolation of Leader and Helper**: **Impossible**. PyO3 forces both Leader and Helper to exist in the exact same process memory space as the FastAPI web app.

---

### Option C: Decoupled HTTP / gRPC Services with mTLS (Independent Service Boundaries)

Leader and Helper run as standalone HTTP/gRPC services in Rust (e.g. built with `axum` or `tonic`). FastAPI interacts as an orchestrator sending reports to Leader and Helper over authenticated internal network interfaces.

- **Security Boundaries**: **Superior & Production-Grade**.
  - Leader and Helper run in distinct Linux containers / cloud instances / trust domains.
  - FastAPI only receives opaque public tokens and encrypted shares; it never sees unshared state.
  - Secret shares for Aggregator 0 and Aggregator 1 are physically partitioned across distinct network endpoints and IP namespaces.
- **Operational Complexity**: Higher. Requires configuring local Docker containers / ports for Leader (e.g. port 8001), Helper (port 8002), and Collector (port 8003).
- **Performance**: Excellent. Long-running daemons maintain warm memory caches, pre-initialized VDAF tables, and connection pools.
- **Isolation of Leader and Helper**: **Complete**. Enforces true non-colluding aggregator domains.

---

## 2. Comparative Matrix

| Evaluation Dimension | Option A: Subprocess CLI | Option B: PyO3 Native FFI | Option C: Decoupled Services (Recommended) |
|---|---|---|---|
| **Cryptographic Trust Isolation** | Poor (Shared host) | Fatal (Shared memory) | **Excellent (Network boundary)** |
| **Leader / Helper Separation** | Simulated / Sequential | Nonexistent | **Physically & Logically Independent** |
| **Throughput / Latency** | Low (< 50 req/sec) | Very High (> 5,000 req/sec) | **High (> 2,500 req/sec)** |
| **Crash Blast Radius** | Process exit per call | Process segfault crashes FastAPI | **Isolated to single aggregator container** |
| **DAP Standard Alignment** | Non-standard | Non-standard | **Directly mirrors IETF DAP specification** |
| **Development Progression** | Throwaway code | Throwaway architecture | **Direct path to Janus / production DAP** |

---

## 3. Recommended Decision: Option C (Decoupled Services)

### Justification:
The foundational security claim of Blindmark is that **no single entity learns a participant's private stipend**. 

If Option B (PyO3) is adopted, the FastAPI backend holds both input shares in memory at the exact same instant, completely defeating the non-collusion assumption. Option A (CLI) is an unscalable simulation artifact that must be discarded immediately for multi-user workloads.

**Decision for Phase 6E & Beyond**:
Adopt **Option C (Decoupled Services)**.
- Implement a lightweight HTTP service layer around `experiments/phase_6c` using `axum` in Rust.
- Run Leader on dedicated endpoint (e.g. `http://localhost:8001`).
- Run Helper on dedicated endpoint (e.g. `http://localhost:8002`).
- FastAPI dispatches share 0 to Leader and share 1 to Helper.
- Leader and Helper communicate over their private prep loop.
- Collector runs as an authenticated batch trigger returning verified results to FastAPI's `/api/campaigns/{id}/results/evaluate` endpoint.

