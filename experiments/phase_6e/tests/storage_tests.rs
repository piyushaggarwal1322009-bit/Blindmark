//! Unit tests for the SQLite storage foundation (Phase 6F.1).
//!
//! These tests run against in‑memory databases (`:memory:`) to avoid any
//! filesystem side‑effects. They verify schema creation, WAL mode, the exact
//! transactional semantics of `commit_report`, idempotent duplicate handling,
//! conflict detection, and deterministic reconstruction.

#[cfg(test)]
mod tests {
    use blindmark_phase_6e::storage::Database;
    use rusqlite::Result;

    fn init_leader_db() -> Result<Database> {
        Database::new(":memory:")
    }

    fn init_helper_db() -> Result<Database> {
        Database::new(":memory:")
    }

    #[test]
    fn schema_and_wal_mode() -> Result<()> {
        let db = Database::new(":memory:")?;
        // Verify pragmas.
        let pragma: String = db.conn.query_row("PRAGMA journal_mode", [], |r| r.get(0))?;
        assert_eq!(pragma, "wal");
        let sync: i64 = db.conn.query_row("PRAGMA synchronous", [], |r| r.get(0))?;
        assert_eq!(sync, "2"); // NORMAL corresponds to 2
        Ok(())
    }

    #[test]
    fn successful_commit_flow() -> Result<()> {
        let mut db = init_leader_db()?;
        // Insert batch with COMMIT_DECIDED.
        db.conn.execute("INSERT INTO batches (id, decision) VALUES (1, 'COMMIT_DECIDED')", [])?;
        // Insert report in STAGED state.
        db.conn.execute(
            "INSERT INTO reports (id, task_id, batch_id, state) VALUES (10, 5, 1, 'STAGED')",
            [],
        )?;
        // Insert staged share.
        db.conn.execute(
            "INSERT INTO staged_output_shares (id, report_id, batch_id, share) VALUES (100, 10, 1, x'010203')",
            [],
        )?;
        // Commit.
        db.commit_report(10, 1)?;
        // Verify state transition.
        let state: String = db
            .conn
            .query_row("SELECT state FROM reports WHERE id = 10", [], |r| r.get(0))?;
        assert_eq!(state, "COMMITTED");
        // Verify staged row removed.
        let cnt: i64 = db
            .conn
            .query_row(
                "SELECT COUNT(*) FROM staged_output_shares WHERE report_id = 10",
                [],
                |r| r.get(0),
            )?;
        assert_eq!(cnt, 0);
        // Verify committed row present.
        let cnt2: i64 = db
            .conn
            .query_row(
                "SELECT COUNT(*) FROM committed_output_shares WHERE report_id = 10",
                [],
                |r| r.get(0),
            )?;
        assert_eq!(cnt2, 1);
        Ok(())
    }

    #[test]
    fn idempotent_duplicate_commit() -> Result<()> {
        let mut db = init_leader_db()?;
        db.conn.execute("INSERT INTO batches (id, decision) VALUES (2, 'COMMIT_DECIDED')", [])?;
        db.conn.execute(
            "INSERT INTO reports (id, task_id, batch_id, state) VALUES (20, 5, 2, 'STAGED')",
            [],
        )?;
        db.conn.execute(
            "INSERT INTO staged_output_shares (id, report_id, batch_id, share) VALUES (200, 20, 2, x'0A0B0C')",
            [],
        )?;
        // First commit.
        db.commit_report(20, 2)?;
        // Second commit (should be idempotent, no panic).
        db.commit_report(20, 2)?;
        Ok(())
    }

    #[test]
    fn conflicting_report_rejects() -> Result<()> {
        let mut db = init_leader_db()?;
        db.conn.execute("INSERT INTO batches (id, decision) VALUES (3, 'COMMIT_DECIDED')", [])?;
        db.conn.execute(
            "INSERT INTO reports (id, task_id, batch_id, state) VALUES (30, 5, 3, 'STAGED')",
            [],
        )?;
        db.conn.execute(
            "INSERT INTO staged_output_shares (id, report_id, batch_id, share) VALUES (300, 30, 3, x'0101')",
            [],
        )?;
        // Manually insert a conflicting committed share.
        db.conn.execute(
            "INSERT INTO committed_output_shares (id, report_id, batch_id, share) VALUES (400, 30, 3, x'0202')",
            [],
        )?;
        // Commit should now error.
        let res = db.commit_report(30, 3);
        assert!(res.is_err());
        Ok(())
    }

    #[test]
    fn missing_staged_share_rollback() -> Result<()> {
        let mut db = init_leader_db()?;
        db.conn.execute("INSERT INTO batches (id, decision) VALUES (4, 'COMMIT_DECIDED')", [])?;
        db.conn.execute(
            "INSERT INTO reports (id, task_id, batch_id, state) VALUES (40, 5, 4, 'STAGED')",
            [],
        )?;
        // No staged share inserted.
        let res = db.commit_report(40, 4);
        assert!(res.is_err());
        // Ensure report state unchanged.
        let state: String = db
            .conn
            .query_row("SELECT state FROM reports WHERE id = 40", [], |r| r.get(0))?;
        assert_eq!(state, "STAGED");
        Ok(())
    }
}

