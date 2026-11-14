# Phase 6F.2 Proxy Architecture

Status: Correction round 3. Proposed for independent human review. Not approved. Does not authorize Checkpoint B or any implementation.

## 1. Scope and intent

This document defines the required trust boundary for participant ingress, internal coordination, local process isolation, and durable message-state handling. It is intentionally design-only and remains non-executable until an independent human review accepts the contract.

The design preserves four distinct control points:

1. Participant-to-ingress-proxy HTTPS.
2. Leader-egress-proxy-to-helper-coordination-proxy mTLS.
3. Helper-proxy-to-backend local protected IPC or another explicitly selected private local transport.
4. Helper-side HMAC verification and durable replay checks.

TLS and mTLS terminate at the relevant proxy. The backend does not receive direct TLS on its own local path unless a private local transport is explicitly chosen and enforced under a local-process policy.

## 2. Target request path

The required coordination path is:

Leader application -> Leader egress proxy -> Helper coordination proxy -> Helper Rust backend

The path is not generic loopback HTTP. It is a process-restricted local proxy path whose local transport enforces the exact service identity of the allowed peer.

## 3. Local transport identity requirements

Loopback TCP alone does not authenticate the connecting process and therefore cannot be treated as a security boundary. The following process-restricted local endpoints are mandatory for the specified proxy connections.

### 3.1 Leader service -> leader egress proxy

Supported local endpoint types:
- Linux: Unix domain stream socket with peer-credential verification (`SO_PEERCRED`) and a socket owner/group restricted to the dedicated leader service account.
- Windows: Named pipe with a DACL granting access only to the leader service SID and the service account that owns the leader process.

Required policy:
- The permitted peer identity is the service account assigned to the leader process, not the binary path or PID.
- SO_PEERCRED and named-pipe SID/DACL checks identify the account; any process running as that account passes the check.
- The proxy rejects any other local process before reading or forwarding the request body.
- The creator of the socket or pipe must be the owning service identity, with ownership or ACLs restricted to that service account and the required local administrative delegates.
- The client must confirm it reached the correct server endpoint by directory permissions on Linux or by explicit server-identity validation on Windows.
- The transport is not enforceable if the OS cannot present the peer identity, restrict access to the owner service account, or validate the server endpoint identity.

A dedicated, otherwise-unused OS account is required for each role: leader application, leader egress proxy, helper coordination proxy, and helper backend. A compromised process running as the same account is not stopped by this check.

Proposed mechanism: [DECISION REQUIRED: local endpoint mechanism for each role | PROPOSED: Unix domain sockets on Linux and named pipes with DACL/SID checks on Windows | REASON: they are the only local transports that can prove service account identity without relying on loopback.] 

If unavailable:
- The leader egress path is disabled. No fallback to unrestricted loopback TCP is permitted.

### 3.2 Helper coordination proxy -> helper backend

Supported local endpoint types:
- Linux: Unix domain stream socket with peer-credential verification and restricted ownership.
- Windows: Named pipe with DACL granting access only to the helper coordination proxy service SID.

Required policy:
- The backend accepts inbound requests only from the helper coordination proxy service account.
- The backend rejects any other local peer before request forwarding.
- A generic `127.0.0.1` listener is not considered process-restricted and cannot be used as proof of authorization.
- Remote clients are rejected on Windows named pipes and are not accepted by any local loopback-only path.
- No PID is used as identity. PID values are transient and not stable trust evidence.

The Rust backend currently uses `std::net` TCP only. Enabling these local transports requires future design work and possibly new dependencies. This is not authorized here, and Windows development machines are affected by the same requirement.

If unavailable:
- The helper backend route remains disabled.

## 4. Authentication and provenance separation

The following are intentionally distinct and must not be collapsed into one claim:

1. TLS peer authentication: the certificate or public-key identity presented on an encrypted transport.
2. Proxy-generated identity assertion: a trusted value produced by the helper coordination proxy only after successful mTLS validation.
3. HMAC message authentication: a symmetric integrity check over canonical bytes.
4. Local process isolation: the local endpoint policy that prevents arbitrary local processes from connecting.

A valid HMAC alone does not prove proxy traversal. A proxy-generated identity value alone does not prove identity if direct backend access remains possible. The backend must require all of the following:

- the request arrived via the authorized local socket or named pipe;
- the helper coordination proxy validated the leader egress proxy certificate;
- the proxy-generated assertion matches the HMAC sender identity;
- the canonical HMAC envelope is valid under the configured key version;
- the durable replay state accepts the message as unique and non-conflicting.

## 5. Required helper provenance flow

The helper backend must validate the complete path:

- The leader application signs a canonical coordination message using the directional leader-to-helper HMAC key.
- The leader egress proxy sends the message to the helper coordination proxy over authenticated mTLS.
- The helper coordination proxy validates the leader certificate and configured trust domain.
- The helper coordination proxy strips any client-supplied identity copies and sets exactly one trusted `X-Authenticated-Coordination-Peer` value derived from the validated peer identity.
- The helper coordination proxy forwards the message only over the authorized local IPC endpoint to the helper backend.
- The helper backend verifies the local IPC peer identity and confirms that the peer is the helper coordination proxy service account.
- The backend verifies that the proxy assertion and the HMAC sender identity match exactly.
- The backend verifies the canonical HMAC envelope, age, sequence, task/report/batch identity, and durable replay state.
- If any check fails, the request is rejected before durable processing.

This design is deliberately strict: direct access to the helper backend is blocked by a process-restricted local route, not by a generic loopback listener.

## 6. State model and scope

The protocol must keep two state sets separate and explicit, because the contract currently mixes message-level and batch-level semantics.

### 6.1 Per-message journal states

The per-message journal records message receipt and decision scope only. It is not a batch decision state. The allowed message-level states are:

- NOT_RECEIVED: absence of a durable record; not a stored state.
- RECEIVED_PENDING: durably received but not yet processed or verified.
- PROCESSED: durable response or processing result recorded.
- REJECTED: message-level rejection recorded without changing the batch decision.

A later conflicting message is recorded as a message-level REJECTED and never changes a batch decision.

### 6.2 Per-batch decision states

The per-batch decision states remain unchanged from the protocol specification:

- INITIATED
- VERIFIED
- COMMIT_DECIDED
- COMMITTED
- ABORTED

Rule: a message-level REJECTED never changes a batch-level decision state. The batch decision is final once COMMIT_DECIDED is durably recorded. A durable COMMIT_DECIDED may only move to COMMITTED.

`COMMITTED` is terminal and read-only. It may not transition to any other state.

[DECISION REQUIRED: per-message state set and per-batch state set | PROPOSED: keep them separate as above | REASON: message-level journal semantics and batch-level decision semantics are different and must not be conflated.]

## 7. HMAC canonical encoding summary

The canonical message format is specified in the contract. This architecture document records the required properties:

- Deterministic binary encoding, not JSON or YAML.
- Explicit domain separation using a fixed 16-byte constant.
- Fixed protocol identifier and versioning fields.
- Explicit key version selection before HMAC verification.
- Sender/recipient identity, task/report/batch identifiers, message type, round number, and sequence data.
- Exact payload length and exact opaque payload bytes.
- HMAC-SHA256 over the canonical body only; output length is 32 bytes.
- Durable replay semantics keyed by stable message identity and canonical digest.

The protocol domain is:

- domain separator bytes: `42 4C 49 4E 44 4D 41 52 4B 5F 36 46 32 43 4F 4E`
- ASCII text: `BLINDMARK_6F2CON`

## 8. Backend parser security gate

Signed or binary coordination ingestion must remain disabled until the backend HTTP parser is independently reviewed and tested. The limitations already identified in the preimplementation audit remain blocking conditions:

- the first read decodes the entire received chunk as UTF-8 before separating headers and body;
- there is no strict request-line validation;
- there are no strict header-size or header-count bounds;
- there are no robust checks for trailing bytes beyond the declared body length;
- requests without `Content-Length` are not sufficiently framed;
- a reverse proxy cannot repair or hide a backend parser's structural vulnerability;
- unescaped error strings and other parser outputs can be attacker-controlled.

This gate applies regardless of proxy presence. A reverse proxy does not remove the burden to review and test the backend parser.

## 9. Wire carriage

How `canonical_body || mac` is carried in HTTP must be specified before implementation. The current parser cannot safely accept raw binary. There are two possible carriage options:

1. Raw binary body: send the bytes directly as the HTTP request body, with `Content-Type: application/octet-stream` and a strict length; this is the most compact but requires a parser and framing review before enabling it.
2. ASCII-encoded body: base64 or hex-encode the binary payload before sending; this is easier for a parser to inspect and debug, but it inflates size, adds extra decoding logic, and changes the effective request limit.

[DECISION REQUIRED: HTTP wire carriage format | PROPOSED: raw binary body over a private internal route only after the parser gate passes; ASCII encoding remains a temporary fallback only if the parser gate cannot be satisfied | REASON: it preserves compactness and avoids silent size inflation, but it requires stricter byte-safe parsing.] 

## 10. Size budget

The fixed fields are 28 bytes; six length-prefixed IDs at worst case are 6 x (2 + 64) = 396 bytes; three `u64` values are 24 bytes; `payload_len` is 4 bytes. The worst-case header is therefore:

- fixed fields = 28
- six IDs = 396
- three `u64` = 24
- payload length = 4
- total header = 452 bytes
- HMAC = 32 bytes
- worst-case overhead = 484 bytes

The backend request limit is 65,536 bytes. The whole message, including any encoding and the MAC, must fit under that limit. The proposed worst-case max payload is:

- 65,536 - 484 = 65,052 bytes

This is the strict payload maximum before any ASCII encoding overhead. If ASCII encoding is chosen, the payload budget becomes smaller.

[DECISION REQUIRED: request-size cap for batch-carrying messages | PROPOSED: 65,052 bytes max encoded payload for a single message, and any /prepare/init or /prepare/commit batch payload must be split or capped before transport | REASON: the request limit and the MAC/header overhead are fixed and must not be exceeded.] 

## 11. Freshness, replay, and sequence

The validation order must be fixed and explicit:

1. parse the structure;
2. select the key by `key_version`;
3. verify the MAC in constant time;
4. lookup the journal entry;
5. exact duplicate returns the stored result even if expired; conflicting duplicate rejects;
6. apply expiry and skew checks;
7. validate sequence;
8. insert the new record durably;
9. process the message.

This order matters because a lost response plus an expired retry can otherwise leave the system stuck in a non-deterministic state.

The `message_id` uniqueness scope is open and must be set explicitly:

[DECISION REQUIRED: message_id uniqueness scope | PROPOSED: unique per (sender_id, recipient_id, task_id) | REASON: the message identity must be stable inside the task flow without implying cross-task global identity.] 

The `sequence_no` scope and rule are also open:

[DECISION REQUIRED: sequence_no scope and rule | PROPOSED: per sender/recipient/task/direction, strictly increasing for new messages, exact duplicates exempt, gaps allowed | REASON: this matches idempotent retry semantics while preserving monotonic ordering for newly observed traffic.] 

The freshness and retention parameters are not silent choices and must be kept explicit:

- [DECISION REQUIRED: MAX_FRESHNESS_WINDOW | PROPOSED: 31.75s | REASON: this matches the documented retry schedule and keeps the window aligned with the retry horizon.]
- [DECISION REQUIRED: MAX_CLOCK_SKEW | PROPOSED: 2s | REASON: a small but non-zero skew is practical for synchronized UTC hosts while maintaining freshness checks.]
- [DECISION REQUIRED: JOURNAL_RETENTION | PROPOSED: window + max retry horizon + batch lifetime | REASON: retained journal entries are required to handle retries, recovery, and unacknowledged commit decisions.] 
- [DECISION REQUIRED: clock source policy | PROPOSED: synchronized UTC with a known skew bound; untrusted clocks are rejected and the message is dropped | REASON: freshness checks rely on a trusted time source.] 

The protocol must reject a reused `message_id` with a different digest instead of treating it as a duplicate. Exact duplicates are idempotent. A later conflicting message is message-level REJECTED and not a batch decision change.

## 12. Restart, recovery, and durability

The journal must record what the node does on restart and what it may tell the peer. The following table defines the minimum required behavior:

| State | Restart behavior | Peer response allowed |
|---|---|---|
| RECEIVED_PENDING | Recover by re-reading the durable record and resuming or rejecting deterministically | May return prior durable result or a retry-required response |
| VERIFIED | Resume only by re-running the own verification input from persisted data; if not possible, leave non-committed and re-ingestible | May report pending verification or non-committed status |
| COMMIT_DECIDED | Treat as immutable and continue with durable commit application | May report committed or safe retry status |
| COMMITTED | Treat as terminal and read-only | May report committed result only |
| REJECTED | Keep the rejection and do not advance the state | May report rejection |
| ABORTED | Keep the batch abort and do not resume | May report aborted |
| IN_DOUBT | Keep pending reconciliation and do not count it as committed | May report in-doubt and request reconciliation |

A message must be durably recorded before any acknowledgement is sent. A successful acknowledgement is therefore a consequence of durable state, not a substitute for it.

Prio verify-state restoration is not supported by inspection of the current Prio sources. A candidate is to persist the local input share plus nonce, public share, task/config context, and verification-key identity, then rerun the local `verify_init` on restart. This is only a candidate and must not be treated as proven. If deterministic recovery cannot be established, the report remains non-committed and re-ingestable and must never be counted twice.

The actual Phase 6F.1 SQLite value is `PRAGMA synchronous = 2`, which is FULL. The protocol document saying NORMAL is therefore inconsistent with the actual storage layer. Because a durable-before-acknowledge requirement is necessary, the design must require FULL in WAL mode for the message journal. [DECISION REQUIRED: SQLite synchronous policy for the durable journal | PROPOSED: FULL | REASON: NORMAL in WAL mode can lose the latest committed transaction on power loss.] 

Lost-message cases must be explicit:

- request lost before it reaches the peer: the sender retries the exact message identity and the receiver applies idempotent handling;
- request processed but response lost: the sender replays the same message identity and the receiver returns the stored result;
- peer unavailable: the sender keeps the durable record and resumes only after the durable state is consistent with the recorded outcome.

For both directions, the retry owner is the party that has the durable state and the exact prior record; the peer must not mint a new ID.

## 13. Helper-to-leader path and key ownership

The helper-initiated path for IN_DOUBT reconciliation is required and must mirror the leader-to-helper path:

Helper egress proxy -> Leader coordination proxy -> Leader backend

This route must use the same local-identity requirements and process-account restrictions. If the helper-initiated path cannot be specified completely, the route remains DISABLED until it is specified.

[DECISION REQUIRED: helper-initiated reconciliation route | PROPOSED: disabled until specified and reviewed; if enabled, it must mirror the leader-to-helper path with matching local identity and HMAC checks | REASON: the same provenance and replay rules must apply in both directions.] 

The directional key table must separate signing, verification, and ownership responsibilities under dedicated service accounts:

| key_id / key_version | direction | signs | verifies | owns secret file | forgery risk |
|---|---|---|---|---|---|
| leader_to_helper | Leader -> Helper | leader application | helper backend | leader egress proxy or dedicated service account | any holder of the symmetric key |
| helper_to_leader | Helper -> Leader | helper application | leader backend | helper egress proxy or dedicated service account | any holder of the symmetric key |

The design must also state that the symmetric key value is never shared with participant clients or logged. Overlap and rotation windows are policy decisions: [DECISION REQUIRED: key overlap window | PROPOSED: one rotation window plus one retry window | REASON: rotation must preserve compatibility during retries without leaving the system vulnerable to stale acceptance.] 

The message types and their direction are:

| message type | route / purpose | direction |
|---|---|---|
| 0x0001 | /prepare/init | Leader -> Helper |
| 0x0002 | /prepare/finish | Leader -> Helper |
| 0x0003 | /prepare/commit | Helper -> Leader or reverse-direction reconciliation | 
| 0x0004 | batch status | Helper -> Leader |
| 0x0005 | commit decision | peer-specific | 
| 0x0006 | commit status | peer-specific |
| 0x0007 | replay check | peer-specific |
| 0x0008 | reject | peer-specific |

Unmapped or future message types remain open and must be treated as reserved until specified.

## 14. Smaller contract clarifications

- The `task_id` pattern must align with the protocol context requirement: `^[a-zA-Z0-9_-]{1,64}$`. Dot and colon are not allowed unless a separate design decision explicitly requires them.
- Phase 6F.1 storage uses integer identifiers while the contract uses ASCII string identifiers; migration is a future requirement and not authorized now.
- The term `protocol_id` appears in two different places: the envelope uses `BLMK` while the context uses a different field. This is a naming ambiguity and must be clarified in the final human review.
- Phase 6F.1 storage is not an inbound message journal and does not satisfy the durable coordination requirements.

The wording rule remains exact: Participant-to-ingress-proxy HTTPS; Leader-egress-proxy-to-helper-coordination-proxy mTLS; Helper-proxy-to-backend local protected IPC or explicitly selected private local transport.

## 15. Final review outcome

This document states the required architecture and the remaining human decisions. It is not an implementation authorization, and it does not authorize Checkpoint B or any implementation work.
