# Phase 6F.2 Proxy Security Review

Review type: Architecture design review only. No proxy, HMAC, journal, or network functionality is implemented or run.

Status: Correction round 3. Proposed for independent human review. Not approved. Does not authorize Checkpoint B or any implementation.

## 1. Security conclusion

The revised design is materially stronger than a generic loopback-only proposal because it clearly separates the authentication, provenance, and local-transport controls that were previously conflated. The design is specific enough to support independent human review without approving implementation.

## 2. Findings resolved by the final design

### Finding A — concrete process-level isolation

The design explicitly rejects the assumption that a listener on `127.0.0.1` is process restricted. The architecture requires enforceable local IPC for both of the following:

- Leader application -> leader egress proxy.
- Helper coordination proxy -> helper backend.

Supported mechanisms are:

- Linux Unix domain sockets with peer-credential verification and restrictive owner/group enforcement.
- Windows named pipes with restrictive DACLs and service-identity allowlists.

If the platform cannot enforce the selected mechanism, the coordination route remains disabled. This is a design requirement, not an implementation claim.

### Finding B — helper backend provenance

The design makes the trusted request path explicit:

Leader application -> leader egress proxy -> helper coordination proxy -> helper backend

The backend requires all of the following:

- the request arrived through the authorized local proxy identity;
- the helper proxy validated the leader certificate and generated the trusted assertion;
- the proxy-generated identity matches the HMAC sender identity;
- the canonical HMAC envelope validates under the configured key version;
- the durable journal proves the message is neither a conflicting duplicate nor a stale replay.

Direct backend access is prevented by requiring a process-restricted local IPC endpoint and rejecting any direct loopback path. A proxy-generated assertion alone is not trusted if direct backend access remains possible.

### Finding C — canonical HMAC message format

The final contract specifies a deterministic binary message layout with explicit field order, width, byte order, string encoding rules, and rejection rules. It also states that the message is HMAC-SHA256 over the canonical body only and that the receiver selects the active key by `key_version` from a configured allowlist.

The contract explicitly records the domain separator, fixed message-type values, payload-length rules, exact duplicate/conflicting-duplicate semantics, and the need to keep message-level and batch-level states separated. The final correction makes the design reproducible and testable without implementation.

### Finding D — backend parser security gate

The design explicitly requires a backend parser gate before signed or binary coordination ingestion is allowed:

- the current parser cannot safely accept raw binary;
- the initial UTF-8 decode, lack of strict request-line validation, missing header-size or count bounds, and framing ambiguities remain open blockers;
- a reverse proxy cannot fix a backend parser differential;
- no binary or signed coordination message is permitted until the parser is independently reviewed and tested.

This is a documented requirement only and does not claim the parser has been fixed.

### Finding E — durable delivery and recovery state machine

The final contract defines the journal fields and state machine for:

- not yet durably received;
- received but not yet processed;
- verified;
- commit decision recorded;
- committed;
- rejected.

The design preserves the Phase 6F.1 invariant that a durable COMMIT_DECIDED must never be rolled back merely because of a timeout or connection failure. The proxy never invents a new message identity, and the application owns retry semantics.

## 3. Remaining explicit assumptions

The final design fixes the major contract gaps but still leaves the following as explicit assumptions requiring human approval or additional design work:

- final OS-specific validation of the selected local IPC mechanism;
- final certificate lifecycle and revocation model;
- final HMAC key-version handling and rotation policy;
- final application retry and resume semantics;
- final durable journal schema and indexing strategy; and
- final deployment-specific host and admin-boundary review for each profile.

[DECISION REQUIRED: local IPC and rotation policy for each supported system | PROPOSED: Unix domain sockets on Linux and named pipes with DACL/SID checks on Windows; rotation window is one rotation window plus one retry window | REASON: this is the minimum explicit control needed to satisfy the review without silently changing security policy.] 

## 4. Cross-document wording correction

The phrase “end-to-end TLS” has been replaced with explicit transport terminology:

- Participant-to-ingress-proxy HTTPS.
- Leader-egress-proxy-to-helper-coordination-proxy mTLS.
- Helper-proxy-to-backend local protected IPC or explicitly selected private local transport.

TLS terminates at the proxy and does not imply TLS is present on the helper backend path.

## 5. Operational controls and limits

The design still relies on operational controls outside the codebase:

- service identity assignment;
- local socket and pipe ACL configuration;
- certificate trust and rotation policy;
- host firewall policy;
- log retention policy; and
- key custody and HMAC secret management.

A secure design document does not make a platform secure by assertion. The design therefore explicitly states that enforcement must be validated on the target OS and deployment profile.

## 6. Proposed versus implemented behavior

Proposed behavior:
- local IPC enforcement;
- helper proxy-generated identity assertion;
- deterministic HMAC message envelope;
- durable journal and state machine;
- parser gate before signed or binary ingestion;
- profile-specific coordination enabling and disabling.

Implemented behavior:
- None of the above is implemented in this task.
- No proxy, certificate, secret, or network configuration is generated or applied.
- No source files, Cargo files, or Phase 6F.1 storage files are altered.

## 7. Final design verdict

This design package is proposed for independent human review. It is not an implementation authorization and it does not authorize Checkpoint B, HMAC construction, journal implementation, certificate generation, TLS configuration, or runtime network change.
