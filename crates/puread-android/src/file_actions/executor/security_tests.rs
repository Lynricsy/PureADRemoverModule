use std::error::Error;
use std::fs;
use std::path::PathBuf;

use puread_core::model::{ProfileKind, RiskLevel, RuleId};

use crate::file_actions::{FileActionKind, FileActionPlanner, FileActionRequest, FileActionTarget};

use super::test_support::{parent, temp_root};

#[test]
fn delete_rejects_existing_backup_without_orphan_move() -> Result<(), Box<dyn Error>> {
    let root = temp_root()?;
    let android_path = "/data/data/com.example.app/cache/delete-existing-backup.bin";
    let host_path = root.join("data/data/com.example.app/cache/delete-existing-backup.bin");
    fs::create_dir_all(parent(&host_path)?)?;
    fs::write(&host_path, b"current")?;
    let target = FileActionTarget::new(android_path, &host_path, &root)?;
    let request = FileActionRequest::new(
        RuleId::parse("existing-backup-delete-test")?,
        FileActionKind::Delete,
        target,
        ProfileKind::Conservative,
        RiskLevel::Low,
    );
    let plan = FileActionPlanner::new().plan(&request)?;
    let backup_dir = root.join("backups");
    fs::create_dir_all(&backup_dir)?;
    fs::write(
        backup_dir.join("existing-backup-delete-test-delete-_data_data_com_example_app_cache_delete_existing_backup_bin.bak"),
        b"stale",
    )?;
    let ledger = puread_core::restore_ledger::RestoreLedger::at(root.join("actions.jsonl"));
    let executor = super::FileActionExecutor::new(ledger.clone(), backup_dir.clone());

    let result = executor.execute(&plan);

    assert!(result.is_err());
    assert_eq!(fs::read(&host_path)?, b"current");
    assert!(ledger.read_records()?.is_empty());
    assert_no_current_backups(&backup_dir)?;
    fs::remove_dir_all(&root)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn empty_file_rejects_dangling_backup_symlink_without_mutating_target() -> Result<(), Box<dyn Error>>
{
    let root = temp_root()?;
    let outside = root.with_extension("outside-backup-target.bin");
    let android_path = "/data/data/com.example.app/cache/backup-link.bin";
    let host_path = root.join("data/data/com.example.app/cache/backup-link.bin");
    fs::create_dir_all(parent(&host_path)?)?;
    fs::write(&host_path, b"current payload")?;
    let target = FileActionTarget::new(android_path, &host_path, &root)?;
    let request = FileActionRequest::new(
        RuleId::parse("backup-link-test")?,
        FileActionKind::EmptyFile,
        target,
        ProfileKind::Conservative,
        RiskLevel::Low,
    );
    let plan = FileActionPlanner::new().plan(&request)?;
    let backup_dir = root.join("backups");
    fs::create_dir_all(&backup_dir)?;
    std::os::unix::fs::symlink(
        &outside,
        backup_dir.join(
            "backup-link-test-empty_file-_data_data_com_example_app_cache_backup_link_bin.bak",
        ),
    )?;
    let ledger = puread_core::restore_ledger::RestoreLedger::at(root.join("actions.jsonl"));
    let executor = super::FileActionExecutor::new(ledger.clone(), backup_dir);

    let result = executor.execute(&plan);

    assert!(result.is_err());
    assert_eq!(fs::read(&host_path)?, b"current payload");
    assert!(!outside.exists());
    assert!(ledger.read_records()?.is_empty());
    fs::remove_dir_all(&root)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn empty_file_rejects_backup_dir_symlink_escape() -> Result<(), Box<dyn Error>> {
    let root = temp_root()?;
    let outside = root.with_extension("outside-backup-dir");
    fs::create_dir_all(&outside)?;
    let android_path = "/data/data/com.example.app/cache/backup-dir-link.bin";
    let host_path = root.join("data/data/com.example.app/cache/backup-dir-link.bin");
    fs::create_dir_all(parent(&host_path)?)?;
    fs::write(&host_path, b"current payload")?;
    let target = FileActionTarget::new(android_path, &host_path, &root)?;
    let request = FileActionRequest::new(
        RuleId::parse("backup-dir-link-test")?,
        FileActionKind::EmptyFile,
        target,
        ProfileKind::Conservative,
        RiskLevel::Low,
    );
    let plan = FileActionPlanner::new().plan(&request)?;
    let backup_dir = root.join("backups");
    std::os::unix::fs::symlink(&outside, &backup_dir)?;
    let ledger = puread_core::restore_ledger::RestoreLedger::at(root.join("actions.jsonl"));
    let executor = super::FileActionExecutor::new(ledger.clone(), backup_dir);

    let result = executor.execute(&plan);

    assert!(result.is_err());
    assert_eq!(fs::read(&host_path)?, b"current payload");
    assert!(fs::read_dir(&outside)?.next().is_none());
    assert!(ledger.read_records()?.is_empty());
    fs::remove_dir_all(&root)?;
    fs::remove_dir_all(&outside)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn empty_file_rejects_hardlinked_target_before_mutation() -> Result<(), Box<dyn Error>> {
    let root = temp_root()?;
    let android_path = "/data/data/com.example.app/cache/hardlinked.bin";
    let host_path = root.join("data/data/com.example.app/cache/hardlinked.bin");
    let sibling = root.join("hardlink-sibling.bin");
    fs::create_dir_all(parent(&host_path)?)?;
    fs::write(&host_path, b"shared payload")?;
    std::fs::hard_link(&host_path, &sibling)?;
    let target = FileActionTarget::new(android_path, &host_path, &root)?;
    let request = FileActionRequest::new(
        RuleId::parse("hardlink-empty-file-test")?,
        FileActionKind::EmptyFile,
        target,
        ProfileKind::Conservative,
        RiskLevel::Low,
    );
    let plan = FileActionPlanner::new().plan(&request)?;
    let ledger = puread_core::restore_ledger::RestoreLedger::at(root.join("actions.jsonl"));
    let executor = super::FileActionExecutor::new(ledger.clone(), root.join("backups"));

    let result = executor.execute(&plan);

    assert!(result.is_err());
    assert_eq!(fs::read(&host_path)?, b"shared payload");
    assert_eq!(fs::read(&sibling)?, b"shared payload");
    assert!(ledger.read_records()?.is_empty());
    fs::remove_dir_all(&root)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn empty_file_rejects_parent_replaced_with_external_symlink_after_plan()
-> Result<(), Box<dyn Error>> {
    let root = temp_root()?;
    let outside = root.with_extension("outside-parent-existing-file");
    let android_path = "/data/data/com.example.app/cache/parent-swap-empty.bin";
    let host_path = root.join("data/data/com.example.app/cache/parent-swap-empty.bin");
    let parent_dir = parent(&host_path)?;
    fs::create_dir_all(parent_dir)?;
    fs::write(&host_path, b"original payload")?;
    fs::create_dir_all(&outside)?;
    fs::write(
        outside.join("parent-swap-empty.bin"),
        b"outside must remain",
    )?;
    let target = FileActionTarget::new(android_path, &host_path, &root)?;
    let request = FileActionRequest::new(
        RuleId::parse("parent-swap-empty-file")?,
        FileActionKind::EmptyFile,
        target,
        ProfileKind::Conservative,
        RiskLevel::Low,
    );
    let plan = FileActionPlanner::new().plan(&request)?;
    fs::remove_dir_all(parent_dir)?;
    std::os::unix::fs::symlink(&outside, parent_dir)?;
    let ledger = puread_core::restore_ledger::RestoreLedger::at(root.join("actions.jsonl"));
    let executor = super::FileActionExecutor::new(ledger.clone(), root.join("backups"));

    let result = executor.execute(&plan);

    assert!(result.is_err());
    assert_eq!(
        fs::read(outside.join("parent-swap-empty.bin"))?,
        b"outside must remain"
    );
    assert!(ledger.read_records()?.is_empty());
    fs::remove_dir_all(&root)?;
    fs::remove_dir_all(&outside)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn delete_rejects_parent_replaced_with_external_symlink_after_plan() -> Result<(), Box<dyn Error>> {
    let root = temp_root()?;
    let outside = root.with_extension("outside-parent-delete-file");
    let android_path = "/data/data/com.example.app/cache/parent-swap-delete.bin";
    let host_path = root.join("data/data/com.example.app/cache/parent-swap-delete.bin");
    let parent_dir = parent(&host_path)?;
    fs::create_dir_all(parent_dir)?;
    fs::write(&host_path, b"original payload")?;
    fs::create_dir_all(&outside)?;
    fs::write(
        outside.join("parent-swap-delete.bin"),
        b"outside must remain",
    )?;
    let target = FileActionTarget::new(android_path, &host_path, &root)?;
    let request = FileActionRequest::new(
        RuleId::parse("parent-swap-delete-file")?,
        FileActionKind::Delete,
        target,
        ProfileKind::Conservative,
        RiskLevel::Low,
    );
    let plan = FileActionPlanner::new().plan(&request)?;
    fs::remove_dir_all(parent_dir)?;
    std::os::unix::fs::symlink(&outside, parent_dir)?;
    let ledger = puread_core::restore_ledger::RestoreLedger::at(root.join("actions.jsonl"));
    let executor = super::FileActionExecutor::new(ledger.clone(), root.join("backups"));

    let result = executor.execute(&plan);

    assert!(result.is_err());
    assert_eq!(
        fs::read(outside.join("parent-swap-delete.bin"))?,
        b"outside must remain"
    );
    assert!(ledger.read_records()?.is_empty());
    fs::remove_dir_all(&root)?;
    fs::remove_dir_all(&outside)?;
    Ok(())
}

fn assert_no_current_backups(backup_dir: &std::path::Path) -> Result<(), Box<dyn Error>> {
    for entry_result in fs::read_dir(backup_dir)? {
        let path: PathBuf = entry_result?.path();
        assert!(!path.display().to_string().contains(".current-"));
    }
    Ok(())
}
