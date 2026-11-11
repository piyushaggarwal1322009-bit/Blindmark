# Blindmark API Specification

Version: `0.1.0-alpha` (Phase 4 Backend Foundation)

This document details the REST API endpoints implemented in the Blindmark backend foundation.

---

## 1. Overview & Privacy Invariant

The Blindmark API handles metadata, participant eligibility verification, and aggregation lifecycle tracking.
**Core Privacy Invariant**: The Blindmark application database and API **never** receive, store, or log raw submitted measurements or reconstructable cryptographic input shares. All submissions flow directly to independent Aggregators via VDAF protocols.

---

## 2. Communities

### `POST /api/communities`
Creates a new organizational or benchmarking community.

- **Request Body**:
```json
{
  "name": "Tech Compensation Benchmark",
  "description": "Anonymous salary benchmarking for software engineers"
}
```
- **Validation**:
  - `name`: String, 1–255 characters.
  - `description`: Optional string.
- **Response**: `201 Created`
```json
{
  "id": 1,
  "name": "Tech Compensation Benchmark",
  "description": "Anonymous salary benchmarking for software engineers",
  "created_at": "2026-11-11T07:25:00Z"
}
```

---

### `GET /api/communities/{community_id}`
Retrieves community details by ID.

- **Response**: `200 OK`
```json
{
  "id": 1,
  "name": "Tech Compensation Benchmark",
  "description": "Anonymous salary benchmarking for software engineers",
  "created_at": "2026-11-11T07:25:00Z"
}
```
- **Errors**: `404 Not Found` if community does not exist.

---

## 3. Campaigns

### `POST /api/campaigns`
Creates a new benchmarking campaign under a community.

- **Request Body**:
```json
{
  "community_id": 1,
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
- **Validation Rules**:
  - `bucket_configuration`: Must contain at least 2 bucket labels.
  - `minimum_participants`: Integer $\ge 3$.
  - `end_at` must be strictly later than `start_at`.
- **Response**: `201 Created` with campaign object (status default: `"draft"`).

---

### `GET /api/campaigns/{campaign_id}`
Fetches campaign metadata.

- **Response**: `200 OK`

---

## 4. One-Time Campaign Tokens

Blindmark enforces a single-submission policy per eligible participant without storing cleartext tokens in the database.

### `POST /api/campaigns/{campaign_id}/tokens/issue`
Validates participant eligibility and issues a one-time bearer token.

- **Request Body**:
```json
{
  "participant_ref": "employee-id-xyz-987"
}
```
- **Behavior**:
  - Checks `CampaignEligibility` for `(campaign_id, participant_ref)`.
  - Computes `nullifier_hash = SHA256(raw_token)`.
  - Persists record with `is_issued = True`.
  - Returns bearer token **only once**.
- **Response**: `200 OK`
```json
{
  "token": "dGhpcy1pcy1hLXRva2Vu...",
  "nullifier_hash": "a591a6d40bf420404a011733cfb7b190d62c65bf0bcda32b57b277d9ad9f146e",
  "expires_at": "2026-11-12T07:25:00Z"
}
```
- **Errors**:
  - `400 Bad Request`: Token already issued for this participant.

---

### `POST /api/campaigns/{campaign_id}/tokens/redeem`
Consumes a one-time token prior to accepting an aggregate contribution.

- **Request Body**:
```json
{
  "token": "dGhpcy1pcy1hLXRva2Vu..."
}
```
- **Behavior**:
  - Computes `SHA256(raw_token)` to find `CampaignToken`.
  - Validates campaign ownership, expiry, and revocation state.
  - Ensures concurrency-safe atomic consumption (`is_redeemed = True`, sets `redeemed_at`).
- **Response**: `200 OK`
```json
{
  "status": "redeemed",
  "nullifier_hash": "a591a6d40bf420404a011733cfb7b190d62c65bf0bcda32b57b277d9ad9f146e"
}
```
- **Errors**:
  - `400 Bad Request`: Token already redeemed, expired, revoked, or belongs to a different campaign.
  - `404 Not Found`: Unknown token nullifier hash.

---

## 5. Aggregation Results & Privacy Release Policy

### `POST /api/campaigns/{campaign_id}/results/evaluate`
Evaluates the release criteria for a campaign's aggregate histogram and conditionally releases it.

- **Request Body**:
```json
{
  "histogram": [0, 5, 12, 8, 3],
  "participant_count": 28
}
```
- **Privacy Evaluation**:
  - Evaluates `participant_count >= campaign.minimum_participants`.
  - If threshold is met: sets `release_status = "released"`, stores histogram, and sets `released_at`.
  - If threshold is not met: sets `release_status = "withheld_threshold_not_met"`, suppresses histogram output.
- **Response**: `200 OK`
```json
{
  "campaign_id": 1,
  "participant_count": 28,
  "release_status": "released",
  "released_at": "2026-11-11T07:30:00Z",
  "histogram": [0, 5, 12, 8, 3]
}
```

---

### `GET /api/campaigns/{campaign_id}/results`
Retrieves the released aggregation result.

- **Response**: `200 OK`
- If unreleased or threshold not met: returns status with empty/omitted histogram data.
- **Errors**: `404 Not Found` if results have not yet been evaluated.

