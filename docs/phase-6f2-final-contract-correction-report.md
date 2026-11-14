# Phase 6F.2 Final Contract Correction Report

Status: READY FOR INDEPENDENT HUMAN CONTRACT REVIEW.

This document is strictly a documentation-only correction package. It does not authorize Checkpoint B, proxy implementation, TLS configuration, HMAC implementation, message-journal implementation, certificate generation, secret creation, or any runtime network deployment.

## 1. Scope of the correction

The contract-correction pass addressed the residual gaps identified in the pre-implementation audit and the historical second-review evidence package:

- local process identity enforcement for the leader egress and helper backend paths;
- clear separation of TLS peer identity, proxy-generated identity assertion, and HMAC sender identity;
- canonical binary HMAC rules, key-version handling, and duplicate/conflicting-duplicate semantics;
- durable message-journal semantics and non-rollback of `COMMIT_DECIDED` states;
- wording cleanup around the transport boundaries so the design no longer says “end-to-end TLS” where only HTTPS and mTLS are present.

The design remains intentionally non-executable and is not an implementation approval.

## 2. Documents updated

The final corrected design package is confined to the approved documentation set:

- [docs/phase-6f2-proxy-architecture.md](phase-6f2-proxy-architecture.md)
- [docs/phase-6f2-proxy-contract.md](phase-6f2-proxy-contract.md)
- [docs/phase-6f2-deployment-profiles.md](phase-6f2-deployment-profiles.md)
- [docs/phase-6f2-proxy-security-review.md](phase-6f2-proxy-security-review.md)

No change was made to the historical evidence report or the pre-implementation audit baseline; those remain preserved as the review record and design input.

## 3. Final resolution of the contract gaps

### 3.1 Process isolation and local provenance

The final design now requires the following local path controls:

- Leader service -> leader egress proxy: local Unix domain socket or named pipe restricted to the exact leader service identity.
- Helper coordination proxy -> helper backend: local Unix domain socket or named pipe restricted to the helper coordination proxy identity.
- Loopback TCP is explicitly not sufficient; it does not authenticate the local process identity.
- Direct backend access is explicitly rejected if its local transport cannot prove the peer identity of the helper coordination proxy.

### 3.2 Identity and authentication separation

The design now avoids collapsing separate trust claims into one value. The backend must require all of the following together:

- authorized local IPC peer identity;
- validated proxy-generated identity assertion;
- matching HMAC sender identity;
- canonical HMAC verification under the active key version;
- durable replay status showing the message is neither a conflicting duplicate nor a stale replay.

### 3.3 HMAC contract precision

The final contract specifies the exact canonical binary format and uses the following fixed domain separator:

- ASCII: `BLINDMARK_6F2CON`
- hex: `424C494E444D41524B5F364632434F4E`

The canonical message contract includes fixed message-type values and explicit duplicate/conflicting-duplicate semantics. It also states that the canonical HMAC is computed over the canonical body only.

### 3.4 Durable journal semantics

The contract now explicitly requires durable handling of message identity and commit state transitions. In particular:

- exact duplicate: same `message_id` and same canonical digest -> idempotent handling;
- conflicting duplicate: same `message_id` but different canonical digest -> reject without state advancement;
- durable `COMMIT_DECIDED` is never rolled back by timeout or connectivity failure;
- the proxy does not mint a new message identity after timeout; retry semantics remain application-owned.

### 3.5 Terminology correction

The design now uses consistent wording:

- Participant-to-ingress-proxy HTTPS.
- Leader-egress-proxy-to-helper-coordination-proxy mTLS.
- Helper-proxy-to-backend local protected IPC or explicitly selected private local transport.

This eliminates the earlier misleading “end-to-end TLS” phrasing.

## 4. Reproducible HMAC vectors

The following synthetic vector values were generated locally with Python’s standard library HMAC implementation for review validation only.

Key used for synthetic vectors:

`00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F 10 11 12 13 14 15 16 17 18 19 1A 1B 1C 1D 1E 1F`

Canonical body for vector 1 (length = 119 bytes):

```text
424c494e444d41524b5f364632434f4e424c4d4b000100010001000100066c6561646572000668656c70657200087461736b2d303031000a7265706f72742d303031000962617463682d30303100086d73672d3030303100000000000000010000018bcfe568000000018bcfe57b880000000401020304
```

HMAC-SHA256 for vector 1:

```text
593cae29ad9660a107c97e1dba5877f451684274df8e0a71ff6e95634edeb766
```

Vector 2 changes only the final payload byte:

```text
7ad9a682a3519cf35e4ea3f22226b2bdad32fe1d7c95cfaa310ef5333ccec508
```

Vector 3 retains the same logical message identity but carries a different canonical body; it must be rejected as a conflicting duplicate:

```text
d305a7f482fb2367ff216c059fd47f6b75518e1ec36fa15a73f6995e80d87fe6
```

These vectors establish the required semantics: valid acceptance, tampering detection, and conflicting-replay rejection.

## 5. Review verdict

READY FOR INDEPENDENT HUMAN CONTRACT REVIEW.

This is a review verdict, not a deployment authorization. The design does not authorize implementation work, network changes, certificates, HMAC secrets, or journal changes. It is ready for a human to review the final contract language and trust boundaries before any implementation decision is made.

## 6. Hard boundary for this task

This documentation-only task does not authorize Checkpoint B, proxy implementation, TLS configuration, HMAC implementation, message-journal implementation, certificate generation, or network changes.
