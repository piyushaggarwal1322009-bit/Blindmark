# Phase 6C Threat Model & Security Boundaries

## 1. Scope & Status

Phase 6C implements an **isolated logical protocol prototype** in Rust. It proves that the cryptographic flow of Prio3Histogram can be decomposed into four distinct communicating entities.

> **CRITICAL NOTICE**: This implementation is NOT a production-ready system. It demonstrates protocol correctness and interface boundaries. It DOES NOT provide production privacy or security guarantees.

---

## 2. Threat Analysis & Boundaries

### 2.1 Aggregator Independence & Collusion
- **Current Limitation**: Both Leader and Helper execute within the same process space in this demonstration.
- **Threat**: In a real deployment, if Leader and Helper are controlled by the same organization, administrator, or infrastructure boundary, they can pool input shares `input_share_0 + input_share_1` and reconstruct each individual user's exact measurement.
- **Requirement for Production**: Leader and Helper must be operated by strictly independent, legally and operationally non-colluding organizations (e.g., University Career Center + Independent Privacy Foundation / Cloud Operator).

### 2.2 Transport Security
- **Current Limitation**: Communication is implemented via an in-memory queue (`TransportHub`).
- **Threat**: There is no encryption in transit, mutual TLS, endpoint authentication, or replay protection.
- **Requirement for Production**: Production DAP requires mTLS between aggregators, signed tokens, and TLS 1.3 encryption between clients and aggregators.

### 2.3 Proof Verification vs. Ground Truth
- **Guarantee Provided**: Prio3's zero-knowledge Fully Linear Proof (FLP) mathematically guarantees that a submitted measurement is a valid 1-hot vector of length 12 over `Field128` (i.e. exactly one bucket is incremented by 1, and no other entries exist).
- **Limitation**: The proof verifies *valid formatting within bounds*, not *real-world empirical truth*. A student submitting an exaggerated or false stipend that falls within a valid bucket will still be accepted.

### 2.4 Differential Privacy (DP)
- **Current Limitation**: Phase 6C does not add aggregator-side noise or differential privacy mechanisms (`add_noise_to_agg_share`).
- **Threat**: If a bucket has only 1 participant, or if differential queries are executed across successive epochs, an observer can infer individual salaries.
- **Requirement for Production**: Discrete Laplace or Gaussian noise addition prior to collector release.

### 2.5 Sybil & Replay Resistance
- **Guarantee Provided**: Report IDs are checked for duplicate submission within a single run.
- **Limitation**: There is no client authentication, rate limiting, student credential validation, or anonymous credential issuance (e.g. Privacy Pass / Blind Signatures). An adversary could submit millions of fake reports.

---

## 3. Security Boundary Summary Table

| Security Property | Phase 6C Status | Production Requirement |
|---|---|---|
| Single-Share Confidentiality | **Enforced** (Leader only gets share 0, Helper share 1) | Maintained over encrypted network channels |
| Non-collusion Domain Isolation | **Logical Only** (Same binary/process) | Distinct cloud tenants / legal entities |
| Input Format Correctness | **Cryptographically Verified** via FLP (Prio3) | Cryptographically Verified via FLP (Prio3) |
| In-Transit Confidentiality | **None** (In-memory transport) | mTLS / TLS 1.3 |
| Sybil Resistance | **None** | Anonymous credential tokens / SSO verification |
| Differential Privacy | **None** | Discrete noise mechanism |
| Replay Protection | **Basic ReportId tracking** | Epoch-based anti-replay database |

