# Phase 6F.2 Proxy Contract (Final Contract Correction Pass)

Status: Proposed contract for independent human review only. This document does not authorize Checkpoint B, proxy implementation, TLS configuration, HMAC implementation, message-journal development, certificate generation, or runtime network deployment.

## 1. External transport requirements

The participant-facing service exposes distinct HTTPS endpoints for Leader and Helper ingress. Each endpoint must:

- require TLS 1.2+ with TLS 1.3 preferred;
- terminate TLS at the proxy;
- validate the full certificate chain and hostname;
- reject expired, not-yet-valid, wrong-purpose, wrong-host, or untrusted certificates;
- forbid public plaintext HTTP for participant traffic;
- record only minimal operational metadata and never the request body, credentials, HMAC values, or private keys.

The proxy host is within the trusted boundary for the share it terminates. Proxy-to-backend traffic is not automatically encrypted; it must be protected by the selected local transport policy.

## 2. Required coordination path and local isolation

The required internal coordination path is:

Leader application -> Leader egress proxy -> Helper coordination proxy -> Helper Rust backend

The leader egress proxy and helper backend must not be reached via generic loopback TCP without a process-restrictive local policy. Local loopback TCP does not identify the process. The following requirements are mandatory:

### 2.1 Leader egress proxy local endpoint

Minimum requirement: the endpoint must authenticate the connecting process identity and bind to the exact leader service identity.

Supported local endpoint types:
- Linux: Unix domain stream socket with `SO_PEERCRED` or equivalent peer-credential verification.
- Windows: Named pipe with restricted DACL and service-specific SID.

If neither mechanism is available on the target OS, the leader egress endpoint is disabled and no fallback to unrestricted loopback TCP is permitted.

### 2.2 Helper backend local endpoint

Minimum requirement: the backend access path must accept requests only from the helper coordination proxy process identity.

Supported local endpoint types:
- Linux: Unix domain stream socket with `SO_PEERCRED`.
- Windows: Named pipe with restricted ACL.

If the OS cannot prove the peer is the helper coordination proxy, the helper backend route remains disabled.

This is an explicit design requirement: generic `127.0.0.1` listeners are not considered process-restricted.

## 3. Authentication domains

The following are intentionally separate and must not be collapsed:

1. TLS peer authentication.
2. Proxy-generated identity assertion.
3. HMAC message authentication.
4. Local process and network isolation.

A valid HMAC by itself does not prove proxy traversal. A proxy-generated identity value by itself does not prove identity if direct backend access is possible.

## 4. mTLS and proxy provenance contract

### 4.1 Leader egress proxy to helper coordination proxy

- The leader egress proxy owns the leader client certificate and private key.
- The leader application does not read or store the certificate private key directly.
- The leader egress proxy validates the helper coordination proxy certificate, hostname, validity period, and configured trust domain.
- The helper coordination proxy validates the leader client certificate under an allowlist of approved leader identities and coordination trust roots.

The mTLS handshake authenticates possession of the private key and validates the configured peer identity under the coordination trust policy.

### 4.2 Helper coordination proxy to helper backend

After mTLS validation, the helper coordination proxy must:

- strip all inbound copies of a reserved identity header;
- set exactly one trusted `X-Authenticated-Coordination-Peer` value derived from the validated peer identity; and
- forward the request only to the internal helper backend route.

The helper backend must:
- accept the request only over the process-restricted local IPC endpoint bound to the helper coordination proxy identity;
- verify the local peer identity is the helper coordination proxy service account;
- verify the proxy-generated identity assertion equals the HMAC sender identity; and
- reject any request that uses direct backend access, a generic loopback listener, or a spoofed identity header.

If direct access to the helper backend is possible, the coordination route must remain disabled.

## 5. HMAC container specification

This is the final proposal-grade binary format for human review. It is not implemented and it is not a claim of cryptographic approval.

The canonical message body is a deterministic binary structure with the following field order:

| Field | Type | Width | Notes |
|---|---|---:|---|
| domain_separator | bytes | 16 | Fixed ASCII bytes; exact protocol-domain constant |
| protocol_id | ASCII string | 4 | Exact identifier `BLMK` |
| protocol_version | uint16 | 2 | Big-endian major version |
| key_version | uint16 | 2 | Explicit key version |
| message_type | uint16 | 2 | Exactly one of the approved message types |
| protocol_round | uint16 | 2 | Round number within the protocol |
| sender_id | ASCII string | 2 + N | Length-prefixed; max 64 bytes |
| recipient_id | ASCII string | 2 + N | Length-prefixed; max 64 bytes |
| task_id | ASCII string | 2 + N | Max 64 bytes |
| report_id | ASCII string | 2 + N | Max 64 bytes |
| batch_id | ASCII string | 2 + N | Max 64 bytes |
| message_id | ASCII string | 2 + N | Stable message identity; max 64 bytes |
| sequence_no | uint64 | 8 | Big-endian |
| issued_at_ms | uint64 | 8 | Unix epoch milliseconds |
| expires_at_ms | uint64 | 8 | Unix epoch milliseconds |
| payload_len | uint32 | 4 | Exact payload length in bytes |
| payload | opaque bytes | payload_len | Exact bytes as delivered |

Fixed message-type values:

| Value | Meaning |
|---|---|
| 0x0000 | Reserved; unsupported |
| 0x0001 | LEADER_TO_HELPER_REQ |
| 0x0002 | LEADER_TO_HELPER_ACK |
| 0x0003 | HELPER_TO_LEADER_REQ |
| 0x0004 | HELPER_TO_LEADER_ACK |
| 0x0005 | COMMIT_DECISION |
| 0x0006 | COMMIT_STATUS |
| 0x0007 | REPLAY_CHECK |
| 0x0008 | REJECT |
| 0xFFFF | Reserved; unsupported |

The exact protocol domain separator is the ASCII string `BLINDMARK_6F2CON`, which encodes to:

`42 4C 49 4E 44 4D 41 52 4B 5F 36 46 32 43 4F 4E`

### 5.1 Encoding rules

- All integers are unsigned and encoded in big-endian byte order.
- All ASCII strings use bytes with allowed characters limited to `[A-Za-z0-9._:-]`.
- Empty values are rejected for `protocol_id`, `sender_id`, `recipient_id`, `task_id`, `report_id`, `batch_id`, and `message_id` unless separately approved.
- Variable-length strings use a 2-byte length prefix in network byte order before the exact bytes.
- `payload_len` is the exact byte length of the `payload` field; payload bytes are opaque and never normalized or re-encoded.
- The message is deterministic and canonical: no JSON, no locale-sensitive formatting, no whitespace folding, and no implicit field reordering.

### 5.2 Maximum lengths

- domain separator: fixed 16 bytes.
- protocol_id: fixed 4 bytes ASCII `BLMK`.
- sender/recipient/task/report/batch/message IDs: max 64 bytes each.
- payload: max 65,536 bytes.
- `expires_at_ms` must be greater than `issued_at_ms` by at least one millisecond and no more than the configured maximum freshness window.

### 5.3 Validation and rejection rules

Reject the message if any of the following hold:

- protocol_id or domain_separator mismatch the configured protocol constant;
- protocol_version is unsupported;
- key_version is not configured or is revoked;
- sender_id or recipient_id is empty or not in the allowed role set;
- message_id is empty, duplicated, or reused with a different digest;
- sequence_no is invalid for the configured protocol state;
- issued_at_ms and expires_at_ms are outside the allowable skew window;
- payload_len exceeds the configured maximum or does not match the actual encoded bytes;
- strings contain forbidden or non-ASCII bytes;
- the message type or round is unsupported;
- HMAC verification fails in constant time.

### 5.4 MAC coverage and algorithm

The MAC is computed as HMAC-SHA256 over the canonical body bytes only, not over the final message including the appended MAC. The final message format is:

`canonical_body || hmac_sha256(canonical_body)`

- MAC algorithm: HMAC-SHA256.
- HMAC output length: 32 bytes.
- Key selection: the receiver selects the validation key by `key_version` from a locally configured allowlist, and validates the sender/recipient association before verification.
- Key rotation: new key versions must be provisioned and validated before they are deployed for sending; old versions remain accepted only within a bounded expiry or retry window.
- Symmetric HMAC provides no non-repudiation. Any party holding the symmetric key may forge valid messages in that sender direction.

### 5.5 Replay identity and durable uniqueness

- The `message_id` is the stable logical identity for the operation.
- Replay uniqueness is durable and must be persisted with the canonical digest.
- Same `message_id` + same canonical digest is idempotent and must return the prior durable result.
- Same `message_id` + different canonical digest is a conflict and must be rejected without state advancement.
- The canonical digest is computed over the full canonical body, not just the payload.

### 5.6 Reproducible example vectors

The following values are synthetic and intentionally not real secrets. They were generated locally with Python’s standard `hmac` library and used for review validation only.

Key used for the synthetic vectors:

`00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F 10 11 12 13 14 15 16 17 18 19 1A 1B 1C 1D 1E 1F`

Canonical body for vector 1 (length = 119 bytes):

```text
424c494e444d41524b5f364632434f4e424c4d4b000100010001000100066c6561646572000668656c70657200087461736b2d303031000a7265706f72742d303031000962617463682d30303100086d73672d3030303100000000000000010000018bcfe568000000018bcfe57b880000000401020304
```

HMAC-SHA256 for vector 1:

```text
593cae29ad9660a107c97e1dba5877f451684274df8e0a71ff6e95634edeb766
```

Vector 2 changes only the final payload byte from `0x04` to `0x05` and therefore must fail validation; its HMAC is:

```text
7ad9a682a3519cf35e4ea3f22226b2bdad32fe1d7c95cfaa310ef5333ccec508
```

Vector 3 keeps the same logical `message_id` but changes the payload to a different canonical body; it must be rejected as a conflicting duplicate. Its HMAC is:

```text
d305a7f482fb2367ff216c059fd47f6b75518e1ec36fa15a73f6995e80d87fe6
```

The acceptance requirement is: same canonical body + matching key version + valid sender/recipient association = accepted. Any other canonical byte sequence is rejected as a mismatch or a conflicting duplicate.

## 6. Durable inbound-message journal and state machine

The durable journal must record, at minimum:

- stable message identity;
- canonical message digest;
- sender and recipient;
- task/report/batch identity;
- message type and sequence;
- processing state;
- creation timestamp and last update timestamp;
- persisted outcome or rejection reason.

### 6.1 Proposed states

1. NOT_RECEIVED
2. RECEIVED_PENDING
3. VERIFIED
4. COMMIT_DECIDED
5. COMMITTED
6. REJECTED

### 6.2 Permitted transitions

- NOT_RECEIVED -> RECEIVED_PENDING
- RECEIVED_PENDING -> VERIFIED
- RECEIVED_PENDING -> REJECTED
- VERIFIED -> COMMIT_DECIDED
- VERIFIED -> REJECTED
- COMMIT_DECIDED -> COMMITTED
- COMMIT_DECIDED -> REJECTED only if the rejection reason is an explicit validation failure before the durable commit decision is recorded; once durable, the state must not be rewritten by timeout or socket disconnect.

### 6.3 Forbidden transitions

- COMMITTED -> anything other than terminal read-only state.
- REJECTED -> VERIFIED or COMMIT_DECIDED.
- COMMIT_DECIDED -> NOT_RECEIVED.
- A timeout or disconnect must never rewrite a durable commit decision to an earlier state.

### 6.4 Restart behavior

On restart, the application inspects the durable journal before retrying or accepting a new request. If a message is already `COMMIT_DECIDED` or `COMMITTED`, the system returns the prior outcome without reprocessing. If the message is `RECEIVED_PENDING`, verification resumes or the message is rejected. If it is `NOT_RECEIVED`, it may be recovered only by reuse of the exact same message identity and bytes.

### 6.5 Retry ownership and backoff

- The application owns logical retry decisions.
- The proxy must not mint a new message identity.
- The proxy may retry only the same identity and same exact bytes if the application permits that path.
- Proxy retries of non-idempotent requests are prohibited unless durable deduplication makes them safe.
- A timeout is an ambiguous outcome and requires a persisted state decision.

Recommended bounded retry progression:

- 250 ms,
- 500 ms,
- 1 s,
- 2 s,
- 4 s,
- 8 s,
- 16 s,
- then stop unless the application explicitly decides to continue.

### 6.6 Exact duplicate vs conflicting duplicate

- Exact duplicate: same `message_id` and same canonical digest -> return the prior durable result or equivalent pending result.
- Conflicting duplicate: same `message_id` but different canonical digest -> reject and do not advance the state.

The Phase 6F.1 storage layer does not provide a message-level journal and must not be treated as satisfying the required durable coordination behavior.

## 7. Profile-specific enforcement and disabled states

### Profile A: local development

- Separate local development hosts may run leader and helper processes on one machine.
- The local endpoint for the helper backend must still be restricted to the helper coordination proxy service identity.
- If the OS cannot enforce peer identity or ACL restriction, the route remains disabled.

### Profile B: single-host deployment

- Leader and Helper run on the same host but use dedicated service identities.
- The local leader egress endpoint and helper backend endpoint must both restrict access to the exact service identities.
- Generic loopback TCP is not accepted.

### Profile C: separate-host deployment

- Leader and Helper run on separate hosts or trust domains.
- Participant HTTPS is terminated at the ingress proxy.
- Leader-to-helper network coordination occurs over private mTLS between proxies.
- The helper backend path remains a local protected IPC or another explicitly selected private local transport.
- If the helper host cannot enforce the local boundary, coordination remains disabled.

## 8. Final review outcome

The contract is ready for independent human contract review. It is not an implementation authorization, and it does not authorize Checkpoint B, proxy implementation, TLS setup, HMAC implementation, message-journal deployment, certificate generation, or runtime network change.

- Linux with Unix domain sockets and peer-credential checks.
- Windows with named pipes and restricted DACLs.

If the required local-identity enforcement is unavailable, coordination is disabled.

### Profile B: Single-host test deployment

Supported OS assumptions:
- same as Profile A; process identity binding is required for both leader egress and helper backend local paths.

If the host cannot provide peer-credential or named-pipe ACL enforcement, coordination is disabled.

### Profile C: Separate-host deployment

Supported OS assumptions:
- same local process-isolation requirements for the helper backend path on each host;
- leader-to-helper uses mTLS between coordination proxies over a private network.

If the helper backend cannot enforce exclusive access from the coordination proxy via protected local IPC, the coordination path remains disabled.

## 8. Cross-document wording correction

The phrase “end-to-end TLS” is replaced with explicit transport terminology:

- Participant-to-ingress-proxy HTTPS.
- Leader-egress-proxy-to-Helper-coordination-proxy mTLS.
- Helper-proxy-to-backend local protected IPC or explicitly selected private local transport.

TLS terminates at the relevant proxy. It does not imply the Rust backend receives TLS.

## 9. Implementation boundary

This contract defines the proposed design and the required enforcement conditions. It does not implement the design, does not create certificates, does not create secrets, and does not authorize a runtime proxy or network path. All decisions in this document are subject to human approval and deployment-specific OS validation.
