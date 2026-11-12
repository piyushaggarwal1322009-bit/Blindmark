# Blindmark API Specification

Version: `0.2.0-remediated` (Phase 5B Security Hardening)

This document specifies the REST API endpoints implemented in the Blindmark backend.

---

## 1. Overview & Privacy Invariant

The Blindmark API handles metadata, participant eligibility verification, and aggregation lifecycle tracking.
**Core Privacy Invariant**: The Blindmark application database and API **never** receive, store, or log raw submitted measurements or reconstructable cryptographic input shares. Submissions flow directly to independent Aggregators via VDAF protocols.

---

## 2. Authentication & Authorization Model

Blindmark distinguishes between three caller contexts:
1. **Administrative Principal**: Requires `X-API-Key` header matching configured admin credential (`BLINDMARK_ADMIN_API_KEY`). Protects community creation, campaign creation, and participant eligibility registration.
2. **Aggregator Service Principal**: Requires `X-Service-Key` header matching configured service credential (`BLINDMARK_AGGREGATOR_API_KEY`). Protects trusted result evaluation and publication.
3. **Participant / Public Client**: Unauthenticated or token-bearer access for public directory metadata, token issuance (for pre-registered participants), token redemption, and benchmark viewing.

---

## 3. Communities

### `POST /api/communities`
Creates a new organizational or benchmarking community.
- **Authorization**: **Admin Only** (`X-API-Key` header required).
- **Request Body**:
```json
{
  "name": "Tech Compensation Benchmark",
  "description": "Anonymous salary benchmarking for software engineers"
}
```
- **Response**: `201 Created`
- **Errors**: `401 Unauthorized` (missing key), `403 Forbidden` (bad key), `400 Bad Request` (duplicate name).

---

### `GET /api/communities/{community_id}`
Retrieves community details by ID.
- **Authorization**: Public.
- **Response**: `200 OK`

---

## 4. Campaigns

### `POST /api/campaigns`
Creates a new benchmarking campaign under a community.
- **Authorization**: **Admin Only** (`X-API-Key` header required).
- **Request Body**:
```json
{
  "community_id": "community-uuid",
  "title": "2026 Q4 Senior SWE Compensation",
  "description": "Base salary distribution in thousands USD",
  "category": "Compensation",
  "bucket_configuration": [
    "0-50k", "50-100k", "100-150k", "150-200k", "200-250k"
  ],
  "minimum_participants": 10,
  "start_at": "2026-11-01T00:00:00Z",
  "end_at": "2026-12-01T00:00:00Z"
}
```
- **Validation**:
  - `bucket_configuration`: Must contain at least 2 bucket labels.
  - `minimum_participants`: Integer $\ge 3$.
  - `extra="forbid"`: Reject unexpected request fields.
- **Response**: `201 Created`
- **Errors**: `401 Unauthorized`, `403 Forbidden`, `400 Bad Request` / `422 Unprocessable Entity`.

---

### `GET /api/campaigns` & `GET /api/campaigns/{campaign_id}`
Public listing and retrieval of campaign metadata.
- **Authorization**: Public.

---

## 5. Participant Eligibility & Token Issuance

Blindmark enforces a strict pre-registration model to prevent Sybil attacks. Anonymous callers cannot self-enroll.

### `POST /api/campaigns/{campaign_id}/eligibility`
Enrolls an authorized participant reference into a campaign's eligibility whitelist.
- **Authorization**: **Admin Only** (`X-API-Key` header required).
- **Request Body**:
```json
{
  "participant_ref": "verified-employee-hash-1234"
}
```
- **Response**: `201 Created`
- **Errors**: `400 Bad Request` if participant is already registered for this campaign.

---

### `POST /api/campaigns/{campaign_id}/tokens/issue`
Issues a one-time bearer token for an eligible participant.
- **Authorization**: Public (Participant presenting reference).
- **Request Body**:
```json
{
  "participant_ref": "verified-employee-hash-1234"
}
```
- **Behavior**:
  - **Rejects unknown participants**: Participant must already be in `CampaignEligibility`.
  - Rejects if token was already issued for this participant.
  - Computes `nullifier_hash = SHA256(raw_token)` and stores nullifier record.
- **Response**: `200 OK`
```json
{
  "token": "dGhpcy1pcy1hLXRva2Vu...",
  "expires_at": "2026-11-12T07:25:00Z",
  "note": "Store this token securely. It cannot be recovered and will be invalidated upon single use."
}
```

---

### `POST /api/campaigns/{campaign_id}/tokens/redeem`
Redeems a one-time token using an atomic database update.
- **Authorization**: Public (Aggregator or client presenting token during submission).
- **Request Body**:
```json
{
  "token": "dGhpcy1pcy1hLXRva2Vu..."
}
```
- **Concurrency Protection**:
  - Executes atomic SQL update: `UPDATE campaign_tokens SET is_redeemed = True WHERE id = :id AND is_redeemed = False`.
  - Exactly one concurrent caller will update the row; all other racers fail with `400 Bad Request`.
- **Response**: `200 OK`

---

## 6. Aggregation Results & Privacy Release Policy

### `POST /api/campaigns/{campaign_id}/results/evaluate`
Evaluates the release criteria for a completed aggregate histogram and conditionally publishes it.
- **Authorization**: **Trusted Aggregator Service Only** (`X-Service-Key` header required).
- **Strict Request Validation**:
  - `histogram`: Counts must be non-negative integers; dimension must match `len(bucket_configuration)`.
  - `participant_count`: Integer $\ge 1$.
  - Extra fields forbidden (`422 Unprocessable Entity`).
- **Privacy Gating**:
  - Withheld if `participant_count < campaign.minimum_participants`.
- **Response**: `200 OK` (histogram withheld if threshold unmet).
- **Errors**: `401 Unauthorized` (anonymous callers blocked), `403 Forbidden` (invalid service key), `400/422` (validation failure).

---

### `GET /api/campaigns/{campaign_id}/results`
Retrieves public released aggregation results.
- **Authorization**: Public.
- **Response**: `200 OK`. If unreleased or threshold not met, `histogram` is `null` and status is `"withheld"`.
