use std::cell::Cell;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use puread_core::model::{ProfileKind, RiskLevel, RuleId};

use crate::file_actions::mutate::{
    with_after_file_helper_guard_hook_for_tests, with_before_file_delete_hook_for_tests,
    with_before_file_helper_guard_hook_for_tests,
};
use crate::file_actions::{FileActionKind, FileActionPlanner, FileActionRequest, FileActionTarget};

use super::test_support::{parent, temp_root};
use super::{TargetSnapshot, guard_target_unchanged};

#[cfg(unix)]
#[test]
fn guard_target_unchanged_rejects_symlink_replacement_before_mutation() -> Result<(), Box<dyn Error>>
{
    let root = temp_root()?;
    let android_path = "/data/data/com.example.app/cache/ad.bin";
    let host_path = root.join("data/data/com.example.app/cache/ad.bin");
    let outside = root.with_extension("outside.bin");
    fs::create_dir_all(parent(&host_path)?)?;
    fs::write(&host_path, b"ad")?;
    fs::write(&outside, b"outside")?;
    let target = FileActionTarget::new(android_path, &host_path, &root)?;
    let snapshot = TargetSnapshot::collect(&target)?;
    fs::remove_file(&host_path)?;
    std::os::unix::fs::symlink(&outside, &host_path)?;
    let request = FileActionRequest::new(
        RuleId::parse("guard-test")?,
        FileActionKind::Delete,
        target,
        ProfileKind::Conservative,
        RiskLevel::Low,
    );
    let plan = FileActionPlanner::new().plan(&request)?;

    let result = guard_target_unchanged(&plan, &snapshot);

    fs::remove_dir_all(&root)?;
    assert!(result.is_err());
    Ok(())
}

#[cfg(unix)]
#[test]
fn file_helper_final_guard_rejects_swap_after_executor_guard() -> Result<(), Box<dyn Error>> {
    let root = temp_root()?;
    let android_path = "/data/data/com.example.app/cache/ad.bin";
    let host_path = root.join("data/data/com.example.app/cache/ad.bin");
    let outside = root.join("outside.bin");
    fs::create_dir_all(parent(&host_path)?)?;
    fs::write(&host_path, b"ad")?;
    fs::write(&outside, b"outside")?;
    let target = FileActionTarget::new(android_path, &host_path, &root)?;
    let request = FileActionRequest::new(
        RuleId::parse("helper-guard-test")?,
        FileActionKind::EmptyFile,
        target,
        ProfileKind::Conservative,
        RiskLevel::Low,
    );
    let plan = FileActionPlanner::new().plan(&request)?;
    let ledger = puread_core::restore_ledger::RestoreLedger::at(root.join("actions.jsonl"));
    let executor = super::FileActionExecutor::new(ledger.clone(), root.join("backups"));
    let swapped = Rc::new(Cell::new(false));
    let hook_swapped = Rc::clone(&swapped);
    let symlink_target = outside.clone();

    let result = with_before_file_helper_guard_hook_for_tests(
        move |path| {
            if hook_swapped.replace(true) {
                return;
            }
            let _ignored = fs::remove_file(path);
            let _ignored = std::os::unix::fs::symlink(&symlink_target, path);
        },
        || executor.execute(&plan),
    );

    let records = ledger.read_records()?;
    assert!(result.is_err());
    assert!(swapped.get());
    assert_eq!(fs::read(&outside)?, b"outside");
    fs::remove_dir_all(&root)?;
    assert!(records.is_empty());
    Ok(())
}

#[cfg(unix)]
#[test]
fn empty_file_rejects_regular_file_replacement_after_helper_guard() -> Result<(), Box<dyn Error>> {
    let root = temp_root()?;
    let android_path = "/data/data/com.example.app/cache/ad.bin";
    let host_path = root.join("data/data/com.example.app/cache/ad.bin");
    let replacement = root.join("replacement.bin");
    fs::create_dir_all(parent(&host_path)?)?;
    fs::write(&host_path, b"ad")?;
    fs::write(&replacement, b"replacement must survive")?;
    let replacement_before = fs::read(&replacement)?;
    let target = FileActionTarget::new(android_path, &host_path, &root)?;
    let request = FileActionRequest::new(
        RuleId::parse("helper-open-guard-test")?,
        FileActionKind::EmptyFile,
        target,
        ProfileKind::Conservative,
        RiskLevel::Low,
    );
    let plan = FileActionPlanner::new().plan(&request)?;
    let ledger = puread_core::restore_ledger::RestoreLedger::at(root.join("actions.jsonl"));
    let executor = super::FileActionExecutor::new(ledger.clone(), root.join("backups"));
    let replacement_for_hook = replacement;

    let result = with_after_file_helper_guard_hook_for_tests(
        move |path| {
            let _ignored = fs::remove_file(path);
            let _ignored = fs::rename(&replacement_for_hook, path);
        },
        || executor.execute(&plan),
    );

    assert!(result.is_err());
    assert_eq!(fs::read(&host_path)?, replacement_before);
    assert!(ledger.read_records()?.is_empty());
    fs::remove_dir_all(&root)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn empty_file_rejects_new_file_created_after_missing_guard() -> Result<(), Box<dyn Error>> {
    let root = temp_root()?;
    let android_path = "/data/data/com.example.app/cache/new-ad.bin";
    let host_path = root.join("data/data/com.example.app/cache/new-ad.bin");
    fs::create_dir_all(parent(&host_path)?)?;
    let target = FileActionTarget::new(android_path, &host_path, &root)?;
    let request = FileActionRequest::new(
        RuleId::parse("helper-create-new-guard-test")?,
        FileActionKind::EmptyFile,
        target,
        ProfileKind::Conservative,
        RiskLevel::Low,
    );
    let plan = FileActionPlanner::new().plan(&request)?;
    let ledger = puread_core::restore_ledger::RestoreLedger::at(root.join("actions.jsonl"));
    let executor = super::FileActionExecutor::new(ledger.clone(), root.join("backups"));

    let result = with_after_file_helper_guard_hook_for_tests(
        move |path| {
            let _ignored = fs::write(path, b"new file must survive");
        },
        || executor.execute(&plan),
    );

    assert!(result.is_err());
    assert_eq!(fs::read(&host_path)?, b"new file must survive");
    assert!(ledger.read_records()?.is_empty());
    fs::remove_dir_all(&root)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn delete_rejects_regular_file_replacement_before_remove() -> Result<(), Box<dyn Error>> {
    let root = temp_root()?;
    let android_path = "/data/data/com.example.app/cache/delete-ad.bin";
    let host_path = root.join("data/data/com.example.app/cache/delete-ad.bin");
    let replacement = root.join("replacement-delete.bin");
    fs::create_dir_all(parent(&host_path)?)?;
    fs::write(&host_path, b"original")?;
    fs::write(&replacement, b"replacement must survive")?;
    let replacement_before = fs::read(&replacement)?;
    let target = FileActionTarget::new(android_path, &host_path, &root)?;
    let request = FileActionRequest::new(
        RuleId::parse("helper-delete-guard-test")?,
        FileActionKind::Delete,
        target,
        ProfileKind::Conservative,
        RiskLevel::Low,
    );
    let plan = FileActionPlanner::new().plan(&request)?;
    let ledger = puread_core::restore_ledger::RestoreLedger::at(root.join("actions.jsonl"));
    let executor = super::FileActionExecutor::new(ledger.clone(), root.join("backups"));
    let replacement_for_hook = replacement;

    let result = with_before_file_delete_hook_for_tests(
        move |path| {
            let _ignored = fs::remove_file(path);
            let _ignored = fs::rename(&replacement_for_hook, path);
        },
        || executor.execute(&plan),
    );

    assert!(result.is_err());
    assert_eq!(fs::read(&host_path)?, replacement_before);
    assert!(ledger.read_records()?.is_empty());
    fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn empty_dir_moves_existing_directory_to_backup_and_recreates_empty_dir()
-> Result<(), Box<dyn Error>> {
    let root = temp_root()?;
    let android_path = "/data/data/com.example.app/cache/ad-dir";
    let host_path = root.join("data/data/com.example.app/cache/ad-dir");
    fs::create_dir_all(&host_path)?;
    fs::write(host_path.join("payload.bin"), b"payload")?;
    let target = FileActionTarget::new(android_path, &host_path, &root)?;
    let request = FileActionRequest::new(
        RuleId::parse("helper-empty-dir-guard-test")?,
        FileActionKind::EmptyDir,
        target,
        ProfileKind::Conservative,
        RiskLevel::Low,
    );
    let plan = FileActionPlanner::new().plan(&request)?;
    let ledger = puread_core::restore_ledger::RestoreLedger::at(root.join("actions.jsonl"));
    let executor = super::FileActionExecutor::new(ledger.clone(), root.join("backups"));

    let outcome = executor.execute(&plan)?;

    assert_eq!(
        outcome.status(),
        crate::file_actions::FileActionStatus::Applied
    );
    assert!(host_path.is_dir());
    assert!(!host_path.join("payload.bin").exists());
    let records = ledger.read_records()?;
    let record = records.first().ok_or("expected one ledger record")?;
    let backup_path = record
        .restore_steps
        .iter()
        .find_map(|step| match step {
            puread_core::restore_ledger::RestoreStep::RestoreContent { backup_path } => {
                Some(PathBuf::from(backup_path))
            }
            puread_core::restore_ledger::RestoreStep::RecreateDirectory
            | puread_core::restore_ledger::RestoreStep::RecreateFile
            | puread_core::restore_ledger::RestoreStep::RemovePlaceholder
            | puread_core::restore_ledger::RestoreStep::SetMode { .. }
            | puread_core::restore_ledger::RestoreStep::SetOwner { .. }
            | puread_core::restore_ledger::RestoreStep::SetSelinuxContext { .. }
            | puread_core::restore_ledger::RestoreStep::SetImmutable { .. } => None,
        })
        .ok_or("expected backup path")?;
    assert_eq!(fs::read(backup_path.join("payload.bin"))?, b"payload");
    fs::remove_dir_all(&root)?;
    Ok(())
}
