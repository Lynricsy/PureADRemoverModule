use std::error::Error;
use std::fs;

use puread_core::restore_ledger::RestoreLedger;

use crate::sqlite_actions::{
    SqliteAction, SqliteActionRequest, SqliteActionRunner, SqliteActionSchedule, SqliteActionTarget,
};

use super::test_support::{parent, temp_root};

#[cfg(unix)]
#[test]
fn sqlite_runner_rejects_backup_dir_symlink_escape() -> Result<(), Box<dyn Error>> {
    let root = temp_root()?;
    let db_path = root.join("data/data/com.example.video/databases/backup-dir-link.db");
    let outside = root.with_extension("outside-sqlite-backup-dir");
    fs::create_dir_all(parent(&db_path)?)?;
    fs::create_dir_all(&outside)?;
    fs::write(&db_path, b"legacy sqlite rows")?;
    let target = SqliteActionTarget::from_android_path(
        "/data/data/com.example.video/databases/backup-dir-link.db",
        &db_path,
        &root,
    )?;
    let state_dir = root.join("state");
    fs::create_dir_all(&state_dir)?;
    std::os::unix::fs::symlink(&outside, state_dir.join("backups"))?;
    let ledger = RestoreLedger::at(state_dir.join("actions.jsonl"));
    let runner = SqliteActionRunner::new(ledger.clone());

    let report = runner.run_batch(&[SqliteActionRequest::new(
        "sqlite.backup-dir-link",
        target,
        SqliteAction::MinimalSqlite,
        SqliteActionSchedule::BootOnce,
    )]);

    assert_eq!(report.succeeded, 0);
    assert_eq!(report.failed, 1);
    assert_eq!(fs::read(&db_path)?, b"legacy sqlite rows");
    assert!(fs::read_dir(&outside)?.next().is_none());
    assert!(ledger.read_records()?.is_empty());
    fs::remove_dir_all(&root)?;
    fs::remove_dir_all(&outside)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn sqlite_runner_rejects_hardlinked_target_before_write() -> Result<(), Box<dyn Error>> {
    let root = temp_root()?;
    let db_path = root.join("data/data/com.example.video/databases/hardlinked.db");
    let sibling = root.join("hardlinked-sibling.db");
    fs::create_dir_all(parent(&db_path)?)?;
    fs::write(&db_path, b"shared sqlite rows")?;
    fs::hard_link(&db_path, &sibling)?;
    let target = SqliteActionTarget::from_android_path(
        "/data/data/com.example.video/databases/hardlinked.db",
        &db_path,
        &root,
    )?;
    let ledger = RestoreLedger::at(root.join("state/actions.jsonl"));
    let runner = SqliteActionRunner::new(ledger.clone());

    let report = runner.run_batch(&[SqliteActionRequest::new(
        "sqlite.hardlinked-target",
        target,
        SqliteAction::MinimalSqlite,
        SqliteActionSchedule::BootOnce,
    )]);

    assert_eq!(report.succeeded, 0);
    assert_eq!(report.failed, 1);
    assert_eq!(fs::read(&db_path)?, b"shared sqlite rows");
    assert_eq!(fs::read(&sibling)?, b"shared sqlite rows");
    assert!(ledger.read_records()?.is_empty());
    fs::remove_dir_all(&root)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn sqlite_missing_write_rejects_parent_symlink_escape() -> Result<(), Box<dyn Error>> {
    let root = temp_root()?;
    let external = root.with_extension("outside-sqlite-parent");
    fs::create_dir_all(&external)?;
    let parent_dir = root.join("data/data/com.example.video/databases");
    fs::create_dir_all(&parent_dir)?;
    let db_path = parent_dir.join("escaped.db");
    let target = SqliteActionTarget::from_android_path(
        "/data/data/com.example.video/databases/escaped.db",
        &db_path,
        &root,
    )?;
    fs::remove_dir_all(&parent_dir)?;
    std::os::unix::fs::symlink(&external, &parent_dir)?;
    let ledger = RestoreLedger::at(root.join("state/actions.jsonl"));
    let runner = SqliteActionRunner::new(ledger.clone());

    let report = runner.run_batch(&[SqliteActionRequest::new(
        "sqlite.parent-symlink-escape",
        target,
        SqliteAction::MinimalSqlite,
        SqliteActionSchedule::BootOnce,
    )]);

    assert_eq!(report.succeeded, 0);
    assert_eq!(report.failed, 1);
    assert!(!external.join("escaped.db").exists());
    assert!(ledger.read_records()?.is_empty());
    fs::remove_dir_all(&root)?;
    fs::remove_dir_all(&external)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn sqlite_parent_symlink_swap_after_request_rejected_for_all_mutations()
-> Result<(), Box<dyn Error>> {
    let cases = [
        (
            "minimal-parent-swap",
            SqliteAction::MinimalSqlite,
            SqliteActionSchedule::BootOnce,
        ),
        (
            "deny-write-parent-swap",
            SqliteAction::DenyWrite,
            SqliteActionSchedule::BootOnce,
        ),
        (
            "delete-parent-swap",
            SqliteAction::Delete,
            SqliteActionSchedule::Manual,
        ),
    ];

    for (case_name, action, schedule) in cases {
        assert_parent_symlink_swap_rejected(case_name, action, schedule)?;
    }

    Ok(())
}

#[cfg(unix)]
fn assert_parent_symlink_swap_rejected(
    case_name: &str,
    action: SqliteAction,
    schedule: SqliteActionSchedule,
) -> Result<(), Box<dyn Error>> {
    let root = temp_root()?;
    let external = root.with_extension(format!("outside-{case_name}"));
    fs::create_dir_all(&external)?;
    let parent_dir = root.join("data/data/com.example.video/databases");
    fs::create_dir_all(&parent_dir)?;
    let db_path = parent_dir.join(format!("{case_name}.db"));
    fs::write(&db_path, b"root sqlite rows")?;
    let external_db = external.join(format!("{case_name}.db"));
    fs::write(&external_db, b"outside sqlite rows")?;
    let target = SqliteActionTarget::from_android_path(
        format!("/data/data/com.example.video/databases/{case_name}.db"),
        &db_path,
        &root,
    )?;
    fs::remove_dir_all(&parent_dir)?;
    std::os::unix::fs::symlink(&external, &parent_dir)?;
    let ledger = RestoreLedger::at(root.join("state/actions.jsonl"));
    let runner = SqliteActionRunner::new(ledger.clone());

    let report = runner.run_batch(&[SqliteActionRequest::new(
        format!("sqlite.{case_name}"),
        target,
        action,
        schedule,
    )]);

    assert_eq!(report.succeeded, 0, "{case_name} should not mutate");
    assert_eq!(report.failed, 1, "{case_name} should fail closed");
    assert_eq!(fs::read(&external_db)?, b"outside sqlite rows");
    assert_ledger_empty(&ledger, case_name)?;
    fs::remove_dir_all(&root)?;
    fs::remove_dir_all(&external)?;
    Ok(())
}

#[cfg(unix)]
fn assert_ledger_empty(ledger: &RestoreLedger, case_name: &str) -> Result<(), Box<dyn Error>> {
    let records = ledger.read_records()?;
    if records.is_empty() {
        return Ok(());
    }
    Err(format!("{case_name} should not append a successful mutation record").into())
}
