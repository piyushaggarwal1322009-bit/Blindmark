//! SQLite storage foundation for Phase 6F.1 (Leader & Helper).
//!
//! This module implements a minimal, durable storage layer using SQLite in WAL
//! mode. It provides:
//!   * database initialization with required pragmas;
//!   * schema migrations for the minimal report/batch state used by the protocol;
//!   * a transactional `commit_report` that enforces exact row counts and a
//!     durable `COMMIT_DECIDED` batch decision;
//!   * a prototype-only aggregate reconstruction that reads committed shares
//!     exclusively and does not claim to be real Prio3 aggregation.
//!
//! The implementation stays inside the `blindmark-phase-6e` crate and does not
//! touch any other phase. It uses the bundled `rusqlite` SQLite library.

use rusqlite::{params, Connection, OptionalExtension, Result, Transaction, TransactionBehavior};
use std::path::Path;

/// High-level wrapper around a SQLite connection.
#[derive(Debug)]
pub struct Database {
    pub conn: Connection,
}

impl Database {
    /// Open or create a SQLite database at `path`.
    ///
    /// Database paths are caller-controlled and explicit.  The caller is
    /// responsible for choosing an on-disk path when persistence is required.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", 2)?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        let db = Self { conn };
        db.run_migrations()?;
        Ok(db)
    }

    /// Execute the full set of CREATE TABLE statements.
    fn run_migrations(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = 2;

            CREATE TABLE IF NOT EXISTS reports (
                id          INTEGER PRIMARY KEY,
                task_id     INTEGER NOT NULL,
                batch_id    INTEGER NOT NULL,
                state       TEXT NOT NULL,
                UNIQUE(id, batch_id)
            );

            CREATE TABLE IF NOT EXISTS staged_output_shares (
                id          INTEGER PRIMARY KEY,
                report_id   INTEGER NOT NULL,
                batch_id    INTEGER NOT NULL,
                share       BLOB   NOT NULL,
                FOREIGN KEY(report_id) REFERENCES reports(id) ON DELETE CASCADE,
                UNIQUE(report_id, batch_id)
            );

            CREATE TABLE IF NOT EXISTS committed_output_shares (
                id          INTEGER PRIMARY KEY,
                report_id   INTEGER NOT NULL,
                batch_id    INTEGER NOT NULL,
                share       BLOB   NOT NULL,
                FOREIGN KEY(report_id) REFERENCES reports(id) ON DELETE CASCADE,
                UNIQUE(report_id, batch_id)
            );

            CREATE TABLE IF NOT EXISTS batches (
                id          INTEGER PRIMARY KEY,
                decision    TEXT NOT NULL CHECK (decision IN ('COMMIT_DECIDED', 'ABORTED'))
            );

            CREATE TABLE IF NOT EXISTS aggregate_checkpoints (
                id          INTEGER PRIMARY KEY,
                task_id     INTEGER NOT NULL,
                aggregate   BLOB NOT NULL,
                UNIQUE(task_id)
            );

            CREATE INDEX IF NOT EXISTS idx_reports_state ON reports(state);
            CREATE INDEX IF NOT EXISTS idx_staged_report ON staged_output_shares(report_id, batch_id);
            CREATE INDEX IF NOT EXISTS idx_committed_report ON committed_output_shares(report_id, batch_id);
            CREATE INDEX IF NOT EXISTS idx_batches_decision ON batches(decision);

            CREATE TRIGGER IF NOT EXISTS prevent_abort
                BEFORE UPDATE OF decision ON batches
                WHEN OLD.decision = 'COMMIT_DECIDED' AND NEW.decision = 'ABORTED'
            BEGIN
                SELECT RAISE(ABORT, 'cannot transition COMMIT_DECIDED to ABORTED');
            END;
            "#,
        )?;
        Ok(())
    }

    /// Verify that a batch has a durable `COMMIT_DECIDED` decision.
    fn ensure_commit_decided_tx(tx: &Transaction<'_>, batch_id: i64) -> Result<()> {
        let decision: Option<String> = tx
            .query_row(
                "SELECT decision FROM batches WHERE id = ?1",
                params![batch_id],
                |row| row.get(0),
            )
            .optional()?;

        match decision.as_deref() {
            Some("COMMIT_DECIDED") => Ok(()),
            _ => Err(rusqlite::Error::InvalidQuery),
        }
    }

    /// Commit a staged report into the committed archive.
    ///
    /// This API intentionally uses only `(report_id, batch_id)` for idempotency.
    /// It cannot verify payload equivalence on retry because the incoming share
    /// bytes are not part of the API contract; therefore this function does not
    /// claim byte-for-byte payload idempotency.
    pub fn commit_report(&mut self, report_id: i64, batch_id: i64) -> Result<()> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Err(err) = Self::ensure_commit_decided_tx(&tx, batch_id) {
            tx.rollback()?;
            return Err(err);
        }

        let report_count: i64 = tx.query_row(
            "SELECT COUNT(*) FROM reports WHERE id = ?1",
            params![report_id],
            |row| row.get(0),
        )?;
        if report_count != 1 {
            tx.rollback()?;
            return Err(rusqlite::Error::InvalidQuery);
        }

        let report_row: (i64, String) = tx.query_row(
            "SELECT batch_id, state FROM reports WHERE id = ?1",
            params![report_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let (report_batch_id, report_state) = report_row;

        let committed_count: i64 = tx.query_row(
            "SELECT COUNT(*) FROM committed_output_shares WHERE report_id = ?1",
            params![report_id],
            |row| row.get(0),
        )?;

        if committed_count > 1 {
            tx.rollback()?;
            return Err(rusqlite::Error::InvalidQuery);
        }

        if committed_count == 1 {
            let existing_batch: i64 = tx.query_row(
                "SELECT batch_id FROM committed_output_shares WHERE report_id = ?1",
                params![report_id],
                |row| row.get(0),
            )?;
            let staged_count: i64 = tx.query_row(
                "SELECT COUNT(*) FROM staged_output_shares WHERE report_id = ?1",
                params![report_id],
                |row| row.get(0),
            )?;

            if existing_batch != batch_id
                || report_batch_id != batch_id
                || report_state != "COMMITTED"
                || staged_count != 0
            {
                tx.rollback()?;
                return Err(rusqlite::Error::InvalidQuery);
            }

            tx.commit()?;
            return Ok(());
        }

        if report_batch_id != batch_id {
            tx.rollback()?;
            return Err(rusqlite::Error::InvalidQuery);
        }

        if report_state != "STAGED" {
            tx.rollback()?;
            return Err(rusqlite::Error::InvalidQuery);
        }

        let other_staged_rows: i64 = tx.query_row(
            "SELECT COUNT(*) FROM staged_output_shares WHERE report_id = ?1 AND batch_id != ?2",
            params![report_id, batch_id],
            |row| row.get(0),
        )?;
        if other_staged_rows != 0 {
            tx.rollback()?;
            return Err(rusqlite::Error::InvalidQuery);
        }

        let staged_count: i64 = tx.query_row(
            "SELECT COUNT(*) FROM staged_output_shares WHERE report_id = ?1 AND batch_id = ?2",
            params![report_id, batch_id],
            |row| row.get(0),
        )?;
        if staged_count != 1 {
            tx.rollback()?;
            return Err(rusqlite::Error::InvalidQuery);
        }

        let staged_share: Vec<u8> = tx.query_row(
            "SELECT share FROM staged_output_shares WHERE report_id = ?1 AND batch_id = ?2",
            params![report_id, batch_id],
            |row| row.get(0),
        )?;

        let inserted = tx.execute(
            "INSERT INTO committed_output_shares (report_id, batch_id, share) VALUES (?1, ?2, ?3)",
            params![report_id, batch_id, staged_share],
        )?;
        if inserted != 1 {
            tx.rollback()?;
            return Err(rusqlite::Error::InvalidQuery);
        }

        let deleted = tx.execute(
            "DELETE FROM staged_output_shares WHERE report_id = ?1 AND batch_id = ?2",
            params![report_id, batch_id],
        )?;
        if deleted != 1 {
            tx.rollback()?;
            return Err(rusqlite::Error::InvalidQuery);
        }

        let updated = tx.execute(
            "UPDATE reports SET state = 'COMMITTED' WHERE id = ?1 AND batch_id = ?2 AND state = 'STAGED'",
            params![report_id, batch_id],
        )?;
        if updated != 1 {
            tx.rollback()?;
            return Err(rusqlite::Error::InvalidQuery);
        }

        tx.commit()?;
        Ok(())
    }

    /// Prototype-only aggregate reconstruction for local recovery checks.
    ///
    /// This function reads only committed output shares and folds them with a
    /// simple XOR placeholder. It is intentionally not a representation of real
    /// Prio3 aggregation and must never be described as production VDAF output.
    pub fn rebuild_aggregate(&self, task_id: i64) -> Result<Vec<u8>> {
        let mut stmt = self.conn.prepare(
            "SELECT committed_output_shares.report_id, committed_output_shares.batch_id, committed_output_shares.share, reports.batch_id AS report_batch_id, reports.state, batches.decision FROM committed_output_shares JOIN reports ON committed_output_shares.report_id = reports.id LEFT JOIN batches ON committed_output_shares.batch_id = batches.id WHERE reports.task_id = ?1 ORDER BY committed_output_shares.report_id ASC, committed_output_shares.batch_id ASC",
        )?;

        let mut aggregate: Vec<u8> = Vec::new();
        let mut expected_len: Option<usize> = None;
        let mut rows = stmt.query(params![task_id])?;
        let mut first_share_seen = false;

        while let Some(row) = rows.next()? {
            let report_id: i64 = row.get(0)?;
            let batch_id: i64 = row.get(1)?;
            let share: Vec<u8> = row.get(2)?;
            let report_batch_id: i64 = row.get(3)?;
            let report_state: String = row.get(4)?;
            let batch_decision: Option<String> = row.get(5)?;

            if report_state != "COMMITTED" {
                return Err(rusqlite::Error::InvalidQuery);
            }
            if batch_id != report_batch_id {
                return Err(rusqlite::Error::InvalidQuery);
            }
            match batch_decision.as_deref() {
                Some("COMMIT_DECIDED") => {}
                _ => return Err(rusqlite::Error::InvalidQuery),
            }

            if let Some(len) = expected_len {
                if share.len() != len {
                    return Err(rusqlite::Error::InvalidQuery);
                }
            } else {
                expected_len = Some(share.len());
            }

            if !first_share_seen {
                aggregate = share.clone();
                first_share_seen = true;
            } else {
                for (a, b) in aggregate.iter_mut().zip(share.iter()) {
                    *a ^= b;
                }
            }

            let _ = report_id;
        }

        Ok(aggregate)
    }
}
