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
