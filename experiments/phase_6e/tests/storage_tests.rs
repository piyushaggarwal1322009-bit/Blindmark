//! Unit tests for the SQLite storage foundation (Phase 6F.1).
//!
//! These tests exercise the durable SQLite transaction contract, exact row-count
//! checks, rollback safety, persistence, and aggregate reconstruction for the
//! experimental Phase 6F storage layer.

#[cfg(test)]
mod tests {
    use blindmark_phase_6e::storage::Database;
    use rusqlite::Result;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn make_temp_db_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let mut path = std::env::temp_dir();
        path.push(format!("blindmark_phase_6e_{name}_{nanos}.db"));
        path
    }

    fn cleanup_temp_db(path: &Path) {
        for suffix in ["", "-wal", "-shm"] {
            let file_name = format!("{}{}", path.display(), suffix);
            let _ = fs::remove_file(file_name);
        }
    }

    fn init_leader_db() -> Result<Database> {
        Database::new(":memory:")
    }

    #[test]
    fn schema_and_database_configuration() -> Result<()> {
        let path = make_temp_db_path("schema");
        {
            let db = Database::new(&path)?;

            let tables: Vec<String> = db
                .conn
                .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")?
                .query_map([], |row| row.get(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;

            for expected in [
                "aggregate_checkpoints",
                "batches",
                "committed_output_shares",
                "reports",
                "staged_output_shares",
            ] {
                assert!(tables.iter().any(|name| name == expected));
            }

            let journal: String = db.conn.query_row("PRAGMA journal_mode", [], |r| r.get(0))?;
            assert_eq!(journal, "wal");

            let foreign_keys: i64 = db.conn.query_row("PRAGMA foreign_keys", [], |r| r.get(0))?;
            assert_eq!(foreign_keys, 1);

            let synchronous: i64 = db.conn.query_row("PRAGMA synchronous", [], |r| r.get(0))?;
            assert_eq!(synchronous, 2);
        }

        let leader_path = make_temp_db_path("leader");
        let helper_path = make_temp_db_path("helper");
        {
            let leader = Database::new(&leader_path)?;
            let helper = Database::new(&helper_path)?;
            drop(leader);
            drop(helper);
        }

        cleanup_temp_db(&path);
        cleanup_temp_db(&leader_path);
        cleanup_temp_db(&helper_path);
        Ok(())
    }

    #[test]
    fn successful_commit_flow() -> Result<()> {
        let mut db = init_leader_db()?;
        db.conn.execute(
            "INSERT INTO batches (id, decision) VALUES (1, 'COMMIT_DECIDED')",
            [],
        )?;
        db.conn.execute(
            "INSERT INTO reports (id, task_id, batch_id, state) VALUES (10, 5, 1, 'STAGED')",
            [],
        )?;
        db.conn.execute(
            "INSERT INTO staged_output_shares (report_id, batch_id, share) VALUES (10, 1, x'010203')",
            [],
        )?;

        db.commit_report(10, 1)?;

        let state: String =
            db.conn
                .query_row("SELECT state FROM reports WHERE id = 10", [], |r| r.get(0))?;
        assert_eq!(state, "COMMITTED");

        let staged_count: i64 = db.conn.query_row(
            "SELECT COUNT(*) FROM staged_output_shares WHERE report_id = 10",
            [],
            |r| r.get(0),
        )?;
        assert_eq!(staged_count, 0);

        let committed_count: i64 = db.conn.query_row(
            "SELECT COUNT(*) FROM committed_output_shares WHERE report_id = 10",
            [],
            |r| r.get(0),
        )?;
        assert_eq!(committed_count, 1);
        Ok(())
    }

    #[test]
    fn idempotent_duplicate_commit_and_conflicting_batch_reject() -> Result<()> {
        let mut db = init_leader_db()?;
        db.conn.execute(
            "INSERT INTO batches (id, decision) VALUES (2, 'COMMIT_DECIDED')",
            [],
        )?;
        db.conn.execute(
            "INSERT INTO reports (id, task_id, batch_id, state) VALUES (20, 5, 2, 'STAGED')",
            [],
        )?;
        db.conn.execute(
            "INSERT INTO staged_output_shares (report_id, batch_id, share) VALUES (20, 2, x'0A0B0C')",
            [],
        )?;

        db.commit_report(20, 2)?;
        db.commit_report(20, 2)?;

        let committed_count: i64 = db.conn.query_row(
            "SELECT COUNT(*) FROM committed_output_shares WHERE report_id = 20",
            [],
            |r| r.get(0),
        )?;
        assert_eq!(committed_count, 1);

        let state: String =
            db.conn
                .query_row("SELECT state FROM reports WHERE id = 20", [], |r| r.get(0))?;
        assert_eq!(state, "COMMITTED");

        db.conn.execute(
            "INSERT INTO batches (id, decision) VALUES (21, 'COMMIT_DECIDED')",
            [],
        )?;
        let conflict = db.commit_report(20, 21);
        assert!(conflict.is_err());
        Ok(())
    }

    #[test]
    fn invalid_batch_decisions_are_rejected() -> Result<()> {
        let mut db = init_leader_db()?;

        db.conn.execute(
            "INSERT INTO reports (id, task_id, batch_id, state) VALUES (30, 5, 99, 'STAGED')",
            [],
        )?;
        db.conn.execute(
            "INSERT INTO staged_output_shares (report_id, batch_id, share) VALUES (30, 99, x'010203')",
            [],
        )?;

        let missing_batch = db.commit_report(30, 99);
        assert!(missing_batch.is_err());

        db.conn.execute(
            "INSERT INTO batches (id, decision) VALUES (31, 'ABORTED')",
            [],
        )?;
        db.conn.execute(
            "INSERT INTO reports (id, task_id, batch_id, state) VALUES (31, 5, 31, 'STAGED')",
            [],
        )?;
        db.conn.execute(
            "INSERT INTO staged_output_shares (report_id, batch_id, share) VALUES (31, 31, x'ABCDEF')",
            [],
        )?;

        let aborted_batch = db.commit_report(31, 31);
        assert!(aborted_batch.is_err());
        let state: String =
            db.conn
                .query_row("SELECT state FROM reports WHERE id = 31", [], |r| r.get(0))?;
        assert_eq!(state, "STAGED");
        let staged_count: i64 = db.conn.query_row(
            "SELECT COUNT(*) FROM staged_output_shares WHERE report_id = 31",
            [],
            |r| r.get(0),
        )?;
        assert_eq!(staged_count, 1);

        Ok(())
    }

    #[test]
    fn missing_or_invalid_staged_data_rejected() -> Result<()> {
        let mut db = init_leader_db()?;
        db.conn.execute(
            "INSERT INTO batches (id, decision) VALUES (40, 'COMMIT_DECIDED')",
            [],
        )?;
        db.conn.execute(
            "INSERT INTO reports (id, task_id, batch_id, state) VALUES (40, 5, 40, 'STAGED')",
            [],
        )?;

        let missing_share = db.commit_report(40, 40);
        assert!(missing_share.is_err());

        db.conn.execute(
            "INSERT INTO staged_output_shares (report_id, batch_id, share) VALUES (40, 40, x'111213')",
            [],
        )?;
        db.conn.execute(
            "INSERT INTO reports (id, task_id, batch_id, state) VALUES (41, 5, 41, 'STAGED')",
            [],
        )?;
        db.conn.execute(
            "INSERT INTO staged_output_shares (report_id, batch_id, share) VALUES (41, 41, x'AAAA')",
            [],
        )?;
        db.conn.execute(
            "INSERT INTO batches (id, decision) VALUES (41, 'COMMIT_DECIDED')",
            [],
        )?;

        let wrong_report_batch = db.commit_report(41, 40);
        assert!(wrong_report_batch.is_err());

        db.conn
            .execute("UPDATE reports SET state = 'COMMITTED' WHERE id = 41", [])?;
        let inconsistent_state = db.commit_report(41, 41);
        assert!(inconsistent_state.is_err());

        Ok(())
    }

    #[test]
    fn rollback_preserves_state_when_transaction_fails_mid_commit() -> Result<()> {
        let mut db = init_leader_db()?;
        db.conn.execute(
            "INSERT INTO batches (id, decision) VALUES (50, 'COMMIT_DECIDED')",
            [],
        )?;
        db.conn.execute(
            "INSERT INTO reports (id, task_id, batch_id, state) VALUES (50, 7, 50, 'STAGED')",
            [],
        )?;
        db.conn.execute(
            "INSERT INTO staged_output_shares (report_id, batch_id, share) VALUES (50, 50, x'DEADBEEF')",
            [],
        )?;

        db.conn.execute(
            "CREATE TRIGGER fail_after_insert AFTER INSERT ON committed_output_shares BEGIN SELECT RAISE(ABORT, 'forced failure after insert'); END;",
            [],
        )?;

        let result = db.commit_report(50, 50);
        assert!(result.is_err());

        let committed_count: i64 = db.conn.query_row(
            "SELECT COUNT(*) FROM committed_output_shares WHERE report_id = 50",
            [],
            |r| r.get(0),
        )?;
        assert_eq!(committed_count, 0);

        let staged_count: i64 = db.conn.query_row(
            "SELECT COUNT(*) FROM staged_output_shares WHERE report_id = 50",
            [],
            |r| r.get(0),
        )?;
        assert_eq!(staged_count, 1);

        let state: String =
            db.conn
                .query_row("SELECT state FROM reports WHERE id = 50", [], |r| r.get(0))?;
        assert_eq!(state, "STAGED");

        Ok(())
    }

    #[test]
    fn duplicate_committed_row_with_inconsistent_report_state_is_rejected() -> Result<()> {
        let mut db = init_leader_db()?;
        db.conn.execute(
            "INSERT INTO batches (id, decision) VALUES (80, 'COMMIT_DECIDED')",
            [],
        )?;
        db.conn.execute(
            "INSERT INTO reports (id, task_id, batch_id, state) VALUES (80, 12, 80, 'STAGED')",
            [],
        )?;
        db.conn.execute(
            "INSERT INTO committed_output_shares (report_id, batch_id, share) VALUES (80, 80, x'112233')",
            [],
        )?;
        let err = db.commit_report(80, 80);
        assert!(err.is_err());
        Ok(())
    }

    #[test]
    fn duplicate_committed_row_with_remaining_staged_share_is_rejected() -> Result<()> {
        let mut db = init_leader_db()?;
        db.conn.execute(
            "INSERT INTO batches (id, decision) VALUES (81, 'COMMIT_DECIDED')",
            [],
        )?;
        db.conn.execute(
            "INSERT INTO reports (id, task_id, batch_id, state) VALUES (81, 12, 81, 'COMMITTED')",
            [],
        )?;
        db.conn.execute(
            "INSERT INTO committed_output_shares (report_id, batch_id, share) VALUES (81, 81, x'99AABB')",
            [],
        )?;
        db.conn.execute(
            "INSERT INTO staged_output_shares (report_id, batch_id, share) VALUES (81, 81, x'CCDDEE')",
            [],
        )?;
        let err = db.commit_report(81, 81);
        assert!(err.is_err());
        Ok(())
    }

    #[test]
    fn file_backed_persistence_round_trip() -> Result<()> {
        let path = make_temp_db_path("persist");

        {
            let db = Database::new(&path)?;
            db.conn.execute(
                "INSERT INTO batches (id, decision) VALUES (60, 'COMMIT_DECIDED')",
                [],
            )?;
            db.conn.execute(
                "INSERT INTO reports (id, task_id, batch_id, state) VALUES (60, 8, 60, 'STAGED')",
                [],
            )?;
            db.conn.execute(
                "INSERT INTO staged_output_shares (report_id, batch_id, share) VALUES (60, 60, x'01020304')",
                [],
            )?;
            drop(db);
        }

        {
            let reopened = Database::new(&path)?;
            let stage_count: i64 = reopened.conn.query_row(
                "SELECT COUNT(*) FROM staged_output_shares WHERE report_id = 60",
                [],
                |r| r.get(0),
            )?;
            assert_eq!(stage_count, 1);
            drop(reopened);
        }

        {
            let mut reopened = Database::new(&path)?;
            reopened.commit_report(60, 60)?;
            drop(reopened);
        }

        {
            let reopened = Database::new(&path)?;
            let committed_count: i64 = reopened.conn.query_row(
                "SELECT COUNT(*) FROM committed_output_shares WHERE report_id = 60",
                [],
                |r| r.get(0),
            )?;
            assert_eq!(committed_count, 1);

            let staged_count: i64 = reopened.conn.query_row(
                "SELECT COUNT(*) FROM staged_output_shares WHERE report_id = 60",
                [],
                |r| r.get(0),
            )?;
            assert_eq!(staged_count, 0);

            let state: String =
                reopened
                    .conn
                    .query_row("SELECT state FROM reports WHERE id = 60", [], |r| r.get(0))?;
            assert_eq!(state, "COMMITTED");
            drop(reopened);
        }

        cleanup_temp_db(&path);
        Ok(())
    }

    #[test]
    fn commit_report_rejects_other_staged_batches_for_same_report() -> Result<()> {
        let mut db = init_leader_db()?;
        db.conn.execute(
            "INSERT INTO batches (id, decision) VALUES (90, 'COMMIT_DECIDED')",
            [],
        )?;
        db.conn.execute(
            "INSERT INTO batches (id, decision) VALUES (91, 'COMMIT_DECIDED')",
            [],
        )?;
        db.conn.execute(
            "INSERT INTO reports (id, task_id, batch_id, state) VALUES (90, 20, 90, 'STAGED')",
            [],
        )?;
        db.conn.execute(
            "INSERT INTO staged_output_shares (report_id, batch_id, share) VALUES (90, 90, x'AA')",
            [],
        )?;
        db.conn.execute(
            "INSERT INTO staged_output_shares (report_id, batch_id, share) VALUES (90, 91, x'BB')",
            [],
        )?;

        let err = db.commit_report(90, 90);
        assert!(err.is_err());
        Ok(())
    }

    #[test]
    fn rebuild_aggregate_rejects_uncommitted_or_mismatched_report_batches() -> Result<()> {
        let db = init_leader_db()?;
        db.conn.execute(
            "INSERT INTO batches (id, decision) VALUES (100, 'COMMIT_DECIDED')",
            [],
        )?;
        db.conn.execute(
            "INSERT INTO reports (id, task_id, batch_id, state) VALUES (100, 21, 100, 'STAGED')",
            [],
        )?;
        db.conn.execute(
            "INSERT INTO committed_output_shares (report_id, batch_id, share) VALUES (100, 100, x'0F0F')",
            [],
        )?;

        let err = db.rebuild_aggregate(21);
        assert!(err.is_err());
        Ok(())
    }

    #[test]
    fn rebuild_aggregate_rejects_empty_first_share_when_lengths_mismatch() -> Result<()> {
        let db = init_leader_db()?;
        db.conn.execute(
            "INSERT INTO batches (id, decision) VALUES (110, 'COMMIT_DECIDED')",
            [],
        )?;
        db.conn.execute(
            "INSERT INTO reports (id, task_id, batch_id, state) VALUES (110, 22, 110, 'COMMITTED')",
            [],
        )?;
        db.conn.execute(
            "INSERT INTO reports (id, task_id, batch_id, state) VALUES (111, 22, 110, 'COMMITTED')",
            [],
        )?;
        db.conn.execute(
            "INSERT INTO committed_output_shares (report_id, batch_id, share) VALUES (110, 110, x'')",
            [],
        )?;
        db.conn.execute(
            "INSERT INTO committed_output_shares (report_id, batch_id, share) VALUES (111, 110, x'0F')",
            [],
        )?;

        let err = db.rebuild_aggregate(22);
        assert!(err.is_err());
        Ok(())
    }

    #[test]
    fn rebuild_aggregate_reads_only_committed_shares() -> Result<()> {
        let db = init_leader_db()?;
        db.conn.execute(
            "INSERT INTO batches (id, decision) VALUES (70, 'COMMIT_DECIDED')",
            [],
        )?;
        db.conn.execute(
            "INSERT INTO reports (id, task_id, batch_id, state) VALUES (70, 9, 70, 'STAGED')",
            [],
        )?;
        db.conn.execute(
            "INSERT INTO staged_output_shares (report_id, batch_id, share) VALUES (70, 70, x'FF00')",
            [],
        )?;

        db.conn.execute(
            "INSERT INTO reports (id, task_id, batch_id, state) VALUES (71, 9, 70, 'COMMITTED')",
            [],
        )?;
        db.conn.execute(
            "INSERT INTO committed_output_shares (report_id, batch_id, share) VALUES (71, 70, x'0F0F')",
            [],
        )?;

        let aggregate = db.rebuild_aggregate(9)?;
        assert_eq!(aggregate, vec![0x0F, 0x0F]);
        Ok(())
    }

    #[test]
    fn rebuild_aggregate_rejects_mismatched_share_lengths() -> Result<()> {
        let db = init_leader_db()?;
        db.conn.execute(
            "INSERT INTO reports (id, task_id, batch_id, state) VALUES (90, 15, 90, 'COMMITTED')",
            [],
        )?;
        db.conn.execute(
            "INSERT INTO reports (id, task_id, batch_id, state) VALUES (91, 15, 90, 'COMMITTED')",
            [],
        )?;
        db.conn.execute(
            "INSERT INTO committed_output_shares (report_id, batch_id, share) VALUES (90, 90, x'0102')",
            [],
        )?;
        db.conn.execute(
            "INSERT INTO committed_output_shares (report_id, batch_id, share) VALUES (91, 90, x'010203')",
            [],
        )?;

        let err = db.rebuild_aggregate(15);
        assert!(err.is_err());
        Ok(())
    }
}
