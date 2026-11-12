# Phase 6D: Corrected API Integration Contract

> **STATUS**: PROPOSED CONTRACT SPECIFICATION (Not Yet Implemented).
> Clearly separates control plane operations from direct aggregator ingestion.

---

## 1. Summary of Operations & Data Boundaries

| # | Operation Name | Caller | Receiver | Endpoint | Sensitive Data Present? |
|---|---|---|---|---|---|
| **Op 1** | Issue Anonymous Nullifier Token | Participant Client | FastAPI Backend | `POST /api/campaigns/{id}/tokens/issue` | No (Token seed only) |
| **Op 2** | Direct Ingestion: Leader Share | Participant Client | Leader Aggregator | `POST /tasks/{id}/reports/0` | **InputShare_0 ONLY** |
| **Op 3** | Direct Ingestion: Helper Share | Participant Client | Helper Aggregator | `POST /tasks/{id}/reports/1` | **InputShare_1 ONLY** |
| **Op 4** | Verification Prep Exchange | Leader & Helper | Each Other | `POST /tasks/{id}/prepare/*` | ZK verifier shares |
| **Op 5** | Trigger Collection | FastAPI Backend | Collector Component | `POST /tasks/{id}/collect` | No (Task ID only) |
| **Op 6** | Unshared Result Evaluation | Collector Component | FastAPI Backend | `POST /api/campaigns/{id}/results/evaluate` | Raw histogram vector |

---

## 2. Detailed Operation Schemas

### Operation 1: Issue Anonymous Nullifier Token
- **Caller**: Participant Client (Authenticated via Student SSO / Session Cookie)
- **Receiver**: FastAPI Backend (`app/routers/campaigns.py`)
- **Purpose**: Authenticates the student, verifies campaign eligibility, and issues a cryptographic single-use token.
- **Privacy Property**: Once issued, FastAPI records that a token was granted to participant reference $P$, but the token's private nullifier $T$ is unlinked from $P$ using blind signature techniques or hash nullifiers.

#### Proposed Request Schema: `POST /api/campaigns/{campaign_id}/tokens/issue`
```json
{
  "participant_ref": "sha256_hash_of_verified_student_email"
}
```

#### Proposed Response Schema:
```json
{
  "token": "tok_nullifier_9f82b7c4a1e5d362f0a8c2...",
  "expires_at": "2026-11-15T00:00:00Z"
}
```

---

### Operation 2: Direct Ingestion — Leader Share
- **Caller**: Participant Client
- **Receiver**: Leader Aggregator (`https://leader.blindmark.org/tasks/{task_id}/reports/0`)
- **Purpose**: Delivers **InputShare_0 only** directly to the Leader.
- **Privacy Property**: Leader learns `InputShare_0` and the nullifier, but has NO access to `InputShare_1` and NO access to participant identity.

#### Proposed Request Schema:
```json
{
  "token": "tok_nullifier_9f82b7c4a1e5d362f0a8c2...",
  "report_id": "rep_5a83c1b092f3",
  "aggregator_id": 0,
  "nonce_b64": "vF8kL3z4Pq1mNx0vRt7wSw==",
  "public_share_b64": "A4g...[Prio3PublicShare base64]...",
  "input_share_b64": "B9x...[Leader InputShare_0 base64]..."
}
```

#### Validation & Idempotency Rules:
1. Leader checks whether `token` or `report_id` has already been recorded in its local nullifier database. If seen, reject with `409 Conflict: Duplicate submission`.
2. Validates that `aggregator_id == 0`.
3. Validates base64 decoding of `public_share_b64` and `input_share_b64`.
4. Executes `verify_init` to generate `(Prio3VerifyState, Prio3VerifierShare)`.
5. Returns `202 Accepted`.

---

### Operation 3: Direct Ingestion — Helper Share
- **Caller**: Participant Client
- **Receiver**: Helper Aggregator (`https://helper.blindmark.org/tasks/{task_id}/reports/1`)
- **Purpose**: Delivers **InputShare_1 only** directly to the Helper.
- **Privacy Property**: Helper learns `InputShare_1` and the nullifier, but has NO access to `InputShare_0` and NO access to participant identity.

#### Proposed Request Schema:
```json
{
  "token": "tok_nullifier_9f82b7c4a1e5d362f0a8c2...",
  "report_id": "rep_5a83c1b092f3",
  "aggregator_id": 1,
  "nonce_b64": "vF8kL3z4Pq1mNx0vRt7wSw==",
  "public_share_b64": "A4g...[Prio3PublicShare base64]...",
  "input_share_b64": "C2z...[Helper InputShare_1 base64]..."
}
```

#### Validation & Idempotency Rules:
1. Helper checks whether `token` or `report_id` has already been recorded in its local nullifier database. If seen, reject with `409 Conflict: Duplicate submission`.
2. Validates that `aggregator_id == 1`.
3. Validates base64 decoding of `public_share_b64` and `input_share_b64`.
4. Executes `verify_init` and emits `VerifierShareMessage` for the Leader.
5. Returns `202 Accepted`.

---

### Operation 4: Encrypted Relay Alternative (For Restricted Environments)
If client firewall constraints prevent direct connections to both aggregators:
- Client encrypts `InputShare_0` using the Leader's HPKE public key: $C_0 = \text{HPKE}_{\text{Leader}}(\text{InputShare}_0)$.
- Client encrypts `InputShare_1` using the Helper's HPKE public key: $C_1 = \text{HPKE}_{\text{Helper}}(\text{InputShare}_1)$.
- Client submits $(C_0, C_1)$ to FastAPI relay endpoint `POST /api/campaigns/{id}/relay`.
- **FastAPI relays $C_0$ to Leader and $C_1$ to Helper, but cannot decrypt either share because it does not possess the aggregators' private decryption keys.**
