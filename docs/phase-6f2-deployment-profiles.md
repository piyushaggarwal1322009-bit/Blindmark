# Phase 6F.2 Deployment Profiles

Status: Correction round 3. Proposed for independent human review. Not approved. Does not authorize Checkpoint B or any implementation.

## 1. Shared requirements

Every profile must satisfy the same core enforcement conditions:

- Leader and Helper participant ingress are separate HTTPS endpoints on distinct trust boundaries.
- Leader-to-helper coordination is performed only by an authenticated leader egress proxy and a helper coordination proxy.
- The local proxy-to-backend path must use a process-restricted IPC endpoint or another explicitly selected private local transport.
- Generic loopback TCP is not treated as process-restricted.
- A valid HMAC proves message integrity under the directional key, but it does not prove that the request traversed the proxy.
- A proxy-generated identity assertion alone does not prove identity if direct backend access remains possible.
- If a profile cannot enforce the required local process isolation, the coordination route remains disabled.
- Dedicated OS service accounts are required for the leader application, leader egress proxy, helper coordination proxy, and helper backend.

## 2. Enforcement table

| Profile | Supported OS assumptions | Local endpoint type | Permitted connecting process identity | Endpoint owner | Unauthorized rejection | Privileged administrator limitation | If unavailable |
|---|---|---|---|---|---|---|---|
| A | Linux with peer-credential Unix sockets; Windows with ACL-protected named pipes | Unix socket or named pipe | leader service account for the egress path; helper coordination proxy service account for the backend path | leader or helper service account | socket or pipe peer mismatch triggers immediate rejection | host-root or admin can still alter host state; ordinary local processes remain restricted | coordination disabled |
| B | Same as A, with dedicated service identities and no shared account reuse | Unix socket or named pipe | leader service account and helper coordination proxy service account | service account owning each process | peer mismatch or ACL denial before request acceptance | host-root/admin remains a privileged boundary | coordination disabled |
| C | Same as A on each host; leader-to-helper uses private network mTLS | Unix socket or named pipe on each host; private network mTLS between proxies | leader service account on the leader host; helper coordination proxy service account on the helper host | service account owning each process | local peer mismatch or ACL denial; remote mTLS misvalidation fails closed | privileged admin can still compromise a host, but ordinary local processes are blocked | coordination disabled |

## 3. Profile A: local development

### 3.1 Topology

- Separate Leader and Helper processes run on the same development host.
- Participant access uses distinct HTTPS endpoints with development certificates.
- The leader application connects to a local egress proxy, not directly to the helper coordination proxy.
- The leader egress proxy connects only to the configured helper coordination proxy over mTLS.
- The helper coordination proxy forwards only to the helper backend over a single local protected IPC endpoint.
- The helper backend accepts requests only from the helper coordination proxy service account.

### 3.2 Process isolation

- Linux: Unix domain socket with `SO_PEERCRED` and socket ownership restricted to the relevant service account.
- Windows: Named pipe with DACL granting access only to the relevant service SID.
- The creator of the socket or pipe must be the service owner; directory permissions and ACLs must match the service identity.

### 3.3 Decision

This profile remains enabled only if the local IPC enforcement is available and validated. If the platform cannot provide the required peer identity or pipe ACL restriction, coordination is disabled.

### 3.4 Limitations

This profile is not a non-collusion guarantee. A single local admin or host compromise can still impact both services. The design is suitable for controlled local testing only and is not a deployment authorization.

## 4. Profile B: single-host test deployment

### 4.1 Topology

- Leader and Helper run on the same machine but use distinct service identities.
- Public ingress is HTTPS at the proxy tier.
- Internal coordination is routed through the leader egress proxy and the helper coordination proxy.
- The helper backend local transport is restricted to the helper coordination proxy identity.
- The application-level HMAC is validated independently of proxy provenance and must match the trusted identity assertion.

### 4.2 Process isolation

- The leader egress listener must be restricted to the leader service identity.
- The helper backend must accept requests only from the helper coordination proxy identity.
- Loopback TCP is not accepted as the isolation mechanism.
- No PID or executable path is accepted as its own trust boundary.

### 4.3 Decision

Profile B is valid only when the host OS enforces the selected local IPC identities. If local IPC enforcement is unavailable, coordination remains disabled.

### 4.4 Limitations

This deployment assumes a single host and a single administrative boundary. It is stronger than a plain loopback design but does not create organizational independence.

## 5. Profile C: separate-host deployment

### 5.1 Topology

- Leader and Helper run on separate hosts or trust domains.
- Participant HTTPS is terminated at each role's ingress proxy.
- The leader egress proxy connects to the helper coordination proxy over private network mTLS.
- The helper coordination proxy forwards to the helper backend over a local protected IPC endpoint on the helper host.
- The helper backend verifies the proxy assertion and the directional HMAC sender identity.

### 5.2 Correct terminology for TLS termination

The wording for this profile is:

- Participant-to-ingress-proxy HTTPS.
- Leader-egress-proxy-to-helper-coordination-proxy mTLS.
- Helper-proxy-to-backend local protected IPC or explicitly selected private local transport.

The phrase “end-to-end TLS” is not used to imply that the Rust backend receives TLS. It was replaced with the explicit transport wording required by the review.

### 5.3 Process isolation

- Leader host: the egress proxy endpoint is process-restricted to the leader service identity.
- Helper host: the backend accepts requests only from the helper coordination proxy identity via a selected local transport.
- If the helper local transport cannot enforce that boundary, coordination remains disabled.

### 5.4 Decision

Profile C is the only profile that can credibly preserve a separate-host trust boundary, but it still requires validated OS-level ACL or peer-credential enforcement and valid certificate rotation and revocation for the coordination path.

### 5.5 Limitations

This profile does not establish organizational non-collusion by itself. It still requires host, network, key-custody, incident-response, and admin-boundary review.

## 6. Summary of profile status

| Profile | Status |
|---|---|
| A | Enabled only with local peer-credential or ACL-protected IPC; otherwise disabled |
| B | Enabled only with local peer-credential or ACL-protected IPC; otherwise disabled |
| C | Enabled only with local peer-credential or ACL-protected IPC on the helper host and authenticated mTLS between proxies; otherwise disabled |

No profile may rely on plain loopback TCP as a process authorization control.

## 7. Final review outcome

This profile package is proposed for independent human review. It is not a deployment authorization and it does not authorize Checkpoint B or any implementation work.
