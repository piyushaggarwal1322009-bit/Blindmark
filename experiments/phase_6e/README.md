# Blindmark Phase 6E: Rust Aggregator HTTP Services

Experimental Rust HTTP service layer for Blindmark's Prio3Histogram aggregators.

## Overview
This crate packages the verified Leader and Helper components from `experiments/phase_6c` into standalone HTTP services with strict parameter validation and aggregator identity enforcement.

## Services
- **Leader Service**: Aggregator 0 (Default Port: `8001`)
- **Helper Service**: Aggregator 1 (Default Port: `8002`)
- **Collector Service**: Unshared result retrieval interface.

## Quick Start

### 1. Run Tests
```powershell
cargo check --offline --all-targets
cargo test --offline --no-fail-fast
```

### 2. Start Leader Service (Port 8001)
```powershell
cargo run --bin blindmark-phase-6e -- leader
```

### 3. Start Helper Service (Port 8002)
```powershell
cargo run --bin blindmark-phase-6e -- helper
```

### 4. Health Check
```powershell
curl http://localhost:8001/health
curl http://localhost:8002/health
```

## Security Notice
This is an experimental engineering milestone for local interface testing. It is **NOT** a production privacy system. Running both services on the same machine does not provide non-collusion protection.

