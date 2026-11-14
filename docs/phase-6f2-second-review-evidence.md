# BlindMark Phase 6F.2 Independent Second-Review Evidence Package

**Audit date:** 2026-11-13  
**Scope:** Read-only documentation and source audit. The only file created by this audit is this evidence package. No implementation is authorized.

## 1. Initial repository state

Initial branch command:

```text
git branch --show-current
feature/phase-6b-rust-prio3-prototype
exit code: 0
```

Initial `git status --short`:

```text
 M experiments/phase_6e/src/storage.rs
 M experiments/phase_6e/tests/storage_tests.rs
?? docs/phase-6f1-storage-implementation-report.md
?? docs/phase-6f2-deployment-profiles.md
?? docs/phase-6f2-preimplementation-audit.md
?? docs/phase-6f2-proxy-architecture.md
?? docs/phase-6f2-proxy-contract.md
?? docs/phase-6f2-proxy-security-review.md
?? "ntent .gitignore"
?? rust_environment_test/
exit code: 0
```

This is a dirty worktree. The Phase 6F.1 tracked changes and the listed untracked items predate this audit and were preserved.

Initial `git diff --stat`:

```text
 experiments/phase_6e/src/storage.rs         | 236 +++++++++-----
 experiments/phase_6e/tests/storage_tests.rs | 460 ++++++++++++++++++++++++----
 2 files changed, 544 insertions(+), 152 deletions(-)
exit code: 0
```

Initial `git diff --name-only`:

```text
experiments/phase_6e/src/storage.rs
experiments/phase_6e/tests/storage_tests.rs
exit code: 0
```

Initial `git diff --cached --stat`: empty output, exit code 0.  
Initial `git diff --cached --name-only`: empty output, exit code 0.

An empty cached diff does not imply a clean worktree; the status and unstaged diff above show existing changes.

## 2. Revised-document inventory

| Document | Physical lines | Pre-report SHA-256 |
|---|---:|---|
| `docs/phase-6f2-proxy-architecture.md` | 82 | `D02947DC075CBE78CAE883D89F0DD93EFCC7BF4CE86B7335E46DA2FF23821293` |
| `docs/phase-6f2-proxy-contract.md` | 187 | `DA9231E6D144A3FC150387756D3A047E5B421FE0508BFB95BD49B6138269E589` |
| `docs/phase-6f2-deployment-profiles.md` | 82 | `95E3FEC9CCD15C05B8FB7064046BB3D9F8E31A7871772FAC9CCAAD09F2248185` |
| `docs/phase-6f2-proxy-security-review.md` | 127 | `16630AC85D7EAC4434EC30DE827A484138857FC3C9CF4B56A51E81D55D0EBB0A` |

All four complete documents were freshly read before this report was created. Their complete contents are reproduced in Section 5. SHA-256 values above were captured before creating this evidence file; the final comparison is recorded in Section 9.

## 3. Five-finding verification matrix

| Finding | Status | Assessment |
|---|---|---|
| A. Leader egress proxy | Partially Resolved | The egress sidecar, `127.0.0.1:8101`, certificate ownership, destination allowlist, mTLS, bounded timeouts, failure behavior, logging restrictions, and all three profiles are described. The local process-level restriction is left to generic OS/network isolation or IPC alternatives; no platform-specific enforceable mechanism is selected or evidenced. |
| B. Helper backend authentication and provenance | Partially Resolved | The Helper proxy validates mTLS, strips and regenerates `X-Authenticated-Coordination-Peer`, and the backend is required to validate that assertion and directional HMAC independently. The assertion depends on an enforceable proxy-only backend path, but concrete enforcement is not demonstrated for each profile. The documents correctly require the coordination route to remain disabled if isolation cannot be enforced. |
| C. Directional HMAC and canonical encoding | Partially Resolved | Directional key names, sender/verifier, process access, out-of-band provisioning, versioning, rotation, revocation, restart behavior, exact payload coverage, durable replay requirements, and lack of non-repudiation are stated. Exact canonical binary encoding is explicitly deferred: field order, widths/length-prefix format, byte order, string encoding, field limits, invalid-value rules, and complete payload serialization are not fixed. |
| D. Backend parser security gate | Resolved at contract level | The documents prohibit security-sensitive signed/binary ingestion until a separate backend parser review and tests pass. They identify parser limitations and explicitly say proxy checks do not repair backend parser differentials. The listed tests cover all requested cases. This is a proposed hard gate, not evidence that the parser was fixed or tested. |
| E. Delivery and retry semantics | Partially Resolved | The three ambiguous outcomes, stable identity, durable deduplication, conflicting-content rejection, state inspection, exact retries, and immutable `COMMIT_DECIDED` rule are clear. Retry schedules/ownership and concrete resume-versus-reject state transitions are not fully specified or implemented. Phase 6F.1 storage provides local commit idempotency but no durable inbound coordination-message journal/content digest. |

### Finding A — Leader egress proxy

**References:** [proxy architecture, sections 3 and 6](docs/phase-6f2-proxy-architecture.md); [proxy contract, sections 2 and 3](docs/phase-6f2-proxy-contract.md); [deployment profiles, shared coordination requirement and profiles A–C](docs/phase-6f2-deployment-profiles.md); [security review, section 2](docs/phase-6f2-proxy-security-review.md).

Supporting quotations:

> “A dedicated Leader egress proxy/sidecar is part of the target design”

> “The egress proxy owns the Leader client certificate and private key”

> “fixed destination allowlist containing only the approved Helper coordination authority/port”

The local endpoint is consistently proposed as `127.0.0.1:8101`, separate from ingress backends `127.0.0.1:8001` (Leader) and `127.0.0.1:8002` (Helper). The flow distinguishes participant ingress from Leader-initiated egress. The egress proxy owns the Leader client certificate/private key, establishes outbound mTLS to the Helper coordination proxy, validates the Helper server identity, and allows only the configured destination/route. Bounded connect/handshake/write/read timeouts, same-message retry, fail-closed behavior, and no sensitive logging are described. Each profile includes the egress proxy either explicitly or through the shared coordination requirement.

**Assumptions/proposal:** `127.0.0.1` does not enforce per-process access. The documents require OS/network identity isolation or protected local IPC, and explicitly warn against treating the diagram or loopback as proof. They do not select a concrete OS policy or prove it works on the current Windows environment or each deployment target. Therefore this is partially resolved as a contract, not verified as enforceable architecture.

### Finding B — Helper backend authentication and provenance

**References:** [proxy architecture, section 6](docs/phase-6f2-proxy-architecture.md); [proxy contract, sections 2–4](docs/phase-6f2-proxy-contract.md); [deployment profiles, shared coordination requirement](docs/phase-6f2-deployment-profiles.md); [security review, sections 2–3](docs/phase-6f2-proxy-security-review.md).

Supporting quotations:

> “The Helper proxy strips all client-supplied copies of `X-Authenticated-Coordination-Peer`”

> “the backend ... verifies the HMAC envelope, and requires proxy identity and HMAC sender to agree”

The documents distinguish the controls as follows:

1. **TLS peer authentication at proxy:** the Helper coordination proxy validates the Leader client certificate; the Leader egress proxy validates the Helper server certificate and hostname.
2. **Proxy-generated identity assertion:** after mTLS validation, the Helper proxy strips inbound copies and sets one `X-Authenticated-Coordination-Peer` value.
3. **Backend message authentication:** the Helper Rust backend independently verifies the directional HMAC, freshness, message identity/content, and durable replay state, and requires the HMAC sender to match the proxy assertion.
4. **Local network/process isolation:** backend access must be enforceably restricted to the Helper proxy. The documents say loopback alone is insufficient and require the route to remain disabled if isolation cannot be enforced.

**Assumptions/proposal:** The backend cannot observe the TLS peer after termination. The proxy assertion is trusted only because of the proxy and protected backend path. Profiles describe OS/network isolation or protected IPC generally but do not provide a tested platform-specific mechanism for every profile. The code has neither this header validation nor HMAC. Classification is Partially Resolved.

### Finding C — Directional HMAC and canonical encoding

**References:** [proxy contract, sections 5 and 6](docs/phase-6f2-proxy-contract.md); [deployment profiles, shared coordination requirement](docs/phase-6f2-deployment-profiles.md); [security review, sections 8–9](docs/phase-6f2-proxy-security-review.md).

Supporting quotations:

> “Use two independent direction-specific keys.”

> “The exact canonical binary encoding, field limits, byte order, domain separator, version semantics, and MAC construction remain prerequisites for implementation”

Key table in the contract:

| Key | Sender | Verifier | Intended holders |
|---|---|---|---|
| `BLINDMARK_COORD_LEADER_TO_HELPER_HMAC_KEY` | Leader application | Helper application | Leader signer and Helper verifier |
| `BLINDMARK_COORD_HELPER_TO_LEADER_HMAC_KEY` | Helper application | Leader application | Helper signer and Leader verifier |

Out-of-band provisioning, least-privilege process access, no secrets in Git/logs, bounded key versions, rotation overlap, revocation, restart fail-closed behavior, and the fact that a verifier can forge with a symmetric key are specified. The authenticated field inventory includes exact payload bytes and payload length. The docs explicitly state HMAC is not non-repudiation.

**Unresolved:** The canonical binary encoding is not completely specified. “Length-prefixed” and “fixed domain separator” are requirements, not a byte-level schema. Field ordering and widths, string encoding, per-field limits/character rules, invalid-field handling, key-version encoding, payload framing, and exact digest/MAC input concatenation are not fully defined. Durable replay is required, but no implementation exists; Phase 6F.1 has no inbound message journal. Classification is Partially Resolved.

### Finding D — Backend parser security gate

**References:** [proxy architecture, section 8](docs/phase-6f2-proxy-architecture.md); [proxy contract, section 4](docs/phase-6f2-proxy-contract.md); [deployment profiles, shared coordination requirement](docs/phase-6f2-deployment-profiles.md); [security review, section 6](docs/phase-6f2-proxy-security-review.md).

Supporting quotations:

> “Do not ingest security-sensitive binary or signed coordination messages through the current backend parser until it receives a separate security review.”

> “The proxy cannot compensate for a backend parser differential.”

Limitations identified include initial-read UTF-8 conversion, missing strict request-line/header size/count bounds, ambiguous request framing, and no exact-byte preservation guarantee. Required tests include fragmented delivery; multiple requests and connection-close behavior; malformed request lines; oversized headers; duplicate headers; ambiguous/conflicting/overflowing Content-Length; Transfer-Encoding alone and combined; truncated/extra body bytes; arbitrary binary including invalid UTF-8; and proxy/backend framing disagreement/request-smuggling. The docs say proxy-only tests do not satisfy the gate.

**Status nuance:** The gate is explicit and mandatory in the proposed design. No parser review or new parser tests were run in this read-only audit. It is Resolved as documentation of the gate, not as implementation validation.

### Finding E — Delivery and retry semantics

**References:** [proxy architecture, sections 7–8](docs/phase-6f2-proxy-architecture.md); [proxy contract, sections 3 and 6](docs/phase-6f2-proxy-contract.md); [security review, section 7](docs/phase-6f2-proxy-security-review.md).

Supporting quotation:

> “the application must distinguish these cases by durable state, not by proxy inference”

The three cases are explicitly distinguished: request never reaches backend; request arrives but remains unprocessed/pending; request is durably committed but its response is lost. The contract requires stable message ID/sequence and exact bytes on retry, atomic persistence of identity/content digest with state, rejection of conflicting reuse, durable state inspection, and exact retry returning prior outcome without duplicate verification/accumulation/commit. Proxy retries cannot mint new IDs or determine transaction outcome. A timeout cannot reverse `COMMIT_DECIDED`.

**Gap against Phase 6F.1:** Existing `commit_report(report_id, batch_id)` uses an IMMEDIATE local transaction and supports local retries based on report/batch state; it expressly cannot check payload equivalence because payload bytes are absent from its API. Its schema has no inbound authenticated-message table, durable message ID/content digest, or restart state machine. The Phase 6F.2 retry contract is therefore future application behavior, not provided by the Phase 6F.1 storage layer. Also unspecified are a concrete retry owner/schedule/backoff and the exact recovery action for each persisted pending state. Classification is Partially Resolved.

## 4. Cross-document consistency

| Topic | Finding | Evidence/assessment |
|---|---|---|
| Proxy topology and direction | Consistent | Client uses separate Leader/Helper ingress; Leader initiates via local egress proxy to Helper coordination proxy; Helper returns on the connection. |
| Backend ports | Consistent | `127.0.0.1:8001` Leader, `127.0.0.1:8002` Helper; `127.0.0.1:8101` proposed Leader egress listener. The latter is not current source config. |
| TLS termination | Consistent, with one wording ambiguity | Architecture/contract/security review say TLS terminates at the Helper proxy and backend loopback is plaintext. Profile C says “end-to-end TLS to the Helper proxy”; this can mean egress-proxy-to-Helper-proxy TLS, but “end-to-end” could be read as application-to-backend TLS. No document says the backend leg is encrypted. Clarification would reduce ambiguity; no text was silently changed. |
| Certificate ownership | Consistent | Leader egress proxy owns Leader client certificate/key; Helper coordination proxy presents server certificate and validates client certificate. |
| Backend identity | Consistent | Option A proxy-generated `X-Authenticated-Coordination-Peer` after mTLS plus independent HMAC; backend path must be proxy-only; loopback alone insufficient; disable coordination if path enforcement is unavailable. |
| HMAC names/directions | Consistent | Contract and security review give the same directional key names and sender/verifier roles. Deployment references the same two directions without renaming them. |
| Payload/framing | Consistent | Contract says 65,536-byte current configured limit, rejects duplicate Content-Length and Transfer-Encoding, and documents parser limitations. Architecture/security review identify the same binary/framing gate. |
| Timeouts/retries | Consistent | Backend timeout is currently three seconds; proxy timeout values are bounded but unspecified. All docs treat timeouts as ambiguous and preserve durable decision state. |
| Profiles | Consistent | A local, B single host, C separate hosts; none claims implementation or organizational non-collusion. Egress proxy appears in each profile/shared profile requirements. |
| Current versus proposed | Consistent | Documents label proxy, mTLS, HMAC envelope, identity header, DB path isolation, and coordination as proposed/not implemented. |

No direct contradictory endpoint, key direction, or TLS termination statements were found. The Profile C “end-to-end” phrase is the one wording ambiguity identified.

## 5. Current implementation versus proposed design

Read-only source inspection covered `experiments/phase_6e/src/config.rs`, `http.rs`, `routes.rs`, `main.rs`, `leader_service.rs`, `helper_service.rs`, `state.rs`, and `storage.rs`.

| Topic | Current code evidence | Proposed documents, not implemented |
|---|---|---|
| Rust services/listeners | `config.rs` defaults to `127.0.0.1:8001` and `127.0.0.1:8002`; services use `std::net` TCP listeners. | Separate TLS ingress proxies and Leader egress proxy at proposed `127.0.0.1:8101`. |
| TLS/mTLS | Rust HTTP server contains no TLS; no proxy configuration is present in `main.rs`. | Public HTTPS, Leader-to-Helper mTLS between proxies, proxy-generated identity metadata. |
| Shared service key | `DEV_SERVICE_KEY` is a hardcoded string copied to both defaults. `routes.rs` compares it using ordinary string equality for task status. Report ingestion itself returns 501 before any auth-based processing. | Directional HMAC, proxy attestation, key rotation/revocation, durable replay protection. Existing `X-Service-Key` does not implement these. |
| Parser | `http.rs` reads an initial 4096-byte buffer and UTF-8 decodes the entire received chunk before separating headers/body; no strict request-line or header-size/count bounds; absent Content-Length uses bytes in initial body portion; it can retain bytes past declared length; it rejects duplicate Content-Length and any Transfer-Encoding/TE+CL and checks incomplete bodies. Listener worker parses one request then responds and exits; it does not loop for multiple requests on a connection. | The contract prohibits signed/binary ingestion until separate review/test gate passes. Proxy validation is not treated as parser repair. |
| Ingestion | `routes.rs` validates route target/body then returns 501, explicitly stating cryptographic ingestion is not connected. | Real participant-share ingestion and coordination routes remain future work. |
| Verification key | `main.rs` creates a random verification key each process start. | Stable configured/restart-safe verification key is future work; not supplied by proxy documents. |
| SQLite | `storage.rs` has a local API; WAL, `synchronous = 2`, local schema, IMMEDIATE `commit_report`, and local idempotency checks exist. `main.rs` does not initialize Database or pass DB paths to services. | Separate DB paths, durable inbound message journal, durable replay/content digest, peer recovery and exact report-set checks are future work. |

The current storage implementation’s `COMMIT_DECIDED` non-abort trigger and local transactional commit are not distributed atomicity and do not implement the proxy contract’s message-level durable retry journal.

## 6. Remaining risks and unresolved questions

1. **Enforcement mechanism:** Select and validate concrete process/network isolation for the Leader egress listener and Helper backend path for each supported OS/profile. Loopback and a spoofable header alone are insufficient.
2. **Canonical message format:** Complete a byte-level reviewed schema before HMAC implementation: field order and widths, exact length-prefix encoding, string/identifier encoding and limits, invalid values, key-version representation, exact payload framing, domain separator bytes, and exact MAC input.
3. **Retry state machine:** Define message receipt/processing/verification/commit states, owner of retries, bounded retry/backoff behavior, pending-state recovery actions, and durable identity/content-digest retention. Reconcile this with Phase 6F.1’s local `(report_id, batch_id)` idempotency semantics.
4. **Parser gate:** Perform the separate backend parser review and required tests before enabling signed/binary messages. The current server consumes one request per connection and the parser’s initial read/framing behavior requires particular attention.
5. **Proxy operation:** No vendor, config, cert lifecycle, revocation service, DNS pinning policy, firewall rules, timeout values, or log-retention configuration has been selected or exercised.
6. **No runtime evidence:** This audit did not run a proxy, create certificates, test mTLS/HMAC, or execute integration tests. Nothing proposed is implemented or production-ready.

## 7. Final recommendation

**NOT READY FOR HUMAN CONTRACT APPROVAL.** The documents are materially improved and explicitly surface the main boundaries, but the canonical authenticated encoding remains underspecified and the required proxy-only local path enforcement and retry state machine are not concrete enough to approve as a complete contract. This is a documentation-completeness recommendation only. It does not authorize implementation or Checkpoint B.

## 8. Complete revised document contents

The following four fenced blocks reproduce the complete files read during this audit. Nested Markdown fences are preserved using four-backtick outer fences.

### 8.1 `docs/phase-6f2-proxy-architecture.md`

````markdown
# Phase 6F.2A TLS Reverse Proxy Architecture

**Status:** Proposed architecture contract for human review. Not implemented. This document does not authorize Phase 6F.2 Checkpoint B.

## 1. Purpose

Place a TLS-terminating reverse proxy in front of each existing Rust HTTP service, preserving separate participant ingress paths while leaving Rust dependencies and the current HTTP implementation unchanged in this task. TLS protects the client-to-proxy leg. The proxy host and its proxy-to-Rust backend leg remain inside the trusted boundary; loopback HTTP is plaintext, not end-to-end encryption.

## 2. Current behavior

- `ServiceConfig::default_leader()` binds Leader to `127.0.0.1:8001`; `default_helper()` binds Helper to `127.0.0.1:8002`.
- The current service binary uses the standard-library HTTP implementation. It does not terminate TLS or read proxy-specific environment variables.
- Report ingestion intentionally returns HTTP 501. No participant share is accepted into the service through that route today.
- The current `X-Service-Key`/`Authorization` check is shared development scaffolding, not production service authentication.
- Phase 6E's SQLite layer is not initialized by the service runners. Independent database configuration and use remain future work.
- These defaults are confirmed source behavior, not a deployed firewall or proxy configuration.

## 3. Proposed topology

```text
Participant client
  | HTTPS, server certificate + hostname verification
  +--> Leader ingress proxy -- plaintext HTTP over loopback --> 127.0.0.1:8001 Leader
  |                                                           Leader-only DB (future)
  |
  +--> Helper ingress proxy -- plaintext HTTP over loopback --> 127.0.0.1:8002 Helper
                                                              Helper-only DB (future)

Leader Rust service
  | local coordination request + application HMAC
  +--> Leader egress proxy (127.0.0.1:8101; Leader-only access policy)
        | outbound TLS/mTLS; Helper destination allowlisted
        +--> Helper coordination proxy -- plaintext HTTP over loopback --> Helper internal route (future)
              | validates Leader client certificate
              +--> Helper Rust backend validates proxy identity metadata + application HMAC
```

The participant sends only InputShare 0 to the Leader HTTPS endpoint and only InputShare 1 to the Helper HTTPS endpoint. FastAPI may handle eligibility/control-plane operations but must not receive or relay both plaintext shares. The Collector must not access individual participant shares.

A dedicated internal Helper coordination listener/route is proposed so it can have stricter routing and mTLS policy than participant ingestion. Its endpoint and implementation do not currently exist. Because the Rust Leader service has no TLS client, a dedicated Leader-side egress proxy/sidecar is part of the target design: the Leader sends coordination requests to the local proxy, which owns the Leader client certificate/key and makes outbound TLS/mTLS connections only to the configured Helper coordination endpoint. The Helper coordination proxy validates the certificate identity and forwards only the allowlisted internal route to the private Helper backend. Responses return over the same authenticated TLS connection. Leader initiates all peer operations; no Helper-to-Leader callback is assumed.

The egress proxy's proposed TCP listener is `127.0.0.1:8101`. Loopback binding alone does not prove that the caller is the Leader process. Deployment must restrict use to the Leader OS identity using enforceable local policy (for example, an isolated service/network namespace and host rules). If the platform cannot enforce process-level access to loopback TCP, use an OS-ACL-protected local IPC endpoint instead and add the required client support later. The proxy must allow only the configured Helper destination/port and coordination route, with no arbitrary URL, hostname, redirect, or proxy chaining.

## 4. Trust boundary

The TLS proxy terminates TLS and can read request bodies, including the one plaintext share addressed to its corresponding backend. Proxy host administrators, processes, logs, crash dumps, and backups are therefore inside that share's trusted boundary. Proxy-to-backend HTTP is plaintext even when sent to loopback. Do not describe this as end-to-end encryption.

HMAC remains useful for application-level message integrity, role binding, and replay checks. It does not encrypt data or replace TLS. TLS does not establish organizational non-collusion. Leader and Helper processes on one host or under one administrator are not independent trust domains.

## 5. Data ownership

- Leader proxy/backend may handle only the Leader-designated input share and the public protocol metadata needed by Prio3.
- Helper proxy/backend may handle only the Helper-designated input share and required public metadata.
- No proxy tier or FastAPI control plane may combine the two plaintext input shares.
- Leader and Helper are intended to use separate SQLite files, separate service identities, and separate access controls. This separation is a future deployment goal; Phase 6E does not currently configure DB paths.
- Verifier shares/messages and commit metadata may cross the authenticated Leader-to-Helper channel as specified by the eventual protocol.

## 6. Proposed ingress and coordination paths

Participant ingress uses separate public HTTPS hostnames, for example `<leader-hostname>` and `<helper-hostname>`, each mapped to exactly one proxy/backend pair. The proxy validates the expected public Host and does not permit a route to switch aggregator roles. Participant HTTPS uses server authentication; ordinary browsers are not assumed to have client certificates. Participant eligibility/authentication remains a separate protocol and is not implemented by this proxy contract.

For internal coordination, Leader connects to a private Helper coordination endpoint with mTLS. The Helper proxy authenticates the Leader client certificate and forwards only the dedicated internal route to the local Helper backend. The response is protected by the same TLS session and separately validated by the application envelope/HMAC contract. Internal coordination must not use a public participant hostname or expose its backend route to arbitrary clients.

The Leader egress proxy owns and reads the Leader client certificate and private key; the Rust Leader does not. It validates the Helper server chain, hostname, validity, and approved identity. The Helper ingress proxy validates the Leader client certificate under its restricted coordination trust policy. The egress proxy uses bounded connect/handshake/read/write timeouts, never retries a new message identity, and may retry only the same durable identity according to the retry contract. Proxy failure is a transport failure, not a transaction decision. Neither proxy logs bodies, shares, credentials, HMAC values, or private-key material.

Two backend-authentication approaches were reviewed. (A) Proxy-generated trusted identity metadata requires the Helper proxy to strip all inbound copies, set a coordination-peer identity only after mTLS validation, and ensure the backend is reachable only from that proxy by enforceable OS/network controls. (B) Application HMAC validates message authenticity and content but, by itself, cannot show that the request traversed the TLS proxy. **Selected for proxy provenance: A, combined with directional application HMAC for message integrity and durable replay protection.** The backend accepts the proxy-generated identity only over a path restricted to the Helper proxy, checks that identity against the authenticated HMAC sender, and validates sender/recipient, payload binding, freshness, and durable replay state. Loopback alone is insufficient. The backend cannot see the terminated TLS peer; without the proxy attestation and enforceable backend-path restriction, it must not claim proxy traversal. Forwarded headers from arbitrary clients are not authentication and must never override either check. This metadata/HMAC integration is not implemented in the current backend.

## 7. Protocol and durable processing boundary

The reverse proxy transports requests; it does not establish cryptographic acceptance. A successful TLS handshake, proxy forwarding, HTTP 2xx, or message HMAC is not proof that `verify_next` succeeded. The application may acknowledge acceptance only after the operation's required durable state transition and Prio verification have completed. A timeout is an ambiguous outcome: recovery must consult durable state and retry the same idempotent message identity. It must not roll back or reverse a durable `COMMIT_DECIDED` decision.

Exact task, batch, configuration, and report-set matching remains an application responsibility. Public Collector readiness must not expose individual report IDs.

## 8. Retry and parser gates

Every coordination operation must use a stable authenticated message ID/sequence and be durably deduplicated by the backend. A retry must inspect durable state and distinguish: (1) the request never reached the backend; (2) it reached the backend but was not processed; (3) it was processed and durably committed but the response was lost. The proxy cannot determine which occurred and must not independently declare success, failure, rollback, or a new message identity. A timeout or proxy failure cannot reverse a durable `COMMIT_DECIDED` decision.

Security-sensitive binary or signed message ingestion is prohibited until the Rust backend HTTP parser has a separate security review and passes tests for fragmented delivery, multiple requests/connection behavior, malformed request lines, oversized headers, duplicate headers, ambiguous Content-Length, Transfer-Encoding, truncated bodies, arbitrary binary payloads, and proxy/backend framing disagreement. The current initial-read UTF-8 conversion, missing strict request-line/header bounds, request-framing behavior, and lack of an exact-byte preservation guarantee are unresolved blockers. No proxy may be treated as a substitute for this review.

## 9. Implementation boundary

This Phase 6F.2A deliverable is documentation only. No egress or ingress proxy is configured or launched; no certificate is created; no route, HMAC, mTLS, database-path, firewall, parser, or network-coordination code is implemented. The existing preimplementation audit remains unchanged. Human review must approve the revised proxy contract and close the open decisions before Phase 6F.2 implementation resumes.
````

### 8.2 `docs/phase-6f2-proxy-contract.md`

````markdown
# Phase 6F.2A Vendor-Neutral Proxy Contract

**Status:** Proposed contract for review; no proxy or application changes are implemented.

## 1. External TLS listeners

Provide distinct public HTTPS endpoints for Leader participant ingress and Helper participant ingress. Each endpoint must:

- Require TLS 1.2 or newer and prefer TLS 1.3; disable obsolete protocol versions and weak cipher suites.
- Present a valid server certificate chain whose SAN matches the requested hostname. Clients must validate both the chain and hostname; no certificate-warning bypass is acceptable.
- Renew certificates before expiry, deploy them atomically, and verify the replacement before retiring the prior certificate.
- Restrict private-key files to the proxy service identity and authorized certificate-management identity; do not make keys world-readable, place them in Git, logs, or this repository.
- Have no public plaintext HTTP service. Do not expose port 80 as an application endpoint or use it to carry participant bodies. Firewall-drop or refuse plaintext ingress.
- Avoid logging request/response bodies, authorization values, HMAC values, cookies, tokens, or input shares.

The proxy host terminates TLS and can read plaintext bodies. This is a trusted part of the relevant share boundary. Proxy-to-backend HTTP is plaintext, even over loopback.

## 2. Backend mapping

| Public route | Only permitted backend | Current source default | Status |
|---|---|---|---|
| Leader ingress | `127.0.0.1:8001` | Leader defaults to this address | Source-confirmed default; proxy not configured |
| Helper ingress | `127.0.0.1:8002` | Helper defaults to this address | Source-confirmed default; proxy not configured |
| Internal coordination | Dedicated private Helper proxy route, then Helper's local backend | No route/listener exists | Proposed; future implementation |

Routing must be role-specific and fail closed. Leader paths cannot be rewritten to Helper and vice versa. The backend ports must not be bound to public interfaces or reachable through public firewall rules. A deployment must verify effective socket bindings and firewall state, not rely on this document.

If Leader and Helper are on separate hosts, `127.0.0.1` refers to each proxy's own host. It is not a cross-host channel. Leader-to-Helper coordination must terminate TLS at the Helper-side private proxy using mTLS over an authenticated encrypted private network or equivalent protected path.

### Leader-side egress proxy

The Rust Leader is not a TLS client in the current implementation. A dedicated Leader egress proxy/sidecar is therefore required for the proposed coordination flow:

1. Leader sends its coordination request to the local egress listener at `127.0.0.1:8101` (proposed port, not current configuration).
2. The egress proxy owns the Leader client certificate and private key; the Rust process does not read them.
3. The egress proxy establishes outbound TLS/mTLS to the single configured Helper coordination-proxy hostname and port.
4. The Helper coordination proxy validates the Leader certificate chain, validity, client-auth purpose, and explicitly authorized identity.
5. The Helper proxy forwards only the dedicated internal coordination route to its private Helper backend.
6. The response returns through the authenticated TLS connection and the local egress listener.

The egress listener is loopback-only, but loopback by itself is not caller authentication. Permit access only from the Leader service identity using enforceable OS/network isolation (for example, service-specific network namespace and host rules). If the target OS cannot enforce process-level TCP access, use OS-ACL-protected local IPC (Unix socket with restrictive owner/mode or a Windows named pipe with a Leader-only ACL), with explicit client support required. Do not silently fall back to unrestricted loopback access.

The egress proxy must have a fixed destination allowlist containing only the approved Helper coordination authority/port and allowed route prefix. Reject redirects, caller-selected URLs, arbitrary CONNECT, DNS rebinding to non-allowlisted addresses, proxy chaining, and requests to participant ingress or unrelated hosts. Resolve and pin/check the configured destination according to the deployment's DNS policy.

Use bounded connect, TLS handshake, request-write, and response-read timeouts. Do not retry under a new message ID. A retry may retransmit only the same durable message identity and exact authenticated bytes after the application recovery policy permits it. The egress proxy must not infer the transaction outcome from its own timeout. Log route identity, status, duration, and non-sensitive correlation metadata only; never log bodies, shares, HMACs, certificate private material, or secrets. If the egress proxy is unavailable or its certificate/configuration is invalid, fail closed and report transport failure; do not queue, transform, or mark the request accepted.

### Helper backend authentication boundary

Two approaches were considered:

- **A. Proxy-generated identity metadata:** after mTLS validation, the Helper proxy strips every incoming copy of a reserved identity header and adds its own value. The backend accepts it only on a path that enforceable OS/network policy restricts to that proxy. This trusts the proxy and local routing controls; a header or loopback connection alone is not proof of identity.
- **B. Application HMAC envelope:** the Helper Rust backend validates a canonical, domain-separated HMAC envelope, including sender and recipient, while proxy-to-proxy mTLS supplies an additional transport control. HMAC proves a message came from a holder of the directional key, but does not prove that the request traversed the TLS proxy.

**Selected for proxy provenance: A, combined with directional application HMAC for message integrity, content binding, and durable replay protection.** The Helper proxy strips all client-supplied copies of `X-Authenticated-Coordination-Peer`, validates the Leader client certificate via mTLS, and sets that header to the configured Leader identity only after validation. The Helper backend accepts the metadata only through a backend path enforceably restricted to the Helper proxy; it also validates the HMAC sender and requires the identity values to agree. The backend does not see the terminated TLS peer. Loopback binding alone is not the path restriction and is not proof of sender or proxy identity. If the deployment cannot enforce that only the Helper proxy reaches the backend (for example through an OS-ACL-protected local IPC path or verified process/network isolation), the route must remain disabled. The current `X-Service-Key` does not implement this design.

## 3. mTLS contract for service coordination

### Initiation and identity

The proposed initial protocol has Leader initiate all coordination requests to the Helper's dedicated private coordination endpoint. Helper replies on the same connection. No Helper callback to Leader is part of this contract. Leader recovery logic is expected to retry from its durable decision state; retry/recovery code is not yet implemented.

- Leader presents a client certificate with an explicit coordination-client identity and appropriate client-auth EKU.
- Helper's coordination proxy presents its server certificate, with SAN matching the configured Helper coordination hostname and server-auth EKU.
- Leader validates the Helper server chain, hostname, validity, and permitted CA/trust domain.
- Helper validates the Leader client chain, identity/SAN, validity, and client-auth EKU against a dedicated coordination-client trust store.
- Participant/browser certificates are not assumed. Participant HTTPS server authentication and eligibility are separate from internal mTLS.
- Do not use the public participant ingress listener as the mTLS coordination route unless separate listener, routing, and authorization controls are verified.

### Trust, rotation, and revocation

Use a narrowly scoped CA/trust store for coordination identities, separate from public web PKI where practical. Rotate certificates using overlap: install the new identity and trust first, verify a test handshake, switch traffic, then remove the old trust/certificate. Expired, not-yet-valid, revoked, wrong-purpose, unknown-CA, and wrong-identity certificates must fail closed. Define the operational revocation source/check (for example, short-lived certificates with promptly updated trust/revocation data); do not silently soft-fail revocation checks. Rotation and revocation automation are future deployment work.

mTLS authenticates possession of private keys under the configured trust policy. It does not protect against a compromised proxy host, prove organizational non-collusion, or replace message-level idempotency/replay protection.

## 4. HTTP and proxy-header handling

### Framing and limits

- Enforce an ingress body limit no larger than the backend's configured limit (currently 65,536 body bytes in Phase 6E); the proxy must reject oversize requests with 413 before forwarding.
- Reject duplicate or conflicting Content-Length values and any request containing both Transfer-Encoding and Content-Length.
- Reject unsupported Transfer-Encoding, including chunked, unless a separately reviewed backend framing change explicitly supports it. The current backend rejects Transfer-Encoding.
- Forward one unambiguous Content-Length matching the exact body delivered. Never forward duplicate Content-Length or both Transfer-Encoding and Content-Length.
- Preserve the backend's three-second socket read/write timeout unless a reviewed requirement changes it. Proxy upstream connect/read/write timeouts must be bounded and must not be interpreted as a transaction outcome.
- Do not let proxy buffering, retries, or protocol translation change authenticated body bytes. Internal retries must preserve the stable message ID/sequence and be safe under application-level deduplication.

The existing parser has framing limitations documented in the Phase 6F.2 preimplementation audit, including UTF-8 conversion of the initial read and lack of strict request-line/header bounds. This proxy contract does not fix those application limitations. Before security-sensitive binary or signed messages are added, backend framing must be independently reviewed; the proxy must not be treated as a parser security boundary.

### Headers

- `Host`: validate the expected public hostname at ingress. Set the backend Host deliberately to the configured upstream identity; do not use a client-supplied Host to select an arbitrary upstream.
- `X-Forwarded-For`: remove every client-supplied instance, then set a single trusted value derived from the connection peer. Define trusted proxy hops if another load balancer precedes this proxy.
- `X-Forwarded-Proto`: remove incoming values and set exactly `https` only after successful TLS termination.
- `X-Forwarded-Host`: remove incoming values and set the validated external hostname.
- `Forwarded` and other proxy metadata: strip client-supplied values and generate only explicitly supported values.
- `Content-Length` / `Transfer-Encoding`: apply the framing rules above and emit one canonical framing form to the backend.
- Authentication headers: never inject a participant credential or HMAC secret. Remove untrusted internal-service authentication headers at public ingress. Preserve only explicitly allowlisted, singular participant-auth headers where an eventual participant contract requires them. For internal coordination, reject duplicate HMAC headers and preserve the authenticated envelope exactly. The Helper coordination proxy must remove every incoming `X-Authenticated-Coordination-Peer` value and set exactly one such header only after successful mTLS identity validation. The Helper backend may trust this metadata only over the enforceably proxy-restricted backend path and must require the value to match the HMAC sender. Never log authentication values.

The Rust services must not trust forwarded headers as proof of client address, scheme, or identity from arbitrary direct clients. They currently do not implement a trusted-proxy allowlist; forwarded-header trust is future work.

### Parser security gate

Do not ingest security-sensitive binary or signed coordination messages through the current backend parser until it receives a separate security review. Known limitations include UTF-8 conversion of the initial read (binary bodies are unsafe), no strict request-line/header size or count bounds, incomplete/excess framing behavior, and no guarantee that authenticated bytes are preserved exactly. The proxy cannot compensate for a backend parser differential.

Before enabling such messages, add focused parser and end-to-end proxy/backend tests for:

- fragmented request line, headers, delimiter, and body delivery;
- multiple requests on one connection and explicit connection-close handling;
- malformed request lines and unsupported HTTP versions;
- oversized request line, headers, and excessive header count;
- duplicate headers, including case-variant duplicates;
- ambiguous, conflicting, signed, or overflowing Content-Length values;
- Transfer-Encoding alone and together with Content-Length;
- truncated request bodies and bytes beyond the declared length;
- arbitrary binary payload bytes, including invalid UTF-8, preserved byte-for-byte;
- proxy/backend framing disagreement and request-smuggling cases.

This task makes no parser change. The gate is not satisfied by proxy-only tests.

## 5. HMAC application-message contract (not implemented)

The existing `X-Service-Key` comparison is development scaffolding. The target coordination protocol uses HMAC-SHA256 over an unambiguous canonical envelope, with a fixed domain separator and length-prefixed fields. The authenticated bytes must bind at least:

- protocol/domain version;
- message ID and sequence;
- sender and recipient roles;
- task ID, report ID, and batch ID;
- message type and round;
- issued-at/expiry or equivalent bounded freshness data;
- exact payload bytes and payload length.

The receiver validates sizes, identifiers, protocol/message type, expected roles, clock skew/expiry, MAC in constant time, and replay state. The durable replay ledger rejects reused IDs/sequences with different content. An exact previously accepted message may receive an idempotent response only after state is checked; conflicting reuse is rejected. Replay tombstone retention must exceed the accepted retry/expiry window and be coordinated with durable state cleanup.

Do not authenticate ambiguous concatenated strings or rely on TLS context as a replacement for the signed message contract. HMAC proves authenticity only relative to the holders of the symmetric key. It does not provide confidentiality, non-repudiation, participant authentication, or organizational independence.

### Directional HMAC key ownership and lifecycle

Use two independent direction-specific keys. Do not reuse one key in both directions, and do not infer direction from a generic shared `BLINDMARK_LEADER_SERVICE_SECRET` or `BLINDMARK_HELPER_SERVICE_SECRET`.

| Key name (proposed) | Message sender | Verifier | Processes that can access the symmetric key |
|---|---|---|---|
| `BLINDMARK_COORD_LEADER_TO_HELPER_HMAC_KEY` | Leader application | Helper application | Leader signer and Helper verifier only |
| `BLINDMARK_COORD_HELPER_TO_LEADER_HMAC_KEY` | Helper application | Leader application | Helper signer and Leader verifier only |

Provision each direction key out of band through the approved secrets manager or protected deployment secret mechanism. Mount/provide it only to the two processes named in its row, with least-privilege read access; keep proxy processes from reading HMAC keys unless a separately reviewed design requires it. Never place values in source, environment examples, logs, crash reports, or Git. A verifier holding a symmetric HMAC key can also forge messages under that key; HMAC is not non-repudiation. Compromise of either holder compromises authenticity in that direction.

Include a bounded key-version identifier in the canonical envelope. The receiver accepts only explicitly configured versions and selects the candidate key from the local allowlist before verifying the MAC; the version field itself is covered by the MAC. Provision a new version to sender and verifier, verify readiness, then switch sender output. Retain the old version only for a documented bounded retry/expiry window; revoke it on compromise and reject it immediately. On restart, fail closed if the active key/version is missing or invalid; never generate a replacement silently. Rotation/revocation state and active version must be consistent across both parties before sending messages.

The exact canonical binary encoding, field limits, byte order, domain separator, version semantics, and MAC construction remain prerequisites for implementation and require protocol review. This document does not invent a construction or authorize HMAC implementation.

The proposed `BLINDMARK_PROXY_TRUSTED`, `BLINDMARK_PROXY_CLIENT_CA`, `BLINDMARK_PROXY_SERVER_CERT`, and `BLINDMARK_PROXY_SERVER_KEY` variables are not current Phase 6E settings. Their exact semantics, whether paths or trust identifiers, and which process consumes them require implementation-specific review. `BLINDMARK_LEADER_DB` and `BLINDMARK_HELPER_DB` are future database-path settings, not present configuration.

## 6. Response and failure contract

The proxy may return only a transport-level result. It must never translate a backend error, timeout, malformed response, or unavailable service into successful cryptographic acceptance. A 2xx response may be emitted only when the backend actually returned that success; application success semantics must be defined by the durable operation, not by proxy forwarding.

| Condition | Required behavior |
|---|---|
| Expired/invalid server cert or hostname | Client rejects handshake; no HTTP request is sent |
| Invalid/missing mTLS client cert on coordination route | Proxy rejects, normally 401/403; no backend request |
| Proxy unavailable | Connection failure; client retries according to stable message identity |
| Backend unavailable | 502/503; never acceptance |
| Backend timeout | 504/timeout; outcome is ambiguous; recover/query durable state, do not roll back a decision |
| Oversize body | 413; do not forward |
| Malformed/ambiguous framing | 400; do not forward |
| Duplicate exact message | Application checks durable identity and returns idempotent result only if same content/state |
| Reused ID with different content or replay rejected | 409 or 401/403 as precisely defined by application; no state advance |
| HMAC authentication failure | Reject; do not forward to processing or alter durable state |
| Proxy restart | No application-state reset; TLS listener reloads validated cert/config or remains unavailable |
| Backend restart | Durable application state is recovered; in-flight requests may be retried by identity; no assumption that timeout means rollback |

A timeout must never reverse or authorize reversal of a durable `COMMIT_DECIDED` state. The proxy cannot decide whether a transaction committed.

### Retry and recovery cases

For every timeout or lost connection, recovery must distinguish these cases by durable state, not by proxy inference:

1. **Request never reached the backend:** no inbound identity exists. The sender may retry the same exact message ID/sequence and authenticated bytes.
2. **Request reached the backend but was not processed:** the durable journal may show received/pending but not completed. Resume or safely reject according to the persisted state machine; do not create a second logical message.
3. **Request was processed and durably committed but the response was lost:** the journal/operation state is authoritative. An exact retry returns the prior durable outcome without repeating verification, accumulation, or commit.

The message identity and content digest must be persisted atomically with processing state. Reuse of an identity with different content is rejected. Before retrying, the application inspects durable state or retransmits the same identity for an idempotent lookup. Proxy retries never independently determine transaction outcome, never mint a new identity, and never move a batch to ABORTED. Once `COMMIT_DECIDED` is durable, timeout, proxy failure, restart, or lost response cannot reverse it.

## 7. Logging and operations

Log bounded operational metadata only: route name, status, duration, upstream health, and a non-sensitive correlation identifier. Do not log request/response bodies, input shares, HMACs, service secrets, cookies, bearer tokens, or private-key material. Restrict and rotate access logs; define retention and access audit. Ensure errors do not echo authentication values or payload fragments.

This is a vendor-neutral contract. No Nginx, Envoy, HAProxy, Caddy, cloud load balancer, TLS configuration, certificate, or firewall rule has been selected or deployed.
````

### 8.3 `docs/phase-6f2-deployment-profiles.md`

````markdown
# Phase 6F.2A Deployment Profiles

**Status:** Proposed deployment profiles. None is implemented by this document. The current Phase 6E binaries are development scaffolding.

## Current source behavior

`ServiceConfig::default_leader()` binds to `127.0.0.1:8001` and `default_helper()` binds to `127.0.0.1:8002`. This is source-confirmed loopback configuration, not proof of an installed proxy, firewall, TLS listener, separate database, or operating-system identity. Report ingestion still returns 501. Phase 6E does not currently load the proposed DB, proxy, certificate, or coordination-secret environment variables.

## Coordination transport requirement shared by profiles

The target flow includes a dedicated Leader-side egress proxy at the proposed local endpoint `127.0.0.1:8101`. It owns the Leader client certificate/key and may connect only to the allowlisted Helper coordination proxy and route. Only the Leader OS identity may use the local listener; loopback alone is not caller authentication. Enforce this with OS/network isolation or use OS-ACL-protected local IPC where per-process TCP restrictions are unavailable. The Helper ingress proxy validates mTLS and forwards only the internal coordination route to its private backend. Responses return over the same TLS connection.

The selected backend proxy-provenance control is proxy-generated identity metadata (Option A): after mTLS validation, the Helper proxy strips all incoming copies of `X-Authenticated-Coordination-Peer` and sets one authenticated peer identity. The Helper backend accepts it only over a path enforceably restricted to the Helper proxy, and requires it to match the sender in the application HMAC envelope. HMAC supplies message integrity/content binding/durable replay protection (Option B's message-authentication properties), but by itself does not prove proxy traversal. TLS terminates at the Helper proxy, so the Rust backend does not see the TLS peer. Loopback alone is not sufficient access control; if enforceable backend-path isolation is unavailable, the coordination route stays disabled. Direction-specific Leader-to-Helper and Helper-to-Leader keys are shared only between each sender and intended verifier, provisioned out of band, versioned in the envelope, rotated with bounded overlap, revoked on compromise, and unavailable-key startup fails closed. Either symmetric key holder can forge that direction's messages.

The current parser is not approved for security-sensitive binary/signed messages. A separate parser review and tests for fragmented and multiple requests, malformed/oversized headers and request lines, duplicate headers, ambiguous Content-Length, Transfer-Encoding, truncation, arbitrary binary exact-byte preservation, and proxy/backend framing disagreement are prerequisites in every profile.

## Profile A: Local development

### Proposed topology

- Run Leader and Helper as separate local processes.
- Use distinct local database files once DB path configuration is implemented; do not point both services at one file.
- Run a local reverse proxy with separate Leader and Helper HTTPS hostnames/ports and development-only certificates trusted only by the local test client.
- Run a separate Leader egress proxy locally, bound to loopback port 8101 or an OS-ACL-protected IPC equivalent, with Leader-only access and a fixed Helper destination allowlist.
- Keep both Rust backends bound to loopback and do not expose them on LAN/public interfaces.
- For internal coordination tests, use a local-only mTLS route with test identities and direction-specific versioned HMAC keys; use synthetic reports and never use production secrets/data.
- Do not expose participant ingress, proxy admin interfaces, or debugging endpoints publicly.

### Limitations

A development certificate offers no trust beyond clients that install its local trust root. The host administrator can access both services and both proxy plaintext requests, so this profile provides no organizational non-collusion guarantee. It is not a production or privacy-boundary validation environment.

### Status

Loopback backend defaults exist in source. Local proxy, certificates, separate persistent DB paths, mTLS, HMAC coordination, and network tests are not implemented or run.

## Profile B: Single-host test deployment

### Proposed topology

- Put one reverse proxy on the same host as the two Rust processes.
- Terminate public TLS at the proxy and forward each ingress route only to its matching loopback backend (`127.0.0.1:8001` for Leader; `127.0.0.1:8002` for Helper).
- Public firewall exposes only intended HTTPS proxy listeners; backend ports, proxy administration, and database files are not reachable from public networks.
- Use separate operating-system service identities and separate DB files/permissions for Leader and Helper, even though the same host kernel and root administrator remain common.
- If coordination is enabled, use a distinct private mTLS route with explicit client identity checks and application HMAC/replay protection. Do not assume loopback itself provides encryption.
- Use a Leader-side egress proxy with a fixed Helper destination allowlist; permit its local listener only to the Leader OS identity using enforceable host isolation or protected local IPC.
- The proxy host is trusted and can read the relevant plaintext request body after TLS termination. Do not log request bodies or secrets.

### Limitations

A compromise of the common host, root account, proxy, or shared operational plane can expose both services and potentially both input shares. Separate processes, ports, service accounts, or databases on one host do not make independent organizations. This profile is for controlled testing, not a non-collusion claim.

### Status

The backend loopback addresses match current defaults. Proxy configuration, firewall restrictions, service identities, database isolation, TLS, and coordination are future work and have not been validated.

## Profile C: Separate-host deployment

### Proposed topology

- Place Leader and Helper on separately administered hosts/trust domains, each with its own ingress proxy, operating-system identity, database file/storage, certificate lifecycle, and security monitoring.
- Participant clients connect directly to each role's separate HTTPS hostname. FastAPI is not a plaintext-share relay.
- Each local proxy forwards to its own loopback Rust backend; public networks cannot reach backend ports or database services.
- Leader initiates internal coordination to a private Helper coordination hostname over TLS with mTLS. Use a private routed network/VPN or equivalent network controls plus end-to-end TLS to the Helper proxy. Do not use a plaintext inter-host HTTP hop.
- Leader sends through its own local egress proxy, which holds the Leader client certificate and key, enforces the fixed Helper destination allowlist, and uses bounded timeouts. The Helper proxy authenticates that certificate and forwards only the internal route.
- Configure server certificate/hostname verification, client-certificate trust and identity authorization, certificate rotation/revocation, firewall allowlists, and separate HMAC message keys/replay state.
- Do not mount or share the databases. Limit access to each DB to its corresponding service identity and authorized backup/recovery operators.

### Limitations and unverified assumptions

This profile is not implemented. Separate hosts do not automatically mean separate legal entities, administrators, cloud control planes, or backups. The deployment must independently assess ownership, privileged access, observability, incident response, key custody, network eavesdropping, and collusion. TLS/mTLS and HMAC do not by themselves establish non-collusion, participant authenticity, truthful measurements, Sybil resistance, or differential privacy.

### Required readiness evidence before any claim

- Verified effective listener bindings and firewall policy on both hosts.
- Verified distinct service identities, DB permissions, and backup boundaries.
- Successful positive and negative TLS/mTLS identity/hostname/expiry/revocation tests.
- Verified proxy framing, body-size, timeout, logging, and route-isolation tests.
- Authenticated message canonicalization, durable replay/deduplication, and recovery tests.
- Independent security review of the deployed topology and the actual Prio verification/commit flow.

No profile authorizes production use or automatically approves Phase 6F.2 implementation.
````

### 8.4 `docs/phase-6f2-proxy-security-review.md`

````markdown
# Phase 6F.2A Proxy Security Review

**Review type:** Architecture contract review, documentation only. No proxy, certificates, source code, or runtime configuration was deployed or tested.

## 1. Security conclusion

A TLS reverse proxy is a viable transport architecture for this prototype only if the proxy host is explicitly trusted, participant ingress routes remain separate, backend listeners are private, and coordination uses a separately authenticated private path. TLS terminates at the proxy; it does not extend through the plaintext proxy-to-backend hop. The proposed contract is ready for human review, not implementation authorization or production use.

## 2. Current behavior versus proposed controls

### Current behavior (source-confirmed)

- Leader defaults to `127.0.0.1:8001`; Helper defaults to `127.0.0.1:8002`.
- The Rust listener is standard-library HTTP and contains no TLS support.
- Report ingestion returns 501; Phase 6E does not yet accept or persist participant input shares through the HTTP routes.
- `X-Service-Key`/`Authorization` uses a hardcoded shared development key and ordinary equality. It is not an HMAC message protocol or mTLS implementation.
- The binary generates a new random verification key at process start; persistence and configuration are not wired into the service.
- The HTTP parser has bounded payloads/timeouts and rejects duplicate Content-Length and Transfer-Encoding, but the Phase 6F.2 audit identifies request-line/header/framing limitations that must be resolved or constrained before security-sensitive signed messages are processed.
- No reverse proxy, firewall, TLS, mTLS, separate service identity, or per-service DB path is installed by the code.

### Proposed controls (not implemented)

- Separate public HTTPS endpoints and fixed role-to-backend routing.
- TLS 1.2 minimum, TLS 1.3 preferred, valid chain and hostname verification, managed renewal and private-key permissions.
- Private loopback backends and a dedicated private Leader-initiated mTLS coordination route to Helper.
- Strict proxy framing/body limits and no body/secret logging.
- Canonical domain-separated HMAC messages with sender/recipient/task/report/batch/protocol/type/message identity/sequence/freshness/payload binding and durable replay handling.
- Separate DB files and OS identities for the two roles.

### Leader egress proxy and backend identity

The target path is Leader Rust service -> local Leader egress proxy at the proposed `127.0.0.1:8101` -> allowlisted Helper coordination proxy over outbound TLS/mTLS -> private Helper Rust route. The egress proxy, not the Rust Leader, owns the Leader client certificate/private key. The Helper proxy validates the certificate identity and forwards only the dedicated internal route; the response returns over the same TLS session. The egress proxy has a fixed destination/route allowlist, bounded connect/handshake/read/write timeouts, no arbitrary URL/redirect/proxy-chain behavior, no sensitive logging, and fails closed when unavailable or misconfigured.

Loopback binding does not identify which process connected. Restrict the egress endpoint to the Leader OS identity using enforceable OS/network isolation; if loopback TCP cannot be restricted per process, use OS-ACL-protected local IPC. Do not equate local reachability with Leader identity.

Two Helper-backend identity approaches were assessed: (A) trust proxy-generated identity metadata only after stripping inbound copies and restricting backend access to the proxy; or (B) validate an application HMAC envelope at the Rust backend, with mTLS as an additional proxy-to-proxy transport control. **Option A is selected for proxy provenance, combined with directional application HMAC for content integrity and durable replay protection.** After validating the Leader client certificate, the Helper proxy strips every incoming `X-Authenticated-Coordination-Peer` header and sets exactly one trusted value. The backend accepts it only on a path enforceably restricted to the Helper proxy, verifies the HMAC envelope, and requires proxy identity and HMAC sender to agree. Because TLS terminates at the Helper proxy, the backend does not itself see the TLS peer. Loopback alone is not adequate access control; if the proxy-only backend path cannot be enforced, coordination must remain disabled. HMAC alone authenticates message key possession but does not prove proxy traversal. The current `X-Service-Key` does not implement either reviewed design.

## 3. Trust boundaries and confidentiality

The participant-to-proxy leg is encrypted when HTTPS is correctly configured and verified. At TLS termination, the proxy can read the request body. The proxy machine, proxy process, administrators, logs, crash dumps, monitoring agents, and backups are therefore within the corresponding aggregator's trust boundary.

Proxy-to-backend loopback traffic is plaintext. Loopback limits remote reachability on a correctly configured host but is not encryption and does not protect against local privileged users or host compromise. On separate machines, loopback is local to each host; Leader-to-Helper traffic must use an authenticated encrypted network path plus TLS/mTLS to the Helper coordination proxy.

TLS does not prove that the two roles are organizationally independent. A shared administrator, cloud control plane, logging account, backup operator, or host can defeat the non-collusion assumption. FastAPI must never receive both plaintext input shares. The Collector must not receive or read individual participant shares.

## 4. Authentication properties must remain distinct

- **TLS server authentication:** lets a client verify the endpoint's certificate/hostname and encrypts the connection to the TLS terminator.
- **mTLS service authentication:** lets the coordination proxy authenticate a client certificate under its configured CA/identity policy. It is not ordinary browser participant authentication.
- **HMAC message authentication:** establishes that a message was produced by a holder of the applicable shared secret and detects modifications. It provides no confidentiality or non-repudiation; any verifier holding a symmetric key can forge messages for that key.
- **Participant authentication/eligibility:** requires a separately specified credential/token protocol. A server certificate, service mTLS certificate, or valid VDAF proof is not participant eligibility.
- **Prio3 proof verification:** establishes only the VDAF's encoded validity relation under the correct context and verification process. It does not prove a real-world measurement is truthful.
- **Sybil resistance:** is not provided by TLS, mTLS, HMAC, or proof verification.
- **Differential privacy:** is not provided by TLS, mTLS, HMAC, VDAF proof verification, or a minimum cohort threshold.
- **Organizational non-collusion:** is an operational/legal trust arrangement, not a property of ports, process separation, or cryptographic transport alone.

## 5. mTLS identity and lifecycle risks

The proposed flow has Leader initiate coordination to Helper. Helper's private coordination proxy presents a server certificate; Leader validates chain, hostname, validity, EKU, and allowed identity. Leader presents a client certificate; Helper validates it against a narrowly scoped coordination trust store and authorizes the exact Leader identity. Public participant ingress uses server TLS and does not assume client certificates.

Certificate rotation must install and validate new credentials before switching, with bounded overlap. Expired, revoked, wrong-purpose, wrong-name, unknown-CA, and not-yet-valid identities fail closed. Revocation behavior must be operationally defined; silently soft-failing revocation checks is not acceptable. Certificate/key distribution, storage, renewal, revocation, reload, and audit are all unimplemented.

## 6. Proxy parsing, header, and logging risks

A proxy can introduce request-smuggling or identity-confusion risks if its HTTP parser and backend parser disagree. It must reject duplicate/conflicting Content-Length and Transfer-Encoding combinations, reject unsupported chunking, send exactly one framing form, enforce a strict body limit, and avoid byte transformations for signed messages. The current Rust parser has limitations documented in `phase-6f2-preimplementation-audit.md`; this contract does not make the backend parser safe by assertion.

At public ingress, client-supplied forwarded headers must be stripped and replaced by trusted proxy-generated values. `Host` must be validated and must not select arbitrary upstreams. The Rust application must not trust forwarded headers from direct clients. Authentication headers and HMAC material must be singular, allowlisted, and never injected by the proxy. The proxy must not log bodies, shares, credentials, HMACs, or private-key data.

**Security gate:** Do not ingest security-sensitive binary or signed messages until the existing backend parser has received a separate security review. Current limitations include initial-read UTF-8 conversion, missing strict request-line/header size/count bounds, ambiguous request framing behavior, and no exact-byte preservation guarantee. Required tests before enabling such messages:

- fragmented request line, headers, delimiter, and body;
- multiple requests per connection and connection-close behavior;
- malformed request line and unsupported HTTP version;
- oversized request line/headers and excessive header count;
- duplicate headers, including case-variant duplicates;
- ambiguous/conflicting/overflowing Content-Length;
- Transfer-Encoding alone and with Content-Length;
- truncated bodies and extra bytes beyond declared length;
- arbitrary binary payload, including invalid UTF-8, preserved byte-for-byte;
- proxy/backend framing disagreement and request-smuggling cases.

Proxy-only tests cannot satisfy this gate. This documentation task does not modify the parser.

## 7. Failure and denial-of-service considerations

- Invalid/expired TLS or mTLS certificates fail handshake/authorization closed.
- Proxy outage produces connection failure; backend outage produces 502/503; backend timeout produces 504 or client timeout. None is cryptographic acceptance.
- Oversized or malformed requests are rejected before forwarding. Application parser limits and the proxy's limits must agree.
- Duplicate messages require durable identity/content comparison. Exact duplicate retries may be idempotent only after checking durable processing state; conflicting reuse is rejected.
- Proxy or backend restart must not clear application state or replay ledgers. A timeout is ambiguous and must not trigger rollback. `COMMIT_DECIDED` is immutable once durably recorded.
- Proxy retries of non-idempotent operations are dangerous unless the original message ID/sequence is preserved and backend deduplication is durable.
- A 64-connection backend limit and 3-second socket timeouts exist today, but do not alone bound TLS handshakes, proxy queues, connection churn, header parsing, CPU usage, or disk growth. Apply rate limits, handshake/connection caps, request-size limits, queue bounds, and storage quotas at deployment; test under load.
- Certificate issuance, HMAC rotation, and revocation endpoints can themselves be denial-of-service or compromise targets and require operational controls.

For every timeout/lost response, the application must distinguish three durable-state cases: (1) the request never reached the backend, so the exact same ID/sequence and bytes may be retried; (2) it reached the backend but was not processed, so recovery resumes or rejects from the recorded pending state without creating a new logical operation; (3) it was durably processed/committed but the response was lost, so an exact retry returns the prior outcome without verifying, accumulating, or committing twice. Persist message identity and content digest atomically with processing state; reject the same identity with different content. Proxies do not infer outcome or mint replacement identities. A timeout or proxy failure must never reverse durable `COMMIT_DECIDED`.

## 8. Secrets and key management

No secret values belong in this document or Git. The existing hardcoded development key must not be reused. Use two independent directional HMAC keys:

| Proposed key name | Sender | Verifier | Processes with key access |
|---|---|---|---|
| `BLINDMARK_COORD_LEADER_TO_HELPER_HMAC_KEY` | Leader application | Helper application | Leader signer and Helper verifier only |
| `BLINDMARK_COORD_HELPER_TO_LEADER_HMAC_KEY` | Helper application | Leader application | Helper signer and Leader verifier only |

Provision each key out of band from an approved secrets manager/protected deployment secret mechanism, only to the sender and verifier named above. Proxies do not receive HMAC keys by default. A symmetric verifier can also forge messages under its key; compromise of either holder compromises authenticity in that direction, and HMAC is not non-repudiation. The canonical envelope includes a bounded key-version identifier covered by the MAC. Accept only configured versions. Rotate by provisioning and validating the new version before switching, retaining the old version only for a bounded retry/expiry window; revoke compromised versions immediately. On restart, fail closed when the active configured key/version is missing; never generate a replacement. File/process permissions, key custody, rotation, revocation, and restart behavior remain unimplemented.

Exact canonical binary encoding (field order, lengths, byte order, domain separation, version semantics, and bounds) is a prerequisite for implementation and must receive protocol review. Do not invent or implement an unreviewed cryptographic construction.

TLS private keys must be readable only by the corresponding proxy identity and approved key-management process.

The proposed DB/proxy environment names are not current runtime settings. Their types, precedence, failure-on-missing behavior, secret source, and rotation/reload semantics are open implementation decisions.

## 9. Remaining attack surfaces and open review items

- Proxy host compromise exposes plaintext for its routed share; common host compromise may expose both shares in Profile B.
- Traffic analysis and timing correlation remain possible.
- Participant authentication, token/nullifier correctness, cross-aggregator credential consistency, and Sybil resistance are not provided by this contract.
- HMAC replay storage, canonical encoding limits, sequence concurrency, clock skew, tombstone retention, and key-rotation interaction need a protocol-level review before implementation.
- TLS does not fix application parser ambiguities or guarantee backend response semantics.
- The Prio state/restart strategy, durable verified-state transitions, immutable commit coordination, and exact report-set Collector gate remain unimplemented and require independent testing/review.
- Separate-host deployment, organizational independence, incident handling, backups, and access governance have not been verified.
- No differential privacy or release-policy guarantees are established.

## 10. Decision

This document set defines a proposed proxy boundary for review. It does not establish operational readiness. Human review must approve the contract and resolve the open decisions before Phase 6F.2 Checkpoint B. No production-readiness claim is made.
````

## 9. Final audit validation

Final `git diff --check`:

```text
warning: in the working copy of 'experiments/phase_6e/src/storage.rs', LF will be replaced by CRLF the next time Git touches it
warning: in the working copy of 'experiments/phase_6e/tests/storage_tests.rs', LF will be replaced by CRLF the next time Git touches it
exit code: 0
```

Final `git status --short`:

```text
 M experiments/phase_6e/src/storage.rs
 M experiments/phase_6e/tests/storage_tests.rs
?? docs/phase-6f1-storage-implementation-report.md
?? docs/phase-6f2-deployment-profiles.md
?? docs/phase-6f2-preimplementation-audit.md
?? docs/phase-6f2-proxy-architecture.md
?? docs/phase-6f2-proxy-contract.md
?? docs/phase-6f2-proxy-security-review.md
?? docs/phase-6f2-second-review-evidence.md
?? "ntent .gitignore"
?? rust_environment_test/
exit code: 0
```

The Phase 6F.1 source/test modifications, report, Phase 6F.2A documents/audit, and protected untracked items were already present in the initial status. The only new path created by this audit is `docs/phase-6f2-second-review-evidence.md`. No paths are staged. No commit or push was performed.

Final `git diff --stat`:

```text
 experiments/phase_6e/src/storage.rs         | 236 +++++++++-----
 experiments/phase_6e/tests/storage_tests.rs | 460 ++++++++++++++++++++++++----
 2 files changed, 544 insertions(+), 152 deletions(-)
exit code: 0
```

Final `git diff --name-only`:

```text
experiments/phase_6e/src/storage.rs
experiments/phase_6e/tests/storage_tests.rs
exit code: 0
```

Final `git diff --cached --stat` and `git diff --cached --name-only` each produced empty output and exit code 0.

### Preservation checks

- The pre-report SHA-256 fingerprints for the four revised proxy documents in Section 2 were rechecked; all four matched exactly.
- The four full document copies embedded in Section 8 were extracted and compared to their source files after normalizing CRLF/LF; all four matched exactly.
- The initial and final Git status show no Cargo file, preimplementation audit, or other source path changed during this audit. No edit operation targeted the Phase 6F.1 files, the audit, source, Cargo files, protected untracked items, or the four proxy documents.
- Current SHA-256 values for the preimplementation audit and inspected source/Phase 6F.1 files were observed during final verification, but no initial digest table for those files is included here. Their preservation is established by the read-only operation history and unchanged status/path listing, not by an independently recorded before/after digest comparison.
- No tests were run: this audit is read-only except for creating this evidence document. No network, proxy, certificate, or dependency operation was performed.

All six Task 1 repository-state commands (branch, status, tracked diff summary/name list, and cached diff summary/name list) completed with exit code 0. All four Task 7 final-validation commands also completed with exit code 0. Successful Git command exits do not imply that the worktree is clean; the initial and final status both show pre-existing changes and untracked files.
