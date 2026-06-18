use std::error::Error;
use std::fs;

use puread_core::restore_ledger::RestoreLedger;

use crate::sqlite_actions::{
    SqliteAction, SqliteActionRequest, SqliteActionRunner, SqliteActionSchedule, SqliteActionTarget,
};

use super::test_support::{parent, temp_root};

#[cfg(unix)]
#[test]
fn sqlite_runner_rejects_symlink_swap_before_backup() -> Result<(), Box<dyn Error>> {
    let root = temp_root()?;
    let db_path = root.join("data/data/com.example.video/databases/backup-race.db");
    let external = root.with_extension("backup-external.db");
    fs::create_dir_all(parent(&db_path)?)?;
    fs::write(&db_path, b"legacy ad rows")?;
    fs::write(&external, b"outside must not be copied")?;
    let target = SqliteActionTarget::from_android_path(
        "/data/data/com.example.video/databases/backup-race.db",
        &db_path,
        &root,
    )?;
    let ledger = RestoreLedger::at(root.join("state/actions.jsonl"));
    let runner = SqliteActionRunner::new(ledger.clone());
    let external_for_hook = external.clone();

    let report = super::hooks::with_before_sqlite_backup_hook_for_tests(
        move |path| {
            let _ignored = fs::remove_file(path);
            let _ignored = std::os::unix::fs::symlink(&external_for_hook, path);
        },
        || {
            runner.run_batch(&[SqliteActionRequest::new(
                "sqlite.backup-symlink-race",
                target,
                SqliteAction::MinimalSqlite,
                SqliteActionSchedule::BootOnce,
            )])
        },
    );

    assert_eq!(report.succeeded, 0);
    assert_eq!(report.failed, 1);
    assert_eq!(fs::read(&external)?, b"outside must not be copied");
    assert!(ledger.read_records()?.is_empty());
    assert_no_sqlite_backups(root.join("state/backups/sqlite").as_path())?;
    fs::remove_dir_all(&root)?;
    fs::remove_file(&external)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn sqlite_delete_restores_replacement_when_path_swaps_after_fd_guard() -> Result<(), Box<dyn Error>>
{
    let root = temp_root()?;
    let db_path = root.join("data/data/com.example.video/databases/delete-post-fd.db");
    let replacement = root.join("sqlite-delete-post-fd-replacement.db");
    fs::create_dir_all(parent(&db_path)?)?;
    fs::write(&db_path, b"legacy ad rows")?;
    fs::write(&replacement, b"replacement must survive")?;
    let replacement_before = fs::read(&replacement)?;
    let target = SqliteActionTarget::from_android_path(
        "/data/data/com.example.video/databases/delete-post-fd.db",
        &db_path,
        &root,
    )?;
    let ledger = RestoreLedger::at(root.join("state/actions.jsonl"));
    let runner = SqliteActionRunner::new(ledger.clone());
    let replacement_for_hook = replacement;

    let report = super::hooks::with_before_sqlite_delete_move_hook_for_tests(
        move |path| {
            let _ignored = fs::remove_file(path);
            let _ignored = fs::rename(&replacement_for_hook, path);
        },
        || {
            runner.run_batch(&[SqliteActionRequest::new(
                "sqlite.delete-post-fd-toctou",
                target,
                SqliteAction::Delete,
                SqliteActionSchedule::Manual,
            )])
        },
    );

    assert_eq!(report.succeeded, 0);
    assert_eq!(report.failed, 1);
    assert_eq!(fs::read(&db_path)?, replacement_before);
    assert!(ledger.read_records()?.is_empty());
    fs::remove_dir_all(&root)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn sqlite_missing_write_rejects_parent_replaced_after_root_check() -> Result<(), Box<dyn Error>> {
    let root = temp_root()?;
    let external = root.with_extension("outside-sqlite-parent-after-check");
    fs::create_dir_all(&external)?;
    let parent_dir = root.join("data/data/com.example.video/databases");
    fs::create_dir_all(&parent_dir)?;
    let db_path = parent_dir.join("after-check.db");
    let target = SqliteActionTarget::from_android_path(
        "/data/data/com.example.video/databases/after-check.db",
        &db_path,
        &root,
    )?;
    let ledger = RestoreLedger::at(root.join("state/actions.jsonl"));
    let runner = SqliteActionRunner::new(ledger.clone());
    let external_for_hook = external.clone();

    let report = super::hooks::with_before_sqlite_write_open_hook_for_tests(
        move |_path| {
            let _ignored = fs::remove_dir_all(&parent_dir);
            let _ignored = std::os::unix::fs::symlink(&external_for_hook, &parent_dir);
        },
        || {
            runner.run_batch(&[SqliteActionRequest::new(
                "sqlite.parent-after-check",
                target,
                SqliteAction::MinimalSqlite,
                SqliteActionSchedule::BootOnce,
            )])
        },
    );

    assert_eq!(report.succeeded, 0);
    assert_eq!(report.failed, 1);
    assert!(!external.join("after-check.db").exists());
    assert!(ledger.read_records()?.is_empty());
    fs::remove_dir_all(&root)?;
    fs::remove_dir_all(&external)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn sqlite_runner_preserves_ledger_when_target_swapped_after_write() -> Result<(), Box<dyn Error>> {
    let root = temp_root()?;
    let db_path = root.join("data/data/com.example.video/databases/post-write-ledger.db");
    let replacement = root.join("post-write-ledger-replacement.db");
    fs::create_dir_all(parent(&db_path)?)?;
    fs::write(&db_path, b"legacy sqlite rows")?;
    fs::write(&replacement, b"replacement rows")?;
    let target = SqliteActionTarget::from_android_path(
        "/data/data/com.example.video/databases/post-write-ledger.db",
        &db_path,
        &root,
    )?;
    let ledger = RestoreLedger::at(root.join("state/actions.jsonl"));
    let runner = SqliteActionRunner::new(ledger.clone());
    let replacement_for_hook = replacement;

    let report = super::hooks::with_after_sqlite_write_hook_for_tests(
        move |path| {
            let _ignored = fs::remove_file(path);
            let _ignored = fs::rename(&replacement_for_hook, path);
        },
        || {
            runner.run_batch(&[SqliteActionRequest::new(
                "sqlite.post-write-ledger",
                target,
                SqliteAction::MinimalSqlite,
                SqliteActionSchedule::BootOnce,
            )])
        },
    );

    assert_eq!(report.succeeded, 0);
    assert_eq!(report.failed, 1);
    assert_eq!(ledger.read_records()?.len(), 1);
    fs::remove_dir_all(&root)?;
    Ok(())
}

fn assert_no_sqlite_backups(backup_dir: &std::path::Path) -> Result<(), Box<dyn Error>> {
    if !backup_dir.exists() {
        return Ok(());
    }
    if fs::read_dir(backup_dir)?.next().is_none() {
        return Ok(());
    }
    Err("sqlite backup directory should stay empty".into())
}
