use std::error::Error;
use std::fs;
use std::path::PathBuf;

use puread_core::restore_ledger::{RestoreLedger, RestoreStep};

use crate::sqlite_actions::{
    SqliteAction, SqliteActionRequest, SqliteActionRunner, SqliteActionSchedule, SqliteActionTarget,
};

use super::test_support::{parent, temp_root};

#[test]
fn sqlite_runner_rejects_orphan_backup_before_minimal_write() -> Result<(), Box<dyn Error>> {
    let root = temp_root()?;
    let db_path = root.join("data/data/com.example.video/databases/orphan-backup.db");
    fs::create_dir_all(parent(&db_path)?)?;
    fs::write(&db_path, b"legacy ad rows")?;
    let target = SqliteActionTarget::from_android_path(
        "/data/data/com.example.video/databases/orphan-backup.db",
        &db_path,
        &root,
    )?;
    let ledger = RestoreLedger::at(root.join("state/actions.jsonl"));
    let runner = SqliteActionRunner::new(ledger.clone());
    let request = SqliteActionRequest::new(
        "sqlite.orphan-backup",
        target,
        SqliteAction::MinimalSqlite,
        SqliteActionSchedule::BootOnce,
    );
    let backup_path = super::super::ledger::ledger_record(
        &root.join("state/backups/sqlite"),
        &request,
        &crate::sqlite_actions::metadata::SqliteTargetMetadata::collect(&db_path)?,
    )?
    .restore_steps
    .iter()
    .find_map(sqlite_restore_content_path)
    .ok_or("expected sqlite backup path")?;
    fs::create_dir_all(parent(&backup_path)?)?;
    fs::write(&backup_path, b"stale backup")?;

    let report = runner.run_batch(&[request]);

    assert_eq!(report.succeeded, 0);
    assert_eq!(report.failed, 1);
    assert_eq!(fs::read(&db_path)?, b"legacy ad rows");
    assert!(ledger.read_records()?.is_empty());
    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn sqlite_delete_reuses_existing_ledger_without_overwriting_first_backup()
-> Result<(), Box<dyn Error>> {
    let root = temp_root()?;
    let db_path = root.join("data/data/com.example.video/databases/recreated-delete.db");
    fs::create_dir_all(parent(&db_path)?)?;
    fs::write(&db_path, b"original sqlite rows")?;
    let target = SqliteActionTarget::from_android_path(
        "/data/data/com.example.video/databases/recreated-delete.db",
        &db_path,
        &root,
    )?;
    let ledger = RestoreLedger::at(root.join("state/actions.jsonl"));
    let runner = SqliteActionRunner::new(ledger.clone());
    let request = SqliteActionRequest::new(
        "sqlite.recreated-delete",
        target,
        SqliteAction::Delete,
        SqliteActionSchedule::Manual,
    );

    let first = runner.run_batch(std::slice::from_ref(&request));
    let backup_path = sqlite_backup_path_from_ledger(&ledger.read_records()?)?;
    fs::write(&db_path, b"recreated sqlite rows")?;
    let second = runner.run_batch(&[request]);

    assert_eq!(first.succeeded, 1);
    assert_eq!(first.failed, 0);
    assert_eq!(second.succeeded, 1);
    assert_eq!(second.failed, 0);
    assert!(!db_path.exists());
    assert_eq!(fs::read(backup_path)?, b"original sqlite rows");
    assert_eq!(ledger.read_records()?.len(), 1);
    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn sqlite_runner_rejects_existing_ledger_when_backup_missing_before_repeated_write()
-> Result<(), Box<dyn Error>> {
    let root = temp_root()?;
    let db_path = root.join("data/data/com.example.video/databases/missing-backup.db");
    fs::create_dir_all(parent(&db_path)?)?;
    fs::write(&db_path, b"original sqlite rows")?;
    let target = SqliteActionTarget::from_android_path(
        "/data/data/com.example.video/databases/missing-backup.db",
        &db_path,
        &root,
    )?;
    let ledger = RestoreLedger::at(root.join("state/actions.jsonl"));
    let runner = SqliteActionRunner::new(ledger.clone());
    let request = SqliteActionRequest::new(
        "sqlite.missing-backup",
        target,
        SqliteAction::MinimalSqlite,
        SqliteActionSchedule::BootOnce,
    );

    let first = runner.run_batch(std::slice::from_ref(&request));
    let backup_path = sqlite_backup_path_from_ledger(&ledger.read_records()?)?;
    fs::remove_file(backup_path)?;
    fs::write(&db_path, b"recreated sqlite rows")?;
    let second = runner.run_batch(&[request]);

    assert_eq!(first.succeeded, 1);
    assert_eq!(first.failed, 0);
    assert_eq!(second.succeeded, 0);
    assert_eq!(second.failed, 1);
    assert_eq!(fs::read(&db_path)?, b"recreated sqlite rows");
    assert_eq!(ledger.read_records()?.len(), 1);
    fs::remove_dir_all(&root)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn sqlite_runner_rejects_existing_ledger_when_backup_is_symlink_before_repeated_write()
-> Result<(), Box<dyn Error>> {
    let root = temp_root()?;
    let outside = root.with_extension("outside-sqlite-repeated-backup.db");
    let db_path = root.join("data/data/com.example.video/databases/symlink-backup.db");
    fs::create_dir_all(parent(&db_path)?)?;
    fs::write(&db_path, b"original sqlite rows")?;
    let target = SqliteActionTarget::from_android_path(
        "/data/data/com.example.video/databases/symlink-backup.db",
        &db_path,
        &root,
    )?;
    let ledger = RestoreLedger::at(root.join("state/actions.jsonl"));
    let runner = SqliteActionRunner::new(ledger.clone());
    let request = SqliteActionRequest::new(
        "sqlite.symlink-backup",
        target,
        SqliteAction::MinimalSqlite,
        SqliteActionSchedule::BootOnce,
    );

    let first = runner.run_batch(std::slice::from_ref(&request));
    let backup_path = sqlite_backup_path_from_ledger(&ledger.read_records()?)?;
    fs::remove_file(&backup_path)?;
    std::os::unix::fs::symlink(&outside, &backup_path)?;
    fs::write(&db_path, b"recreated sqlite rows")?;
    let second = runner.run_batch(&[request]);

    assert_eq!(first.succeeded, 1);
    assert_eq!(first.failed, 0);
    assert_eq!(second.succeeded, 0);
    assert_eq!(second.failed, 1);
    assert_eq!(fs::read(&db_path)?, b"recreated sqlite rows");
    assert!(!outside.exists());
    assert_eq!(ledger.read_records()?.len(), 1);
    fs::remove_dir_all(&root)?;
    Ok(())
}

fn sqlite_backup_path_from_ledger(
    records: &[puread_core::restore_ledger::LedgerRecord],
) -> Result<PathBuf, Box<dyn Error>> {
    let record = records.first().ok_or("expected one sqlite ledger record")?;
    let path = record
        .restore_steps
        .iter()
        .find_map(sqlite_restore_content_path)
        .ok_or("expected sqlite restore content step")?;
    Ok(path)
}

fn sqlite_restore_content_path(step: &RestoreStep) -> Option<PathBuf> {
    match step {
        RestoreStep::RestoreContent { backup_path } => Some(PathBuf::from(backup_path)),
        RestoreStep::RecreateDirectory
        | RestoreStep::RecreateFile
        | RestoreStep::RemovePlaceholder
        | RestoreStep::SetMode { .. }
        | RestoreStep::SetOwner { .. }
        | RestoreStep::SetSelinuxContext { .. }
        | RestoreStep::SetImmutable { .. } => None,
    }
}
