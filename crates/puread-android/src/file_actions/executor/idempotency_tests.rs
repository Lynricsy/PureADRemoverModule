use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use puread_core::model::{ProfileKind, RiskLevel, RuleId};
use puread_core::restore_ledger::RestoreStep;

use crate::file_actions::{FileActionKind, FileActionPlanner, FileActionRequest, FileActionTarget};

use super::test_support::temp_root;

#[test]
fn empty_dir_reuses_existing_ledger_when_recreated_directory_is_already_empty()
-> Result<(), Box<dyn Error>> {
    let root = temp_root()?;
    let android_path = "/data/data/com.example.app/cache/recreated-dir";
    let host_path = root.join("data/data/com.example.app/cache/recreated-dir");
    fs::create_dir_all(&host_path)?;
    fs::write(host_path.join("payload.bin"), b"payload")?;
    let target = FileActionTarget::new(android_path, &host_path, &root)?;
    let request = FileActionRequest::new(
        RuleId::parse("idempotent-empty-dir-test")?,
        FileActionKind::EmptyDir,
        target,
        ProfileKind::Conservative,
        RiskLevel::Low,
    );
    let plan = FileActionPlanner::new().plan(&request)?;
    let ledger = puread_core::restore_ledger::RestoreLedger::at(root.join("actions.jsonl"));
    let executor = super::FileActionExecutor::new(ledger.clone(), root.join("backups"));

    executor.execute(&plan)?;
    let second = executor.execute(&plan)?;

    assert_eq!(
        second.status(),
        crate::file_actions::FileActionStatus::Applied
    );
    assert!(host_path.is_dir());
    assert!(host_path.read_dir()?.next().is_none());
    assert_eq!(ledger.read_records()?.len(), 1);
    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn delete_reuses_existing_ledger_without_overwriting_first_backup() -> Result<(), Box<dyn Error>> {
    let root = temp_root()?;
    let android_path = "/data/data/com.example.app/cache/recreated-delete.bin";
    let host_path = root.join("data/data/com.example.app/cache/recreated-delete.bin");
    fs::create_dir_all(super::test_support::parent(&host_path)?)?;
    fs::write(&host_path, b"original backup payload")?;
    let target = FileActionTarget::new(android_path, &host_path, &root)?;
    let request = FileActionRequest::new(
        RuleId::parse("idempotent-delete-test")?,
        FileActionKind::Delete,
        target,
        ProfileKind::Conservative,
        RiskLevel::Low,
    );
    let plan = FileActionPlanner::new().plan(&request)?;
    let ledger = puread_core::restore_ledger::RestoreLedger::at(root.join("actions.jsonl"));
    let executor = super::FileActionExecutor::new(ledger.clone(), root.join("backups"));

    executor.execute(&plan)?;
    let backup_path = backup_path_from_ledger(&ledger.read_records()?)?;
    fs::write(&host_path, b"recreated ad payload")?;
    let second = executor.execute(&plan)?;

    assert_eq!(
        second.status(),
        crate::file_actions::FileActionStatus::Applied
    );
    assert!(!host_path.exists());
    assert_eq!(fs::read(backup_path)?, b"original backup payload");
    assert_eq!(ledger.read_records()?.len(), 1);
    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn empty_file_rejects_orphan_backup_without_mutating_target() -> Result<(), Box<dyn Error>> {
    let root = temp_root()?;
    let android_path = "/data/data/com.example.app/cache/orphan-backup.bin";
    let host_path = root.join("data/data/com.example.app/cache/orphan-backup.bin");
    fs::create_dir_all(super::test_support::parent(&host_path)?)?;
    fs::write(&host_path, b"target must remain")?;
    let target = FileActionTarget::new(android_path, &host_path, &root)?;
    let request = FileActionRequest::new(
        RuleId::parse("orphan-backup-empty-file")?,
        FileActionKind::EmptyFile,
        target,
        ProfileKind::Conservative,
        RiskLevel::Low,
    );
    let plan = FileActionPlanner::new().plan(&request)?;
    let backup_dir = root.join("backups");
    fs::create_dir_all(&backup_dir)?;
    fs::write(
        backup_dir.join("orphan-backup-empty-file-empty_file-_data_data_com_example_app_cache_orphan_backup_bin.bak"),
        b"stale backup",
    )?;
    let ledger = puread_core::restore_ledger::RestoreLedger::at(root.join("actions.jsonl"));
    let executor = super::FileActionExecutor::new(ledger.clone(), backup_dir);

    let result = executor.execute(&plan);

    assert!(result.is_err());
    assert_eq!(fs::read(&host_path)?, b"target must remain");
    assert!(ledger.read_records()?.is_empty());
    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn empty_file_rejects_existing_ledger_when_backup_missing_before_repeated_write()
-> Result<(), Box<dyn Error>> {
    let root = temp_root()?;
    let android_path = "/data/data/com.example.app/cache/missing-backup.bin";
    let host_path = root.join("data/data/com.example.app/cache/missing-backup.bin");
    fs::create_dir_all(super::test_support::parent(&host_path)?)?;
    fs::write(&host_path, b"original payload")?;
    let target = FileActionTarget::new(android_path, &host_path, &root)?;
    let request = FileActionRequest::new(
        RuleId::parse("missing-backup-empty-file")?,
        FileActionKind::EmptyFile,
        target,
        ProfileKind::Conservative,
        RiskLevel::Low,
    );
    let plan = FileActionPlanner::new().plan(&request)?;
    let ledger = puread_core::restore_ledger::RestoreLedger::at(root.join("actions.jsonl"));
    let executor = super::FileActionExecutor::new(ledger.clone(), root.join("backups"));

    executor.execute(&plan)?;
    let backup_path = backup_path_from_ledger(&ledger.read_records()?)?;
    fs::remove_file(backup_path)?;
    fs::write(&host_path, b"recreated target must remain")?;
    let result = executor.execute(&plan);

    assert!(result.is_err());
    assert_eq!(fs::read(&host_path)?, b"recreated target must remain");
    assert_eq!(ledger.read_records()?.len(), 1);
    fs::remove_dir_all(&root)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn empty_file_rejects_existing_ledger_when_backup_is_symlink_before_repeated_write()
-> Result<(), Box<dyn Error>> {
    let root = temp_root()?;
    let outside = root.with_extension("outside-repeated-backup.bin");
    let android_path = "/data/data/com.example.app/cache/symlink-backup.bin";
    let host_path = root.join("data/data/com.example.app/cache/symlink-backup.bin");
    fs::create_dir_all(super::test_support::parent(&host_path)?)?;
    fs::write(&host_path, b"original payload")?;
    let target = FileActionTarget::new(android_path, &host_path, &root)?;
    let request = FileActionRequest::new(
        RuleId::parse("symlink-backup-empty-file")?,
        FileActionKind::EmptyFile,
        target,
        ProfileKind::Conservative,
        RiskLevel::Low,
    );
    let plan = FileActionPlanner::new().plan(&request)?;
    let ledger = puread_core::restore_ledger::RestoreLedger::at(root.join("actions.jsonl"));
    let executor = super::FileActionExecutor::new(ledger.clone(), root.join("backups"));

    executor.execute(&plan)?;
    let backup_path = backup_path_from_ledger(&ledger.read_records()?)?;
    fs::remove_file(&backup_path)?;
    std::os::unix::fs::symlink(&outside, &backup_path)?;
    fs::write(&host_path, b"recreated target must remain")?;
    let result = executor.execute(&plan);

    assert!(result.is_err());
    assert_eq!(fs::read(&host_path)?, b"recreated target must remain");
    assert!(!outside.exists());
    assert_eq!(ledger.read_records()?.len(), 1);
    fs::remove_dir_all(&root)?;
    Ok(())
}

fn backup_path_from_ledger(
    records: &[puread_core::restore_ledger::LedgerRecord],
) -> Result<PathBuf, Box<dyn Error>> {
    let record = records.first().ok_or("expected one ledger record")?;
    let path = record
        .restore_steps
        .iter()
        .find_map(restore_content_path)
        .ok_or("expected restore content step")?;
    Ok(path)
}

fn restore_content_path(step: &RestoreStep) -> Option<PathBuf> {
    match step {
        RestoreStep::RestoreContent { backup_path } => Some(Path::new(backup_path).to_path_buf()),
        RestoreStep::RecreateDirectory
        | RestoreStep::RecreateFile
        | RestoreStep::RemovePlaceholder
        | RestoreStep::SetMode { .. }
        | RestoreStep::SetOwner { .. }
        | RestoreStep::SetSelinuxContext { .. }
        | RestoreStep::SetImmutable { .. } => None,
    }
}
