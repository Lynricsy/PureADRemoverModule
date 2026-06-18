use std::error::Error;
use std::fs;

use puread_core::model::{ProfileKind, RiskLevel, RuleId};

use crate::file_actions::mutate::{
    with_before_file_discard_cleanup_hook_for_tests, with_before_file_move_to_backup_hook_for_tests,
};
use crate::file_actions::{FileActionKind, FileActionPlanner, FileActionRequest, FileActionTarget};

use super::test_support::{parent, temp_root};

#[cfg(unix)]
#[test]
fn delete_restores_replacement_when_path_swaps_after_fd_guard() -> Result<(), Box<dyn Error>> {
    let root = temp_root()?;
    let android_path = "/data/data/com.example.app/cache/delete-post-fd.bin";
    let host_path = root.join("data/data/com.example.app/cache/delete-post-fd.bin");
    let replacement = root.join("replacement-post-fd.bin");
    fs::create_dir_all(parent(&host_path)?)?;
    fs::write(&host_path, b"original")?;
    fs::write(&replacement, b"replacement must survive")?;
    let replacement_before = fs::read(&replacement)?;
    let target = FileActionTarget::new(android_path, &host_path, &root)?;
    let request = FileActionRequest::new(
        RuleId::parse("post-fd-delete-test")?,
        FileActionKind::Delete,
        target,
        ProfileKind::Conservative,
        RiskLevel::Low,
    );
    let plan = FileActionPlanner::new().plan(&request)?;
    let ledger = puread_core::restore_ledger::RestoreLedger::at(root.join("actions.jsonl"));
    let executor = super::FileActionExecutor::new(ledger.clone(), root.join("backups"));
    let replacement_for_hook = replacement;

    let result = with_before_file_move_to_backup_hook_for_tests(
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
fn missing_empty_file_rejects_parent_symlink_escape() -> Result<(), Box<dyn Error>> {
    let root = temp_root()?;
    let external = root.with_extension("outside-parent");
    fs::create_dir_all(&external)?;
    let parent_dir = root.join("data/data/com.example.app/cache");
    fs::create_dir_all(&parent_dir)?;
    let host_path = parent_dir.join("new-ad.bin");
    let target = FileActionTarget::new(
        "/data/data/com.example.app/cache/new-ad.bin",
        &host_path,
        &root,
    )?;
    fs::remove_dir_all(&parent_dir)?;
    std::os::unix::fs::symlink(&external, &parent_dir)?;
    let request = FileActionRequest::new(
        RuleId::parse("parent-symlink-empty-file-test")?,
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
    assert!(!external.join("new-ad.bin").exists());
    assert!(ledger.read_records()?.is_empty());
    fs::remove_dir_all(&root)?;
    fs::remove_dir_all(&external)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn missing_empty_file_rejects_parent_replaced_after_root_check() -> Result<(), Box<dyn Error>> {
    let root = temp_root()?;
    let external = root.with_extension("outside-parent-after-check");
    fs::create_dir_all(&external)?;
    let parent_dir = root.join("data/data/com.example.app/cache");
    fs::create_dir_all(&parent_dir)?;
    let host_path = parent_dir.join("created-after-check.bin");
    let target = FileActionTarget::new(
        "/data/data/com.example.app/cache/created-after-check.bin",
        &host_path,
        &root,
    )?;
    let request = FileActionRequest::new(
        RuleId::parse("parent-symlink-after-check")?,
        FileActionKind::EmptyFile,
        target,
        ProfileKind::Conservative,
        RiskLevel::Low,
    );
    let plan = FileActionPlanner::new().plan(&request)?;
    let ledger = puread_core::restore_ledger::RestoreLedger::at(root.join("actions.jsonl"));
    let executor = super::FileActionExecutor::new(ledger.clone(), root.join("backups"));
    let external_for_hook = external.clone();

    let result = crate::file_actions::mutate::with_after_file_helper_guard_hook_for_tests(
        move |_path| {
            let _ignored = fs::remove_dir_all(&parent_dir);
            let _ignored = std::os::unix::fs::symlink(&external_for_hook, &parent_dir);
        },
        || executor.execute(&plan),
    );

    assert!(result.is_err());
    assert!(!external.join("created-after-check.bin").exists());
    assert!(ledger.read_records()?.is_empty());
    fs::remove_dir_all(&root)?;
    fs::remove_dir_all(&external)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn delete_rejects_dangling_backup_symlink_without_mutating_target() -> Result<(), Box<dyn Error>> {
    let root = temp_root()?;
    let outside = root.with_extension("outside-dangling-delete-backup.bin");
    let android_path = "/data/data/com.example.app/cache/delete-dangling-backup.bin";
    let host_path = root.join("data/data/com.example.app/cache/delete-dangling-backup.bin");
    fs::create_dir_all(parent(&host_path)?)?;
    fs::write(&host_path, b"target must remain")?;
    let target = FileActionTarget::new(android_path, &host_path, &root)?;
    let request = FileActionRequest::new(
        RuleId::parse("dangling-backup-delete-test")?,
        FileActionKind::Delete,
        target,
        ProfileKind::Conservative,
        RiskLevel::Low,
    );
    let plan = FileActionPlanner::new().plan(&request)?;
    let backup_dir = root.join("backups");
    fs::create_dir_all(&backup_dir)?;
    std::os::unix::fs::symlink(
        &outside,
        backup_dir.join("dangling-backup-delete-test-delete-_data_data_com_example_app_cache_delete_dangling_backup_bin.bak"),
    )?;
    let ledger = puread_core::restore_ledger::RestoreLedger::at(root.join("actions.jsonl"));
    let executor = super::FileActionExecutor::new(ledger.clone(), backup_dir);

    let result = executor.execute(&plan);

    assert!(result.is_err());
    assert_eq!(fs::read(&host_path)?, b"target must remain");
    assert!(!outside.exists());
    assert!(ledger.read_records()?.is_empty());
    fs::remove_dir_all(&root)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn delete_discard_cleanup_rejects_backup_parent_symlink_swap() -> Result<(), Box<dyn Error>> {
    let root = temp_root()?;
    let outside = root.with_extension("outside-file-discard-cleanup");
    let android_path = "/data/data/com.example.app/cache/discard-cleanup.bin";
    let host_path = root.join("data/data/com.example.app/cache/discard-cleanup.bin");
    fs::create_dir_all(parent(&host_path)?)?;
    fs::write(&host_path, b"original backup payload")?;
    let target = FileActionTarget::new(android_path, &host_path, &root)?;
    let request = FileActionRequest::new(
        RuleId::parse("discard-cleanup-delete-test")?,
        FileActionKind::Delete,
        target,
        ProfileKind::Conservative,
        RiskLevel::Low,
    );
    let plan = FileActionPlanner::new().plan(&request)?;
    let ledger = puread_core::restore_ledger::RestoreLedger::at(root.join("actions.jsonl"));
    let executor = super::FileActionExecutor::new(ledger.clone(), root.join("backups"));

    executor.execute(&plan)?;
    fs::write(&host_path, b"recreated payload")?;
    let moved_parent = root.join("backups-old");
    let outside_for_hook = outside.clone();
    let result = with_before_file_discard_cleanup_hook_for_tests(
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
            fs::rename(parent, &moved_parent).expect("backup parent should be renamed");
            std::os::unix::fs::symlink(&outside_for_hook, parent)
                .expect("backup parent symlink should be created");
        },
        || executor.execute(&plan),
    );

    assert!(result.is_err());
    assert_eq!(ledger.read_records()?.len(), 1);
    let external_file = fs::read_dir(&outside)?
        .next()
        .ok_or("missing outside sentinel")??;
    assert_eq!(fs::read(external_file.path())?, b"external must remain");
    fs::remove_dir_all(&root)?;
    fs::remove_dir_all(&outside)?;
    Ok(())
}
