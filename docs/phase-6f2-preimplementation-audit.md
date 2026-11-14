# Phase 6F.2 Preimplementation Audit

## Scope and baseline

This audit is based on read-only inspection of the current worktree on branch `feature/phase-6b-rust-prio3-prototype`. The worktree was already dirty before Phase 6F.2 work began: the Phase 6F.1 storage source and tests were modified, the Phase 6F.1 report was untracked, and the protected untracked `"ntent .gitignore"` and `rust_environment_test/` items were present. These are preserved.

No Phase 6F.2 implementation changes have been made yet.

## Current service architecture

`experiments/phase_6e` is a Rust crate using `std::net` HTTP listeners, one Leader process on port 8001 and one Helper process on port 8002. `leader_service.rs` and `helper_service.rs` each accept connections and dispatch bounded worker threads. `main.rs` selects one role, but currently creates a fresh random VDAF verification key for each process start. Consequently the service binary does not currently provide stable shared verification configuration across restarts or between roles.

The service state in `state.rs` wraps the Phase 6C in-memory `LeaderAggregator` and `HelperAggregator`. These have process-local pending verification maps, aggregate shares, counters, and duplicate-report sets. Phase 6E's SQLite `Database` exists in `storage.rs`, but the services do not instantiate it or use it. The services also have no peer HTTP client or background recovery/reconciliation loop. The Collector service accepts both encoded aggregate shares in one method and delegates to Phase 6C; it does not query independent durable services or compare exact report identities.

The existing Phase 6F.1 database preserves WAL, `synchronous = 2`, and foreign keys. Its API covers local `commit_report` and prototype XOR reconstruction only; the schema uses integer task/report/batch identifiers and has no durable inbound-message journal, verification metadata/state, peer decision acceptance, exact report-set API, or per-service configured database path.

## HTTP parser capabilities and limitations

`http.rs` is a dependency-free HTTP/1.x-like parser with a 64 KiB caller-supplied payload limit, a three-second socket read/write timeout, duplicate `Content-Length` rejection, rejection of `Transfer-Encoding` (including chunked), rejection of simultaneous Transfer-Encoding and Content-Length, malformed-header checks, incomplete-body rejection, and no more than 64 concurrent workers in the service runners.

Limitations relevant to authenticated binary/canonical messages:

- The first read decodes the entire received chunk as UTF-8 before separating headers and body. Arbitrary binary message bodies are therefore not supported safely as-is.
- It does not strictly validate the request-line version/field count, explicitly bound header bytes/count, or reject bytes beyond the declared body length in the same initial read.
- Requests without `Content-Length` are accepted using the first read's body length, which is not an adequate framing rule for authenticated internal messages.
- Several parser error strings interpolate untrusted values without JSON escaping.
- The parser uses one `TcpStream` timeout value for both directions and does not provide TLS.

The existing HTTP tests exercise several parser protections and bounded concurrency. Before introducing authenticated request processing, framing must be tightened and byte-safe, or the coordination envelope must use a strictly ASCII encoding with strict length and framing validation. No current route processes report bodies: ingestion intentionally returns 501.

## Existing authentication capabilities

`routes.rs` currently checks `X-Service-Key` or `Authorization` against a shared static development key defined in `config.rs`. The key is hardcoded, has a default, is not message-bound, has no timestamp/sequence or replay protection, is compared with ordinary string equality, and is shared by Leader and Helper. It is not suitable for service-to-service identity. `main.rs` does not load database paths, service keys, or the verification key from environment configuration.

The Phase 6F design calls for an experimental HMAC-SHA256 envelope with domain separation and constant-time verification. The local Cargo dependency graph contains HMAC 0.13.0, SHA-2 0.11.1, and `subtle` 2.6.1 transitively through Prio. Their source and compiled artifacts are cached locally. They are not direct dependencies of Phase 6E; adding direct entries at the already-resolved versions may be possible without changing the Prio or rusqlite versions. No `serde_json` source or compiled artifact was found in the local cache, so adding a JSON wire protocol would need a dependency change and is not assumed. Cargo's offline metadata command attempted to fetch `libc 0.2.191`; this is an environment/cache caveat to recheck if new dependency resolution is needed.

HMAC authenticates integrity and possession of a configured shared key; it does not encrypt data or create organizational independence. There is no TLS implementation in this standard-library server. The current service binds to loopback by default, but a deployable inter-host transport cannot safely expose participant input shares or coordination traffic without a separately secured transport boundary. This phase must not claim confidentiality or production security from HMAC alone.

## Existing Prio verification integration

Phase 6C uses the required `prio = "0.18.1-alpha.3"` API and calls `verify_init`, `verifier_shares_to_message`, `verify_next`, and `Aggregatable::accumulate`. Both logical aggregators call `verify_next` independently before their in-memory accumulation. The protocol documentation correctly distinguishes verifier-share combination from proof validation.

The Phase 6C code uses an empty context (`&[]`) and has no durable context/config identity. Its Prio state is held only in memory. Inspection of the locally cached Prio 0.18.1 source shows `Prio3VerifyState` implements `Encode`, but its fields are private and the inspected source provides no matching decode/restore implementation. Do not invent a state decoder. A candidate recovery design is to persist each aggregator's own original input share plus nonce, public share, task/config context, and verification-key identity, then rerun that aggregator's own `verify_init` after restart before processing the authenticated next-round message. This candidate must be verified by tests and must keep the original private input share only in its own database. If deterministic reinitialization cannot be established, the report must remain noncommitted/re-ingestable and never be counted twice.

## Missing functionality

- Explicit independent Leader/Helper database paths and database construction during service startup.
- Stable configured verification key; current per-process random key changes on restart.
- HMAC-authenticated typed internal messages with canonical encoding, independent direction keys, timestamps/sequences, and durable replay rejection.
- Strict byte-safe HTTP framing suitable for the authenticated envelope.
- Durable inbound message/message-ID state and transactional state transitions.
- Durable per-service verification input/context needed for safe reinitialization.
- Integration of actual Prio rounds with persistence and peer communication.
- Durable immutable Leader commit decision propagation and Helper decision acceptance/retry/recovery.
- Exact committed report-set comparison before Collector readiness, without exposing report IDs in the public result.
- Service configuration for database paths and authentication material without insecure defaults.
- Focused tests for the listed auth, durable-message, verification, commit/recovery, report-set, and isolation requirements.

## Files proposed for modification

Proposed implementation scope, subject to checkpoint findings:

- `experiments/phase_6e/Cargo.toml` and `Cargo.lock` only for direct use of already-resolved crypto crates if confirmed offline; preserve `prio` and `rusqlite` versions.
- `experiments/phase_6e/src/config.rs`, `main.rs`, and `error.rs` for explicit paths, stable keys/configuration, and typed failures.
- `experiments/phase_6e/src/http.rs` for strict byte-safe request framing, with parser regression tests.
- `experiments/phase_6e/src/routes.rs`, `leader_service.rs`, `helper_service.rs`, `state.rs`, and `storage.rs` for authenticated coordination, isolated durable state, integration, and recovery.
- New narrowly scoped Phase 6E modules may be needed for typed message encoding/authentication and context construction.
- Existing focused tests under `experiments/phase_6e/tests/`, plus new tests only where no existing home is suitable.
- The seven Phase 6F.2 documents requested by the implementation prompt.

No Phase 6B, Phase 6C, Phase 6D, Phase 6F.1 source, FastAPI source, dependency versions, or protected untracked files are proposed for modification.

## Security risks and constraints

- HMAC does not provide confidentiality. The current std-only HTTP server has no TLS; independently operated network deployment remains blocked absent a secured transport boundary. Loopback-only local prototype tests do not establish remote deployment safety.
- The current `DEV_SERVICE_KEY` is a hardcoded credential and must not be used by a coordination implementation.
- A single shared HMAC key would not distinguish message direction/sender; separately configured directional keys and explicit sender/recipient checks are required.
- In-memory verification state, duplicate sets, and aggregates are lost on restart; stable replay protection must be durable before acknowledging processing.
- The verification key is currently randomly regenerated per process and is not persisted/configured.
- Phase 6D token/nullifier eligibility and participant authentication are not implemented in the Rust service. Matching metadata or a valid Prio proof does not establish participant identity, authorization, real-world input truthfulness, or Sybil resistance.
- The Collector currently receives two shares together through an in-process method; no independently authenticated aggregate readiness protocol exists.
- Same-host processes, shared administrators, or shared secrets do not create organizational non-collusion.
- No differential privacy, privacy budget, or release-policy enforcement is implemented in this crate.
- Retry and commit coordination can block during partitions; no distributed atomicity or guaranteed liveness can be claimed.
- Existing parser concerns must be addressed before it is trusted for security-sensitive internal messages.

## Offline dependency availability

- `prio` remains `0.18.1-alpha.3` in the Phase 6E manifest.
- `rusqlite` remains `0.29` with `bundled`.
- HMAC 0.13.0, SHA-2 0.11.1, and subtle 2.6.1 are resolved transitively through Prio and have local source/artifact cache entries.
- `serde_json` is not present in the inspected source/artifact cache; a new JSON dependency is not assumed.
- Offline `cargo metadata` encountered a missing `libc 0.2.191` download request. Existing recent Phase 6E, 6C, and Prio demo checks/tests had passed from the workspace before this audit, but newly resolved dependency builds must be verified rather than presumed available.

## Checkpoint A disposition

Checkpoint A is complete and this audit is the required deliverable. The implementation is blocked before Checkpoint B: the requested topology is between independent services, while the available server is plaintext `std::net` HTTP and the offline cache/lockfile has no TLS implementation. HMAC would authenticate message bytes but would not keep an aggregator's plaintext input share confidential from network observers. Proceeding with share-bearing network routes over this transport would weaken the stated privacy boundary. No dependencies were fetched or changed. Resume implementation only after an approved TLS transport is available (or a deployment-enforced authenticated/encrypted proxy boundary is specified and can be verified), then rerun the offline dependency and transport checks. The remaining planned checkpoints are not represented as implemented.

## Explicit implementation plan

1. Checkpoint A (this document): finish source/doc/dependency audit. Do not start implementation until this audit exists.
2. Checkpoint B: define typed domain-separated envelope and deterministic length-prefixed encoding; implement HMAC with constant-time verification; validate version, roles, IDs, size, expiry/sequence; add auth/replay tests.
3. Checkpoint C: add durable inbound-message identity and transition APIs using separate databases; test exact retries, conflicting IDs, invalid transitions, rollback, and reopen behavior.
4. Checkpoint D: configure independent DB paths and stable secrets; integrate only authenticated, durable internal routes while preserving parser protections and honest response semantics.
5. Checkpoint E: integrate exact Prio methods; ensure both sides independently verify before staging/accumulation; test tampered and valid reports. Persist only each side's own input share.
6. Checkpoint F: implement immutable durable Leader decision and idempotent Helper commit application/reconciliation; test restart and timeout ambiguity without aborting decided work.
7. Checkpoint G: compare exact committed report identity sets and task/batch/config metadata before Collector readiness; expose only minimal readiness metadata.
8. Checkpoint H: add security review, implementation report, requested protocol/state/recovery/architecture docs, run focused and existing offline suites, inspect diff/status, and report all unimplemented boundaries.

At every checkpoint, stop on a failed security-critical invariant or an unavailable dependency/transport requirement rather than weakening verification, persistence, authentication, or privacy boundaries.
