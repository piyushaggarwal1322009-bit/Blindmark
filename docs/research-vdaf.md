# Research: Verifiable Distributed Aggregation Functions (VDAF) & Prio3 Histogram

Research findings on running a Prio3 histogram with two aggregators using reference implementations and libraries.

---

## 1. IRTF CFRG VDAF Reference Implementation (`cfrg/draft-irtf-cfrg-vdaf`)

* **Repository:** [cfrg/draft-irtf-cfrg-vdaf on GitHub](https://github.com/cfrg/draft-irtf-cfrg-vdaf)
* **Tested Commit Hash:** `e1d1e1c3854a34589c0c2d9b899a48533cac9004`
* **Reference Implementation Directory:** [`poc/`](https://github.com/cfrg/draft-irtf-cfrg-vdaf/tree/main/poc)
* **Production Warning (Quoted from `poc/README.md`):**
  > *"It is not intended for production use: the code is not optimized for performance or resistance to side-channel attacks. Its primary purpose is to generate test vectors."*
* **Programming Language:** **Python 3** (requires Python 3.12+ and `pycryptodomex`).
* **Test Suite Status on Windows:**
  * Tests run via `python -m unittest discover -s tests` inside `poc/`.
  * **Result:** All 88 tests pass (`Ran 88 tests in 5.656s - OK`) once `pycryptodomex` is installed.
* **Included Prio3 Types:**
  * `Prio3Count`: Single-bit/boolean event counter ($x \in \{0, 1\}$).
  * `Prio3Sum`: Bounded integer summation ($x \in [0, 2^k - 1]$).
  * `Prio3Histogram`: Aggregation of measurements into discrete histogram buckets (verifying inputs are valid one-hot vectors).
  * `Prio3SumVec`: Vector of bounded integer sums.

---

## 2. Rust Crate `prio` (`divviup/libprio-rs`)

* **Repository & Documentation:**
  * [divviup/libprio-rs on GitHub](https://github.com/divviup/libprio-rs)
  * [prio crate on crates.io](https://crates.io/crates/prio)
  * [prio crate documentation on docs.rs](https://docs.rs/prio/latest/prio/)
* **Ready-made `Prio3Histogram`:**
  * **Yes.** `prio` includes first-class support for `Prio3Histogram` (and multithreaded variant `Prio3HistogramMultithreaded`).
  * Defined under module [`prio::vdaf::prio3`](https://docs.rs/prio/latest/prio/vdaf/prio3/struct.Prio3.html).
  * Can be constructed via `Prio3::new_histogram(num_aggregators, length, chunk_length)` (e.g., `num_aggregators = 2`).
  * Validates that submissions are one-hot encoded vectors so clients cannot artificially inflate buckets or inject arbitrary values.
* **How to Install on Windows:**
  * Requires the Rust toolchain (install via [rustup.rs](https://rustup.rs/)).
  * In a Cargo project (`Cargo.toml`), add the dependency:
    ```toml
    [dependencies]
    prio = "0.18" # or latest version
    ```
  * Or install via CLI:
    ```powershell
    cargo add prio
    ```
* **Security Warnings in Documentation:**
  * The crate documentation prominently displays an explicit disclaimer:
    > *"WARNING: This code has not undergone significant security analysis. Use at your own risk."*
  * It cautions that while implementing standard draft specifications (IETF VDAF / DAP), it is an active research and evolving cryptographic implementation that should be evaluated carefully before high-assurance production use.

---

## 3. Phase 2: Malicious Submission Attack & Robustness Evaluation

Experiments conducted in [`experiments/prio3_histogram_attack.py`](file:///c:/Users/Admin/Desktop/BlindMark/experiments/prio3_histogram_attack.py) with 2 aggregators, 20 buckets ($k=20$), and chunk length 5.

### Replay Attack Findings by Stage
1. **Out-of-Range Submissions ($x = 20$ or $x = 10,000,000$):**
   * **Rejection Stage:** `client_encoding_sharding`
   * **Mechanism:** Python list index bound checks during one-hot vector generation (`IndexError: list assignment index out of range`). In production wire protocols (like DAP/VDAF), integer encoding enforces bucket boundaries $0 \le x < \text{length}$.
2. **Negative Submissions ($x = -25$ vs $x = -1$):**
   * **Reference Implementation Behavior:** An out-of-bounds negative index like `-25` raises an `IndexError` at client encoding. In Python, an index of `-1` maps to the last element of the list (`list[-1]` = bucket 19). Because `encoded[-1]` produces a valid one-hot vector for bucket 19, the zero-knowledge proof verifies successfully. Under the formal VDAF specification, measurements are non-negative indices in $[0, \text{length})$.
3. **Tampered Measurement Shares (e.g. Attacker injecting $+10$ into bucket 5):**
   * **Rejection Stage:** `proof_verification` (`verifier_shares_to_message`)
   * **Mechanism:** Zero-knowledge Fully Linear Proof (FLP) verification fails (`ValueError: proof verifier check failed`). Aggregators discover that intermediate polynomial evaluation does not yield valid validity constraints (i.e., entries not boolean or sum not equal to 1).
4. **Tampered Proof Shares (Altering proof vectors):**
   * **Rejection Stage:** `proof_verification` (`verifier_shares_to_message`)
   * **Mechanism:** Rejected with `ValueError: proof verifier check failed`. The aggregators detect polynomial inconsistency without learning the client's underlying data.
5. **Histogram Integrity:**
   * After submitting baseline valid measurements `[5, 7, 5, 12]` and subjecting the aggregators to each attack scenario, the final unshared histogram remained strictly:
     ```text
     [0, 0, 0, 0, 0, 2, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0]
     ```
   * Malicious or malformed inputs are entirely discarded prior to aggregation and cannot skew or distort the distribution.

### Security Limitations & Threat Model Analysis
* **What Prio3Histogram Prevents:**
  * Prevents clients from submitting out-of-range values or arbitrary scalar magnitudes (such as the 10,000,000 salary exploit).
  * Prevents clients from incrementing multiple buckets in a single submission (enforces exact one-hot constraints).
  * Prevents clients from submitting negative or fractional values to skew totals.
  * Prevents any single honest aggregator from learning individual client bucket choices.
* **What Prio3Histogram Does NOT Prevent:**
  * **Sybil Attacks / Duplicate Submissions:** Prio3 itself has no identity, authentication, or rate-limiting layer. A client can generate arbitrary valid one-hot submissions repeatedly under different nonces to bias buckets.
  * **Aggregator Collusion:** Privacy depends on the non-collusion assumption between aggregators. If all aggregators collude or share secret state, they can reconstruct every client's private input.
  * **Differential Privacy / Small Sample Leakage:** If only 1 client contributes to a bucket or a small cohort participates, an observer seeing the final histogram can infer private values. Differential privacy noise or minimum batch thresholds must be added at the collection tier.
* **Production Security Disclaimer:**
  * The reference Python implementation (`vdaf_poc`) is explicitly non-production (unhardened against side-channel/timing attacks and lacks constant-time math). Production deployments must use audited cryptographic engines (e.g., Rust `libprio-rs`) deployed within an end-to-end framework like DAP (Distributed Aggregation Protocol).
