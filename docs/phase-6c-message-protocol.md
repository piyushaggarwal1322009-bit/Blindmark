# Phase 6C Message Protocol Specification

## 1. Protocol Architecture & Transports

The Phase 6C message protocol standardizes the serialization, routing, and data boundaries of messages passed between `Client`, `Leader`, `Helper`, and `Collector`.

In Phase 6C, messages are transmitted via an in-memory transport abstraction (`TransportHub` in `transport.rs`). Each message uses explicit byte buffers (`Vec<u8>`) for cryptographic structures serialized with `prio`'s standardized `Encode` / `ParameterizedDecode` traits.

---

## 2. Message Inventory

### 2.1 `ClientReport`
- **Sender**: `Client`
- **Recipient**: `Leader` (when `target == AggregatorId::Leader`) or `Helper` (when `target == AggregatorId::Helper`)
- **Purpose**: Transmits a single participant's sharded contribution to an aggregator.
- **Fields**:
  - `report_id`: Unique string identifier (`ReportId`).
  - `target`: `AggregatorId` enum (`Leader = 0`, `Helper = 1`).
  - `nonce`: `[u8; 16]` cryptographic nonce.
  - `public_share_encoded`: `Vec<u8>` serialized `Prio3PublicShare<32>`.
  - `input_share_encoded`: `Vec<u8>` serialized `Prio3InputShare<Field128, 32>`.
- **Validation**:
  - Recipient MUST verify `target == self.id`.
  - Recipient MUST reject duplicate `report_id`.
  - Recipient MUST successfully decode `input_share_encoded` using parameter `(vdaf, self.id)`.

### 2.2 `VerifierShareMessage`
- **Sender**: `Helper`
- **Recipient**: `Leader`
- **Purpose**: Provides Helper's Round 1 proof verification material to Leader for message combination.
- **Fields**:
  - `report_id`: `ReportId`.
  - `agg_id`: `AggregatorId::Helper`.
  - `verifier_share_encoded`: `Vec<u8>` serialized `Prio3VerifierShare<Field128, 32>`.
- **Validation**:
  - Leader MUST match `report_id` to an existing pending verification initiated in `verify_init`.
  - Leader decodes with `ParameterizedDecode` using Leader's verify state.

### 2.3 `VerifierMessageBroadcast`
- **Sender**: `Leader`
- **Recipient**: `Helper`
- **Purpose**: Distributes the combined verifier message allowing Helper to complete Round 2 transition (`verify_next`).
- **Fields**:
  - `report_id`: `ReportId`.
  - `verifier_message_encoded`: `Vec<u8>` serialized `Prio3VerifierMessage<32>`.
- **Validation**:
  - Helper MUST have a pending `Prio3VerifyState` for `report_id`.
  - Passing invalid or tampered message causes `verify_next` to return `VdafError`.

### 2.4 `CollectionRequest`
- **Sender**: `Collector`
- **Recipient**: `Leader`
- **Purpose**: Requests collection for a specified query or epoch.
- **Fields**:
  - `job_id`: Unique collection job string.

### 2.5 `AggregateShareMessage`
- **Sender**: `Leader` or `Helper`
- **Recipient**: `Leader` (from Helper) or `Collector`
- **Purpose**: Contains an aggregator's additive contribution to the total histogram.
- **Fields**:
  - `agg_id`: `AggregatorId`.
  - `aggregate_share_encoded`: `Vec<u8>` serialized `AggregateShare<Field128>`.
  - `report_count`: Number of accepted reports accumulated into this share.
- **Validation**:
  - Collector MUST receive one share from Leader and one share from Helper.
  - Collector MUST verify `leader.report_count == helper.report_count`.

### 2.6 `CollectionResponse`
- **Sender**: `Leader`
- **Recipient**: `Collector`
- **Purpose**: Bundles both aggregate shares for Collector unsharding.
- **Fields**:
  - `job_id`: Collection job identifier.
  - `leader_share`: `AggregateShareMessage`.
  - `helper_share`: `AggregateShareMessage`.

---

## 3. Data Classification Matrix

| Message | Public Data | Private Input Share | Verifier Material | Aggregate Share | Plaintext Measurement |
|---|---|---|---|---|---|
| `ClientReport` | Nonce, Public Share | Yes (1 share only) | No | No | Never |
| `VerifierShareMessage` | Report ID | No | Yes (Helper's) | No | Never |
| `VerifierMessageBroadcast` | Report ID | No | Yes (Combined) | No | Never |
| `AggregateShareMessage` | Report Count | No | No | Yes (1 share only) | Never |
| `CollectionResponse` | Job ID, Counts | No | No | Yes (Both shares) | Never |

