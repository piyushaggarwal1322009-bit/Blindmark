# Blindmark Backend Architecture

## 1. Architectural Overview

Blindmark is designed as a privacy-preserving benchmarking system where users can compare compensation, metrics, and operational rates across cohorts without exposing individual raw data to any central service, peer, or single aggregator.

The system topology separates data ingestion, cryptographic validation, accumulation, and policy-gated result publication into four distinct roles:

```
               +--------------------------------------------+
               |                   Client                   |
               |  (Validates bucket, shards into reports)   |
               +----------------------+---------------------+
                                      |
                 +--------------------+--------------------+
                 | Public Share +                          | Public Share +
                 | Input Share A                           | Input Share B
                 v                                         v
       +-------------------+                     +-------------------+
       |   Aggregator A    | <=================> |   Aggregator B    |
       |  (Independent op) |    Verification     |  (Independent op) |
       |                   |  Round (Prep/State) |                   |
       |  Running Aggregate|                     |  Running Aggregate|
       |      Share A      |                     |      Share B      |
       +---------+---------+                     +---------+---------+
                 |                                         |
                 | Aggregate Share A                       | Aggregate Share B
                 | (Encrypted/Gated)                       | (Encrypted/Gated)
                 +--------------------+--------------------+
                                      |
                                      v
                        +---------------------------+
                        |         Collector         |
                        |   (Orchestrates rounds,   |
                        |    enforces k-anonymity,  |
                        |     reconstructs result)  |
                        +---------------------------+
```

---

## 2. Core Node Roles & Boundaries

### 2.1. Client
* **Execution Boundary:** Client environment (browser runtime via WebAssembly or client CLI).
* **Responsibilities:**
  1. Authenticates against the campaign authority to claim an authorized submission credential or token.
  2. Maps user raw metrics (e.g., $65,000 salary) to a discrete histogram bucket (e.g., bucket index $7 \in [0, 19]$).
  3. Enforces boundary validation ($0 \le \text{bucket} \le 19$).
  4. Generates an ephemeral report nonce and randomness.
  5. Uses the VDAF scheme (`Prio3Histogram`) to produce:
     * A public share (containing binding context and joint randomness shares).
     * Input Share $0$ (for Aggregator A).
     * Input Share $1$ (for Aggregator B).
     * Zero-Knowledge Fully Linear Proof (FLP) shares proving that the input is a valid one-hot vector.
  6. Sends encrypted shares directly to independent endpoints over TLS.
* **Strict Rule:** Raw client measurements are never transmitted over network boundaries or persisted anywhere on backend disks.

### 2.2. Aggregator A (Leader / Helper)
* **Execution Boundary:** Independent cloud infrastructure or hosting provider A.
* **Responsibilities:**
  1. Exposes authenticated submission ingestion (`POST /reports/upload`).
  2. Verifies the single-use submission token to prevent duplicate submissions.
  3. Holds `verify_init` state for each active report.
  4. Exchanges verifier messages with Aggregator B to run zero-knowledge proof verification.
  5. Upon valid verification, updates its running aggregate share $\text{AggShare}_A \leftarrow \text{AggShare}_A + \text{OutShare}_A$.
  6. Rejects and drops invalid or tampered reports immediately before aggregation.
* **State Retained:**
  * Only the running aggregate share vector (length = 20) and a counter of validated reports.
  * Ephemeral verification state is discarded as soon as the report round finishes.
  * Individual input shares, request bodies, or IP-to-share links are never logged or persisted.

### 2.3. Aggregator B (Helper)
* **Execution Boundary:** Independent infrastructure operated by a non-colluding second organization (e.g., independent community trustee, university, or separate cloud provider).
* **Responsibilities:**
  * Same cryptographic protocol responsibilities as Aggregator A.
  * Holds only `InputShare_B` and accumulates $\text{AggShare}_B$.
  * Ensures that even if Aggregator A's infrastructure is subpoenaed, hacked, or inspected, zero individual inputs can be reconstructed.

### 2.4. Collector
* **Execution Boundary:** Coordination layer / API service.
* **Responsibilities:**
  1. Orchestrates aggregation batches and epochs for active campaigns.
  2. Requests aggregate shares from both Aggregators once a collection epoch finishes.
  3. **Enforces Privacy Gates:**
     * Checks that the total count of aggregated valid reports meets the minimum sample threshold ($N \ge N_{\text{min}}$, e.g., $N \ge 20$).
     * If $N < N_{\text{min}}$, aggregation is rejected and aggregate shares are destroyed.
  4. Combines $\text{AggShare}_A$ and $\text{AggShare}_B$ via `unshard()`.
  5. Applies differential privacy perturbation noise (Laplace or Gaussian) to bucket counts before public presentation.
  6. Publishes the verified community histogram.

---

## 3. Campaign & Data Domain Entities

To support multiple communities and recurring compensation benchmarks, the application data model is structured as follows (relational schema):

```
+------------------+         +--------------------------+
|    Community     | 1     * |        Campaign          |
|------------------|---------|--------------------------|
| id (UUID, PK)    |         | id (UUID, PK)            |
| name             |         | community_id (FK)        |
| slug             |         | title                    |
| created_at       |         | category_id (FK)         |
+------------------+         | status (Draft/Open/Done) |
                             | min_cohort_size (int)    |
                             | bucket_config (JSONB)    |
                             | start_at, end_at         |
                             +------------+-------------+
                                          | 1
                                          |
                                          | *
                             +------------+-------------+
                             |    CampaignTokenRecord   |
                             |--------------------------|
                             | id (UUID, PK)            |
                             | campaign_id (FK)         |
                             | token_hash (SHA-256, UQ) |
                             | consumed_at (timestamp)  |
                             +--------------------------+
```

### Entity Specifications
1. **Community:** Represents an organization, cohort, alumni network, or discord guild (e.g., *"YC Founders 2026"*, *"EU Remote Rust Engineers"*).
2. **Campaign:** A benchmarking period bounded by time and minimum group size (e.g., *"Q4 2026 Senior Frontend Compensation"*).
   * Defines the 20 discrete bucket boundaries (e.g., `["<$50k", "$50k-$60k", ..., ">$240k"]`).
3. **Category:** Classification of benchmark (e.g., Full-Time Salary, Freelance Hourly Rate, Seed Equity).
4. **CampaignTokenRecord:** Tracks single-use cryptographic tokens to guarantee duplicate-submission prevention without linking user identity to any report.
5. **AggregationResult:** Stores the finalized output histogram `[0, 0, 2, 5, ...]` published after threshold and noise verification.

**Database Invariant:**
No table in the database contains `share_0`, `share_1`, raw scalar salary, or user identity references in the same record as submission payloads.

