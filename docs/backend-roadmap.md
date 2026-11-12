# Backend Architecture & Production Roadmap

## 1. Current State vs. Production Requirements

| Component | Current Prototype / PoC | Production Architecture |
|---|---|---|
| **Cryptographic Engine** | Python reference implementation (`vdaf_poc`) | Rust `libprio-rs` (`prio` crate) compiled natively for backend & WebAssembly for client |
| **Aggregator Protocol** | Basic Python FastAPI with local in-memory dict | Distributed Aggregation Protocol (DAP / RFC draft-ietf-ppm-dap) |
| **Verification Rounds** | Synchronous sequential loop | Asynchronous batch aggregation via Leader-Helper orchestration |
| **Storage & Persistence** | Python memory state | Distributed DB / Redis with transactional nullifier sets for nonces and tokens |
| **Client Interface** | Python CLI (`client/submit.py`) | TypeScript/Wasm browser client running in user's browser sandbox |
| **Duplicate Prevention** | None (vulnerable to replays) | Decoupled one-time token nullifier set $\rightarrow$ Blind Signatures (Privacy Pass) |
| **Output Disclosure** | Exact scalar reconstruction | $k$-Anonymity batch gating + Differential Privacy output perturbation |

---

## 2. Production Cryptography & DAP Research

### 2.1. Why Python PoC Cannot Be Used in Production
* **Side-Channel Vulnerability:** Python's integer arithmetic and memory allocations are variable-time and susceptible to CPU cache/timing side-channel attacks.
* **Performance:** Python FLP evaluations over $\mathbb{F}_{2^{128}}$ take milliseconds per report. Under production loads (thousands of users), Python would exhaust compute resources.
* **Disclaimer:** As cited in `draft-irtf-cfrg-vdaf/poc/README.md`, the code is intended solely to generate test vectors.

### 2.2. The Standard: Distributed Aggregation Protocol (DAP)
The IETF Privacy-Preserving Measurement (PPM) Working Group specifies DAP (`draft-ietf-ppm-dap`):
* Standardizes how clients upload reports to the Leader Aggregator via HTTP POST.
* Standardizes the aggregation sub-protocol between Leader and Helper Aggregators (the "ping-pong" state exchange).
* Standardizes collector authorization and batch unmasking queries.
* Production open-source references include:
  * **Janus** (ISRG / Divvi Up): Rust implementation of DAP and VDAFs.
  * **libprio-rs**: Rust implementation of Prio, Prio3, and FLP validity circuits.

---

## 3. Step-by-Step Implementation Roadmap

```
  Phase 3 (Current)          Phase 4                    Phase 5                    Phase 6
+-------------------+      +-------------------+      +-------------------+      +-------------------+
| Architecture &    | ---> | Multi-Campaign    | ---> | Rust / Wasm       | ---> | Client Web UI &   |
| Core Validation   |      | Backend Services  |      | Cryptography Core |      | Independent Pilot |
| (Input guards &   |      | (Tokens, DB schema|      | (libprio-rs       |      | (2 independent    |
| Threat Models)    |      |  & Aggregator APIs|      |  integration)     |      |  aggregators)     |
+-------------------+      +-------------------+      +-------------------+      +-------------------+
```

### Phase 3: Architecture & Security Foundation (Completed in this Phase)
- [x] Codebase audit & component classification.
- [x] Strict input validation module (`core/validation.py`) with boundary testing.
- [x] Architecture, Threat Model, Privacy Model, and Backend Roadmap documentation.

### Phase 4: Multi-Campaign Backend Service
- [ ] Implement FastAPI campaign service with PostgreSQL schema (Communities, Campaigns, Categories).
- [ ] Implement Token Authority issuing one-time submission credentials.
- [ ] Implement Leader-Helper Aggregator API endpoints with nonce replay protection and nullifier tracking.
- [ ] Implement Collector service with $k$-anonymity threshold gate ($N \ge 20$) and differential privacy noise.

### Phase 5: Production Cryptography Integration
- [ ] Compile Rust `libprio-rs` into a WebAssembly library for client-side one-hot sharding in browsers.
- [ ] Replace Python VDAF backend logic with high-performance Rust service bindings.
- [ ] Commission independent cryptographic security audit before handling real compensation data.

### Phase 6: Web Client & Production Pilot
- [ ] Implement clean web interface allowing users to select communities, authenticate via verified domains, and submit encrypted shares directly from the browser.
- [ ] Deploy Aggregator A (Community Trustee) and Aggregator B (Independent Auditing Host) on separate infrastructure.
- [ ] Execute first live pilot.

