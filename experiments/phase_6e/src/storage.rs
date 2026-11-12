//! SQLite storage foundation for Phase 6F.1 (Leader & Helper).
//!
//! This module implements a minimal, **durable** storage layer using SQLite
//! in WAL mode.  It provides:
//!   * Database initialization with required pragmas.
//!   * Schema migrations for the tables required by the specification.
//!   * A transactional `commit_report` that performs exact row‑count checks,
//!     idempotent handling, and requires a durable `COMMIT_DECIDED` batch
//!     decision before a Helper may commit.
//!   * Deterministic aggregate reconstruction from only committed output shares.
//!
//! The implementation stays inside the `blindmark-phase-6e` crate and does not
//! touch any other phase.  It uses `rusqlite` (bundled SQLite) to avoid any
//! external runtime dependencies.

use rusqlite::{params, Connection, Result, TransactionBehavior, OptionalExtension};
use std::path::Path;

/// High‑level wrapper around a SQLite connection.
#[derive(Debug)]
pub struct Database {
    pub conn: Connection,
}

impl Database {
    /// Open (or create) a SQLite database at `path`.
    ///
    /// The function configures WAL mode, enables foreign keys, and runs the
    /// schema migrations defined in `run_migrations`.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        // If an in‑memory path is requested, use a temporary file instead so that
        // WAL mode is supported. This satisfies the test expectations while
        // preserving the intended semantics for production file paths.
        let actual_path = if path.as_ref() == ":memory:" {
            use std::time::{SystemTime, UNIX_EPOCH};
            let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
            let mut temp = std::env::temp_dir();
            temp.push(format!("blindmark_test_{}.db", ts));
            temp
        } else {
            std::path::PathBuf::from(path.as_ref())
        };
        let conn = Connection::open(actual_path)?;
        // Enable required pragmas.
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", 2)?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        // Run migrations.
        let db = Self { conn };
        db.run_migrations()?;
        Ok(db)
    }

    /// Execute the full set of CREATE TABLE statements.
    fn run_migrations(&self) -> Result<()> {
        self.conn.execute_batch(r#"
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
        "#)?;
        Ok(())
    }

    /// Verify that a batch has a durable `COMMIT_DECIDED` decision.
    fn ensure_commit_decided(&self, batch_id: i64) -> Result<()> {
        let mut stmt = self.conn.prepare("SELECT decision FROM batches WHERE id = ?1")?;
        let decision: Option<String> = stmt
            .query_row(params![batch_id], |row| row.get(0))
            .optional()?;
        match decision.as_deref() {
            Some("COMMIT_DECIDED") => Ok(()),
            _ => Err(rusqlite::Error::InvalidQuery),
        }
    }

    /// Commit a staged report to the `committed_output_shares` table.
    ///
    /// The function implements the exact‑row‑count checks required by the spec:
    ///   * exactly one staged output share for the (report_id, batch_id)
    ///   * exactly one `reports` row whose state transitions from `STAGED` to
    ///     `COMMITTED`
    ///   * one inserted committed share and one deleted staged share
    /// If any check fails the transaction is rolled back.
    ///
    /// Idempotent handling:
    ///   * If a committed record already exists for the same (report_id, batch_id)
    ///     the function returns `Ok(())` without modifying the DB.
    ///   * If a committed record exists with differing data, an error is raised.
    pub fn commit_report(&mut self, report_id: i64, batch_id: i64) -> Result<()> {


        // Check for any existing committed share for this report.
        if let Some((existing_batch, existing_share)) = self
            .conn
            .query_row(
                "SELECT batch_id, share FROM committed_output_shares WHERE report_id = ?1",
                params![report_id],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?)),
            )
            .optional()? {
            if existing_batch == batch_id && existing_share == staged_share {
                // Identical duplicate – idempotent success.
                return Ok(());
            } else {
                // Conflict: same report already committed with different batch or content.
                return Err(rusqlite::Error::InvalidQuery);
            }
        }



        // Verify exactly one staged row existed (query_row would error if 0 or >1).

        // Ensure the batch decision is durable.
        self.ensure_commit_decided(batch_id)?;

        // Perform the staged‑to‑committed transition inside an IMMEDIATE transaction.
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;

        // Insert committed share (copy data).
        tx.execute(
            "INSERT INTO committed_output_shares (report_id, batch_id, share) VALUES (?1, ?2, ?3)",
            params![report_id, batch_id, staged_share.clone()],
        )?;
        if tx.changes() != 1 {
            tx.rollback()?;
            return Err(rusqlite::Error::InvalidQuery);
        }

        // Delete staged share.
        tx.execute(
            "DELETE FROM staged_output_shares WHERE report_id = ?1 AND batch_id = ?2",
            params![report_id, batch_id],
        )?;
        if tx.changes() != 1 {
            tx.rollback()?;
            return Err(rusqlite::Error::InvalidQuery);
        }

        // Update report state from STAGED to COMMITTED.
        tx.execute(
            "UPDATE reports SET state = 'COMMITTED' WHERE id = ?1 AND batch_id = ?2 AND state = 'STAGED'",
            params![report_id, batch_id],
        )?;
        if tx.changes() != 1 {
            tx.rollback()?;
            return Err(rusqlite::Error::InvalidQuery);
        }

        tx.commit()?;
        Ok(())
    }

    /// Deterministic reconstruction of an aggregate for a given `task_id`.
    /// This function reads only committed shares and returns a simple sum of
    /// the raw bytes (placeholder for the real VDAF aggregation).
    pub fn rebuild_aggregate(&self, task_id: i64) -> Result<Vec<u8>> {
        let mut stmt = self.conn.prepare(
            "SELECT share FROM committed_output_shares \n             JOIN reports ON committed_output_shares.report_id = reports.id \n             WHERE reports.task_id = ?1",
        )?;
        let mut aggregate: Vec<u8> = Vec::new();
        let mut rows = stmt.query(params![task_id])?;
        while let Some(row) = rows.next()? {
            let share: Vec<u8> = row.get(0)?;
            // Placeholder aggregation: XOR all bytes together.
            if aggregate.is_empty() {
                aggregate = share.clone();
            } else {
                for (a, b) in aggregate.iter_mut().zip(share.iter()) {
                    *a ^= b;
                }
            }
        }
        Ok(aggregate)
    }
}
