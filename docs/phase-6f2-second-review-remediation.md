# Phase 6F.2 Second-Review Remediation Report

## 1. Objective

This document records the design-only remediation for the four incomplete findings and the cross-document wording ambiguity identified in the historical second-review evidence package. This document is a contract and approval record only; it does not implement any runtime feature.

## 2. Finding A — concrete process-level isolation

### Selected design decision

The architecture requires enforceable local process identity for the two proxy connections:

1. Leader application -> Leader egress proxy.
2. Helper coordination proxy -> Helper backend.

Required local endpoint types:
- Linux: Unix domain stream socket with peer-credential verification.
- Windows: Named pipe with restrictive DACL and service identity allowlist.

A generic loopback TCP listener is not considered process-restricted and is explicitly rejected as proof of authorization. If the required mechanism is unavailable on a profile, coordination is disabled for that profile.

### Exact document sections updated

- [docs/phase-6f2-proxy-architecture.md](phase-6f2-proxy-architecture.md)
- [docs/phase-6f2-deployment-profiles.md](phase-6f2-deployment-profiles.md)
- [docs/phase-6f2-proxy-contract.md](phase-6f2-proxy-contract.md)

### Remaining assumptions

- The target OS must provide peer credentials or named-pipe ACL enforcement.
- Local administrator privileges remain a host-level exception.
- No fallback to unrestricted loopback TCP is permitted.

### Human approval required

- OS-specific selection of the local transport mechanism.
- Service-account assignment and socket/pipe ACL policy.
- Final decision on whether a given deployment profile supports the required enforcement.

## 3. Finding B — helper backend provenance

### Selected design decision

The complete request path is defined as:

Leader application -> Leader egress proxy -> Helper coordination proxy -> Helper backend

The helper backend verifies the full chain:

- mTLS validation at the helper coordination proxy;
- proxy-generated identity assertion after certificate validation;
- local-protected backend IPC peer identity;
- HMAC sender identity must match the proxy assertion;
- canonical HMAC verification and durable replay check.

Direct access to the helper backend is blocked by refusing direct loopback access or any non-proxy local transport and by requiring the helper coordination proxy's exact process identity.

### Exact document sections updated

- [docs/phase-6f2-proxy-architecture.md](phase-6f2-proxy-architecture.md)
- [docs/phase-6f2-proxy-contract.md](phase-6f2-proxy-contract.md)
- [docs/phase-6f2-proxy-security-review.md](phase-6f2-proxy-security-review.md)

### Remaining assumptions

- The helper backend must be restricted to a local protected IPC endpoint.
- The helper proxy must strip all incoming identity copies and set exactly one trusted value.
- The backend must reject any direct access path not associated with the helper coordination proxy identity.

### Human approval required

- Final choice of protected local transport implementation.
- Approval of the trust boundary between helper proxy and helper backend.
- Confirmation that the coordination route remains disabled when local enforcement is unavailable.

## 4. Finding C — exact canonical HMAC message format

### Selected design decision

The contract defines a proposed deterministic binary HMAC envelope with:

- domain separator;
- protocol identifier and version;
- key version;
- message type and round;
- sender and recipient identities;
- task/report/batch identifiers;
- stable message identity;
- sequence number;
- issued-at and expiry timestamps;
- payload length and exact opaque payload bytes;
- HMAC-SHA256 over the canonical body only.

The format explicitly rejects ambiguous JSON, Unicode normalization, and implicit field encoding. It also states that symmetric HMAC provides no non-repudiation.

### Exact document sections updated

- [docs/phase-6f2-proxy-contract.md](phase-6f2-proxy-contract.md)
- [docs/phase-6f2-proxy-security-review.md](phase-6f2-proxy-security-review.md)

### Remaining assumptions

- Final cryptographic review remains required before implementation.
- The proposed lengths, field order, and encoding rules are design review material, not implementation approval.
- Key-version mapping and rotation policy remain deployment-specific.

### Human approval required

- Final approval of the canonical binary encoding.
- Approval of the selected limits and invalid-value rules.
- Approval of the HMAC-SHA256 construction and key-version policy.

## 5. Finding E — durable delivery and recovery state machine

### Selected design decision

The documentation now defines a durable inbound message journal and a state machine covering:

1. not durably received;
2. durably received but not processed;
3. verified;
4. commit decision durably recorded;
5. durably committed;
6. rejected.

The design preserves the required invariant:

A durable `COMMIT_DECIDED` must never be rolled back merely because of a timeout or connection failure occurred.

The application owns retry decisions; the proxy does not create new identities, and any retried message must reuse the same stable identity and exact body or be explicitly rejected as conflict.

### Exact document sections updated

- [docs/phase-6f2-proxy-architecture.md](phase-6f2-proxy-architecture.md)
- [docs/phase-6f2-proxy-contract.md](phase-6f2-proxy-contract.md)
- [docs/phase-6f2-proxy-security-review.md](phase-6f2-proxy-security-review.md)

### Remaining assumptions

- A new durable journal schema must be designed and implemented later.
- The existing Phase 6F.1 schema does not satisfy the journal requirements.
- The application must decide whether to resume, return a prior result, or reject after durable state inspection.

### Human approval required

- Acceptance of the proposed state machine and retry ownership model.
- Approval of the bounded backoff policy and durable exact-duplicate semantics.
- Agreement that the existing Phase 6F.1 schema is not sufficient for the required coordination journal.

## 6. Cross-document wording ambiguity — “end-to-end TLS”

### Selected design decision

The corrected terminology is:

- Participant-to-ingress-proxy HTTPS.
- Leader-egress-proxy-to-Helper-coordination-proxy mTLS.
- Helper-proxy-to-backend local protected IPC or explicitly selected private local transport.

TLS is stated to terminate at the relevant proxy. The Rust backend is not described as receiving TLS directly unless an explicit local transport is selected and enforced.

### Exact document sections updated

- [docs/phase-6f2-deployment-profiles.md](phase-6f2-deployment-profiles.md)
- [docs/phase-6f2-proxy-security-review.md](phase-6f2-proxy-security-review.md)

## 7. Profiles remaining disabled

The following states remain explicit:

- If local process identity enforcement is unavailable, the coordination route remains disabled.
- Profile A remains disabled unless a Linux peer-credential socket or Windows named-pipe ACL policy is available.
- Profile B remains disabled unless the same local enforcement is available for the leader and helper service identities.
- Profile C remains disabled unless the helper backend path remains restricted to the helper coordination proxy identity and remote connection authentication remains valid.

## 8. Proposed vs implemented behavior

### Proposed behavior

- Local process-restricted IPC endpoints.
- Proxy-generated trusted identity assertions.
- mTLS coordination between proxies.
- Deterministic HMAC message format.
- Durable inbound journal and recovery state machine.
- Profile-by-profile coordination enablement rules.

### Implemented behavior

None of the above behavior is implemented in this task. No network, proxy, certificate, HMAC, parser, or journal functionality has been created or changed. The task remains documentation-only.

## 9. Decisions requiring human approval before implementation

1. Accept the process-bound IPC design as the required isolation mechanism on each supported host.
2. Approve the helper proxy-generated identity assertion and the backend validation contract.
3. Approve the deterministic canonical HMAC encoding and key-version policy.
4. Approve the durable inbound message journal and state transitions.
5. Approve the Profile A/B/C enablement and disabled-state decisions.
6. Approve the clarified transport terminology replacing “end-to-end TLS”.

## 10. Final remediation status

The design is now explicit enough to support a human review of the remaining assumptions, but it remains a proposal and is not an implementation authorization. No Checkpoint B approval is granted.
