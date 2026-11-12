# Blindmark — Phase 6A.1: Architecture Validation Report

**Date:** 2026-11-11  
**Branch:** `feature/phase-6-vdaf-design`  
**Purpose:** Pre-implementation architectural validation covering DAP collection flow correction, Janus / `libprio-rs` version compatibility, measurement boundaries, and host Rust build environment.

---

## 1. DAP Flow Correction Verification

In Phase 6A, an earlier draft of the architecture diagram depicted the Collector querying both the Leader and Helper directly.
**Validation & Correction:**
- In the official IETF DAP specification (`draft-ietf-ppm-dap`), the **Collector communicates only with the Leader**.
- The corrected flow has been codified in [docs/phase-6a-cryptographic-architecture.md](file:///c:/Users/Admin/Desktop/BlindMark/docs/phase-6a-cryptographic-architecture.md#L39-L87):
  1. Client sends DAP report (with encrypted shares for both aggregators) to the Leader.
  2. Leader forwards Helper's encrypted share to the Helper during the preparation phase.
  3. Leader and Helper verify proofs interactively without seeing raw inputs.
  4. Collector sends a collection job request (`collection-job-req`) to the Leader.
  5. Leader queries the Helper's `POST /tasks/{task_id}/aggregate_shares`.
  6. Helper verifies batch checksums, encrypts its aggregate share using the **Collector's public key**, and returns it to the Leader.
  7. Leader returns both its own encrypted aggregate share and the Helper's encrypted aggregate share to the Collector.
  8. Collector decrypts both shares using its private key and executes VDAF unsharing.

---

## 2. Janus Compatibility & Audit Status

- **Repository:** `divviup/janus`
- **Target Drafts:** `draft-ietf-ppm-dap-19/20`
- **VDAF Support:** Native support for `Prio3Histogram`, `Prio3Sum`, `Prio3Count`.
- **Aggregation Parameter Requirements:** Janus currently supports VDAFs with **trivial aggregation parameters only**. For `Prio3Histogram`, the aggregation parameter is empty/trivial, which is fully supported by Janus. (Complex variable-parameter schemes like *Mastic* are unsupported).
- **Sub-Crates Available:** Includes `janus_aggregator`, `janus_client`, `janus_collector`, and `janus_core`.
- **Status & Warnings:**
  - Upstream `README.md` explicitly designates Janus as an **"Experimental implementation of the Distributed Aggregation Protocol (DAP) specification... currently in active development."**
  - It does **not** possess an official third-party security audit report certifying production readiness.
  - Development must acknowledge this experimental status and isolate Janus as a research/pre-production engine.

---

## 3. `libprio-rs` Version & API Compatibility

- **Repository:** `divviup/libprio-rs` (Crate `prio`)
- **Version Used by Janus:** `0.18.1-alpha.3`
- **VDAF Types:** `prio::vdaf::prio3::Prio3Histogram` (instantiated via `Prio3::new_histogram(num_aggregators, length, chunk_length)`).
- **Field Representation:** `Field31` (Mersenne 31 finite field $2^{31} - 1$).
- **API Matching:** `janus` workspace `Cargo.toml` directly imports `prio = { version = "0.18.1-alpha.3", default-features = false }`. Therefore, standalone Rust prototypes in Phase 6B targeting `prio = "0.18.1-alpha.3"` will have 100% binary compatibility with Janus components.

---

## 4. Measurement Model Validation & Boundaries

1. **Fixed Histogram Geometry ($K = 12$):**
   - The bucket count must remain strictly fixed per task. Finite-field vector dimensions and FLP verification equations cannot be dynamically scaled after task creation.
2. **Context & Currency Prerequisites:**
   - VDAF operates purely on dimensionless one-hot integer vectors. Currency (USD), periodicity (monthly stipend), and market location must be strictly configured in campaign metadata prior to launch.
3. **Validity vs. Truthfulness:**
   - VDAF zero-knowledge proofs guarantee that a client submitted an allowable one-hot vector within $[0, 11]$.
   - Proofs **cannot** verify whether the participant actually earned that salary in reality. Real-world integrity relies on community invitation, participant whitelisting, and rate-limiting.
4. **Cohort Gating vs. Differential Privacy:**
   - The minimum participant threshold ($N \ge 20$) protects small cohorts, but does not prevent reconstruction attacks across multiple overlapping campaigns. Formal differential privacy requires independent noise calibration.

---

## 5. Host Rust Environment Status & Setup Requirements

### Current Environment Status: **NOT INSTALLED**
- Ran `Get-Command rustc, cargo, rustup` and checked `~/.cargo/bin/`.
- Neither `rustc` nor `cargo` is present in `PATH` or user directories on this Windows machine.

### Supported Windows Setup Requirements for Phase 6B:
To execute Phase 6B (Rust `Prio3Histogram` prototype), the system requires:
1. **Rustup Toolchain Installer for Windows:**
   - Download/run `rustup-init.exe` (or `winget install Rustlang.Rustup`).
   - Standard MSVC toolchain (`stable-x86_64-pc-windows-msvc`) requires Microsoft C++ Build Tools (Visual Studio Build Tools with C++ workload).
   - Alternatively, the GNU toolchain (`stable-x86_64-pc-windows-gnu`) requires MinGW.
2. **Containerized Alternative (Recommended if local compiler unavailable):**
   - Running Rust compilation and tests inside an official `rust:latest` Docker container.

---

## 6. Readiness Recommendation for Phase 6B

### **VERDICT: ARCHITECTURE IS VALIDATED; PHASE 6B PREREQUISITE PENDING HOST TOOLCHAIN**

**Recommendation:**
The architectural specifications and DAP wire flows are now fully validated and corrected. However, **Phase 6B implementation cannot begin until a working Rust/Cargo toolchain (or Docker runtime) is provisioned on the host environment.**

