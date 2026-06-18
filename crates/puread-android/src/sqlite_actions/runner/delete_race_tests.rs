use std::error::Error;
use std::fs;

use puread_core::restore_ledger::{RestoreLedger, RestoreStep};

use crate::sqlite_actions::{
    SqliteAction, SqliteActionRequest, SqliteActionRunner, SqliteActionSchedule, SqliteActionTarget,
};

use super::test_support::{parent, temp_root};

#[cfg(unix)]
#[test]
fn sqlite_delete_rejects_dangling_backup_symlink_without_mutating_target()
-> Result<(), Box<dyn Error>> {
    let root = temp_root()?;
    let outside = root.with_extension("outside-sqlite-dangling-delete.db");
    let db_path = root.join("data/data/com.example.video/databases/delete-dangling-backup.db");
    fs::create_dir_all(parent(&db_path)?)?;
    fs::write(&db_path, b"legacy sqlite rows")?;
    let target = SqliteActionTarget::from_android_path(
        "/data/data/com.example.video/databases/delete-dangling-backup.db",
        &db_path,
        &root,
    )?;
    let ledger = RestoreLedger::at(root.join("state/actions.jsonl"));
    let request = SqliteActionRequest::new(
        "sqlite.delete-dangling-backup",
        target,
        SqliteAction::Delete,
        SqliteActionSchedule::Manual,
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
    std::os::unix::fs::symlink(&outside, &backup_path)?;
    let runner = SqliteActionRunner::new(ledger.clone());

    let report = runner.run_batch(&[request]);

    assert_eq!(report.succeeded, 0);
    assert_eq!(report.failed, 1);
    assert_eq!(fs::read(&db_path)?, b"legacy sqlite rows");
    assert!(!outside.exists());
    assert!(ledger.read_records()?.is_empty());
    fs::remove_dir_all(&root)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn sqlite_delete_discard_cleanup_rejects_backup_parent_symlink_swap() -> Result<(), Box<dyn Error>>
{
    let root = temp_root()?;
    let outside = root.with_extension("outside-sqlite-discard-cleanup");
    let db_path = root.join("data/data/com.example.video/databases/discard-cleanup.db");
    fs::create_dir_all(parent(&db_path)?)?;
    fs::write(&db_path, b"original sqlite rows")?;
    let target = SqliteActionTarget::from_android_path(
        "/data/data/com.example.video/databases/discard-cleanup.db",
        &db_path,
        &root,
    )?;
    let ledger = RestoreLedger::at(root.join("state/actions.jsonl"));
    let runner = SqliteActionRunner::new(ledger.clone());
    let request = SqliteActionRequest::new(
        "sqlite.discard-cleanup",
        target,
        SqliteAction::Delete,
        SqliteActionSchedule::Manual,
    );

    let first = runner.run_batch(std::slice::from_ref(&request));
    assert_eq!(first.succeeded, 1);
    fs::write(&db_path, b"recreated sqlite rows")?;
    let moved_parent = root.join("state/backups/sqlite-old");
    let outside_for_hook = outside.clone();
    let second = super::hooks::with_before_sqlite_discard_cleanup_hook_for_tests(
        move |discard_path| {
            let parent = discard_path
                .parent()
                .expect("discard path should have a parent");
            fs::create_dir_all(&outside_for_hook).expect("outside dir should be created");
            fs::write(
                outside_for_hook.join(
                    discard_path
                        .file_name()
                        .expect("discard path should have a file name"),
                ),
                b"external must remain",
            )
            .expect("outside sentinel should be written");
            fs::rename(parent, &moved_parent).expect("sqlite backup parent should be renamed");
            std::os::unix::fs::symlink(&outside_for_hook, parent)
                .expect("sqlite backup parent symlink should be created");
        },
        || runner.run_batch(&[request]),
    );

    assert_eq!(second.succeeded, 0);
    assert_eq!(second.failed, 1);
    assert_eq!(ledger.read_records()?.len(), 1);
    let external_file = fs::read_dir(&outside)?
        .next()
        .ok_or("missing outside sentinel")??;
    assert_eq!(fs::read(external_file.path())?, b"external must remain");
    fs::remove_dir_all(&root)?;
    fs::remove_dir_all(&outside)?;
    Ok(())
}

fn sqlite_restore_content_path(step: &RestoreStep) -> Option<std::path::PathBuf> {
    match step {
        RestoreStep::RestoreContent { backup_path } => Some(std::path::PathBuf::from(backup_path)),
        RestoreStep::RecreateDirectory
        | RestoreStep::RecreateFile
        | RestoreStep::RemovePlaceholder
        | RestoreStep::SetMode { .. }
        | RestoreStep::SetOwner { .. }
        | RestoreStep::SetSelinuxContext { .. }
        | RestoreStep::SetImmutable { .. } => None,
    }
}
