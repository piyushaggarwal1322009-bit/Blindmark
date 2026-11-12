# BlindMark Phase 6E.3 — Concurrency & HTTP Parser Hardening Remediation Report

**Status**: Completed  
**Milestone**: Phase 6E.3 Remediation  
**Date**: November 12, 2026  
**Target Codebase**: `experiments/phase_6e/`  
**Related Documents**: `docs/phase-6e-independent-remediation-review.md`, `docs/phase-6e-security-remediation-report.md`, `docs/phase-6e-authentication-design.md`  

---

## 1. Executive Summary

In response to the Phase 6E.2 independent audit findings (`docs/phase-6e-independent-remediation-review.md`), Phase 6E.3 implemented direct source-level hardening for the custom HTTP networking layer and connection admission architecture in `experiments/phase_6e/`.

Specifically:
1. **Atomic Concurrency Admission**: Replaced the vulnerable check-then-increment logic with a lock-free atomic `compare_exchange_weak` reservation loop via `ConnectionGuard::try_acquire`, strictly enforcing a 64-connection ceiling without TOCTOU race conditions.
2. **RAII Slot Lifecycle**: Implemented RAII cleanup via `ConnectionGuard::drop`, guaranteeing that connection slots are automatically returned to the pool on both normal return and thread panics.
3. **HTTP Header Hardening**: Hardened `experiments/phase_6e/src/http.rs` to detect and reject duplicate `Content-Length` headers (HTTP 400), conflicting `Content-Length` headers (HTTP 400), `Transfer-Encoding: chunked` requests (HTTP 501), requests containing both `Transfer-Encoding` and `Content-Length` (HTTP 400), and malformed header names/values (HTTP 400).
4. **Authentication Boundary Clarification**: Updated `docs/phase-6e-authentication-design.md` with Section 3.4 explaining the cryptographic inadequacy of HMAC for unlinkability and the necessity of coordinated cross-aggregator anti-replay.
5. **Comprehensive Verification**: Expanded the integration and parser test suite to 20 passing unit/integration tests with 100% pass rates across Phase 6E (20/20), Phase 6C (11/11), and Phase 6B (5/5).

---

## 2. Details of Remediations Implemented

### Task 1 — Atomic Connection Admission & RAII Slot Management
- **Files Modified**: `experiments/phase_6e/src/leader_service.rs`, `experiments/phase_6e/src/helper_service.rs`
- **Mechanism**:
  - Implemented `ConnectionGuard` struct wrapping `counter: Arc<AtomicUsize>`.
  - Implemented `ConnectionGuard::try_acquire(&counter, MAX_CONCURRENT_CONNECTIONS)` using `counter.compare_exchange_weak` in a loop. Connection reservation occurs on the listener thread *before* any worker thread is spawned.
  - If 64 connections are currently active, the listener writes an HTTP 503 `Service Unavailable` JSON response immediately and closes the stream.
  - If worker thread spawning fails (`thread::Builder::spawn`), the guard drops on the listener thread, immediately returning the slot.
  - When the worker executes, ownership of the guard is moved into the thread closure. When the thread finishes or encounters a panic, `Drop::drop` triggers `counter.fetch_sub(1, Ordering::SeqCst)`.

### Task 2 — HTTP Header & Request Parser Hardening
- **File Modified**: `experiments/phase_6e/src/http.rs`
- **Mechanism**:
  - **Duplicate & Conflicting `Content-Length`**: Added `seen_content_length` tracker. If `Content-Length` appears more than once, the parser returns `400 Bad Request` with `Duplicate Content-Length header is forbidden`.
  - **Transfer-Encoding Handling**: Added `seen_transfer_encoding` tracker. If both `Transfer-Encoding` and `Content-Length` are detected, returns `400 Bad Request` per RFC 9112 Section 6.3. If `Transfer-Encoding` is present, returns `501 Not Implemented` with `Transfer-Encoding: chunked is not supported; use Content-Length`.
  - **Header Name/Value Validation**: Added RFC 9110 token character validation `is_valid_header_name` and visible ASCII / tab validation `is_valid_header_value`. Malformed headers return `400 Bad Request`.
  - **Truncated Body Handling**: If a client closes the connection before transmitting the declared `Content-Length` bytes, returns `400 Bad Request` with `Incomplete request body`.
  - **Payload Size Enforcement**: Enforces `MAX_PAYLOAD_BYTES` (64 KB) on header declaration and stream consumption, returning `413 Payload Too Large`.

### Task 3 — Authentication Design Documentation Update
- **File Modified**: `docs/phase-6e-authentication-design.md`
- **Section Added**: Section 3.4 (*Cryptographic Limitations of HMAC: Why HMAC Alone Does Not Provide Anonymous Issuance or Unlinkability*).
- **Core Insights**:
  - Documented that HMAC-SHA256 is deterministic; any entity with the secret key $K$ can compute tags and link participants to submissions.
  - Clarified that true participant privacy requires blind credentials (e.g., Privacy Pass RFC 9578).
  - Clarified that Leader and Helper require joint anti-replay binding across reports.

---

## 3. Concurrency and Parser Verification Matrix

| Vulnerability / Requirement | Remediation Status | Verification Test |
|---|---|---|
| Exactly 64 concurrent admitted connections | Fixed (Atomic reservation) | `test_exactly_64_concurrent_admitted_connections_and_rejection` |
| Rejection of 65th connection with HTTP 503 | Fixed (Instant 503) | `test_exactly_64_concurrent_admitted_connections_and_rejection` |
| Simultaneous connection burst protection | Fixed (Lock-free CAS) | `test_simultaneous_connection_attempts` (80 concurrent requests) |
| Worker panic slot release | Fixed (RAII Drop guard) | `test_worker_panic_slot_cleanup` |
| Duplicate Content-Length headers | Fixed (Explicit 400 rejection) | `test_duplicate_content_length_header_rejection` |
| Conflicting Content-Length values | Fixed (Explicit 400 rejection) | `test_conflicting_content_length_rejection` |
| Transfer-Encoding: chunked | Fixed (Explicit 501 rejection) | `test_transfer_encoding_chunked_rejection` |
| Transfer-Encoding + Content-Length conflict | Fixed (Explicit 400 rejection) | `test_transfer_encoding_and_content_length_conflict_rejection` |
| Malformed headers (missing colons, bad tokens)| Fixed (RFC 9110 validation) | `test_malformed_header_rejection` |
| Incomplete request timeout / early EOF | Fixed (Explicit 400/408 handling) | `test_incomplete_request_timeout_or_eof` |
| Request payload size limit (64 KB) | Maintained | `test_oversized_payload_rejection` |

---

## 4. Test Suite Execution & Regression Audit

All tests were executed offline using `cargo test --offline --no-fail-fast`.

### 4.1 Phase 6E (`experiments/phase_6e`)
- **Command**: `cargo test --offline --no-fail-fast`
- **Exit Code**: `0`
- **Output Summary**:
  ```text
  running 20 tests
  test test_helper_health_endpoint ... ok
  test test_authenticated_task_status_unauthorized_rejection ... ok
  test test_authenticated_task_status_success ... ok
  test test_leader_health_endpoint ... ok
  test test_oversized_payload_rejection ... ok
  test test_malformed_empty_body_rejection ... ok
  test test_duplicate_content_length_header_rejection ... ok
  test test_conflicting_content_length_rejection ... ok
  test test_incomplete_request_timeout_or_eof ... ok
  test test_malformed_header_rejection ... ok
  test test_task_id_validation_rejection ... ok
  test test_unsupported_report_ingestion_not_implemented_501 ... ok
  test test_wrong_aggregator_target_rejection_at_leader ... ok
  test test_wrong_aggregator_target_rejection_at_helper ... ok
  test test_transfer_encoding_chunked_rejection ... ok
  test test_transfer_encoding_and_content_length_conflict_rejection ... ok
  test test_worker_panic_slot_cleanup ... ok
  test test_live_bounded_concurrent_connections_and_timeout ... ok
  test test_simultaneous_connection_attempts ... ok
  test test_exactly_64_concurrent_admitted_connections_and_rejection ... ok

  test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.38s
  ```

### 4.2 Phase 6C (`experiments/phase_6c`)
- **Command**: `cargo test --offline --no-fail-fast`
- **Exit Code**: `0`
- **Output Summary**: `test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s`

### 4.3 Phase 6B (`experiments/rust_prio3_demo`)
- **Command**: `cargo test --offline --no-fail-fast`
- **Exit Code**: `0`
- **Output Summary**: `test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s`

---

## 5. Security Posture Assessment

### Is concurrency genuinely bounded?
**Yes.** Connection slots are acquired atomically on the listener thread before any worker thread is spawned or stream processed. At no point can more than 64 active connection guards exist. If a thread panics or terminates prematurely, the RAII Drop guard guarantees the slot is immediately released.

### Does HTTP parsing reject ambiguous requests?
**Yes.** The parser strictly conforms to RFC 9112 Section 6.3 and RFC 9110:
- Requests with duplicate or conflicting `Content-Length` headers are rejected with HTTP 400.
- Requests attempting HTTP request smuggling via mixed `Transfer-Encoding` and `Content-Length` are rejected with HTTP 400.
- Requests specifying unsupported `Transfer-Encoding: chunked` are rejected with HTTP 501.

### Remaining Production Requirements (Out of Scope for Phase 6E)
1. **Async Runtime**: Replace `std::thread::spawn` with an asynchronous runtime (Tokio / Hyper / Axum) to avoid OS thread creation overhead for production scale.
2. **TLS / HTTPS Termination**: Deploy an mTLS reverse proxy (e.g. Envoy / Nginx) or native Rustls termination to protect tokens and shares in flight.
3. **Cryptographic Anonymous Credentials**: Implement Privacy Pass (RFC 9578) blind tokens for true cryptographic participant unlinkability.
4. **Phase 6F Implementation**: Implement actual cryptographic report ingestion pipelines and inter-aggregator verification preparation rounds.

