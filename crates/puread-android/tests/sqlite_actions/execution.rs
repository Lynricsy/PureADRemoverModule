use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io;

use fs2::FileExt;
use puread_android::sqlite_actions::{
    SqliteAction, SqliteActionRunner, SqliteActionSchedule, SqliteActionStatus,
};
use puread_core::restore_ledger::{LedgerAction, OriginalFileType};

use super::{
    TestDir, assert_all_succeeded, assert_sqlite_integrity_ok, first_outcome, first_record,
    missing_parent, readonly_bits, request,
};

#[test]
fn sqlite_actions_write_minimal_database_when_boot_once_requested() -> Result<(), Box<dyn Error>> {
    // Given: an ad database file and a boot-once minimal-sqlite request.
    let dir = TestDir::new()?;
    let db_path = dir.db_path("ad.db");
    fs::create_dir_all(db_path.parent().ok_or_else(missing_parent)?)?;
    fs::write(&db_path, b"legacy ad rows")?;
    let ledger = dir.ledger();
    let runner = SqliteActionRunner::new(ledger.clone());

    // When: the `SQLite` action is executed.
    let report = runner.run_batch(&[request(
        "sqlite.minimal",
        &dir,
        "/data/data/com.example.video/databases/ad.db",
        SqliteAction::MinimalSqlite,
        SqliteActionSchedule::BootOnce,
    )?]);

    // Then: the file is a real `SQLite` database and restoration metadata is recorded.
    assert_all_succeeded(&report);
    assert_sqlite_integrity_ok(&db_path)?;
    let records = ledger.read_records()?;
    println!(
        "sqlite_actions_minimal_size={}",
        fs::metadata(&db_path)?.len()
    );
    assert_eq!(records.len(), 1);
    let record = first_record(&records)?;
    assert_eq!(record.action, LedgerAction::MinimalSqlite);
    assert_eq!(record.original_file_type, OriginalFileType::File);
    assert!(
        record
            .restore_steps
            .iter()
            .any(|step| format!("{step:?}").contains("RestoreContent"))
    );
    Ok(())
}

#[test]
fn sqlite_actions_delete_database_when_manual_requested() -> Result<(), Box<dyn Error>> {
    // Given: an existing ad database and a manual delete request.
    let dir = TestDir::new()?;
    let db_path = dir.db_path("delete-me.db");
    fs::create_dir_all(db_path.parent().ok_or_else(missing_parent)?)?;
    fs::write(&db_path, b"delete target")?;
    let ledger = dir.ledger();
    let runner = SqliteActionRunner::new(ledger.clone());

    // When: delete is executed.
    let report = runner.run_batch(&[request(
        "sqlite.delete",
        &dir,
        "/data/data/com.example.video/databases/delete-me.db",
        SqliteAction::Delete,
        SqliteActionSchedule::Manual,
    )?]);

    // Then: the database is removed and a reversible ledger row exists.
    assert_all_succeeded(&report);
    assert!(!db_path.exists());
    let records = ledger.read_records()?;
    println!("sqlite_actions_delete_records={}", records.len());
    assert_eq!(records.len(), 1);
    assert_eq!(first_record(&records)?.action, LedgerAction::Delete);
    Ok(())
}

#[test]
fn sqlite_actions_create_deny_write_placeholder_when_low_frequency_requested()
-> Result<(), Box<dyn Error>> {
    // Given: a missing SDK database path and a low-frequency deny-write request.
    let dir = TestDir::new()?;
    let db_path = dir.db_path("sdk_ad.db");
    fs::create_dir_all(
        db_path
            .parent()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing sqlite parent"))?,
    )?;
    let ledger = dir.ledger();
    let runner = SqliteActionRunner::new(ledger.clone());

    // When: deny-write is executed.
    let report = runner.run_batch(&[request(
        "sqlite.deny",
        &dir,
        "/data/data/com.example.video/databases/sdk_ad.db",
        SqliteAction::DenyWrite,
        SqliteActionSchedule::LowFrequency,
    )?]);

    // Then: a read-only placeholder exists and the missing original state is restorable.
    assert_all_succeeded(&report);
    assert!(db_path.is_file());
    let metadata = fs::metadata(&db_path)?;
    println!("sqlite_actions_deny_mode={:o}", readonly_bits(&metadata));
    assert!(metadata.permissions().readonly());
    let records = ledger.read_records()?;
    assert_eq!(records.len(), 1);
    let record = first_record(&records)?;
    assert_eq!(record.action, LedgerAction::DenyWrite);
    assert_eq!(record.original_file_type, OriginalFileType::Missing);
    Ok(())
}

#[test]
fn sqlite_actions_reject_high_frequency_schedule_before_touching_file() -> Result<(), Box<dyn Error>>
{
    // Given: an existing file and a disallowed high-frequency request.
    let dir = TestDir::new()?;
    let db_path = dir.db_path("hot.db");
    fs::create_dir_all(db_path.parent().ok_or_else(missing_parent)?)?;
    fs::write(&db_path, b"untouched")?;
    let ledger = dir.ledger();
    let runner = SqliteActionRunner::new(ledger.clone());

    // When: the batch sees a high-frequency schedule.
    let report = runner.run_batch(&[request(
        "sqlite.hot",
        &dir,
        "/data/data/com.example.video/databases/hot.db",
        SqliteAction::Delete,
        SqliteActionSchedule::HighFrequency,
    )?]);

    // Then: the item is reported as failed without changing target or ledger state.
    assert_eq!(report.succeeded, 0);
    assert_eq!(report.failed, 1);
    assert_eq!(fs::read(&db_path)?, b"untouched");
    assert!(ledger.read_records()?.is_empty());
    assert!(matches!(
        first_outcome(&report)?.status,
        SqliteActionStatus::Failed(_)
    ));
    Ok(())
}

#[test]
fn sqlite_actions_continue_batch_when_one_target_is_locked_or_permission_denied()
-> Result<(), Box<dyn Error>> {
    // Given: one locked database, one invalid target path, and one valid target.
    let dir = TestDir::new()?;
    let locked_path = dir.db_path("locked.db");
    let ok_path = dir.db_path("ok.db");
    fs::create_dir_all(locked_path.parent().ok_or_else(missing_parent)?)?;
    fs::write(&locked_path, b"locked")?;
    fs::write(&ok_path, b"ok")?;
    let lock_file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&locked_path)?;
    lock_file.try_lock_exclusive()?;
    let ledger = dir.ledger();
    let runner = SqliteActionRunner::new(ledger.clone());

    // When: the mixed batch is executed.
    let report = runner.run_batch(&[
        request(
            "sqlite.locked",
            &dir,
            "/data/data/com.example.video/databases/locked.db",
            SqliteAction::MinimalSqlite,
            SqliteActionSchedule::BootOnce,
        )?,
        request(
            "sqlite.ok",
            &dir,
            "/data/data/com.example.video/databases/ok.db",
            SqliteAction::Delete,
            SqliteActionSchedule::Manual,
        )?,
    ]);
    lock_file.unlock()?;

    // Then: the locked item fails, but the valid item still completes.
    println!(
        "sqlite_actions_mixed_batch succeeded={} failed={}",
        report.succeeded, report.failed
    );
    assert_eq!(report.succeeded, 1);
    assert_eq!(report.failed, 1);
    assert!(locked_path.exists());
    assert!(!ok_path.exists());
    assert_eq!(ledger.read_records()?.len(), 1);
    Ok(())
}
