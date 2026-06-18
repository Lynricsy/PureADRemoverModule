use std::error::Error;
use std::fs;

use puread_core::restore_ledger::RestoreLedger;

use crate::sqlite_actions::{
    SqliteAction, SqliteActionRequest, SqliteActionRunner, SqliteActionSchedule,
    SqliteActionStatus, SqliteActionTarget,
};

use super::test_support::{parent, temp_root};

#[cfg(unix)]
#[test]
fn sqlite_runner_removes_pending_ledger_when_target_swapped_after_append()
-> Result<(), Box<dyn Error>> {
    let root = temp_root()?;
    let db_path = root.join("data/data/com.example.video/databases/ad.db");
    let external = root.with_extension("external.db");
    fs::create_dir_all(parent(&db_path)?)?;
    fs::write(&db_path, b"legacy ad rows")?;
    fs::write(&external, b"outside")?;
    let target = SqliteActionTarget::from_android_path(
        "/data/data/com.example.video/databases/ad.db",
        &db_path,
        &root,
    )?;
    let ledger = RestoreLedger::at(root.join("state/actions.jsonl"));
    let runner = SqliteActionRunner::new(ledger.clone());
    let external_for_hook = external.clone();

    let report = super::hooks::with_after_sqlite_ledger_append_hook_for_tests(
        move |path| {
            let _ignored = fs::remove_file(path);
            let _ignored = std::os::unix::fs::symlink(&external_for_hook, path);
        },
        || {
            runner.run_batch(&[SqliteActionRequest::new(
                "sqlite.toctou",
                target,
                SqliteAction::MinimalSqlite,
                SqliteActionSchedule::BootOnce,
            )])
        },
    );

    assert_eq!(report.succeeded, 0);
    assert_eq!(report.failed, 1);
    let first_outcome = report.outcomes.first().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, "missing sqlite outcome")
    })?;
    assert!(matches!(
        first_outcome.status,
        SqliteActionStatus::Failed(_)
    ));
    assert_eq!(fs::read(&external)?, b"outside");
    assert!(ledger.read_records()?.is_empty());
    fs::remove_dir_all(&root)?;
    fs::remove_file(&external)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn sqlite_runner_preserves_pending_ledger_when_target_replaced_after_write()
-> Result<(), Box<dyn Error>> {
    use std::os::unix::fs::PermissionsExt;

    let root = temp_root()?;
    let db_path = root.join("data/data/com.example.video/databases/post-write.db");
    let external = root.with_extension("post-write-external.db");
    fs::create_dir_all(parent(&db_path)?)?;
    fs::write(&db_path, b"legacy ad rows")?;
    fs::write(&external, b"outside")?;
    fs::set_permissions(&external, fs::Permissions::from_mode(0o600))?;
    let external_mode = fs::metadata(&external)?.permissions().mode() & 0o777;
    let target = SqliteActionTarget::from_android_path(
        "/data/data/com.example.video/databases/post-write.db",
        &db_path,
        &root,
    )?;
    let ledger = RestoreLedger::at(root.join("state/actions.jsonl"));
    let runner = SqliteActionRunner::new(ledger.clone());
    let external_for_hook = external.clone();

    let report = super::hooks::with_after_sqlite_write_hook_for_tests(
        move |path| {
            let _ignored = fs::remove_file(path);
            let _ignored = std::os::unix::fs::symlink(&external_for_hook, path);
        },
        || {
            runner.run_batch(&[SqliteActionRequest::new(
                "sqlite.post-write-toctou",
                target,
                SqliteAction::MinimalSqlite,
                SqliteActionSchedule::BootOnce,
            )])
        },
    );

    assert_eq!(report.succeeded, 0);
    assert_eq!(report.failed, 1);
    let first_outcome = report.outcomes.first().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, "missing sqlite outcome")
    })?;
    assert!(matches!(
        first_outcome.status,
        SqliteActionStatus::Failed(_)
    ));
    assert_eq!(fs::read(&external)?, b"outside");
    assert_eq!(
        fs::metadata(&external)?.permissions().mode() & 0o777,
        external_mode
    );
    assert_eq!(ledger.read_records()?.len(), 1);
    fs::remove_dir_all(&root)?;
    fs::remove_file(&external)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn sqlite_runner_rejects_regular_file_replacement_before_write_open() -> Result<(), Box<dyn Error>>
{
    let root = temp_root()?;
    let db_path = root.join("data/data/com.example.video/databases/pre-open.db");
    let replacement = root.join("replacement.db");
    fs::create_dir_all(parent(&db_path)?)?;
    fs::write(&db_path, b"legacy ad rows")?;
    fs::write(&replacement, b"replacement must survive")?;
    let replacement_before = fs::read(&replacement)?;
    let target = SqliteActionTarget::from_android_path(
        "/data/data/com.example.video/databases/pre-open.db",
        &db_path,
        &root,
    )?;
    let ledger = RestoreLedger::at(root.join("state/actions.jsonl"));
    let runner = SqliteActionRunner::new(ledger.clone());
    let replacement_for_hook = replacement;

    let report = super::hooks::with_before_sqlite_write_open_hook_for_tests(
        move |path| {
            let _ignored = fs::remove_file(path);
            let _ignored = fs::rename(&replacement_for_hook, path);
        },
        || {
            runner.run_batch(&[SqliteActionRequest::new(
                "sqlite.pre-open-toctou",
                target,
                SqliteAction::MinimalSqlite,
                SqliteActionSchedule::BootOnce,
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
fn sqlite_runner_rejects_new_file_created_after_missing_guard() -> Result<(), Box<dyn Error>> {
    let root = temp_root()?;
    let db_path = root.join("data/data/com.example.video/databases/newly-created.db");
    fs::create_dir_all(parent(&db_path)?)?;
    let target = SqliteActionTarget::from_android_path(
        "/data/data/com.example.video/databases/newly-created.db",
        &db_path,
        &root,
    )?;
    let ledger = RestoreLedger::at(root.join("state/actions.jsonl"));
    let runner = SqliteActionRunner::new(ledger.clone());

    let report = super::hooks::with_before_sqlite_write_open_hook_for_tests(
        move |path| {
            let _ignored = fs::write(path, b"new sqlite candidate must survive");
        },
        || {
            runner.run_batch(&[SqliteActionRequest::new(
                "sqlite.create-new-toctou",
                target,
                SqliteAction::MinimalSqlite,
                SqliteActionSchedule::BootOnce,
            )])
        },
    );

    assert_eq!(report.succeeded, 0);
    assert_eq!(report.failed, 1);
    assert_eq!(fs::read(&db_path)?, b"new sqlite candidate must survive");
    assert!(ledger.read_records()?.is_empty());
    fs::remove_dir_all(&root)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn sqlite_runner_rejects_regular_file_replacement_before_delete() -> Result<(), Box<dyn Error>> {
    let root = temp_root()?;
    let db_path = root.join("data/data/com.example.video/databases/delete-race.db");
    let replacement = root.join("delete-replacement.db");
    fs::create_dir_all(parent(&db_path)?)?;
    fs::write(&db_path, b"legacy ad rows")?;
    fs::write(&replacement, b"replacement must survive")?;
    let replacement_before = fs::read(&replacement)?;
    let target = SqliteActionTarget::from_android_path(
        "/data/data/com.example.video/databases/delete-race.db",
        &db_path,
        &root,
    )?;
    let ledger = RestoreLedger::at(root.join("state/actions.jsonl"));
    let runner = SqliteActionRunner::new(ledger.clone());
    let replacement_for_hook = replacement;

    let report = super::hooks::with_before_sqlite_delete_hook_for_tests(
        move |path| {
            let _ignored = fs::remove_file(path);
            let _ignored = fs::rename(&replacement_for_hook, path);
        },
        || {
            runner.run_batch(&[SqliteActionRequest::new(
                "sqlite.delete-toctou",
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
