# Phase 6F.2 Security Policy Decisions

**Status:** A–H are APPROVED DESIGN DECISIONS as of 2026-11-14. Approval does not mean implemented, tested, independently reviewed, deployed, or authorized for route enablement. It does not approve Checkpoint B.

**Scope:** Message identity and replay, freshness, journal lifecycle, key custody, storage integrity, transport trust, and parser gating. This document complements, and does not silently replace, the [proxy contract](phase-6f2-proxy-contract.md), [proxy architecture](phase-6f2-proxy-architecture.md), [deployment profiles](phase-6f2-deployment-profiles.md), or [journal audit](phase-6f2-journal-security-audit.md).

## Status Vocabulary

- **Implemented and tested:** behavior present in the current isolated Phase 6E code and covered by tests. This is not a claim of deployment readiness.
- **Approved design decision:** a human-approved requirement. Runtime implementation and verification may still be outstanding.
- **Blocked on external support:** requires verified OS, infrastructure, operational, or security-service capabilities beyond this codebase.
- **Required before live integration:** a hard gate. Coordination routes remain disabled until satisfied.

## Current Baseline

### Implemented and tested

- The canonical protocol body and HMAC-SHA256 frame format are implemented with byte-level tests. The caller supplies the key and expected key version; the code does not define provisioning, direction association, rotation, or revocation policy.
- The HTTP parser has bounded headers and header count, strict request-line validation, binary body preservation, duplicate-header and transfer-encoding rejection, a required Content-Length for POST, truncation checks, and rejection of trailing bytes already read with the body. The parser does not wait for EOF to detect bytes that arrive after the declared body. Existing tests are not an independent security approval.
- The separate file-backed journal verifies an incoming frame before receipt, uses WAL and `synchronous = FULL`, stores a SHA-256 body digest, handles exact duplicates and conflicting authenticated reuse, and protects state/identity columns with SQLite triggers. Processing has no callback or raw transaction capability. Independent-connection regression tests cover concurrent exact receipt, conflicting receipt, and processing.
- The journal has no database uniqueness constraint for the approved identity tuple. Its current IMMEDIATE transaction serializes calls through this API; that is not equivalent to a database-enforced uniqueness invariant. Conflicting reuse is currently stored separately as a rejected row, but the approved database-level uniqueness requirement still needs a compatible schema implementation.
- The journal remains disconnected from HTTP routes. Phase 6F.1 report-storage behavior remains separate and unchanged. The reported baseline is 56 passing tests, including 10 journal tests; the exact requested offline test command is to be rerun for this decision-preparation task.

### Not implemented or not established

No authenticated coordination route, reverse Helper-to-Leader route, TLS/mTLS proxy, proxy identity assertion, directional-key loader, freshness enforcement, sequence enforcement, retention/cleanup policy, service-account/IPC enforcement, file ACL validation, anti-rollback recovery, or independent parser gate approval is established. SHA-256 and mirrored metadata checks detect ordinary inconsistencies, but do not authenticate a database whose body, digest, and mirrored metadata were coherently rewritten. HMAC recovery verification is not claimed.

## Cross-Document Conflicts and Residual Specification Work

The decisions below resolve policy questions A–H where explicitly stated. Historical documents are not rewritten by implication; they remain evidence of prior proposals and findings. Remaining conflicts are identified rather than silently harmonized:

1. **Identity scope:** approved scope is `(sender_id, recipient_id, task_id, message_id)`, with database-level uniqueness for accepted identities and separate conflict recording. Older documents that omit or differ on scope are historical proposals. The current schema does not yet satisfy the approved constraint. Sender identity must ultimately bind to authenticated credentials; HMAC alone does not provide that binding.
2. **State ownership:** the proxy contract's durable journal state list includes `VERIFIED` and `COMMIT_DECIDED`; the architecture separates per-message states from per-batch states. The current journal implements only `RECEIVED_PENDING`, `PROCESSED`, and `REJECTED`. Retention decision D does not merge message and batch state machines; batch decision implementation remains a gate.
3. **HMAC key custody:** directional keys, secure provisioning, OS-protected credential storage, configured rotation overlap, immediate revocation, no secret logging, and the non-repudiation limitation are approved. Historical documents differ on whether proxies or applications own/read HMAC key files. Exact process access and credential-store wiring remain implementation details; TLS private keys remain distinct proxy credentials.
4. **Local transport:** OS-authenticated IPC and OS access controls are approved; localhost is not authentication. Earlier loopback-TCP alternatives are superseded and do not authorize a fallback.
5. **SQLite durability:** `synchronous = FULL` is approved. Historical `NORMAL` text is not the selected policy and must not guide journal implementation.
6. **Freshness:** maximum message lifetime 31.75 seconds and permitted clock skew 2 seconds are approved. Authentication precedes freshness checks for new messages; exact authenticated duplicates remain recognizable; expired/excessively future-dated messages and untrusted/rolled-back clocks fail closed for new messages. Exact implementation boundary tests remain outstanding.
7. **Reverse direction:** Helper-to-Leader communication is approved disabled initially. Historical language describing required reverse reconciliation is not authorization; any later enablement requires a separate decision and review.
8. **HTTP gate:** independent parser hardening/review and route-disabled-until-all-gates-pass are approved. Current parser improvements do not equal independent approval; late-arriving surplus-byte behavior and comprehensive security tests remain gates.
9. **Wire size and state details:** A–H do not approve a new frame/payload limit or the full batch state machine. Preserve current protocol limits as implementation facts, not newly approved policy; reconcile any limit or state-model changes before implementation without silently altering the approved decisions.

## Security Invariants

These are constraints on every option and any future implementation:

- Verify HMAC before treating a receipt as trusted or persisting an accepted message.
- Exact authenticated duplicates are idempotent and return the prior durable state/outcome; they do not repeat processing.
- Conflicting authenticated reuse cannot overwrite the original message or advance its state.
- Terminal journal states remain immutable. A durable batch `COMMIT_DECIDED` cannot be reversed by timeout, disconnect, or restart.
- No processing callback receives or can reach a raw SQLite transaction/connection. The current callback-free journal API is retained.
- Pending recovery must be explicit and durable; pending messages are not silently discarded or treated as committed.
- No unauthenticated route may invoke coordination processing. Routes remain disabled until all applicable gates pass.
- Authentication claims must be narrow: HMAC proves possession by a holder of the applicable symmetric key and integrity of covered bytes; it does not alone prove a unique sender, proxy traversal, participant identity, non-repudiation, or confidentiality.

## A. Message Identity Scope and Database Uniqueness

**Existing proposal:** The architecture proposes uniqueness per `(sender_id, recipient_id, task_id)`, with `message_id` inside that scope. The contract also describes same-ID/same-digest idempotency and same-ID/different-digest conflict, but does not consistently name the full tuple. The journal currently searches `(sender_id, recipient_id, task_id, message_id)` and allows multiple rows so it can retain rejected conflicting attempts.

**Security and operational consequences:** A scope that is too broad can collide unrelated senders/tasks; one that is too narrow can allow a replay under a different route, task, or direction. A non-unique index relies on all writers using the serialized journal API and does not protect against a future insertion path or direct database writer. A unique constraint on every row in the current table would also prevent recording a conflicting attempt as a separate rejected row.

**Approved design decision:** The accepted-message identity is `(sender_id, recipient_id, task_id, message_id)`. Exact duplicates are idempotent. Conflicting reuse is rejected and recorded separately without overwriting the original. Accepted identities require a database-level unique constraint. Sender identity must ultimately bind to authenticated credentials; a valid HMAC alone does not establish a unique sender.

**Alternatives and trade-offs:** A globally unique `message_id` is simpler but creates cross-task coordination and collision coupling. Adding explicit direction or protocol ID to the tuple may permit reuse that the proposed scope would treat as conflict; it must be justified by protocol semantics. A partial unique index on non-conflict rows may work, but requires careful state/schema migration and must not allow rejected attempts to replace the accepted row. Keeping only an application-level IMMEDIATE transaction is least disruptive but does not give database-level enforcement.

**Implementation gate:** Design a database representation that enforces uniqueness for accepted identities while recording conflicts separately. The current schema has not yet been changed and does not satisfy this approved requirement.

**Implementation status:** The tuple behavior is partly implemented and tested, but database-level uniqueness, separate conflict persistence compatible with that constraint, and sender-to-credential binding remain unimplemented and unverified. Live integration must wait.

## B. Sequence Scope, Ordering, Replay, and Restart

**Existing proposal:** The architecture proposes a sequence stream per sender/recipient/task/direction, strictly increasing for new messages, exact duplicates exempt, and gaps allowed. The codec stores a `uint64` but does not enforce sequence policy. The contract leaves validity dependent on configured protocol state.

**Security and operational consequences:** Without durable sequence enforcement, a different message ID can carry a stale or reused sequence. Strict ordering can reject delayed legitimate messages; allowing gaps avoids waiting for every number but means a lower sequence arriving after a higher one is stale. In-memory high-water marks would reset at restart and reopen replay opportunities.

**Approved design decision:** Maintain independent strictly increasing sequence streams per `(sender_id, recipient_id, task_id)`. Gaps are allowed; stale or replayed sequence numbers are rejected. Sequence state survives process/database restart. Reset is permitted only with a new authenticated sender epoch/generation. Exact authenticated duplicates remain recognizable and idempotent under decision A; they do not cause a second processing action.

**Alternatives and trade-offs:** Contiguous numbering detects gaps but makes message loss/reordering block progress. A sliding replay window permits bounded reordering but requires a durable bitmap/window and more complex cleanup/recovery. No sequence enforcement is simpler but leaves message-ID deduplication as the only replay control.

**Implementation gate:** Persist and atomically advance stream state with accepted message state; bind reset to an authenticated new sender epoch/generation. Define and test epoch storage/validation and the behavior at sequence exhaustion without resetting an existing stream implicitly.

**Implementation status:** The codec stores a `uint64` sequence but enforces no ordering, replay rejection, durable high-water mark, or authenticated epoch reset. These remain unimplemented and unverified; live acceptance cannot rely on sequence security yet.

## C. Message Freshness, Expiry, and Clock Skew

**Existing proposal:** The architecture proposes a maximum freshness window of 31.75 seconds, maximum clock skew of 2 seconds, and a synchronized UTC clock with known skew. The codec only checks `expires_at_ms > issued_at_ms`; it does not enforce age, expiry, or skew. The retry schedule is also 31.75 seconds, but its relationship to message lifetime is not normatively defined.

**Security and operational consequences:** A long window increases replay exposure; a short window can reject retries during outages or clock drift. Unsynchronized or rolled-back clocks can make expired messages appear fresh. Ambiguous treatment of skew at the expiry boundary can make peers disagree. Exact duplicates must remain idempotent even after expiry, but must never trigger fresh processing.

**Approved design decision:** Maximum message lifetime is 31.75 seconds and permitted clock skew is 2 seconds. Authenticate before applying freshness checks to a new message. Reject expired and excessively future-dated new messages. Exact authenticated duplicates remain recognizable even when outside the new-message freshness window and must not be reprocessed. A clock that is untrusted or detected as rolled back fails closed for new messages.

**Alternatives and trade-offs:** A smaller freshness window reduces replay opportunity but needs tighter operational clocks and retry availability. A wider window improves outage tolerance but increases the amount of time an unseen valid message can be accepted. A monotonic local clock helps elapsed-time measurement but cannot by itself compare a remote Unix timestamp across restart; a trusted UTC source and explicit rollback detection are still needed.

**Implementation gate:** Implement a trusted clock-health/rollback check and document exact boundary arithmetic consistent with the approved lifetime and skew. Test valid, expired, excessively future-dated, boundary, exact-duplicate, unavailable-clock, and rolled-back-clock cases. Do not change the approved numeric values without a new decision.

**Implementation status:** Only timestamp ordering (`expires_at_ms > issued_at_ms`) is enforced by the codec. Runtime freshness, clock skew, trusted-clock failure, and rollback handling are not implemented or verified.

## D. Journal Retention, Cleanup, and Recovery

**Existing proposal:** The architecture proposes retention for `freshness window + maximum retry horizon + batch lifetime`. It requires durable-before-acknowledgment, exact retry handling, pending recovery, and immutable commit decisions. The current journal enumerates pending rows but has no retention, pruning, backup, or recovery-authenticity policy. The state models differ between the contract and architecture as described above.

**Security and operational consequences:** Deleting an identity too early permits replay after cleanup. Deleting pending or in-doubt work can lose a message or leave peers divergent. Keeping all records indefinitely increases disk use and sensitive-data exposure. Restoring an old database snapshot can erase accepted identities or roll back sequence state. `FULL` synchronous mode improves SQLite commit durability but does not establish power-loss behavior, backup correctness, anti-rollback, or cryptographic authenticity.

**Approved design decision:** Retain pending records until they are processed or explicitly rejected. Retain processed and rejected records for 30 days. Retain compact identity/digest tombstones for 90 days. Use SQLite WAL with `synchronous = FULL`. Run cleanup only after a backup has been verified. A corrupted journal must fail closed and must never be silently reset. These retention rules do not merge message state with batch decision state.

**Alternatives and trade-offs:** Indefinite retention is easiest for replay safety but costly and privacy-sensitive. Time-based deletion is operationally bounded but requires exact horizons, durable tombstones, and coordinated handling of backups. Re-authenticating stored frames on recovery could detect tampering but would require retaining authentication evidence and keys; that key lifecycle is not defined and is not presumed here.

**Implementation gate:** Define retention timestamps and deletion ordering consistent with the approved 30-day and 90-day periods; make backup verification a prerequisite to cleanup; preserve pending data; and provide a fail-closed operator-visible path for corruption and recovery. Implement anti-rollback/replacement detection and verified restore without clearing a corrupted journal automatically.

**Implementation status:** The journal currently enumerates pending records and uses `FULL`, but has no retention cleanup, tombstones, verified-backup integration, anti-rollback check, or corruption recovery procedure. These are unimplemented and unverified.

## E. Directional HMAC Key Ownership and Lifecycle

**Existing proposal:** Two independent direction-specific HMAC-SHA256 keys are proposed. The final proxy contract says the sending application signs and the receiving application verifies; proxies own TLS certificate keys. It also notes every HMAC verifier can forge messages under its symmetric key. The architecture's older key table ambiguously names the egress proxy or a dedicated account as a possible HMAC secret-file owner. Current code receives a caller-selected key/version and has no key lifecycle.

**Security and operational consequences:** A single shared or bidirectional key erases direction separation. Giving a proxy an application HMAC key broadens the set of principals able to forge messages. A receiver holding the symmetric key can also forge sender messages, so HMAC does not provide non-repudiation or prove which holder created a frame. Uncoordinated rotation can reject valid retries; keeping compromised keys active can permit forgery.

**Approved design decision:** Use separate directional HMAC keys, securely provisioned and held in OS-protected credential storage. Use a configured rotation overlap and immediate revocation. Never log secrets. HMAC does not provide non-repudiation: any holder of the relevant symmetric key can produce a valid MAC. Sender identity must ultimately bind to authenticated credentials under decision A. No rotation-overlap duration or specific credential-store product is approved here.

**Alternatives and trade-offs:** Keeping HMAC is compact and matches the implemented primitive but grants both holders forgery capability. A key-management service/HSM can reduce extraction risk but does not remove symmetric verifier forgery and requires platform support. Asymmetric signatures reduce the verifier's ability to forge but change protocol, key lifecycle, and implementation scope.

**Implementation gate:** Specify and enforce OS credential-store access for the services that need each direction's key; define operational rotation/overlap configuration and immediate revocation behavior; test missing credentials, rotation, revocation, restart, and secret-redaction paths. Keep HMAC claims limited to key-holder authentication and message integrity.

**Implementation status:** Frame signing/verification with caller-supplied test keys is implemented. Secure provisioning, OS credential storage, directional key selection, configured rotation, revocation, sender credential binding, and operational verification are not implemented.

## F. Database Permissions, Service Isolation, and Recovery Integrity

**Existing proposal:** Separate Leader and Helper databases and dedicated service accounts are proposed. The deployment documents describe Unix ownership/permissions and Windows ACLs for IPC, but the current journal does not enforce or verify file ACLs. The database stores body plus SHA-256 digest and mirrored metadata. The journal uses WAL and `FULL`; database administrators can rewrite rows or files.

**Security and operational consequences:** SQLite triggers and digest checks are not a trust boundary against a writer who can alter the database, schema, body, digest, and metadata coherently, replace backups, or roll back a file. WAL/SHM files and backups need the same protection as the main file. A shared service account or administrator can cross process boundaries. Permissions protect ordinary access but do not prove data authenticity or anti-rollback.

**Approved design decision:** Use a dedicated non-administrator service account. Restrict database, WAL, and SHM permissions. Keep backups controlled and encrypted, verify backups, and detect rollback/replacement. State the administrator/OS compromise boundary explicitly: privileged OS or administrator compromise can defeat local file controls. SHA-256 digest and mirrored metadata checks detect ordinary inconsistencies only; they do not authenticate a coherently rewritten database. No HMAC recovery-verification claim is made.

**Alternatives and trade-offs:** OS ACLs and isolated accounts reduce access by ordinary processes but trust administrators and do not detect privileged coherent rewriting. Encryption at rest protects media exposure but not a compromised running service or authorized writer. Per-record MACs could detect modification only if key storage, rotation, backup, recovery, and anti-rollback are defined. A remote append-only audit service could improve tamper evidence but adds infrastructure and availability dependencies.

**Implementation gate:** Provision and verify the non-admin service identity and effective permissions for the database directory, database, WAL/SHM, and backup paths. Implement controlled encrypted backups, verification before cleanup/restore, rollback/replacement detection, and documented privileged-administrator boundaries. Do not claim cryptographic authenticity from the current digest/metadata checks.

**Implementation status:** The test journal uses a file-backed SQLite database, WAL, and `FULL`, but does not create service accounts, enforce/check OS permissions, manage encrypted backups, or detect rollback/replacement. Those controls require platform implementation and verification.

## G. Authenticated IPC and TLS/mTLS Trust Boundaries

**Existing proposal:** Separate participant HTTPS ingress and Leader-egress-proxy-to-Helper-coordination-proxy mTLS are proposed. The helper proxy generates a trusted peer assertion after mTLS. Corrected documents require protected local IPC to the proxy/backend and say generic loopback is not process authentication. Older architecture language allows TCP under enforceable OS/network policy. The Rust services currently use standard TCP; no TLS, mTLS, IPC identity, or proxy assertion is implemented.

**Security and operational consequences:** TLS authenticates the peer at the TLS endpoint only. After termination, the backend needs a protected proxy-only path and trusted assertion; HMAC alone does not prove proxy traversal. Generic loopback and a caller-supplied header are insufficient. Host administrators and compromised proxy processes remain trusted. Missing certificate trust, hostname/EKU, rotation, or revocation rules can admit the wrong peer or break availability.

**Approved design decision:** Use mTLS for network communication and OS-authenticated local IPC with OS access controls. Localhost is not authentication. Define and enforce certificate lifecycle controls. Reject unauthenticated connections. Keep reverse Helper-to-Leader communication disabled initially. Historical proposals for generic loopback TCP do not override this decision.

**Alternatives and trade-offs:** Verified network namespaces/firewall policy can isolate TCP but are platform-specific and must prove process identity, not merely address binding. End-to-end mTLS to the backend is another architecture, but is not current behavior and requires separate design. Shared local accounts are simpler but weaken isolation.

**Implementation gate:** Select platform-supported IPC mechanisms and service identities; implement/test mTLS peer validation, certificate issuance/renewal/rotation/revocation, IPC ACLs, direct-backend denial, proxy identity handling, and unauthenticated-connection rejection on each deployment profile. The approved policy does not itself configure or verify these controls.

**Implementation status:** The current Rust services use TCP and have no mTLS proxy, authenticated IPC, certificate lifecycle, or proxy assertion. Required OS/infrastructure support and validation remain outstanding.

## H. HTTP Parser Gate and Conditions for Route Enablement

**Existing proposal:** Signed/binary coordination remains disabled until independent review of backend parsing and proxy/backend framing. The older parser audit is stale in several details; current source includes byte-safe body handling, header/request-line limits and validation, exact declared-body reads, and rejection of prefetched surplus bytes. The current code still does not wait for EOF to detect bytes arriving after the declared body. Five parser regression tests exist; they do not establish an independent security gate.

**Security and operational consequences:** A proxy cannot correct backend framing ambiguity. Parser differentials can cause components to authenticate or process different byte sequences. An otherwise valid HMAC must not bypass HTTP route authentication, proxy provenance, policy checks, or durable replay checks. Any unauthenticated alternate path to the same processing function defeats the boundary.

**Approved design decision:** Use a hardened, independently reviewed parser with strict request, header, body, and framing limits. Reject malformed or truncated requests and unexpected trailing bytes; preserve binary bodies safely. Verify HMAC before coordination processing. Add comprehensive malformed, fragmented, and oversized request tests. Coordination routes remain disabled until all security gates pass. This does not claim the current parser has passed independent review.

**Alternatives and trade-offs:** Raw binary avoids encoding expansion but requires byte-exact parser and proxy validation. ASCII encoding can simplify transport inspection but inflates size and introduces strict decode/canonicalization risks. Waiting for connection close to prove no surplus bytes may conflict with keep-alive; alternatively, enforce one request per connection and test the complete transport behavior.

**Implementation gate:** Complete the independent parser review and the comprehensive parser/proxy/backend tests. Resolve framing behavior for bytes arriving after the declared body, demonstrate HMAC-before-processing and authentication on every route path, and verify all other approved security controls. Only then may a separate route-enablement review be considered; this decision does not enable routes.

**Implementation status:** Several parser hardening features and five parser tests exist. Late-arriving trailing data is not detected by waiting for EOF, and independent review, comprehensive required tests, proxy/backend differential validation, and route authorization are outstanding. No coordination route is enabled.

## Approved Decisions and Status

Human approval has been recorded for the following design decisions. This is policy approval only; it is not evidence that the controls have been implemented, tested, independently reviewed, or deployed.

- A. Identity tuple, idempotent exact duplicate, separate conflict rejection, accepted-identity database uniqueness, and eventual sender-to-credential binding: **APPROVED DESIGN DECISION**.
- B. Strictly increasing sequence streams per `(sender_id, recipient_id, task_id)`, gaps allowed, stale/replay rejection, restart persistence, and authenticated epoch/generation reset: **APPROVED DESIGN DECISION**.
- C. 31.75-second maximum lifetime, 2-second permitted skew, authenticate-before-freshness for new messages, expired/future rejection, recognizable exact authenticated duplicates, and fail-closed untrusted/rolled-back clock behavior: **APPROVED DESIGN DECISION**.
- D. Pending retention until processed or explicitly rejected; 30-day processed/rejected retention; 90-day compact identity/digest tombstones; `synchronous = FULL`; cleanup only after verified backup; corrupted journal never silently reset: **APPROVED DESIGN DECISION**.
- E. Separate directional HMAC keys, secure provisioning, OS-protected credential storage, configured rotation overlap, immediate revocation, no secret logging, and explicit lack of non-repudiation: **APPROVED DESIGN DECISION**.
- F. Dedicated non-admin service account, restricted database/WAL/SHM permissions, controlled encrypted and verified backups, rollback/replacement detection, and explicit administrator/OS compromise boundary: **APPROVED DESIGN DECISION**.
- G. mTLS for network communication, OS-authenticated local IPC and access controls, localhost is not authentication, defined certificate lifecycle, unauthenticated-connection rejection, and reverse Helper-to-Leader disabled initially: **APPROVED DESIGN DECISION**.
- H. Hardened independently reviewed parser, strict request/header/body/framing limits, malformed/truncated/trailing-data rejection, binary-safe bodies, HMAC-before-processing, comprehensive malformed/fragmented/oversized tests, and routes disabled until all security gates pass: **APPROVED DESIGN DECISION**.

### Outstanding Gates

- **Implementation:** database uniqueness compatible with separate conflict recording; durable sequence and authenticated epoch state; freshness/clock enforcement; retention and tombstone cleanup; verified backup and corruption recovery; credential-store/key lifecycle wiring; service accounts and file ACLs; mTLS, authenticated IPC, and certificate operations; parser and route integration.
- **Testing:** adversarial/concurrent uniqueness and replay tests; sequence restart/epoch tests; freshness, skew, and clock rollback tests; retention/backup/restore/corruption tests; key rotation/revocation tests; OS permission and IPC identity tests; certificate rejection tests; and parser fragmentation, malformed, oversized, truncation, and late-trailing-data tests.
- **Independent review:** parser and proxy/backend framing review; HMAC/key-custody and sender-credential binding review; journal backup/rollback integrity review; and review of the final integration evidence.
- **Deployment:** provision non-admin identities and OS credential storage; enforce database/WAL/SHM and IPC permissions; configure mTLS trust and certificate lifecycle; establish controlled encrypted backup and verification operations; verify each supported OS/profile and the administrator/OS compromise boundary.

The current digest/metadata checks still do not authenticate a coherently rewritten database, and no HMAC recovery-verification claim is made. Historical audit findings remain historically accurate: JSA-01 is resolved; identity uniqueness and other A–H controls are approved as design but remain unimplemented/unverified. Phase 6F.2 remains PARTIAL and NOT COMPLETE. Checkpoint B is NOT APPROVED. No route is enabled, reverse Helper-to-Leader communication remains disabled, and Phase 6F.1 storage behavior is unchanged.
