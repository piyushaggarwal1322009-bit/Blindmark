# Phase 6E: Authentication & Anti-Replay Design Proposal

## 1. Executive Summary & Objective

The Phase 6E security review highlighted a fundamental tension in privacy-preserving benchmarking:
- **Participant Anonymity**: Aggregators must not learn who submitted which report.
- **Sybil & Replay Protection**: Aggregators must ensure that only authorized participants submit, and that no participant submits more than once per campaign epoch.

This document proposes an authentication and anti-replay architecture that satisfies both requirements without inventing fake cryptography or claiming that the current prototype provides cryptographic unlinkability.

---

## 2. Review of Existing FastAPI Token Implementation

The current FastAPI backend (`app/services/token_service.py` & `app/models.py:CampaignToken`) uses:
1. `raw_token = "tok_" + secrets.token_urlsafe(32)` (192 bits of entropy).
2. The database stores: `nullifier_hash = hashlib.sha256(raw_token.encode()).hexdigest()`.
3. When issuing a token: FastAPI checks `CampaignEligibility.participant_ref` and flags `token_issued = True`.
4. When redeeming a token: FastAPI checks whether `nullifier_hash` exists, has not expired, and `is_redeemed == False`. It then sets `is_redeemed = True`.

### Limitations of the Existing Backend Token Design:
- **It is NOT a Blind Signature or Privacy Pass**: Because FastAPI generates the token string directly in cleartext, FastAPI could theoretically log the token before returning it to the user. If FastAPI also observed the token during redemption, it could correlate participant identity with the submission.
- **It relies on Organizational Decoupling**: If the client submits the token **directly to the independent aggregators** (and NOT back to FastAPI), FastAPI cannot link the token to any specific report because FastAPI never sees when or where the token is redeemed.

---

## 3. Proposed Aggregator Token Validation & Anti-Replay Architecture

```
[Student Device]
       │
       │ 1. Authenticate via University SSO
       ▼
[FastAPI Control Plane]
       │ 2. Issues Single-Use Token T (checks eligibility table)
       │    Stores H = SHA256(T) in nullifier table
       ▼
[Student Device]
       │
       ├─── 3a. Direct Ingestion (Token T, Share 0) ───► [Leader Aggregator]
       │                                                    │ Checks local nullifier cache
       │                                                    │ Stores H_leader = SHA256(T)
       │
       └─── 3b. Direct Ingestion (Token T, Share 1) ───► [Helper Aggregator]
                                                            │ Checks local nullifier cache
                                                            │ Stores H_helper = SHA256(T)
```

### 3.1 How Aggregators Validate the Token
- Each aggregator is initialized with a public validation key (or a shared task verification root) or queries an append-only token registry epoch snapshot.
- Upon receiving `(T, Nonce, PublicShare, InputShare)`:
  1. The aggregator calculates the nullifier tag:
     $$N_{\text{tag}} = \text{HMAC-SHA256}(\text{task\_id}, T)$$
  2. The aggregator checks whether $N_{\text{tag}}$ exists in its local in-memory/database nullifier set.
  3. If $N_{\text{tag}}$ has already been seen: **Reject immediately with HTTP 409 Conflict (Duplicate Submission)**.
  4. If $N_{\text{tag}}$ is new: Insert $N_{\text{tag}}$ into the local nullifier set and proceed to Prio3 verification.

### 3.2 Preventing Double-Counting Across Aggregators
- Both Leader and Helper independently maintain their own local $N_{\text{tag}}$ sets.
- During the inter-aggregator preparation loop (`verifier_shares_to_message`), the Leader and Helper exchange the list of report IDs / nonces being verified.
- If an attacker submitted to Leader but reused an old token, Leader rejects. If an attacker submits a different token to Helper, the nonce and report ID mismatch prevents successful proof message combination.

### 3.3 Partial Submissions and Retries
- **Scenario**: A client successfully submits `InputShare_0` to the Leader, but the mobile connection drops before `InputShare_1` reaches the Helper.
- **Handling**:
  - Reports in the aggregator remain in a `pending` state until the verification prep loop executes.
  - If a batch prep exchange occurs and one aggregator is missing the corresponding report ID, **the incomplete report is discarded**.
  - A client retry window is supported: if a client resubmits the exact same `(report_id, nonce, input_share)` within 60 seconds, the aggregator updates the pending record idempotently instead of treating it as a new duplicate.

### 3.4 Cryptographic Limitations of HMAC: Why HMAC Alone Does Not Provide Anonymous Issuance or Unlinkability

While Section 3.1 outlines how HMAC-SHA256 nullifiers can prevent duplicate submissions locally, **HMAC-SHA256 alone is cryptographically insufficient for participant unlinkability and anonymous credential issuance**:

1. **Deterministic Tag Generation & Deanonymization**:
   Because HMAC is a deterministic symmetric primitive, anyone who knows the secret key $K$ (e.g. FastAPI control plane) can compute $N_{\text{tag}} = \text{HMAC}(K, \text{ParticipantID} \parallel \text{CampaignID})$ for every participant in advance. If FastAPI or an adversary inspects aggregator submission nullifiers or server access logs, they can instantaneously map nullifiers back to specific participants, destroying participant anonymity.
2. **Lack of Blinded Issuance**:
   A standard HMAC cannot be issued "blindly". FastAPI cannot authenticate an eligible participant and sign their token without seeing the resulting token. Because FastAPI sees the token at issuance time, if that same token or a deterministic derivative is presented to the aggregators, the submission is immediately linkable to the user's issuance session.
3. **Requirement for Coordinated Cross-Aggregator Anti-Replay**:
   Because Leader and Helper operate as separate network services with partitioned storage, neither aggregator can unilaterally prevent distributed replay attacks. If a malicious participant submits `InputShare_0` with Token $A$ to the Leader, but submits `InputShare_1` with Token $B$ to the Helper, local checks on each aggregator would succeed if both tokens were validly issued. Anti-replay must therefore be cryptographically bound to the joint report metadata (such as a shared report nonce, task ID, and public share binding) validated during the inter-aggregator verification preparation phase.
4. **Conclusion**:
   HMAC-SHA256 provides message authentication and integrity, not privacy-preserving credential issuance. A production deployment requires true cryptographic blind credentials (e.g. Privacy Pass RFC 9578 or RSA-FDH blind signatures) where tokens are unlinkable even to the issuing authority.

### 3.5 Future Cryptographic Upgrade: Blind Signatures (Privacy Pass)
To eliminate trust in FastAPI's token issuance:
- **Client Blind Token**: The client generates a random token $r$, blinds it as $r' = \text{Blind}(r)$, and sends $r'$ to FastAPI.
- **FastAPI Signs Blinded Token**: FastAPI signs $r'$ using its private key without learning $r$: $s' = \text{Sign}(r')$.
- **Client Unblinds**: The client computes $s = \text{Unblind}(s')$.
- **Aggregators Verify**: Aggregators verify the signature $s$ against FastAPI's public key. **FastAPI cannot link $r$ to the participant even if it logs every issuance request.**

---

## 4. Summary of Decisions Awaiting Approval

1. **Phase 6F Target**: Use SHA-256 token nullifiers with direct aggregator submission for the initial microservice integration.
2. **Phase 7 Upgrade**: Implement true cryptographic blind signatures (RFC 9474 / Privacy Pass) once the end-to-end service pipeline is functional.

