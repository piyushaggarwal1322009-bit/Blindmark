# Phase 6D: Corrected Rust–FastAPI Integration Architecture

## 1. Executive Summary & The Privacy Flaw Correction

### The Identified Flaw
In the preliminary draft of Phase 6D, the participant client submitted both `InputShare_0` (Leader share) and `InputShare_1` (Helper share) to FastAPI, which then forwarded them to the aggregators. 

**Why this was a critical flaw**:
Whoever operates or accesses the FastAPI backend (or its database/logs) could observe both secret shares for the same report ID and reconstruct the student's exact private stipend:
$$\text{Measurement} = \text{InputShare}_0 + \text{InputShare}_1 \pmod p$$
Even if FastAPI promised not to combine them, holding both shares in memory or at a single reverse proxy represents a single point of failure that destroys the non-collusion guarantee.

---

### The Corrected Privacy-Preserving Architecture
In the corrected architecture:
1. **FastAPI NEVER sees both input shares.**
2. **Direct Aggregator Ingestion (Primary Architectural Model)**:
   - The participant's client device contacts FastAPI **only** for identity verification and to obtain an anonymous, single-use eligibility credential (blind token / nullifier).
   - The participant's client then connects **directly** to the **Leader Aggregator** over HTTPS and submits `(PublicShare, InputShare_0, Credential)`.
   - The participant's client connects **directly** to the **Helper Aggregator** over HTTPS and submits `(PublicShare, InputShare_1, Credential)`.
3. **Encrypted Relay Option (Alternative for Restricted Networks)**:
   - If clients cannot reach both aggregator IP addresses directly (e.g., enterprise firewalls), the client encrypts `InputShare_0` using the Leader's public key (HPKE) and `InputShare_1` using the Helper's public key (HPKE).
   - FastAPI can relay the encrypted blobs, but **cannot decrypt either share**.
4. **Decoupled Identity vs. Submission**:
   - The aggregators never receive the participant's name, email, student ID, or session token. They only verify cryptographic nullifiers and VDAF validity.

---

## 2. Global Architecture Diagram (Corrected)

```
                                  +------------------------------------+
                                  |        PARTICIPANT / CLIENT        |
                                  |   (Browser / Mobile Device / CLI)  |
                                  +------------------------------------+
                                     /             |              \
               1. Prove Eligibility /              |               \
             & Receive Anonymous   /               |                \
               Credential Token   /   2. Direct    |     3. Direct   \
                                 /    Leader Share |     Helper Share \
                                v     (Share 0)    |     (Share 1)     v
+------------------------------------+             |             +------------------------------------+
|       FASTAPI CONTROL PLANE        |             |             |          HELPER AGGREGATOR         |
|                                    |             |             |            (Rust Engine)           |
|  - Manages Communities & Campaigns |             |             |                                    |
|  - Verifies Student Identity       |             |             |  - Independent Organization/Cloud  |
|  - Issues Single-Use Blind Tokens  |             |             |  - Evaluates FLP ZK Proofs         |
|  - Never Sees Input Shares         |             |             |  - Receives ONLY InputShare_1      |
|  - Enforces Public Release Policy  |             |             |  - Independent Verify Key          |
+------------------------------------+             |             |  - Maintains Own AggregateShare    |
                 |                                 |             +------------------------------------+
                 | (Query Metadata Only)           |                                ^
                 v                                 v                                |
+----------------------------------------------------+                              |
|                  LEADER AGGREGATOR                 |                              |
|                    (Rust Engine)                   |<=============================+
|                                                    |     4. Internal mTLS Prep Loop
|  - Independent Organization/Cloud                  |        (Exchanges Verifier Shares)
|  - Evaluates FLP ZK Proofs                         |
|  - Receives ONLY InputShare_0                      |
|  - Verifies Single-Use Nullifier Token             |
|  - Maintains Own AggregateShare                    |
+----------------------------------------------------+
                 \                                   /
                  \                                 /
             Leader\                               /Helper
             Share  \                             / Share
                     v                           v
                   +-------------------------------+
                   |           COLLECTOR           |
                   |        (Rust / Service)       |
                   |  - Verifies Report Count N    |
                   |  - Executes vdaf.unshard()    |
                   +-------------------------------+
                                   |
                                   | 5. Reconstructed Raw Histogram
                                   v
                   +-------------------------------+
                   |     FASTAPI RELEASE POLICY    |
                   |  - Minimum Threshold Gate     |
                   |  - Differential Privacy Hook  |
                   |  - Public Result Presentation |
                   +-------------------------------+
```

---

## 3. End-to-End Sequence Diagram

```mermaid
sequenceDiagram
    autonumber
    participant Client as Participant Client (App)
    participant API as FastAPI Backend (Control Plane)
    participant Leader as Leader Aggregator (Independent Domain)
    participant Helper as Helper Aggregator (Independent Domain)
    participant Collector as Collector Component

    Note over Client,API: Phase 1: Anonymous Credential Issuance
    Client->>API: POST /api/campaigns/{id}/tokens/issue (Authenticated Student)
    Note over API: Checks eligibility list & issues blind/nullifier token
    API-->>Client: Nullifier Token (T_nullifier)
    Note over API: Participant identity is now decoupled from the token

    Note over Client: Phase 2: Client-Side Sharding
    Note over Client: Shards stipend into PublicShare, InputShare_0, InputShare_1

    Note over Client,Helper: Phase 3: Direct Independent Ingestion
    Client->>Leader: POST /tasks/{id}/reports/0 (T_nullifier, Nonce, PublicShare, InputShare_0)
    Client->>Helper: POST /tasks/{id}/reports/1 (T_nullifier, Nonce, PublicShare, InputShare_1)

    Note over Leader: Validates T_nullifier not seen before; verify_init(Share_0)
    Note over Helper: Validates T_nullifier not seen before; verify_init(Share_1)

    Note over Leader,Helper: Phase 4: Inter-Aggregator Verification Loop
    Helper->>Leader: POST /tasks/{id}/prepare/share (VerifierShare_1)
    Note over Leader: verifier_shares_to_message([v_0, v_1]) -> CombinedMsg
    Note over Leader: verify_next(State_0, CombinedMsg) -> OutShare_0
    Note over Leader: agg_share_0.accumulate(OutShare_0)

    Leader->>Helper: POST /tasks/{id}/prepare/broadcast (CombinedMsg)
    Note over Helper: verify_next(State_1, CombinedMsg) -> OutShare_1
    Note over Helper: agg_share_1.accumulate(OutShare_1)

    Note over API,Collector: Phase 5: Collection & Public Release
    API->>Collector: Trigger Collection for Campaign
    Collector->>Leader: GET /tasks/{id}/aggregate-share
    Collector->>Helper: GET /tasks/{id}/aggregate-share
    Leader-->>Collector: AggregateShare_0 (Report Count N)
    Helper-->>Collector: AggregateShare_1 (Report Count N)
    Note over Collector: vdaf.unshard(Share_0, Share_1, N) -> Raw Histogram Vector
    Collector->>API: POST /api/campaigns/{id}/results/evaluate (Raw Histogram, Count N)
    Note over API: Evaluates DevelopmentReleasePolicy (Minimum Cohort Check)
    API-->>Client: GET /api/campaigns/{id}/results (Published Anonymized Benchmark)
```

---

## 4. Trust Domain Reality Check

> **CRITICAL REALITY CHECK**:
> Splitting software into separate processes (e.g. `leader.exe` on port 8001 and `helper.exe` on port 8002) **DOES NOT automatically establish independent trust domains** if:
> - Both services are deployed on the same virtual machine or Kubernetes cluster.
> - Both services are owned by the same organization.
> - Both services share the same database, disk, or root system administrator.
>
> In production, the Leader and Helper MUST be operated by **two distinct legal entities** (e.g., University Career Center + Independent Privacy Foundation / Cloud Operator).
