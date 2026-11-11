# Blindmark — Phase 6A: Development Roadmap

**Scope:** Step-by-step phased roadmap for integrating real VDAF/DAP aggregation into Blindmark.

---

## Milestone Breakdown

### Phase 6A: Research & Architecture Specification (Current Milestone)
- **Objective:** Establish the mathematical, cryptographic, and system architecture for Prio3Histogram and DAP aggregation without modifying application code.
- **Deliverables:**
  - `docs/phase-6a-vdaf-research.md`
  - `docs/phase-6a-cryptographic-architecture.md`
  - `docs/phase-6a-integration-decision.md`
  - `docs/phase-6a-development-roadmap.md`
- **Dependencies:** Completion of Phase 5C security re-audit.
- **Acceptance Criteria:** Documented evaluation of VDAF, DAP drafts, Janus, and hybrid architecture; clear stipend bucket parameters.

---

### Phase 6B: Isolated Rust Prio3Histogram Prototype
- **Objective:** Build an isolated Rust CLI experiment in `experiments/rust_prio3_demo/` using `libprio-rs` to establish native Rust cryptographic sharding, verification, and unsharing outside the Python runtime.
- **Deliverables:**
  - `Cargo.toml` with `prio = "0.18+"`
  - Client sharding of a 12-bucket stipend measurement.
  - Aggregator preparation and FLP proof verification test.
  - Rejection tests for out-of-bounds bucket indices and manipulated proof vectors.
- **Dependencies:** Local Rust toolchain (`cargo`, `rustc`).
- **Acceptance Criteria:** Clean execution and automated test passing on Windows.

---

### Phase 6C: Client Sharding & Report Submission
- **Objective:** Develop the client-side sharding module for web/app clients to encode measurements and generate HPKE-encrypted DAP report envelopes.
- **Deliverables:**
  - WebAssembly (Wasm) or lightweight TypeScript/Python client package for stipend encoding.
  - Local creation of 12-bucket one-hot vector and Prio3 proof.
  - HPKE encryption targeting Leader and Helper public keys.
- **Dependencies:** Phase 6B.
- **Acceptance Criteria:** Client produces binary/JSON DAP reports accepted by standard parsers without leaking cleartext stipends.

---

### Phase 6D: Two-Aggregator Verification & State Accumulation
- **Objective:** Deploy local Leader and Helper Janus aggregation services with dedicated PostgreSQL databases.
- **Deliverables:**
  - `docker-compose.yml` orchestrating Leader Janus + PostgreSQL and Helper Janus + PostgreSQL.
  - Inter-aggregator TLS mutual authentication.
  - Successful execution of multi-round aggregation jobs for batch reports.
- **Dependencies:** Docker/container runtime or local binaries.
- **Acceptance Criteria:** Leader and Helper verify reports and accumulate aggregate shares without error.

---

### Phase 6E: Collector Integration & Threshold Gating
- **Objective:** Build the DAP collector service that retrieves partial aggregate shares, enforces cohort thresholds, and unshares final histograms.
- **Deliverables:**
  - Collector client integrating with Janus collector APIs.
  - Threshold enforcement ($N \ge 20$).
  - Result unsharing over Mersenne 31 finite field.
- **Dependencies:** Phase 6D.
- **Acceptance Criteria:** Collector combines shares into accurate histogram distributions and rejects small batches.

---

### Phase 6F: Blindmark Backend Integration
- **Objective:** Connect the Blindmark FastAPI backend with the DAP aggregation infrastructure.
- **Deliverables:**
  - Task provisioning: FastAPI automatically registers DAP tasks in Janus when a campaign is created.
  - Collector delivery: Collector submits verified unshared histograms to `POST /api/campaigns/{id}/results/evaluate` using the existing `X-Service-Key` authorization.
- **Dependencies:** Phase 6E, Phase 4/5 backend foundation.
- **Acceptance Criteria:** End-to-end flow from participant submission to community benchmark display.

---

### Phase 6G: Security, Differential Privacy & Interoperability Testing
- **Objective:** Comprehensive end-to-end security audit and differential privacy calibration.
- **Deliverables:**
  - Malicious submission penetration tests against live Janus aggregators.
  - Differential privacy noise injection calibration (Laplace/Gaussian mechanism for bounded histogram sensitivity).
  - Multi-worker load and failover tests.
- **Dependencies:** Phase 6F.
- **Acceptance Criteria:** Full passing test suite and verified privacy protection under adversarial conditions.
