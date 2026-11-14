# Phase 6F.2 Coordination Journal Security Audit

Audit date: 2026-11-14  
Scope: Independent journal security findings and the targeted transaction-boundary remediation, regression tests, protocol/HMAC implementation, and consolidated execution report.

## Verdict

**The HIGH transaction-escape finding is resolved in the current implementation. Human policy decisions A–H are now approved as design, but Phase 6F.2 remains NOT APPROVED as a security-complete system.** Processing no longer exposes a callback or any SQLite handle. The journal owns the state-transition transaction, and a processing error is returned before the transaction begins, leaving the row pending. The approved identity scope is `(sender_id, recipient_id, task_id, message_id)` and accepted identities require database-level uniqueness; that constraint and compatible separate conflict recording are not implemented. Keep the journal disconnected from HTTP routes.

## Audit results

| Audit area | Status | Result |
|---|---|---|
| Duplicate and identity correctness | PARTIAL / IMPLEMENTATION OPEN | Exact canonical-body duplicates return the stored record; conflicting authenticated reuse is stored separately without overwriting the original, including under concurrent receipt. Policy A approves the identity tuple and database-level uniqueness for accepted identities. The current index is non-unique and the schema has not been migrated. |
| Transaction safety | PASS for current journal-local processing | The former callback API is removed. `process_pending` accepts only `Result<Vec<u8>, JournalError>`; it exposes no callback, `Transaction`, `Connection`, or connection-dereferencing capability. The journal owns and commits the state transition. |
| Concurrent processing | PASS for state transition | Independent connections contend on the journal-owned IMMEDIATE transaction. Regression coverage proves only one call transitions the row and the losing call observes the winning persisted outcome. |
| Recovery and corruption | BLOCKED | Reopen/pending recovery and ordinary digest/metadata inconsistency detection are tested. SHA-256 and mirrored metadata do not authenticate a database whose body, digest, and mirrored metadata have all been coherently rewritten; the MAC is not stored/reverified during recovery, and file access controls are unverified. |
| Authentication and boundaries | BLOCKED | HMAC verification precedes receipt persistence and the journal is not wired to HTTP or Phase 6F.1 report storage. Sender-to-key/direction association, key ownership, and local identity are not established by this module. |
| Test evidence | PASS | The exact requested offline command completed successfully after remediation: 56 passed, 0 failed, including 10 journal tests. Journal coverage includes processing failure, exactly-once terminal behavior, concurrent processing, concurrent exact receipt, and concurrent conflicting receipt. |

## Findings

### JSA-01 — Callback can escape the journal transaction

**Severity: HIGH. Original status: FAIL. Current status: RESOLVED.**

Original finding: `process_pending` exposed `&Transaction` to caller-controlled callback code. In rusqlite 0.29, `Transaction` dereferences to `Connection`, so callback code could issue arbitrary SQL, including `COMMIT` or `ROLLBACK`, outside the intended journal-controlled boundary. A callback error after an explicit commit could leave the journal row pending while external effects had committed, allowing effects to be repeated on retry.

Exact remediation: the callback was removed rather than wrapped in a different reference type. The current signature accepts `process_pending(journal_id, processing_result: Result<Vec<u8>, JournalError>)`. No caller-controlled closure or object is invoked. The exposed parameter is an owned result value and cannot dereference to a SQLite connection. On `Err`, `?` returns before opening a transaction, so the record remains pending. On success, the journal alone opens an IMMEDIATE transaction, checks the current state, updates the outcome and state with a `RECEIVED_PENDING` predicate, and commits. Existing database triggers retain terminal-state and identity immutability protections. No arbitrary SQL, SQL `COMMIT`, or SQL `ROLLBACK` capability is exposed.

Regression evidence: `processing_failure_leaves_message_pending` verifies failure leaves the row pending; `processed_and_rejected_messages_are_terminal` verifies a processed record cannot be processed again and its first outcome remains stored; `concurrent_processing_transitions_the_message_once` uses two independent connections and distinct result values, proving one transition wins and the retry observes that persisted result. The API itself has no callback boundary to exercise with a SQL-commit probe.

### JSA-02 — Identity index is not a uniqueness constraint

**Severity: MEDIUM. Original status: FAIL. Current status: approved design; implementation open.**

The schema creates `idx_inbound_identity` as a regular index over `(sender_id, recipient_id, task_id, message_id)`; the only table uniqueness is the generated `journal_id` primary key (`experiments/phase_6e/src/journal.rs#L131`). `load_identity_records` searches by that composite tuple, and `receive_authenticated_frame` serializes its check-and-insert with `BEGIN IMMEDIATE` (`journal.rs#L165`, `journal.rs#L341`). Thus normal calls through this API are serialized, but the database itself does not enforce the identity invariant against another writer or a future insertion path.

The current behavior under the approved scope is otherwise sound: a byte-identical body returns the original record; different canonical bytes under the same tuple create a distinct `REJECTED` row and leave the prior row untouched. This includes changes to report/batch/type/sequence/timestamp/payload fields because those are in the canonical body. Changing sender, recipient, or task changes the identity tuple and is treated as a different identity under the approved scope.

Human policy decision A now approves the tuple `(sender_id, recipient_id, task_id, message_id)`, uniqueness for accepted identities, separate conflict recording, and eventual sender-to-credential binding. This audit preserves the original finding: at review time the index was non-unique. No schema change is made by this documentation update. Implementation must add database-level uniqueness without losing separately recorded conflicts, bind sender identity to authenticated credentials, and test the migration and concurrent writes. Existing two-connection tests validate API behavior only; they do not substitute for the approved database constraint.

### JSA-03 — Persisted receipt authenticity is not re-established on recovery

**Severity: MEDIUM / policy-dependent. Status: BLOCKED.**

Receipt processing verifies the supplied frame before starting the write transaction (`journal.rs#L165`, `journal.rs#L171`). The table stores the canonical body and a SHA-256 digest, but not the HMAC tag or full authenticated frame. `RawMessageRecord::into_record` checks the digest and duplicated metadata against the parsed body (`journal.rs#L485`). `pending_messages` and `get` perform those consistency checks but cannot re-run HMAC verification because no tag/key is available on recovery.

This detects ordinary inconsistencies such as a changed body with an unchanged digest, as covered by `corrupt_persisted_records_fail_closed`. SHA-256 and metadata checks do not authenticate a database whose body, digest, and mirrored metadata have all been coherently rewritten or whose rows were deleted. Recovery then cannot establish that the pending body is the originally authenticated receipt. No HMAC recovery verification is claimed: approved decision E defines the general key lifecycle direction, but storage and recovery of authentication evidence, key selection for historical records, and backup/recovery integration remain unspecified and unimplemented; database access controls are also unverified.

Historical remediation recommendation: preserve authentication evidence with the durable receipt and reverify it during recovery using a key selected outside the journal, or explicitly define and verify a storage-access integrity boundary. Decisions E and F now approve the general key and storage protection requirements, but the recovery-specific mechanism and its verification remain blocked. Do not claim at-rest tamper resistance.

## Confirmed controls

- **HMAC before receipt:** `receive_authenticated_frame` calls `verify_frame` before opening the write transaction (`journal.rs#L165-L180`). `verify_frame` validates the expected key version and calls HMAC `verify_slice` (`experiments/phase_6e/src/protocol.rs#L104`). The invalid-MAC test asserts zero records are written (`tests/journal_tests.rs#L126`). This verifies possession of the supplied key only; it does not prove sender identity or proxy traversal.
- **Conflict handling:** exact body replay returns the original stored record; a different authenticated body for the same selected identity is inserted as `REJECTED`, preserving the original (`journal.rs#L185-L217`; `tests/journal_tests.rs#L148`).
- **State transitions:** SQLite triggers allow only `RECEIVED_PENDING` to `PROCESSED` or `REJECTED`, and identity/body columns are immutable under ordinary updates (`journal.rs#L136-L154`). `process_pending` and `reject_pending` also check the expected prior state (`journal.rs#L224`, `journal.rs#L263`).
- **Concurrent processing:** `BEGIN IMMEDIATE` plus the 5-second busy timeout serializes writers. The two-connection test observes one `Processed` result and one `AlreadyProcessed` result, and checks that the latter retains the winner's outcome.
- **Restart recovery:** pending records are enumerated rather than deleted or silently discarded; close/reopen and duplicate-after-reopen behavior is tested (`journal.rs#L318`; `tests/journal_tests.rs#L66`). The test simulates restart by dropping and reopening connections; it does not kill a process or simulate power loss.
- **Isolation:** the journal owns a separate `inbound_messages` table/database and has no HTTP route integration. Phase 6F.1 report/batch storage is not used by the journal.

## Approved policy and unresolved implementation

The human-approved design decisions A–H are recorded in [phase-6f2-policy-decisions.md](phase-6f2-policy-decisions.md). Approval does not mean implemented, verified, or deployed. In particular:

- Identity uniqueness is approved but not enforced by the current schema; sender-to-credential binding is not implemented.
- Sequence scope/reset, freshness and trusted-clock failure, 30/90-day retention, verified-backup cleanup, key lifecycle, OS file controls, mTLS/IPC, and parser/route gates are approved as policies but are not implemented or independently verified here.
- Database body/digest/metadata coherence cannot be authenticated by current SHA-256 consistency checks. No HMAC recovery verification is claimed.
- HMAC verification proves possession of a shared key and message integrity only; the journal does not establish a unique sender or proxy traversal.

The journal does not enforce these policies and is not connected to live routes. Phase 6F.2 is not complete; Checkpoint B is not approved.

## Test evidence

Command run exactly as requested after remediation:

```text
cargo test --manifest-path C:\Users\Admin\Desktop\BlindMark\experiments\phase_6e\Cargo.toml --offline
```

Actual output totals:

- `health_tests`: 20 passed, 0 failed
- `http_parser_tests`: 5 passed, 0 failed
- `journal_tests`: 10 passed, 0 failed
- `protocol_tests`: 7 passed, 0 failed
- `storage_tests`: 14 passed, 0 failed
- library, binary, and doc-test harnesses: 0 tests, no failures
- Overall executed test cases: 56 passed, 0 failed

The journal regressions include processing failure leaving the row pending, terminal-state immutability, exactly-once state transition across independent connections, concurrent exact authenticated receipt without a duplicate accepted row, and concurrent conflicting authenticated reuse without overwriting the original message. Synchronization uses barriers; database connections are opened before workers wait at the barrier to avoid a setup failure stranding a peer.

## Worktree preservation

The remediation changed the journal test file and this report; the journal implementation and comprehensive execution report also contain the current callback-free processing behavior. Existing dirty Phase 6F.1 files, prior Phase 6F.2 files, untracked documents, and other untracked items were preserved. No file was staged, committed, pushed, reset, cleaned, or deleted.

## Remediation status and remaining limitations

- JSA-01 HIGH transaction escape: RESOLVED. The raw transaction callback is absent; the processing API exposes no SQLite connection capability.
- Identity tuple database uniqueness: APPROVED DESIGN; implementation remains outstanding. The existing non-unique index and behavior remain unchanged in this documentation-only update.
- Recovery authenticity: NOT ESTABLISHED. SHA-256 digest and mirrored metadata checks detect ordinary inconsistencies, but cannot authenticate a coherently rewritten database body, digest, and metadata. No HMAC recovery verification is claimed; recovery-specific authentication evidence/key handling is not implemented or verified.
- A–H policy decisions: APPROVED DESIGN; sequence, freshness, retention, directional-key, file-access-control, IPC/mTLS, parser, and route controls remain unimplemented or unverified as listed above.
- Phase 6F.2 is NOT COMPLETE. Checkpoint B is NOT APPROVED.
