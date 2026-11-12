# BlindMark Phase 6E: Focused Source Code Security Review

## 1. Executive Summary

This report conducts a strict, independent security review of the Phase 6E HTTP service implementation in `experiments/phase_6e/`.

Phase 6E established a standalone HTTP service layer wrapping the Leader and Helper aggregators. However, a rigorous audit reveals **critical architectural gaps and security vulnerabilities** that must be resolved before Phase 6F:
1. **Report Ingestion Is Stubbed**: The `POST /tasks/{task_id}/reports/{agg_id}` endpoint returns HTTP `202 Accepted` immediately without parsing, decoding, validating, or running Prio3 verification. No shares are accumulated into `AggregateShare`.
2. **Missing Token / Participant Authentication on Report Ingestion**: Anyone who can reach ports 8001/8002 can submit arbitrary payloads without presenting a single-use nullifier token or authentication credential, enabling DoS and poisoning.
3. **HTTP Parser Denial-of-Service (Slowloris & Single-Thread Blocking)**: The custom HTTP listener handles connections sequentially on a single thread with no read/write timeouts; a slow client will hang the entire service.
4. **Unencrypted Plain HTTP**: Services run on unencrypted `std::net::TcpListener` without TLS or mTLS, exposing private input shares in cleartext over the network.
5. **No Campaign / Task Binding**: Any task ID in the URL is accepted without checking if a campaign exists or is active.

---

## 2. Findings Classification & Analysis

### Finding 1 [CRITICAL]: Report Ingestion Does Not Execute Prio3 Verification (Stubbed Aggregation)
- **Affected File & Function**: `experiments/phase_6e/src/routes.rs` -> `RouteHandler::handle_report_ingestion` (lines 96–136).
- **Description**:
  In `routes.rs`, `handle_report_ingestion` performs only two checks:
  1. It checks whether the URL path contains the expected aggregator ID (`expected_id == target_id`).
  2. It checks whether `req.body.is_empty()`.
  If those two checks pass, it returns:
  ```rust
  HttpResponse::from_status_and_body(
      202,
      format!("{{\"status\":\"accepted\",\"target_aggregator\":{}}}", expected_id),
  )
  ```
  It **does not decode** `ClientReport`, **does not call** `vdaf.verify_init`, **does not check** zero-knowledge proofs, and **does not accumulate** output shares into `AggregateShare`.
- **Security Impact**:
  Currently, any bytes sent to the endpoint are marked "accepted". If a client submits malformed or tampered data, the server reports success without verifying mathematical correctness. The aggregator's internal `accepted_count` remains 0.
- **Recommended Correction**:
  In Phase 6F, deserialize the request body into a concrete `ClientReport` structure, call `aggregator.lock().unwrap().handle_client_report(report)`, and store the resulting `Prio3VerifyState` for the subsequent prep loop.

---

### Finding 2 [CRITICAL]: Unauthenticated Report Ingestion Allows Unrestricted Denial-of-Service & Poisoning
- **Affected File & Function**: `experiments/phase_6e/src/routes.rs` -> `RouteHandler::handle` & `handle_report_ingestion` (lines 49–51).
- **Description**:
  `handle_task_status` enforces `self.verify_auth(req)` requiring `X-Service-Key`. In contrast, `handle_report_ingestion` has **zero authentication or credential validation**.
- **Security Impact**:
  - Anyone on the network can flood the aggregator with fake reports.
  - While participant anonymity is required, participants must still prove *eligibility* via a single-use nullifier token (`CampaignToken` / blind signature).
  - Without token validation at ingestion, the aggregator cannot verify whether the submission was authorized by FastAPI or if the same participant is submitting millions of fake shares (Sybil attack).
- **Recommended Correction**:
  Require a valid, unredeemed anonymous nullifier token in the request header or body. Check the token against the task's local nullifier cache to prevent replay.

---

### Finding 3 [HIGH]: Custom HTTP Server Lacks Timeouts & Runs on a Single Blocking Thread (Slowloris Vulnerability)
- **Affected File & Function**: 
  - `experiments/phase_6e/src/leader_service.rs` & `helper_service.rs` -> `spawn` loop (lines 51–71).
  - `experiments/phase_6e/src/http.rs` -> `parse_request` (lines 64–124).
- **Description**:
  The TCP listener accepts streams on a single background worker thread:
  ```rust
  let handle = thread::spawn(move || {
      while is_running.load(Ordering::SeqCst) {
          match listener.accept() {
              Ok((mut stream, _)) => {
                  let _ = stream.set_nonblocking(false); // <--- Sets blocking!
                  match parse_request(&mut stream, max_payload) { ... }
  ```
  Once a connection is accepted, `stream.set_nonblocking(false)` makes socket reads completely blocking. There is no `stream.set_read_timeout(...)` or `stream.set_write_timeout(...)`.
  In `parse_request`, the while-loop:
  ```rust
  while full_body.len() < content_length {
      let n = stream.read(&mut buffer)...
  ```
  waits indefinitely if a client sends 1 byte every 30 seconds.
- **Security Impact**:
  A single attacker opening one TCP connection and trickling bytes (classic Slowloris attack) will completely freeze the entire Leader or Helper service. All subsequent requests from legitimate clients will queue indefinitely.
- **Recommended Correction**:
  1. Set explicit socket timeouts: `stream.set_read_timeout(Some(Duration::from_secs(5)))`.
  2. Spawn a lightweight thread per connection or migrate to an async runtime (Tokio/Axum or thread pool) for concurrent handling.

---

### Finding 4 [HIGH]: Lack of In-Transit Encryption (Plaintext HTTP vs. HTTPS)
- **Affected File & Function**: `experiments/phase_6e/src/leader_service.rs` & `helper_service.rs`.
- **Description**:
  Both services bind plain TCP sockets: `TcpListener::bind(&addr)`. All HTTP traffic (headers, nonces, public shares, and private input shares) is transmitted unencrypted in cleartext.
- **Security Impact**:
  Any network eavesdropper on the local network, Wi-Fi, or ISP path can intercept `InputShare_0` or `InputShare_1`. If an attacker intercepts both network streams (e.g., at an upstream router), they can immediately reconstruct the student's salary:
  $$\text{Stipend} = \text{Share}_0 + \text{Share}_1 \pmod p$$
- **Recommended Correction**:
  Enforce TLS 1.3 or place services behind an HTTPS reverse proxy (e.g. Nginx, Envoy, or Caddy) with Mutual TLS (mTLS) for aggregator-to-aggregator communications.

---

### Finding 5 [MEDIUM]: URL Aggregator ID Routing Does Not Validate Internal State Binding
- **Affected File & Function**: `experiments/phase_6e/src/routes.rs` -> lines 98–118.
- **Description**:
  The route checks:
  ```rust
  let target_id: u8 = match target_id_str.parse() ...
  if target_id != expected_id { ... return WrongAggregator ... }
  ```
  While it rejects `/reports/1` at Leader and `/reports/0` at Helper, it does not validate whether the cryptographic payload inside the body actually targets this aggregator.
- **Security Impact**:
  If an attacker sends a Helper's cryptographic input share inside a request to `/reports/0`, the URL check passes. If the aggregator does not decode the share with its own aggregator role parameter (`ShareDecodingParameter::Leader` vs `ShareDecodingParameter::Helper`), deserialization or verification will panic or desynchronize.
- **Recommended Correction**:
  Ensure decoding uses the service's internal `config.aggregator_id`, and verify that the public share / proof metadata matches the service's expected role.

---

### Finding 6 [LOW]: Lack of Structured Audit Logging & Accidental Exposure Risk
- **Affected File & Function**: `experiments/phase_6e/src/main.rs`.
- **Description**:
  The services currently use basic `println!` and `eprintln!`. There is no structured logging framework (e.g., `tracing`) with privacy filters.
- **Security Impact**:
  During future debugging or error logging, developers might print incoming `HttpRequest.body`, inadvertently writing confidential input shares to disk or terminal standard output.
- **Recommended Correction**:
  Adopt `tracing` with an explicit security rule: never log raw request bodies or decoded share fields.

---

## 3. Summary of Findings

| ID | Severity | Component | Finding Summary | Impact |
|---|---|---|---|---|
| **SEC-01** | **CRITICAL** | `routes.rs` | Report ingestion is stubbed (returns 202 without VDAF verification) | Invalid/tampered shares not detected; aggregation count remains 0. |
| **SEC-02** | **CRITICAL** | `routes.rs` | Missing nullifier / participant token verification on ingestion | Anonymous DoS and Sybil poisoning vulnerabilities. |
| **SEC-03** | **HIGH** | `http.rs` / `service.rs` | Blocking single-thread socket loop without read timeouts | Trivial Slowloris Denial of Service freezes the entire aggregator. |
| **SEC-04** | **HIGH** | `leader_service.rs` | Plaintext HTTP without TLS / mTLS | Network eavesdropper can intercept shares in transit. |
| **SEC-05** | **MEDIUM** | `routes.rs` | Target check relies solely on URL string without payload binding | Potential payload mismatch during deserialization. |
| **SEC-06** | **LOW** | `main.rs` | Lack of structured redaction logging | Risk of logging private input shares in future debug statements. |

---

## 4. Conclusion & Next Steps

Phase 6E successfully demonstrated independent service addressing, identity checking (`WrongAggregatorTarget`), and size limiting (`PayloadTooLarge`). However, **it is strictly a scaffolding milestone**.

**Do NOT proceed to Phase 6F until the following plan is approved**:
1. Implement real deserialization and connect `handle_client_report` from Phase 6C into `handle_report_ingestion`.
2. Add read/write timeouts to `http.rs` to eliminate the blocking DoS vulnerability.
3. Incorporate nullifier token checking in the ingestion request schema.

