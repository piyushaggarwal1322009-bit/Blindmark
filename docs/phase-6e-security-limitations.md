# Phase 6E: Security Boundaries & Remaining Limitations

## 1. Trust Boundaries & Collusion Assumptions

### 1.1 Local Co-location Warning
> **IMPORTANT NOTICE**:
> Running the Leader on port `8001` and Helper on port `8002` on the same operating system or virtual machine **DOES NOT** provide real-world privacy.
> If both services run under the same administrator, cloud account, or machine memory space, an adversary can easily read both processes and recombine secret shares:
> $$\text{Stipend} = \text{Share}_0 + \text{Share}_1 \pmod p$$
> 
> In a production deployment, the Leader and Helper MUST be hosted by **two legally and operationally separate organizations** (e.g. University Career Center and an external privacy auditing non-profit).

### 1.2 Development Authentication vs. Production Security
- The `X-Service-Key` mechanism with `DEV_SERVICE_KEY = "dev-aggregator-secret-key-blindmark"` is a **development-only placeholder**.
- It provides basic route gating during local integration testing.
- **Production Requirement**: Production DAP services must enforce **Mutual TLS (mTLS)** with X.509 client certificates and signed authorization tokens.

---

## 2. Unresolved Protocol Decisions (Required Before Production)

Before moving from Phase 6E service scaffolding to full production ingestion, the following architectural decisions must be implemented:

1. **Task Configuration Distribution**:
   - *Current status*: Aggregators are initialized with local VDAF parameters.
   - *Production requirement*: A secure task configuration protocol (e.g. IETF DAP `POST /tasks`) where campaign parameters (bucket count, chunk length, epoch schedule) are signed by the campaign authority and verified by both aggregators.
2. **Public Share Distribution**:
   - The public share contains metadata (e.g. joint randomness parts) needed by both aggregators. The protocol must specify whether the client sends it in duplicate to both endpoints, or if the Leader distributes it during the verification preparation exchange.
3. **Task & Aggregator Binding**:
   - The client report must include an authenticated binding to `task_id` and the designated `aggregator_id`. Aggregators must cross-validate that the token, nonce, and report belong to the specific active campaign epoch.
4. **Anonymous Single-Use Token Verification**:
   - Aggregators must check nullifier tokens to prevent double-voting without learning the participant's real identity (e.g., using blind signatures, Privacy Pass, or blinded nullifier sets).
5. **Partial Submission Garbage Collection**:
   - If a participant client submits a share to the Leader but crashes before submitting to the Helper, the Leader holds an orphaned pending state. An epoch-based timeout mechanism must purge unverified reports so they never corrupt aggregate shares.
6. **Aggregator-to-Aggregator Preparation Protocol**:
   - How Leader and Helper batch and exchange `VerifierShareMessage` over mTLS.
7. **Collector Authorized Release**:
   - Enforcing that the Collector cannot trigger unsharding until the campaign's scheduled end time and minimum participant threshold have been reached.

