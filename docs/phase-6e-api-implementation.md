# Phase 6E: API Implementation & Endpoint Specification

## 1. Implemented Endpoints

### 1. Health Probe (`GET /health`)
- **Authentication**: None (Public liveness check).
- **Leader Response (Port 8001)**:
  ```json
  {
    "status": "healthy",
    "role": "leader",
    "aggregator_id": 0
  }
  ```
- **Helper Response (Port 8002)**:
  ```json
  {
    "status": "healthy",
    "role": "helper",
    "aggregator_id": 1
  }
  ```

---

### 2. Task Status (`GET /tasks/{task_id}/status`)
- **Authentication**: Required via `X-Service-Key` header matching `DEV_SERVICE_KEY` ("dev-aggregator-secret-key-blindmark").
- **Unauthenticated Response**:
  ```json
  HTTP/1.1 401 Unauthorized
  {
    "error": "Unauthorized",
    "detail": "Invalid or missing X-Service-Key credential"
  }
  ```
- **Authenticated Response**:
  ```json
  HTTP/1.1 200 OK
  {
    "task_id": "active",
    "aggregator_id": 0,
    "accepted_count": 0
  }
  ```

---

### 3. Report Ingestion (`POST /tasks/{task_id}/reports/{agg_id}`)
- **Target Aggregator Enforcement**:
  - The service parses the target aggregator ID in the URL path:
    - Leader strictly requires `{agg_id} == 0`.
    - Helper strictly requires `{agg_id} == 1`.
  - If a client mistakenly sends a Helper report to the Leader service (`/tasks/123/reports/1`), the server immediately rejects the request:
    ```json
    HTTP/1.1 400 Bad Request
    {
      "error": "WrongAggregatorTarget",
      "detail": "Request addressed to aggregator 1, but this service is aggregator 0"
    }
    ```
- **Size Bounds Enforcement**:
  - Every request body is checked against `MAX_PAYLOAD_BYTES` (65,536 bytes / 64 KB).
  - Any request exceeding this limit is rejected immediately without parsing:
    ```json
    HTTP/1.1 413 Payload Too Large
    {
      "error": "PayloadTooLarge",
      "detail": "Payload length 65636 exceeds maximum allowed limit of 65536 bytes"
    }
    ```
- **Empty Body Protection**:
  - Rejects empty bodies with `400 Bad Request: Request body cannot be empty`.
- **Accepted Response**:
  ```json
  HTTP/1.1 202 Accepted
  {
    "status": "accepted",
    "target_aggregator": 0
  }
  ```

