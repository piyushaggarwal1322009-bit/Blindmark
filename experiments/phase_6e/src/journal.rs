//! Durable inbound coordination-message journal.
//!
//! This journal is separate from report/batch storage and is not connected to
//! an HTTP route. Callers must select the HMAC key and version before calling
//! `receive_authenticated_frame`.

use crate::protocol::{verify_frame, ProtocolError, ProtocolMessage};
use rusqlite::{params, Connection, OptionalExtension, Row, Transaction, TransactionBehavior};
use sha2::{Digest, Sha256};
use std::convert::TryInto;
use std::path::Path;
use std::time::Duration;

const JOURNAL_SCHEMA_VERSION: i64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageIdScope {
    SenderRecipientTask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageState {
    NotReceived,
    ReceivedPending,
    Processed,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageRecord {
    pub journal_id: i64,
    pub message: ProtocolMessage,
    pub canonical_body: Vec<u8>,
    pub canonical_digest: [u8; 32],
    pub state: MessageState,
    pub outcome: Option<Vec<u8>>,
    pub rejection_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReceiveOutcome {
    Recorded(MessageRecord),
    ExactDuplicate(MessageRecord),
    ConflictingDuplicate(MessageRecord),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessOutcome {
    Processed(MessageRecord),
    AlreadyProcessed(MessageRecord),
    AlreadyRejected(MessageRecord),
}

#[derive(Debug)]
pub enum JournalError {
    Sql(rusqlite::Error),
    Protocol(ProtocolError),
    PersistentPathRequired,
    UnsupportedSchemaVersion(i64),
    NotFound(i64),
    InvalidTransition,
    CorruptRecord,
    ProcessingFailed,
}

impl From<rusqlite::Error> for JournalError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sql(error)
    }
}

impl From<ProtocolError> for JournalError {
    fn from(error: ProtocolError) -> Self {
        Self::Protocol(error)
    }
}

pub struct CoordinationJournal {
    conn: Connection,
}

impl CoordinationJournal {
    /// Open a file-backed journal. The identity scope must be selected
    /// explicitly; the documented proposal is per sender, recipient, and task.
    pub fn open<P: AsRef<Path>>(
        path: P,
        identity_scope: MessageIdScope,
    ) -> Result<Self, JournalError> {
        let path = path.as_ref();
        if path == Path::new(":memory:") {
            return Err(JournalError::PersistentPathRequired);
        }
        match identity_scope {
            MessageIdScope::SenderRecipientTask => {}
        }

        let conn = Connection::open(path)?;
        conn.busy_timeout(Duration::from_secs(5))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", 2)?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        let schema_version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        match schema_version {
            0 => {
                conn.execute_batch(
                    r#"
                    CREATE TABLE inbound_messages (
                        journal_id          INTEGER PRIMARY KEY AUTOINCREMENT,
                        sender_id           TEXT NOT NULL,
                        recipient_id        TEXT NOT NULL,
                        task_id             TEXT NOT NULL,
                        message_id          TEXT NOT NULL,
                        report_id           TEXT NOT NULL,
                        batch_id            TEXT NOT NULL,
                        message_type        INTEGER NOT NULL,
                        protocol_round      INTEGER NOT NULL,
                        sequence_no         BLOB NOT NULL CHECK(length(sequence_no) = 8),
                        issued_at_ms        BLOB NOT NULL CHECK(length(issued_at_ms) = 8),
                        expires_at_ms       BLOB NOT NULL CHECK(length(expires_at_ms) = 8),
                        key_version         INTEGER NOT NULL CHECK(key_version BETWEEN 0 AND 65535),
                        canonical_digest    BLOB NOT NULL CHECK(length(canonical_digest) = 32),
                        canonical_body      BLOB NOT NULL,
                        state               TEXT NOT NULL CHECK(state IN ('RECEIVED_PENDING', 'PROCESSED', 'REJECTED')),
                        outcome             BLOB,
                        rejection_reason    TEXT,
                        created_at          TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                        updated_at          TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                    );

                    CREATE INDEX idx_inbound_identity
                        ON inbound_messages(sender_id, recipient_id, task_id, message_id, journal_id);
                    CREATE INDEX idx_inbound_state
                        ON inbound_messages(state, journal_id);

                    CREATE TRIGGER prevent_inbound_state_rewrite
                    BEFORE UPDATE OF state ON inbound_messages
                    WHEN NOT (OLD.state = 'RECEIVED_PENDING'
                              AND NEW.state IN ('PROCESSED', 'REJECTED'))
                    BEGIN
                        SELECT RAISE(ABORT, 'invalid inbound message state transition');
                    END;

                    CREATE TRIGGER prevent_inbound_identity_rewrite
                    BEFORE UPDATE OF sender_id, recipient_id, task_id, message_id,
                        report_id, batch_id, message_type, protocol_round, sequence_no,
                        issued_at_ms, expires_at_ms, key_version, canonical_digest,
                        canonical_body ON inbound_messages
                    BEGIN
                        SELECT RAISE(ABORT, 'inbound message identity is immutable');
                    END;

                    PRAGMA user_version = 1;
                    "#,
                )?;
            }
            JOURNAL_SCHEMA_VERSION => {}
            other => return Err(JournalError::UnsupportedSchemaVersion(other)),
        }

        Ok(Self { conn })
    }

    /// Verify the frame before starting a transaction or persisting any data.
    pub fn receive_authenticated_frame(
        &mut self,
        frame: &[u8],
        expected_key_version: u16,
        selected_key: &[u8],
    ) -> Result<ReceiveOutcome, JournalError> {
        let message = verify_frame(frame, expected_key_version, selected_key)?;
        let body_len = frame
            .len()
            .checked_sub(crate::protocol::HMAC_SHA256_BYTES)
            .ok_or(JournalError::CorruptRecord)?;
        let canonical_body = frame[..body_len].to_vec();
        if message.canonical_body()? != canonical_body {
            return Err(JournalError::CorruptRecord);
        }
        let canonical_digest = sha256(&canonical_body);

        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing = load_identity_records(&tx, &message)?;
        if let Some(record) = existing
            .iter()
            .find(|record| record.canonical_body == canonical_body)
        {
            let record = record.clone();
            tx.commit()?;
            return Ok(ReceiveOutcome::ExactDuplicate(record));
        }

        let (state, rejection_reason) = if existing.is_empty() {
            (MessageState::ReceivedPending, None)
        } else {
            (
                MessageState::Rejected,
                Some("message_id reused with different canonical bytes".to_owned()),
            )
        };
        let journal_id = insert_message(
            &tx,
            &message,
            &canonical_body,
            &canonical_digest,
            state,
            rejection_reason.as_deref(),
        )?;
        tx.commit()?;

        let record = self.get(journal_id)?.ok_or(JournalError::CorruptRecord)?;
        if state == MessageState::Rejected {
            Ok(ReceiveOutcome::ConflictingDuplicate(record))
        } else {
            Ok(ReceiveOutcome::Recorded(record))
        }
    }

    /// Persist a completed processing result and mark the message processed.
    /// Processing happens outside the journal; an error leaves the row pending.
    /// The journal owns the complete transaction and exposes no SQLite handle.
    pub fn process_pending(
        &mut self,
        journal_id: i64,
        processing_result: Result<Vec<u8>, JournalError>,
    ) -> Result<ProcessOutcome, JournalError> {
        let outcome = processing_result?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let record = load_record(&tx, journal_id)?.ok_or(JournalError::NotFound(journal_id))?;
        match record.state {
            MessageState::Processed => {
                tx.commit()?;
                return Ok(ProcessOutcome::AlreadyProcessed(record));
            }
            MessageState::Rejected => {
                tx.commit()?;
                return Ok(ProcessOutcome::AlreadyRejected(record));
            }
            MessageState::NotReceived => return Err(JournalError::CorruptRecord),
            MessageState::ReceivedPending => {}
        }

        let changed = tx.execute(
            "UPDATE inbound_messages SET state = 'PROCESSED', outcome = ?1, rejection_reason = NULL, updated_at = CURRENT_TIMESTAMP WHERE journal_id = ?2 AND state = 'RECEIVED_PENDING'",
            params![outcome, journal_id],
        )?;
        if changed != 1 {
            return Err(JournalError::InvalidTransition);
        }
        tx.commit()?;

        let record = self.get(journal_id)?.ok_or(JournalError::CorruptRecord)?;
        Ok(ProcessOutcome::Processed(record))
    }

    pub fn reject_pending(
        &mut self,
        journal_id: i64,
        reason: &str,
    ) -> Result<MessageRecord, JournalError> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let record = load_record(&tx, journal_id)?.ok_or(JournalError::NotFound(journal_id))?;
        match record.state {
            MessageState::Rejected => {
                tx.commit()?;
                return Ok(record);
            }
            MessageState::ReceivedPending => {}
            MessageState::Processed | MessageState::NotReceived => {
                return Err(JournalError::InvalidTransition);
            }
        }

        let changed = tx.execute(
            "UPDATE inbound_messages SET state = 'REJECTED', outcome = NULL, rejection_reason = ?1, updated_at = CURRENT_TIMESTAMP WHERE journal_id = ?2 AND state = 'RECEIVED_PENDING'",
            params![reason, journal_id],
        )?;
        if changed != 1 {
            return Err(JournalError::InvalidTransition);
        }
        tx.commit()?;
        self.get(journal_id)?.ok_or(JournalError::CorruptRecord)
    }

    pub fn get(&self, journal_id: i64) -> Result<Option<MessageRecord>, JournalError> {
        let raw = self
            .conn
            .query_row(
                &format!("{} WHERE journal_id = ?1", RECORD_SELECT),
                params![journal_id],
                RawMessageRecord::from_row,
            )
            .optional()?;
        raw.map(RawMessageRecord::into_record).transpose()
    }

    /// Return the original record state for an identity, or NOT_RECEIVED when
    /// no durable receipt exists. Conflicting attempts are separate REJECTED rows.
    pub fn state_for(&self, message: &ProtocolMessage) -> Result<MessageState, JournalError> {
        message.canonical_body()?;
        let records = load_identity_records(&self.conn, message)?;
        Ok(records
            .first()
            .map(|record| record.state)
            .unwrap_or(MessageState::NotReceived))
    }

    pub fn pending_messages(&self) -> Result<Vec<MessageRecord>, JournalError> {
        let ids = {
            let mut statement = self.conn.prepare(
                "SELECT journal_id FROM inbound_messages WHERE state = 'RECEIVED_PENDING' ORDER BY journal_id",
            )?;
            let rows = statement.query_map([], |row| row.get::<_, i64>(0))?;
            rows.collect::<Result<Vec<_>, _>>()?
        };
        ids.into_iter()
            .map(|id| self.get(id)?.ok_or(JournalError::CorruptRecord))
            .collect()
    }

    pub fn record_count(&self) -> Result<i64, JournalError> {
        Ok(self
            .conn
            .query_row("SELECT COUNT(*) FROM inbound_messages", [], |row| {
                row.get(0)
            })?)
    }
}

const RECORD_SELECT: &str = "SELECT journal_id, sender_id, recipient_id, task_id, message_id, report_id, batch_id, message_type, protocol_round, sequence_no, issued_at_ms, expires_at_ms, key_version, canonical_digest, canonical_body, state, outcome, rejection_reason FROM inbound_messages";

fn load_identity_records(
    conn: &Connection,
    message: &ProtocolMessage,
) -> Result<Vec<MessageRecord>, JournalError> {
    let identity = message_identity(message);
    let mut statement = conn.prepare(&format!(
        "{} WHERE sender_id = ?1 AND recipient_id = ?2 AND task_id = ?3 AND message_id = ?4 ORDER BY journal_id",
        RECORD_SELECT
    ))?;
    let rows = statement.query_map(
        params![identity.0, identity.1, identity.2, identity.3],
        RawMessageRecord::from_row,
    )?;
    rows.map(|row| row.map_err(JournalError::from)?.into_record())
        .collect()
}

fn load_record(
    tx: &Transaction<'_>,
    journal_id: i64,
) -> Result<Option<MessageRecord>, JournalError> {
    let raw = tx
        .query_row(
            &format!("{} WHERE journal_id = ?1", RECORD_SELECT),
            params![journal_id],
            RawMessageRecord::from_row,
        )
        .optional()?;
    raw.map(RawMessageRecord::into_record).transpose()
}

fn insert_message(
    tx: &Transaction<'_>,
    message: &ProtocolMessage,
    canonical_body: &[u8],
    canonical_digest: &[u8; 32],
    state: MessageState,
    rejection_reason: Option<&str>,
) -> Result<i64, JournalError> {
    tx.execute(
        "INSERT INTO inbound_messages (sender_id, recipient_id, task_id, message_id, report_id, batch_id, message_type, protocol_round, sequence_no, issued_at_ms, expires_at_ms, key_version, canonical_digest, canonical_body, state, rejection_reason) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
        params![
            message.sender_id,
            message.recipient_id,
            message.task_id,
            message.message_id,
            message.report_id,
            message.batch_id,
            message.message_type as u16,
            message.protocol_round,
            message.sequence_no.to_be_bytes().as_slice(),
            message.issued_at_ms.to_be_bytes().as_slice(),
            message.expires_at_ms.to_be_bytes().as_slice(),
            message.key_version,
            canonical_digest.as_slice(),
            canonical_body,
            state.as_db_value(),
            rejection_reason,
        ],
    )?;
    Ok(tx.last_insert_rowid())
}

fn message_identity(message: &ProtocolMessage) -> (String, String, String, String) {
    (
        message.sender_id.clone(),
        message.recipient_id.clone(),
        message.task_id.clone(),
        message.message_id.clone(),
    )
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    let digest = Sha256::digest(bytes);
    let mut result = [0u8; 32];
    result.copy_from_slice(&digest);
    result
}

impl MessageState {
    fn as_db_value(self) -> &'static str {
        match self {
            Self::NotReceived => "NOT_RECEIVED",
            Self::ReceivedPending => "RECEIVED_PENDING",
            Self::Processed => "PROCESSED",
            Self::Rejected => "REJECTED",
        }
    }

    fn from_db_value(value: &str) -> Result<Self, JournalError> {
        match value {
            "RECEIVED_PENDING" => Ok(Self::ReceivedPending),
            "PROCESSED" => Ok(Self::Processed),
            "REJECTED" => Ok(Self::Rejected),
            _ => Err(JournalError::CorruptRecord),
        }
    }
}

struct RawMessageRecord {
    journal_id: i64,
    sender_id: String,
    recipient_id: String,
    task_id: String,
    message_id: String,
    report_id: String,
    batch_id: String,
    message_type: i64,
    protocol_round: i64,
    sequence_no: Vec<u8>,
    issued_at_ms: Vec<u8>,
    expires_at_ms: Vec<u8>,
    key_version: i64,
    canonical_digest: Vec<u8>,
    canonical_body: Vec<u8>,
    state: String,
    outcome: Option<Vec<u8>>,
    rejection_reason: Option<String>,
}

impl RawMessageRecord {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            journal_id: row.get(0)?,
            sender_id: row.get(1)?,
            recipient_id: row.get(2)?,
            task_id: row.get(3)?,
            message_id: row.get(4)?,
            report_id: row.get(5)?,
            batch_id: row.get(6)?,
            message_type: row.get(7)?,
            protocol_round: row.get(8)?,
            sequence_no: row.get(9)?,
            issued_at_ms: row.get(10)?,
            expires_at_ms: row.get(11)?,
            key_version: row.get(12)?,
            canonical_digest: row.get(13)?,
            canonical_body: row.get(14)?,
            state: row.get(15)?,
            outcome: row.get(16)?,
            rejection_reason: row.get(17)?,
        })
    }

    fn into_record(self) -> Result<MessageRecord, JournalError> {
        let message = ProtocolMessage::from_canonical_body(&self.canonical_body)?;
        let canonical_digest: [u8; 32] = self
            .canonical_digest
            .as_slice()
            .try_into()
            .map_err(|_| JournalError::CorruptRecord)?;
        if sha256(&self.canonical_body) != canonical_digest
            || message.sender_id != self.sender_id
            || message.recipient_id != self.recipient_id
            || message.task_id != self.task_id
            || message.message_id != self.message_id
            || message.report_id != self.report_id
            || message.batch_id != self.batch_id
            || i64::from(message.message_type as u16) != self.message_type
            || i64::from(message.protocol_round) != self.protocol_round
            || message.sequence_no.to_be_bytes().as_slice() != self.sequence_no
            || message.issued_at_ms.to_be_bytes().as_slice() != self.issued_at_ms
            || message.expires_at_ms.to_be_bytes().as_slice() != self.expires_at_ms
            || i64::from(message.key_version) != self.key_version
        {
            return Err(JournalError::CorruptRecord);
        }
        let state = MessageState::from_db_value(&self.state)?;
        match state {
            MessageState::ReceivedPending
                if self.outcome.is_some() || self.rejection_reason.is_some() =>
            {
                return Err(JournalError::CorruptRecord);
            }
            MessageState::Processed
                if self.outcome.is_none() || self.rejection_reason.is_some() =>
            {
                return Err(JournalError::CorruptRecord);
            }
            MessageState::Rejected if self.rejection_reason.is_none() || self.outcome.is_some() => {
                return Err(JournalError::CorruptRecord);
            }
            _ => {}
        }

        Ok(MessageRecord {
            journal_id: self.journal_id,
            message,
            canonical_body: self.canonical_body,
            canonical_digest,
            state,
            outcome: self.outcome,
            rejection_reason: self.rejection_reason,
        })
    }
}
