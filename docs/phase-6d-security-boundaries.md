# Phase 6D: Corrected Security Boundaries, Failure Modes & Threat Model

## 1. Threat Model & Trust Assumptions

### 1.1 Non-Collusion Boundary
Blindmark relies on the 2-party non-colluding aggregator model:
- **Leader Aggregator Domain**: Operates Aggregator 0. Learns only `InputShare_0` and the zero-knowledge verifier shares.
- **Helper Aggregator Domain**: Operates Aggregator 1. Learns only `InputShare_1` and the zero-knowledge verifier shares.
- **FastAPI Control Plane Domain**: Manages web UI, user accounts, and campaigns. **LEARNS ZERO INPUT SHARES.**

> **COLLUSION WARNING**:
> If the Leader and Helper collude or are hosted in the same cloud tenant/container under one administrator, the non-collusion boundary collapses. Running them on different ports on the same computer is an engineering convenience for testing, NOT a security guarantee.

---

## 2. Failure Mode & Recovery Matrix (Corrected)

| Failure Event | Detection Point | Handling & Recovery Action | Impact on Privacy & Output |
|---|---|---|---|
| **Client submits only to Leader** | Helper has no report for `report_id`. | Helper never sends verifier share to Leader. Leader's pending verification times out and is purged. | **No Leakage**: Report never enters aggregate. |
| **Client submits only to Helper** | Leader has no report for `report_id`. | Helper generates verifier share, but Leader rejects it as unknown `report_id`. | **No Leakage**: Report never enters aggregate. |
| **Replay / Duplicate Token** | Aggregator nullifier lookup detects duplicate token or `report_id`. | Aggregator rejects submission with `409 Conflict`. | **No Double-Counting**: Replay blocked. |
| **Tampered FLP Proof** | Polynomial check fails during `verifier_shares_to_message`. | Leader detects mathematical check failure. Prep message aborted; both aggregators drop the report. | **Integrity Preserved**: Malicious vector rejected. |
| **Aggregator Disagreement on Batch Count** | Collector queries both aggregators and finds `N_leader != N_helper`. | Collector halts unsharding with `CountMismatchError`. Aborts without calculating results. | **Safe Fail-Closed**: No corrupted histogram emitted. |
| **Network Partition During Prep** | Helper cannot reach Leader via mTLS. | Verification loop halts. Reports remain unaccumulated until connectivity restores or timeout purges batch. | **Safe Fail-Closed**: No partial release. |
| **Collector Crash** | Collector unavailable at scheduled epoch end. | Leader and Helper keep aggregate shares safely in memory/state until a valid authenticated collector reconnects. | **State Preserved**: Retried on recovery. |
