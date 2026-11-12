# Threat Model & Attack Surface Analysis

## 1. System Scope & Assets
* **Protected Assets:**
  * Raw individual benchmark values (e.g., exact individual salary or rate).
  * Linkability between participant identity (email, IP, account) and reported bucket.
  * Integrity of aggregate benchmark distributions (preventing artificial skewing or bias).

---

## 2. Threat Actors & Capabilities

| Threat Actor | Motivation | Capabilities |
|---|---|---|
| **Malicious Client** | Skew community metrics or discover survey results without sharing honest data. | Submits arbitrary payloads, out-of-bounds numbers, malformed proofs, or repeats submissions. |
| **Curious Aggregator (Honest-but-curious)** | Learn participant values while nominally executing the protocol. | Inspects inbound shares, network headers, execution state, and logs. |
| **Colluding Aggregators** | Complete compromise of participant privacy. | Pool their respective state and shares together. |
| **External Eavesdropper** | Intercept network communication. | Listens on public networks or ISP backbones. |
| **Disgruntled Participant / Observer** | Infer whether a specific high-profile individual earned an extreme amount. | Inspects finalized published aggregate histograms. |

---

## 3. Threat Scenarios & Mitigations

### 3.1. Arbitrary / Extreme Value Injection (The 10,000,000 Attack)
* **Threat:** A rogue client submits an out-of-range value (e.g. `10,000,000` or `-1`) to warp the mean/distribution.
* **Mitigation:**
  * **Client Layer:** Explicit range check ($0 \le \text{bucket} \le 19$).
  * **VDAF Cryptographic Layer:** Zero-knowledge FLP proofs (`Prio3Histogram`) mathematically verify that the submitted shares sum to an exact one-hot vector ($[0, \dots, 1, \dots, 0]$ with exactly one $1$). If a client attempts to forge a share with an inflated number, proof verification fails with `proof verifier check failed` and the share is rejected.

### 3.2. Single Aggregator Inspection (Honest-but-Curious)
* **Threat:** Aggregator A logs or inspects its database to read a user's salary.
* **Mitigation:**
  * Aggregator A only ever holds an additive share over finite field $\mathbb{F}_{p}$ where $p \approx 2^{128}$.
  * By information-theoretic secret sharing, a single share is indistinguishable from uniform random noise. Aggregator A gains zero bits of information regarding the bucket index.
  * Operational policy prohibits storing or logging request bodies or single-share uploads.

### 3.3. Collusion Between Aggregators
* **Threat:** Aggregator A and Aggregator B share private keys, databases, or share logs to decrypt individual inputs.
* **Mitigation:**
  * The protocol's privacy guarantee formally requires a **$1$-out-of-$2$ non-collusion assumption**.
  * Operational defense: The two aggregators must be owned, governed, and operated by distinct, legally and administratively independent entities (e.g., Community DAO/Trustee + Independent Academic/Auditing Organization) on separate cloud providers.
  * *Unresolved Risk:* If both aggregators conspire or are breached by the same state/attacker, privacy is completely broken.

### 3.4. Sybil & Ballot-Stuffing Attacks
* **Threat:** A user submits 1,000 valid one-hot shares to artificially make their desired salary bucket dominant.
* **Mitigation:**
  * Cryptographic validity checks cannot detect duplicate valid submissions with fresh nonces.
  * Campaign admission requires token-based gating (see [Privacy Model](privacy-model.md)). Single-use signed tokens or blind signatures must be presented to Aggregators and consumed in a nullifier set.

### 3.5. Small Group Inference & Reconstruction Attacks
* **Threat:** Only 2 people participate in a benchmark, or a single participant is in the $>\$200\text{k}$ bucket. Observers easily deduce individual numbers.
* **Mitigation:**
  * **$k$-Anonymity Gate:** The Collector refuses to unshard or publish any batch where $N < N_{\text{min}}$ (e.g. minimum cohort size $\ge 20$).
  * **Differential Privacy:** Add controlled Laplacian noise ($\epsilon, \delta$) to output bucket counts to ensure individual participation cannot be reverse-engineered from public output.

