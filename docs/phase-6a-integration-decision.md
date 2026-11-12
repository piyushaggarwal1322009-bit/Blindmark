# Blindmark — Phase 6A: Integration Options & Strategic Decision

**Document Status:** Architectural Evaluation & Decision Record (Refined with Janus Audit Status)

---

## 1. Evaluation of Architectural Options

| Evaluation Dimension | Option A: Custom Rust Service using `libprio-rs` | Option B: Pure Janus Deployment | Option C: Hybrid Architecture (FastAPI + Janus DAP Engine) |
| :--- | :--- | :--- | :--- |
| **Description** | Build custom Rust microservices implementing Prio3 math from scratch using `libprio-rs`. | Discard the FastAPI backend completely; rewrite all Blindmark community/campaign features inside Janus. | Retain FastAPI backend for metadata, campaigns, eligibility, and UI APIs. Deploy Janus instances for Leader/Helper DAP aggregation, with a dedicated Python/Rust collector worker. |
| **Protocol Compatibility** | **High Risk:** Must implement HPKE envelopes, multi-round aggregation wire messages, batching, and DAP drafts manually. High likelihood of subtle protocol bugs. | **High:** Implements exact IETF draft specs (`draft-ietf-ppm-dap-19/20`). | **High:** Uses standard Janus instances for cryptographic wire protocol, while FastAPI orchestrates campaigns and displays benchmarks. |
| **Implementation Complexity** | **Very High:** Re-inventing network protocols, batch state machines, and key exchanges that took years of standards work. | **High:** Janus is a low-level telemetry daemon; writing community portals, user dashboards, and role management in it would be difficult and cumbersome. | **Moderate & Modular:** Decoupled. Cryptography is handled by open-source Rust DAP software; product logic remains in clean Python/FastAPI. |
| **Maintenance Burden** | **Severe:** Maintaining custom cryptographic networking code. | **Moderate:** Tracking upstream Janus releases. | **Low / Manageable:** Upstream Janus updates are decoupled from Blindmark application schema and frontend features. |
| **Client Integration** | Custom client library required. | Uses existing `janus_client` / Wasm client bindings. | Uses `janus_client` compiled to Wasm or lightweight JavaScript DAP client. |
| **Audit Status & Operational Readiness** | Unaudited internal code. | **Experimental:** Upstream explicitly notes active development. Not an audited turn-key enterprise system, but the reference standard for DAP. | **Isolated Risk:** Cryptographic execution is contained within Janus; product business logic does not introduce home-grown crypto bugs. |

---

## 2. Selected Approach: Option C (Hybrid Architecture)

### Rationale
1. **Adherence to Core Principles:**
   *"Do not invent our own cryptographic protocol."* Option A violates this spirit by forcing us to implement transport, HPKE session handling, and aggregation scheduling from scratch.
2. **Leveraging Standards While Acknowledging Pre-Production Status:**
   Janus is actively maintained by Divvi Up / ISRG for DAP development. While upstream marks it as experimental, using standard DAP draft implementations is far safer than writing a bespoke two-party aggregation protocol.
3. **Preserving Completed Work:**
   Phases 4 and 5 established a database schema, Alembic migrations, token nullifier system, and hardened APIs. Option C preserves these investments without code rewrites. The Blindmark FastAPI backend provisions tasks in Janus and receives finalized benchmark results from the collector worker.

---

## 3. Deployment & Operational Topology

1. **Blindmark Core:** FastAPI API + PostgreSQL application database (Community, Campaign, Eligibility, Results).
2. **Leader Aggregator:** Containerized Janus instance connected to its own PostgreSQL database (operated by Blindmark).
3. **Helper Aggregator:** Containerized Janus instance connected to an independent PostgreSQL database (operated by an independent community partner or secondary privacy enclave).
4. **Collector Worker:** Background scheduled task that triggers DAP batch aggregation jobs via Janus Collector API once a campaign deadline arrives.
