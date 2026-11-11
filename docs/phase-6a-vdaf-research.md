# Blindmark — Phase 6A: VDAF Research & Protocol Analysis

**Document Status:** Research & Specification Analysis (Updated with Precise Version Findings)  
**Scope:** VDAF (`draft-irtf-cfrg-vdaf`), DAP (`draft-ietf-ppm-dap`), Rust `libprio-rs`, Divvi Up `janus`.

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

---

## 2. Protocol & Implementation Compatibility Matrix

| Component | Repository | Tested / Target Version | Upstream Draft Standards | Status & Limitations |
| :--- | :--- | :--- | :--- | :--- |
| **VDAF Cryptographic Core** | `divviup/libprio-rs` (Crate `prio`) | `0.18.1-alpha.3` | `draft-irtf-cfrg-vdaf-11` through `-22` | **Active, Pre-Release Alpha.** Native support for `Prio3Histogram`, `Prio3Sum`, `Prio3Count`. MPL-2.0 license. Does not have a formal third-party audit report linked in repo; side-channel risks on non-x86_64 require attention. |
| **DAP Aggregator & Engine** | `divviup/janus` | Workspace `0.8+` (branch `release/0.8` or `main`) | `draft-ietf-ppm-dap-19/20` | **Experimental / Active Development.** The README explicitly states: *"Experimental implementation of the DAP specification... currently in active development."* Supports VDAFs with trivial aggregation parameters (`Prio3`). Does NOT support nontrivial aggregation parameters (`Mastic`). |
| **DAP Client** | `janus_client` | Sub-crate in `janus` | `draft-ietf-ppm-dap-19/20` | Generates HPKE encrypted reports. Suitable for CLI and integration harnesses; browser clients typically require Wasm compilation. |
| **DAP Collector** | `janus_collector` | Sub-crate in `janus` | `draft-ietf-ppm-dap-19/20` | Creates collection jobs on the Leader, retrieves Leader and Helper aggregate shares from Leader, decrypts, and unshares. |

---

## 3. Detailed DAP Workflow Mechanics

In the official DAP specification (`draft-ietf-ppm-dap`):
1. **Client to Leader:** Client produces `Report` with `encrypted_input_share_leader` and `encrypted_input_share_helper`, sending it to `POST /tasks/{task_id}/reports` on the Leader.
2. **Leader to Helper:** Leader forwards the Helper's encrypted share during the `POST /tasks/{task_id}/aggregate` preparation request.
3. **Helper to Leader:** Helper decrypts its share, computes the FLP proof step, and returns a preparation message.
4. **Leader to Helper:** Leader finishes FLP verification, computes state, and exchanges round confirmation. Both aggregators update their local partial batch shares.
5. **Collector to Leader:** When a batch window completes, Collector issues `POST /tasks/{task_id}/collection_jobs` to the Leader.
6. **Leader to Helper:** Leader requests the Helper's encrypted aggregate share via `POST /tasks/{task_id}/aggregate_shares`. Helper validates the batch checksum, encrypts its aggregate share with the Collector's public key, and returns it to the Leader.
7. **Leader to Collector:** Leader packages its own encrypted aggregate share and the Helper's encrypted aggregate share and returns them in the collection job response.
8. **Collector Unshards:** Collector decrypts both aggregate shares using its private key, adds them over the finite field, and recovers the plaintext histogram.
