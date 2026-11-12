# Blindmark — Phase 6A: Cryptographic Architecture & System Design

**Document Status:** Architecture Design (Validated & Protocol Corrected)  
**Application Target:** Internship Stipend & Compensation Benchmarking  
**Protocols:** VDAF (`draft-irtf-cfrg-vdaf`), DAP (`draft-ietf-ppm-dap`)

---

## 1. Initial Benchmark Measurement Model (Internship Stipends)

### 1.1 Provisional Fixed Histogram Bucket Configuration
For the initial benchmark, Blindmark establishes a 12-bucket monthly stipend histogram (USD equivalent):

| Bucket Index | Monthly Stipend Range | Hourly Equivalent (40h/wk) |
| :---: | :---: | :---: |
| **0** | $< \$2,000$ | $< \$12.50 / \text{hr}$ |
| **1** | $\$2,000 - \$3,499$ | $\$12.50 - \$21.87 / \text{hr}$ |
| **2** | $\$3,500 - \$4,999$ | $\$21.88 - \$31.24 / \text{hr}$ |
| **3** | $\$5,000 - \$6,499$ | $\$31.25 - \$40.62 / \text{hr}$ |
| **4** | $\$6,500 - \$7,999$ | $\$40.63 - \$49.99 / \text{hr}$ |
| **5** | $\$8,000 - \$9,499$ | $\$50.00 - \$59.37 / \text{hr}$ |
| **6** | $\$9,500 - \$10,999$ | $\$59.38 - \$68.74 / \text{hr}$ |
| **7** | $\$11,000 - \$12,499$ | $\$68.75 - \$78.12 / \text{hr}$ |
| **8** | $\$12,500 - \$13,999$ | $\$78.13 - \$87.49 / \text{hr}$ |
| **9** | $\$14,000 - \$15,999$ | $\$87.50 - \$99.99 / \text{hr}$ |
| **10** | $\$16,000 - \$19,999$ | $\$100.00 - \$124.99 / \text{hr}$ |
| **11** | $\ge \$20,000$ | $\ge \$125.00 / \text{hr}$ |

### 1.2 Measurement Parameters & Constraints
- **VDAF Scheme:** `Prio3Histogram`
- **Finite Field:** $p = 2^{31} - 1$ (Mersenne 31 prime field `prio::field::Field31`).
- **Input Encoding:** One-hot vector of length $K = 12$.
- **Chunk Length:** Default parallel sum gadget chunk size $C = 4$.
- **Why Bucket Configuration Must Be Fixed Per Task:**
  In VDAF/Prio3, the proof verification polynomial structure and finite-field vector dimensions depend strictly on $K$. Aggregators cannot add shares from tasks with different bucket definitions or dynamic scales.
- **Why Currency and Geography Must Be Chosen Before Launch:**
  VDAF operates on abstract integers (one-hot vectors). It has no concept of currency conversion or location adjustment. Blending €3,000 with $3,000 or London stipends with Bangalore stipends produces invalid statistics. Every task must be pegged to an explicit currency, payment interval (monthly/hourly), and geographic market.
- **Validity vs. Truthfulness:**
  Zero-knowledge proof verification proves **only** that the submitted vector is a valid 1-hot vector within the discrete range $[0, 11]$. It does **NOT** prove that the user earned that stipend in the real world. A malicious client can submit a valid 1-hot report claiming $\$20,000/\text{mo}$ while earning minimum wage. Preventing false reporting requires community eligibility verification and social gating, not cryptography alone.
- **Cohort Thresholds vs. Differential Privacy:**
  Enforcing $M \ge 20$ (k-anonymity gate) ensures an aggregate is not published for small groups, but does **NOT** prevent differencing attacks across overlapping campaigns. Differential privacy requires injecting calibrated mathematical noise (Laplace/Gaussian) into the unshared distribution.

---

## 2. Corrected Target System Architecture (Standard DAP Protocol Flow)

In compliance with the official IETF Distributed Aggregation Protocol (`draft-ietf-ppm-dap`), the Collector communicates **exclusively with the Leader** during collection jobs. The Collector never opens direct connections to the Helper; instead, the Leader coordinates aggregate share requests from the Helper and returns both encrypted shares to the Collector.

```mermaid
sequenceDiagram
    autonumber
    actor Participant as Participant Client
    participant Backend as Blindmark FastAPI
    participant Leader as Leader Aggregator (Janus)
    participant Helper as Helper Aggregator (Janus)
    participant Collector as Blindmark Collector

    %% 1. Task Setup
    Note over Backend,Helper: 1. Campaign & Task Provisioning
    Backend->>Leader: Provision Task (Task ID, K=12, Prio3Histogram, Collector PubKey)
    Backend->>Helper: Provision Task (Task ID, K=12, Prio3Histogram, Collector PubKey)
    Participant->>Backend: Check Eligibility & Fetch Task Config + Aggregator HPKE Keys

    %% 2. Sharding & Submission
    Note over Participant,Leader: 2. Client Sharding & Report Upload
    Participant->>Participant: Map Stipend -> Bucket (e.g. 5)<br/>Create 1-Hot Vector & FLP Proof<br/>Split into Share A + Share B<br/>HPKE-encrypt Share A (Leader) & Share B (Helper)
    Participant->>Leader: POST /tasks/{task_id}/reports (Report with Encrypted Share A & B)

    %% 3. Aggregation Protocol
    Note over Leader,Helper: 3. Inter-Aggregator Verification (MPC)
    Leader->>Helper: POST /tasks/{task_id}/aggregate (Forward Encrypted Share B & Public Share)
    Helper->>Helper: Decrypt Share B, Compute FLP Verification State
    Helper-->>Leader: Return Preparation Message
    Leader->>Leader: Verify FLP Proof & Accumulate Valid Share A
    Helper->>Helper: Accumulate Valid Share B

    %% 4. Collection Protocol (Standard DAP Flow)
    Note over Collector,Helper: 4. Collection Job Execution (Via Leader)
    Collector->>Leader: POST /tasks/{task_id}/collection_jobs (Query Batch)
    Leader->>Helper: POST /tasks/{task_id}/aggregate_shares (Batch Selector & Checksum)
    Helper->>Helper: Verify Batch & Encrypt Aggregate Share B (to Collector PubKey)
    Helper-->>Leader: Return helper_encrypted_agg_share
    Leader->>Leader: Encrypt Aggregate Share A (to Collector PubKey)
    Leader-->>Collector: Return Collection Response (Leader & Helper Encrypted Shares)

    %% 5. Unsharding & Publication
    Note over Collector,Backend: 5. Unsharding & Benchmark Release
    Collector->>Collector: Check N >= 20 Threshold<br/>Decrypt Share A + Share B<br/>Combine over Field31: H = Share A + Share B
    Collector->>Backend: POST /api/campaigns/{id}/results/evaluate (with X-Service-Key)
    Backend->>Backend: Store Verified Histogram & Present Benchmark to Community
```

---

## 3. Tier Responsibilities

### A. Client Tier (Browser / Desktop Application)
- Computes local stipend bracket $b \in [0, 11]$.
- Constructs length-12 one-hot vector and Prio3 proof.
- Creates DAP Report envelope:
  - `encrypted_input_share_leader` (encrypted under Leader HPKE public key).
  - `encrypted_input_share_helper` (encrypted under Helper HPKE public key).
- Sends full report directly to the **Leader**.

### B. Leader Aggregator Tier (Janus Instance 1)
- Ingests incoming DAP reports.
- Forwards `encrypted_input_share_helper` to Helper over TLS.
- Exchanges preparation messages with Helper to verify FLP proofs without learning measurement values.
- Accumulates verified Share A into its batch state.
- Receives collection job requests from Collector, fetches encrypted Share B from Helper, packages both encrypted shares, and returns them to the Collector.

### C. Helper Aggregator Tier (Janus Instance 2)
- Receives report shares forwarded by Leader.
- Decrypts Share B using private HPKE key.
- Verifies FLP proof consistency with Leader.
- Accumulates verified Share B into its batch state.
- Upon receiving `aggregate_shares` request with matching checksum from Leader, encrypts its accumulated aggregate share under the **Collector's public key** and returns it to the Leader.

### D. Collector Tier (Blindmark Collector Engine)
- Initiates collection jobs through the Leader.
- Receives the paired encrypted aggregate shares from Leader.
- Decrypts both aggregate shares using its private key.
- Verifies cohort threshold ($M \ge 20$).
- Executes VDAF `unshard` to recover final histogram frequencies.
- Hands the verified distribution to the Blindmark backend via authenticated endpoint.

### E. Blindmark Application Backend (FastAPI)
- Manages organizational accounts, communities, campaigns, and participant whitelists.
- Exposes metadata to clients and provisions DAP task definitions in aggregators.
- Ingests approved histogram benchmarks for display to authorized community members.
