# Privacy Model: Duplicate Prevention & Unlinkability

## 1. The Core Privacy Dilemma

In privacy-preserving telemetry and benchmarking, two critical requirements stand in direct tension:
1. **Sybil Resistance:** Only eligible participants may submit, and each participant may submit at most once per campaign.
2. **Unlinkability:** The system must never be able to link a participant's real-world identity (email address, university domain, or employee ID) to their submitted value or bucket index.

If an identity token is directly attached to the cryptographic report upload, any entity observing the upload could associate that user's IP or identity token with that submission share.

---

## 2. Evaluation of Duplicate Prevention Approaches

### Approach A: Verified Email Eligibility + Central Database Check
* **Mechanism:** User verifies an `@company.com` or `@university.edu` email via magic link. The backend issues a single-use JWT. When the client submits its share, the JWT is validated and marked "used" in the database.
* **Privacy Trade-off:**
  * **Weakness:** The server receiving the upload sees the identity token and the report share at the exact same time. If the backend logs the token timestamp alongside the share timestamp, or if it acts as one of the aggregators, it links identity to submission.
  * **Verdict:** Unacceptable for high-assurance confidentiality without strict cryptographic unlinking.

### Approach B: Decoupled One-Time Submission Tokens (Phase 3 Baseline)
* **Mechanism:**
  1. **Token Issuance Stage:**
     * User verifies email/membership with the Campaign Authority (Auth Service).
     * Auth Service marks `has_claimed_token = True` for that user and issues a cryptographically random token $T$ (e.g. 256-bit high-entropy secret).
     * The Auth Service records only the commitment/hash $H(T)$ in the `CampaignTokenRecord` table, never saving the link between $T$ and the user's email.
  2. **Submission Stage (Separated by time & connection):**
     * The client submits its report shares along with token $T$ (or a zero-knowledge proof of token knowledge).
     * The Aggregators verify that $H(T)$ exists in the eligible set and has not been marked as consumed in the nullifier set.
* **Privacy Trade-off:**
  * **Pros:** Identity verification is completely decoupled from report submission.
  * **Cons:** If token issuance and token redemption happen in rapid succession over the same IP address or session, metadata correlation (timing attack) can link the user to the report.

### Approach C: Cryptographic Blind Signatures / Privacy Pass (Future Production)
* **Mechanism:**
  * Based on RSA blind signatures or Privacy Pass / VOPRF protocol (RFC 9578).
  * The client generates a random token $T$, blinds it as $T'$, and asks the Auth Authority to sign $T'$ upon proving eligibility.
  * The Auth Authority signs $T'$ without ever seeing $T$.
  * The client unblinds the signature locally to yield valid signature $\sigma(T)$.
  * When submitting, the client presents $(T, \sigma(T))$ to the Aggregators over Tor/VPN or an oblivious proxy.
* **Privacy Trade-off:**
  * **Pros:** Perfect cryptographic unlinkability. Even a completely malicious or colluding Auth Authority cannot correlate which eligible user redeemed which token.
  * **Complexity:** Requires client-side cryptographic blinding libraries and token issuance orchestration.

---

## 3. Privacy-Release Guarantees (Collector Layer)

1. **Threshold Gating ($k$-Anonymity):**
   * If a campaign closes with $N < N_{\text{min}}$ (e.g. fewer than 20 verified responses), the campaign is automatically aborted, and all intermediate aggregator shares are erased.

2. **Differential Privacy (DP):**
   * Before displaying bucket bars (e.g., *Bucket 5: 14 responses*), the collector samples calibrated discrete Laplacian noise:
     $$\tilde{C}_i = \max\left(0, \text{Round}(C_i + \text{Laplace}(0, 1/\epsilon))\right)$$
   * This provides mathematical $(\epsilon, \delta)$-differential privacy, ensuring that an adversary cannot determine with confidence whether any specific individual participated or changed buckets.

