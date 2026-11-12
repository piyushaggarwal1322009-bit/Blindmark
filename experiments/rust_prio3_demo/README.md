# Rust Prio3Histogram Prototype (`experiments/rust_prio3_demo`)

This directory contains the isolated Rust prototype demonstrating the 2-aggregator `Prio3Histogram` workflow for Blindmark, based on `prio = "0.18.1-alpha.3"` (the exact crate dependency powering Janus DAP).

---

## 1. Objective

Demonstrate:
1. Instantiation of `Prio3::new_histogram` with $K = 12$ buckets and chunk length $C = 4$.
2. Measurement encoding into discrete one-hot vectors and Fully Linear Proofs (FLPs) over the Mersenne 31 field (`Field31`).
3. 2-aggregator secret sharding (`vdaf.shard`).
4. Two-round inter-aggregator preparation and proof verification (`verify_init`, `verify_shares_to_message`, `verify_next`).
5. Output share accumulation (`vdaf.aggregate`).
6. Final aggregate unsharing (`vdaf.unshard`).

---

## 2. Prerequisites & Build Instructions

### Toolchain Prerequisites
- **Rust:** `1.99.1+` (or standard stable).
- **Linker:** Windows requires either Microsoft C++ Build Tools (`link.exe`) for `x86_64-pc-windows-msvc`, or MinGW GCC for `x86_64-pc-windows-gnu`.
- **Disk Space Requirement:** At least 2.5 GB of free disk space on drive `C:` to accommodate intermediate crate build artifacts.

### Running the Prototype
```powershell
cargo check
cargo test
cargo run
```

