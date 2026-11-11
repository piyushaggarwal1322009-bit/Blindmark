# Blindmark — Phase 6A: VDAF Research & Protocol Analysis

**Document Status:** Research & Architecture Specification  
**Scope:** VDAF (draft-irtf-cfrg-vdaf), DAP (draft-ietf-ppm-dap), Rust `libprio-rs`, Divvi Up `janus`, and Cloudflare `daphne`.

---

## 1. Audit of Existing Blindmark Experiments

### What the Original Experiments Proved
1. **`experiments/prio3_histogram_demo.py`:**
   - Proved that the 2-party additive secret sharing and Fully Linear Proof (FLP) verification in `Prio3Histogram` functions mathematically across discrete measurement inputs.
   - Verified that a client encoding measurements into a 1-hot vector across $N=20$ buckets correctly unshares to exact bucket frequencies ($[0, \dots, 2, \dots, 1, \dots, 1, \dots]$) when processed by two aggregators.
2. **`experiments/prio3_histogram_attack.py`:**
   - Proved that out-of-range bucket selections (e.g. bucket 20 for a 20-bucket histogram, negative buckets) are caught at client encoding or lead to invalid proof states.
   - Proved that tampering with cryptographic proof shares or polynomial shares results in rejection during aggregator preparation/verification (`verify_and_prep`), preventing manipulated submissions (like our Phase 1 toy attack of `10,000,000`) from affecting aggregate totals.

### What the Original Experiments Did NOT Prove
- **No Network Protocol:** The experiments executed in-memory within a single Python script. They did not test network transmission, wire serialization, or transport protocols.
- **Python Reference Is Non-Production:** The CFRG Python reference repository explicitly states:
  > *"This repository contains a reference implementation of VDAFs... It is not intended for production use. It is neither fast nor secure against side-channel attacks."*
- **No Transport or Replay Defense:** The Python POC lacks Hybrid Public Key Encryption (HPKE) report envelopes, task authorization, or aggregator-to-aggregator state synchronization.

### Reusable vs. Replaceable Components
| Component | Status | Action |
| :--- | :--- | :--- |
| `experiments/*.py` | Reference POC | **Retain as historical test fixtures.** Do not use in production runtime. |
| `app/models.py` & PostgreSQL schema | Foundation | **Reusable.** Community and Campaign metadata models directly map to DAP tasks. |
| `app/services/token_service.py` | Scaffold | **Replace/Upgrade.** The current single-server nullifier token must transition to DAP report replay tracking and/or client-side anonymous credential issuance. |
| `app/services/vdaf_interface.py` | Abstract Interface | **Upgrade.** Maps directly to DAP aggregator API endpoints. |
| `app/services/release_policy.py` | Threshold Policy | **Reusable.** Must gate DAP collector query releases behind $N \ge \text{minimum\_participants}$. |

---

## 2. Protocol Analysis: VDAF vs. DAP

### 2.1 Verifiable Distributed Aggregation Functions (VDAF)
- **Specification:** [draft-irtf-cfrg-vdaf-22](https://datatracker.ietf.org/doc/draft-irtf-cfrg-vdaf/) (IRTF CFRG)
- **Role:** Pure cryptographic layer (zero-knowledge proofs, secret sharing, finite-field arithmetic).
- **Core Abstractions:**
  - `shard(measurement)` $\to$ `(public_share, [input_share_1, input_share_2])`
  - `prep(aggregator_id, input_share, ...)` $\to$ `(prep_state, prep_msg)`
  - `prep_finish(prep_state, prep_shares)` $\to$ `aggregate_share`
  - `unshard(agg_param, [agg_share_1, agg_share_2])` $\to$ `aggregate_result`

### 2.2 Distributed Aggregation Protocol (DAP)
- **Specification:** [draft-ietf-ppm-dap-20](https://datatracker.ietf.org/doc/draft-ietf-ppm-dap/) (IETF PPM Working Group)
- **Role:** End-to-end distributed system and wire protocol that operationalizes VDAFs across the internet.
- **What DAP Adds on Top of VDAF:**
  1. **HPKE Envelope Encryption:** Client encrypts `input_share_1` with Leader's public key and `input_share_2` with Helper's public key using Hybrid Public Key Encryption (RFC 9180). Neither aggregator can read the other's share.
  2. **Task Coordination:** Formalizes `task_id`, query types, epoch windows, and aggregation parameters.
  3. **Aggregator Protocol:** HTTP endpoints over TLS for the Leader to drive the multi-round aggregation jobs with the Helper (`/aggregate`, `/aggregate_shares`).
  4. **Collector Protocol:** Collector requests batch results from Leader and Helper, decrypts or combines partial aggregate shares, and receives the final metric.
  5. **Replay & Batching Protection:** Strict report deduplication within batch windows to prevent malicious clients from submitting identical shares multiple times.

---

## 3. Protocol Implementation Repositories

| Repository | Current Version | Draft Targets | License | Architecture & Limitations |
| :--- | :--- | :--- | :--- | :--- |
| **`divviup/libprio-rs`** | Latest Git / Crates.io | `draft-irtf-cfrg-vdaf-22` | MPL-2.0 | Pure Rust cryptographic library. Implements `Prio3Histogram`, `Prio3Sum`, `Prio3Count`. Production-grade speed; constant-time operations; no network or server code. |
| **`divviup/janus`** | Active (Release branches track draft revisions) | `draft-ietf-ppm-dap-19/20` | MPL-2.0 | Full production DAP implementation in Rust by Divvi Up / ISRG. Modular crates (`janus_aggregator`, `janus_client`, `janus_collector`, `janus_core`). Requires PostgreSQL backend per aggregator. Native support for `Prio3Histogram`. |
| **`cloudflare/daphne`** | Active | `draft-ietf-ppm-dap-19/20` | BSD-3-Clause | DAP implementation designed for Cloudflare Workers / serverless runtime. Excellent, but tightly coupled to Cloudflare ecosystem primitives (KV/D1). |

---

## 4. Conceptual Mechanics of Prio3Histogram

```
[ Client ]
   |  Measurement: Bucket Index (e.g., 3 out of 10)
   |  Vector: [0, 0, 0, 1, 0, 0, 0, 0, 0, 0] (One-Hot)
   |  Generates FLP Proof that vector has exactly one '1' and rest '0's
   |  Splits vector & proof into 2 additive shares over finite field GF(2^31 - 1)
   |  Encrypts Share A with Leader HPKE Key, Share B with Helper HPKE Key
   |
   +---------------------------------------+
   | (Leader Report Share)                 | (Helper Report Share)
   v                                       v
[ Leader Aggregator ]                   [ Helper Aggregator ]
   |                                       |
   |<====== Round 1: Verify Proofs =======>| (Verify FLP validity without learning inputs)
   |                                       |
   | Accumulates Share A                   | Accumulates Share B
   |                                       |
   +-------------------+-------------------+
                       |
                       v
             [ Collector / Blindmark ]
                       |
                       +---> Unshard: Share A + Share B = Final Histogram
```

1. **Client Sharding:** The client selects one discrete bucket $b \in [0, K-1]$. It encodes $b$ as a length-$K$ vector $V$ where $V[b] = 1$ and $V[i] = 0$ for $i \ne b$. It generates an algebraic proof asserting:
   $$\sum_{i=0}^{K-1} V[i] = 1 \quad \text{and} \quad V[i]^2 - V[i] = 0 \; \forall i$$
2. **Proof Verification:** The Leader and Helper verify the proof interactively over finite fields. If a client attempts to submit an out-of-range bucket, multiple 1s, or a scalar like $10,000,000$, the polynomial equality fails and the report is discarded prior to aggregation.
3. **Additive Unsharing:** When aggregators add valid shares across $M$ participants, the aggregate vectors satisfy:
   $$A_{\text{final}} = \sum_{j=1}^M S_{A}^{(j)} + \sum_{j=1}^M S_{B}^{(j)} = \sum_{j=1}^M V^{(j)}$$
   Each bucket contains the exact count of participants whose measurement landed in that bucket.
