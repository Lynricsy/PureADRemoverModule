#![doc = "文件动作执行器行为测试。"]

use std::error::Error;
use std::fs;

use puread_android::file_actions::{
    ExecutionMode, FileActionExecutor, FileActionKind, FileActionPlanner, FileActionStatus,
    MetadataChange, MetadataOperation,
};
use puread_core::model::RiskLevel;
use puread_core::restore_ledger::{LedgerAction, OriginalFileType, RestoreStep};

include!("file_actions/support.rs");

#[test]
fn file_actions_dry_run_reports_plan_without_mutating_files_or_ledger() -> Result<(), Box<dyn Error>>
{
    // Given: an existing app-private ad cache file and an empty state ledger.
    let root = TestRoot::new()?;
    let android_path = "/data/data/com.example.app/cache/splash-ad.bin";
    root.write(android_path, b"ad payload")?;
    let plan = plan_for(
        &root,
        android_path,
        FileActionKind::EmptyFile,
        RiskLevel::Low,
    )?;
    let executor = FileActionExecutor::new(root.ledger(), root.backup_dir());

    // When: the action is evaluated through the dry-run surface.
    let outcome = executor.dry_run(&plan)?;

    // Then: the output is explicit about not mutating anything.
    println!("file_actions_dry_run={outcome:?}");
    assert_eq!(outcome.mode(), ExecutionMode::DryRun);
    assert_eq!(outcome.status(), FileActionStatus::Planned);
    assert!(!outcome.will_mutate());
    assert_eq!(fs::read(root.host_path(android_path))?, b"ad payload");
    assert!(root.ledger().read_records()?.is_empty());
    Ok(())
}

#[test]
fn file_actions_empty_file_executes_in_temp_root_and_records_reversible_ledger()
-> Result<(), Box<dyn Error>> {
    // Given: a real file under the temporary Android filesystem.
    let root = TestRoot::new()?;
    let android_path = "/data/data/com.example.app/cache/ad.bin";
    root.write(android_path, b"payload")?;
    let plan = plan_for(
        &root,
        android_path,
        FileActionKind::EmptyFile,
        RiskLevel::Low,
    )?;
    let executor = FileActionExecutor::new(root.ledger(), root.backup_dir());

    // When: the action is really applied.
    let outcome = executor.execute(&plan)?;

    // Then: only the temp-root file is changed and restore evidence is recorded first.
    let records = root.ledger().read_records()?;
    println!("file_actions_empty_file={outcome:?} records={records:?}");
    assert_eq!(outcome.mode(), ExecutionMode::Apply);
    assert_eq!(outcome.status(), FileActionStatus::Applied);
    assert_eq!(fs::read(root.host_path(android_path))?, b"");
    assert_eq!(records.len(), 1);
    let record = first_record(&records)?;
    assert_eq!(record.action, LedgerAction::EmptyFile);
    assert_eq!(record.original_file_type, OriginalFileType::File);
    assert!(
        record
            .restore_steps
            .iter()
            .any(|step| matches!(step, RestoreStep::RestoreContent { .. }))
    );
    Ok(())
}

#[test]
fn file_actions_repeated_execution_preserves_first_backup_and_single_ledger_record()
-> Result<(), Box<dyn Error>> {
    // Given: a file action has already captured the original payload once.
    let root = TestRoot::new()?;
    let android_path = "/data/data/com.example.app/cache/repeated-ad.bin";
    root.write(android_path, b"original payload")?;
    let plan = plan_for(
        &root,
        android_path,
        FileActionKind::EmptyFile,
        RiskLevel::Low,
    )?;
    let executor = FileActionExecutor::new(root.ledger(), root.backup_dir());
    executor.execute(&plan)?;
    let first_records = root.ledger().read_records()?;
    let backup_path = restore_backup_path(first_record(&first_records)?)?;
    root.write(android_path, b"recreated ad payload")?;

    // When: the exact same plan runs again against the same target.
    let second = executor.execute(&plan)?;

    // Then: the first backup is not overwritten, the ledger stays idempotent,
    // and the recreated ad payload is removed again.
    let records = root.ledger().read_records()?;
    println!("file_actions_repeated={second:?} records={records:?}");
    assert_eq!(records.len(), 1);
    assert_eq!(fs::read(&backup_path)?, b"original payload");
    assert_eq!(fs::read(root.host_path(android_path))?, b"");
    assert_eq!(second.status(), FileActionStatus::Applied);
    Ok(())
}

#[test]
fn file_actions_delete_rejects_symlink_targets_before_backup_or_mutation()
-> Result<(), Box<dyn Error>> {
    // Given: a package-local path that is a symlink to a file outside the temp root.
    let root = TestRoot::new()?;
    let outside = std::env::temp_dir().join(format!(
        "puread-file-actions-outside-{}",
        std::process::id()
    ));
    fs::write(&outside, b"outside")?;
    root.mkdir("/data/data/com.example.app/cache")?;
    std::os::unix::fs::symlink(
        &outside,
        root.host_path("/data/data/com.example.app/cache/link.bin"),
    )?;

    // When: a delete action is planned for the symlink.
    let result = root.target("/data/data/com.example.app/cache/link.bin");

    // Then: the target is rejected and no outside file is touched.
    println!("file_actions_symlink_target_result={result:?}");
    assert!(result.is_err());
    assert_eq!(fs::read(&outside)?, b"outside");
    fs::remove_file(outside)?;
    assert!(root.ledger().read_records()?.is_empty());
    Ok(())
}

#[test]
fn file_actions_rejects_dangerous_android_paths_before_execution() -> Result<(), Box<dyn Error>> {
    // Given: broad Android roots that must never become mutation targets.
    let root = TestRoot::new()?;
    let dangerous = [
        "/",
        "/data",
        "/sdcard",
        "/system",
        "/vendor",
        "/data/adb",
        "/data/adb/modules/puread/cache.bin",
        "/data/local/tmp/ad.bin",
    ];

    // When / Then: target construction rejects them before an executor exists.
    for android_path in dangerous {
        let result = root.target(android_path);
        println!("file_actions_dangerous_path={android_path} result={result:?}");
        assert!(result.is_err(), "{android_path} should be rejected");
    }
    Ok(())
}

#[test]
fn file_actions_skips_high_risk_chcon_when_current_context_is_unknown() -> Result<(), Box<dyn Error>>
{
    // Given: a high-risk action asking the executor to preserve the current context.
    let root = TestRoot::new()?;
    let android_path = "/data/data/com.example.app/cache/ad.bin";
    root.write(android_path, b"payload")?;
    let mut request = request_for(
        &root,
        android_path,
        FileActionKind::Chmod000,
        RiskLevel::High,
    )?;
    request.push_metadata_change(MetadataChange::restore_selinux_context());
    let plan = FileActionPlanner::new().plan(&request)?;
    let executor = FileActionExecutor::new(root.ledger(), root.backup_dir());

    // When: SELinux context cannot be discovered on the host test filesystem.
    let outcome = executor.execute(&plan)?;

    // Then: chmod is applied, chcon is explicitly skipped, and no blind context is written.
    let records = root.ledger().read_records()?;
    println!("file_actions_chcon_skip={outcome:?} records={records:?}");
    assert_eq!(outcome.status(), FileActionStatus::Applied);
    assert!(
        outcome
            .metadata_operations()
            .contains(&MetadataOperation::SkippedChconUnknownContext)
    );
    assert!(
        records
            .iter()
            .all(|record| record.selinux_context.is_none())
    );
    assert!(records.iter().all(|record| {
        !record
            .restore_steps
            .iter()
            .any(|step| matches!(step, RestoreStep::SetSelinuxContext { .. }))
    }));
    Ok(())
}
