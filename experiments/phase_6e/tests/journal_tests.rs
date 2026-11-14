use blindmark_phase_6e::journal::{
    CoordinationJournal, JournalError, MessageIdScope, MessageState, ProcessOutcome, ReceiveOutcome,
};
use blindmark_phase_6e::protocol::{sign_frame, MessageType, ProtocolMessage};
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

fn make_journal_path(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("blindmark_journal_{name}_{nonce}.db"))
}

fn cleanup(path: &Path) {
    for suffix in ["", "-wal", "-shm"] {
        let _ = fs::remove_file(format!("{}{}", path.display(), suffix));
    }
}

fn open_journal(path: &Path) -> CoordinationJournal {
    CoordinationJournal::open(path, MessageIdScope::SenderRecipientTask).unwrap()
}

fn sample_message(payload: &[u8]) -> ProtocolMessage {
    let issued_at_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    ProtocolMessage {
        key_version: 7,
        message_type: MessageType::LeaderToHelperRequest,
        protocol_round: 1,
        sender_id: "leader".into(),
        recipient_id: "helper".into(),
        task_id: "task-7".into(),
        report_id: "report-9".into(),
        batch_id: "batch-3".into(),
        message_id: "message-11".into(),
        sequence_no: 12,
        issued_at_ms,
        expires_at_ms: issued_at_ms + 60_000,
        payload: payload.to_vec(),
    }
}

fn authenticated_frame(message: &ProtocolMessage) -> Vec<u8> {
    sign_frame(message, &[0x42; 32]).unwrap()
}

fn journal_id(outcome: &ReceiveOutcome) -> i64 {
    match outcome {
        ReceiveOutcome::Recorded(record)
        | ReceiveOutcome::ExactDuplicate(record)
        | ReceiveOutcome::ConflictingDuplicate(record) => record.journal_id,
    }
}

#[test]
fn authenticated_receipt_survives_restart_and_pending_recovery() {
    let path = make_journal_path("restart");
    let message = sample_message(&[1, 2, 3]);
    let frame = authenticated_frame(&message);
    let first_id;

    {
        let mut journal = open_journal(&path);
        assert_eq!(
            journal.state_for(&message).unwrap(),
            MessageState::NotReceived
        );
        let receipt = journal
            .receive_authenticated_frame(&frame, 7, &[0x42; 32])
            .unwrap();
        assert!(matches!(receipt, ReceiveOutcome::Recorded(_)));
        first_id = journal_id(&receipt);
        let record = journal.get(first_id).unwrap().unwrap();
        assert_eq!(record.state, MessageState::ReceivedPending);
        assert_eq!(record.message.key_version, 7);
        assert_eq!(record.message.sequence_no, 12);
        assert_eq!(record.message.expires_at_ms, message.expires_at_ms);
    }

    {
        let mut journal = open_journal(&path);
        let pending = journal.pending_messages().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].journal_id, first_id);

        let duplicate = journal
            .receive_authenticated_frame(&frame, 7, &[0x42; 32])
            .unwrap();
        assert!(matches!(duplicate, ReceiveOutcome::ExactDuplicate(_)));
        assert_eq!(journal_id(&duplicate), first_id);

        let processed = journal
            .process_pending(first_id, Ok(b"durable-result".to_vec()))
            .unwrap();
        match processed {
            ProcessOutcome::Processed(record) => {
                assert_eq!(record.state, MessageState::Processed);
                assert_eq!(record.outcome.as_deref(), Some(&b"durable-result"[..]));
            }
            other => panic!("unexpected processing result: {:?}", other),
        }
        assert!(journal.pending_messages().unwrap().is_empty());
    }

    {
        let journal = open_journal(&path);
        let record = journal.get(first_id).unwrap().unwrap();
        assert_eq!(record.state, MessageState::Processed);
        assert_eq!(record.outcome.as_deref(), Some(&b"durable-result"[..]));
    }

    cleanup(&path);
}

#[test]
fn invalid_hmac_is_rejected_before_any_journal_record_is_written() {
    let path = make_journal_path("bad-mac");
    let message = sample_message(b"authenticated");
    let mut frame = authenticated_frame(&message);
    let last = frame.len() - 1;
    frame[last] ^= 0x80;

    let mut journal = open_journal(&path);
    assert!(matches!(
        journal.receive_authenticated_frame(&frame, 7, &[0x42; 32]),
        Err(JournalError::Protocol(_))
    ));
    assert_eq!(journal.record_count().unwrap(), 0);
    assert_eq!(
        journal.state_for(&message).unwrap(),
        MessageState::NotReceived
    );
    drop(journal);
    cleanup(&path);
}

#[test]
fn exact_duplicates_are_idempotent_and_conflicting_reuse_is_rejected() {
    let path = make_journal_path("duplicates");
    let first_message = sample_message(b"first");
    let conflicting_message = sample_message(b"different");
    let first_frame = authenticated_frame(&first_message);
    let conflict_frame = authenticated_frame(&conflicting_message);

    let mut journal = open_journal(&path);
    let first = journal
        .receive_authenticated_frame(&first_frame, 7, &[0x42; 32])
        .unwrap();
    let first_id = journal_id(&first);
    assert!(matches!(first, ReceiveOutcome::Recorded(_)));

    let duplicate = journal
        .receive_authenticated_frame(&first_frame, 7, &[0x42; 32])
        .unwrap();
    assert!(matches!(duplicate, ReceiveOutcome::ExactDuplicate(_)));
    assert_eq!(journal.record_count().unwrap(), 1);

    let conflict = journal
        .receive_authenticated_frame(&conflict_frame, 7, &[0x42; 32])
        .unwrap();
    assert!(matches!(conflict, ReceiveOutcome::ConflictingDuplicate(_)));
    let conflict_id = journal_id(&conflict);
    assert_ne!(conflict_id, first_id);
    assert_eq!(
        journal.get(conflict_id).unwrap().unwrap().state,
        MessageState::Rejected
    );
    assert_eq!(
        journal.get(first_id).unwrap().unwrap().state,
        MessageState::ReceivedPending
    );
    assert_eq!(journal.record_count().unwrap(), 2);

    let repeated_conflict = journal
        .receive_authenticated_frame(&conflict_frame, 7, &[0x42; 32])
        .unwrap();
    assert!(matches!(
        repeated_conflict,
        ReceiveOutcome::ExactDuplicate(_)
    ));
    assert_eq!(journal_id(&repeated_conflict), conflict_id);
    drop(journal);
    cleanup(&path);
}

#[test]
fn concurrent_receipt_of_the_same_authenticated_message_is_idempotent() {
    let path = make_journal_path("concurrent-exact-receipt");
    let message = sample_message(b"same authenticated body");
    let frame = authenticated_frame(&message);
    let barrier = Arc::new(Barrier::new(2));
    let mut workers = Vec::new();
    for mut journal in [open_journal(&path), open_journal(&path)] {
        let frame = frame.clone();
        let barrier = Arc::clone(&barrier);
        workers.push(thread::spawn(move || {
            barrier.wait();
            journal
                .receive_authenticated_frame(&frame, 7, &[0x42; 32])
                .unwrap()
        }));
    }

    let receipts: Vec<ReceiveOutcome> = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect();
    assert_eq!(
        receipts
            .iter()
            .filter(|receipt| matches!(receipt, ReceiveOutcome::Recorded(_)))
            .count(),
        1
    );
    assert_eq!(
        receipts
            .iter()
            .filter(|receipt| matches!(receipt, ReceiveOutcome::ExactDuplicate(_)))
            .count(),
        1
    );
    assert_eq!(journal_id(&receipts[0]), journal_id(&receipts[1]));

    let journal = open_journal(&path);
    assert_eq!(journal.record_count().unwrap(), 1);
    assert_eq!(
        journal
            .get(journal_id(&receipts[0]))
            .unwrap()
            .unwrap()
            .state,
        MessageState::ReceivedPending
    );
    drop(journal);
    cleanup(&path);
}

#[test]
fn concurrent_conflicting_receipts_preserve_the_original_message() {
    let path = make_journal_path("concurrent-conflicting-receipt");
    let original_message = sample_message(b"original authenticated body");
    let conflicting_message = sample_message(b"conflicting authenticated body");
    let original_frame = authenticated_frame(&original_message);
    let conflicting_frame = authenticated_frame(&conflicting_message);
    let original_body = original_message.canonical_body().unwrap();

    let mut journal = open_journal(&path);
    let original_id = journal_id(
        &journal
            .receive_authenticated_frame(&original_frame, 7, &[0x42; 32])
            .unwrap(),
    );
    drop(journal);

    let barrier = Arc::new(Barrier::new(2));
    let mut workers = Vec::new();
    for mut journal in [open_journal(&path), open_journal(&path)] {
        let frame = conflicting_frame.clone();
        let barrier = Arc::clone(&barrier);
        workers.push(thread::spawn(move || {
            barrier.wait();
            journal
                .receive_authenticated_frame(&frame, 7, &[0x42; 32])
                .unwrap()
        }));
    }

    let receipts: Vec<ReceiveOutcome> = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect();
    assert_eq!(
        receipts
            .iter()
            .filter(|receipt| matches!(receipt, ReceiveOutcome::ConflictingDuplicate(_)))
            .count(),
        1
    );
    assert_eq!(
        receipts
            .iter()
            .filter(|receipt| matches!(receipt, ReceiveOutcome::ExactDuplicate(_)))
            .count(),
        1
    );

    let journal = open_journal(&path);
    let original = journal.get(original_id).unwrap().unwrap();
    assert_eq!(original.canonical_body, original_body);
    assert_eq!(original.state, MessageState::ReceivedPending);
    assert_eq!(journal.record_count().unwrap(), 2);
    drop(journal);
    cleanup(&path);
}

#[test]
fn processed_and_rejected_messages_are_terminal() {
    let path = make_journal_path("terminal");
    let processed_message = sample_message(b"process");
    let rejected_message = ProtocolMessage {
        message_id: "message-12".into(),
        ..sample_message(b"reject")
    };
    let mut journal = open_journal(&path);
    let processed_id = journal_id(
        &journal
            .receive_authenticated_frame(&authenticated_frame(&processed_message), 7, &[0x42; 32])
            .unwrap(),
    );
    let rejected_id = journal_id(
        &journal
            .receive_authenticated_frame(&authenticated_frame(&rejected_message), 7, &[0x42; 32])
            .unwrap(),
    );

    let first_processing = journal.process_pending(processed_id, Ok(vec![1])).unwrap();
    assert!(matches!(first_processing, ProcessOutcome::Processed(_)));
    let retry = journal.process_pending(processed_id, Ok(vec![2])).unwrap();
    match retry {
        ProcessOutcome::AlreadyProcessed(record) => {
            assert_eq!(record.outcome.as_deref(), Some(&[1][..]));
        }
        other => panic!("unexpected processing result: {:?}", other),
    }

    let rejected = journal
        .reject_pending(rejected_id, "validation failure")
        .unwrap();
    assert_eq!(rejected.state, MessageState::Rejected);
    assert_eq!(
        journal
            .reject_pending(rejected_id, "different retry reason")
            .unwrap(),
        rejected
    );
    assert!(matches!(
        journal.process_pending(rejected_id, Ok(vec![3])).unwrap(),
        ProcessOutcome::AlreadyRejected(_)
    ));
    assert!(matches!(
        journal.reject_pending(processed_id, "cannot reverse"),
        Err(JournalError::InvalidTransition)
    ));
    drop(journal);
    cleanup(&path);
}

#[test]
fn processing_failure_leaves_message_pending() {
    let path = make_journal_path("rollback");
    let message = sample_message(b"rollback");
    let mut journal = open_journal(&path);
    let id = journal_id(
        &journal
            .receive_authenticated_frame(&authenticated_frame(&message), 7, &[0x42; 32])
            .unwrap(),
    );

    let result = journal.process_pending(id, Err(JournalError::ProcessingFailed));
    assert!(matches!(result, Err(JournalError::ProcessingFailed)));
    let record = journal.get(id).unwrap().unwrap();
    assert_eq!(record.state, MessageState::ReceivedPending);
    assert_eq!(record.outcome, None);
    assert_eq!(record.rejection_reason, None);
    drop(journal);
    cleanup(&path);
}

#[test]
fn concurrent_processing_transitions_the_message_once() {
    let path = make_journal_path("concurrent");
    let message = sample_message(b"concurrent");
    let mut journal = open_journal(&path);
    let id = journal_id(
        &journal
            .receive_authenticated_frame(&authenticated_frame(&message), 7, &[0x42; 32])
            .unwrap(),
    );
    drop(journal);

    let barrier = Arc::new(Barrier::new(2));
    let mut workers = Vec::new();
    for (mut journal, processing_result) in [
        (open_journal(&path), vec![0x10]),
        (open_journal(&path), vec![0x20]),
    ] {
        let barrier = Arc::clone(&barrier);
        workers.push(thread::spawn(move || {
            barrier.wait();
            journal.process_pending(id, Ok(processing_result)).unwrap()
        }));
    }

    let outcomes: Vec<ProcessOutcome> = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect();
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| matches!(outcome, ProcessOutcome::Processed(_)))
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| matches!(outcome, ProcessOutcome::AlreadyProcessed(_)))
            .count(),
        1
    );
    let winning_outcome = match outcomes
        .iter()
        .find(|outcome| matches!(outcome, ProcessOutcome::Processed(_)))
        .unwrap()
    {
        ProcessOutcome::Processed(record) => record.outcome.clone().unwrap(),
        _ => unreachable!(),
    };
    let retry_outcome = match outcomes
        .iter()
        .find(|outcome| matches!(outcome, ProcessOutcome::AlreadyProcessed(_)))
        .unwrap()
    {
        ProcessOutcome::AlreadyProcessed(record) => record.outcome.as_ref().unwrap(),
        _ => unreachable!(),
    };
    assert_eq!(&winning_outcome, retry_outcome);

    let journal = open_journal(&path);
    let processed = journal.get(id).unwrap().unwrap();
    assert_eq!(processed.state, MessageState::Processed);
    assert_eq!(processed.outcome.as_ref(), Some(&winning_outcome));
    drop(journal);
    cleanup(&path);
}

#[test]
fn corrupt_persisted_records_fail_closed() {
    let path = make_journal_path("corrupt");
    let message = sample_message(b"integrity");
    let mut journal = open_journal(&path);
    let id = journal_id(
        &journal
            .receive_authenticated_frame(&authenticated_frame(&message), 7, &[0x42; 32])
            .unwrap(),
    );
    drop(journal);

    let conn = Connection::open(&path).unwrap();
    conn.execute_batch("DROP TRIGGER prevent_inbound_identity_rewrite;")
        .unwrap();
    conn.execute(
        "UPDATE inbound_messages SET canonical_digest = zeroblob(32) WHERE journal_id = ?1",
        [id],
    )
    .unwrap();
    drop(conn);

    let journal = open_journal(&path);
    assert!(matches!(journal.get(id), Err(JournalError::CorruptRecord)));
    drop(journal);
    cleanup(&path);
}

#[test]
fn journal_requires_a_persistent_path() {
    assert!(matches!(
        CoordinationJournal::open(":memory:", MessageIdScope::SenderRecipientTask),
        Err(JournalError::PersistentPathRequired)
    ));
}
