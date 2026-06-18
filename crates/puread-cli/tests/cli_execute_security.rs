#![doc = "CLI execute/restore 安全边界测试。"]

use std::error::Error;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs as unix_fs;
use std::path::Path;

#[path = "support/cli_restore_adversarial.rs"]
#[cfg(unix)]
pub(crate) mod restore_adversarial;
#[path = "support/cli_execute_security.rs"]
pub(crate) mod support;

use support::{TempFixture, assert_success, field, parse_stdout_json, run_puread};

#[test]
fn cli_execute_restore_recovers_directory_from_ledger() -> Result<(), Box<dyn Error>> {
    // Given: apply-profile --execute emptied a copied splash cache directory.
    let fixture = TempFixture::new("restore-execute")?;
    let root = fixture.copy_android_fs()?;
    let root_arg = root.to_string_lossy().into_owned();
    assert_success(&run_puread([
        "apply-profile",
        "conservative",
        "--execute",
        "--root",
        root_arg.as_str(),
        "--module-root",
        fixture.module_root_str(),
    ])?)?;
    let target = root.join("sdcard/Android/data/com.ss.android.ugc.aweme/splashCache");
    assert!(fs::read_dir(&target)?.next().is_none());

    // When: restore --execute is driven through the real CLI surface.
    let ledger = root.join("data/adb/modules/puread/state/actions.jsonl");
    let output = run_puread([
        "restore",
        "--execute",
        "--ledger",
        ledger.to_string_lossy().as_ref(),
    ])?;

    // Then: the original directory marker is restored and restored records leave the ledger.
    assert_success(&output)?;
    let document = parse_stdout_json(&output)?;
    assert_eq!(field(&document, "mode")?, "execute");
    assert_eq!(field(&document, "failed")?, 0);
    assert!(target.join(".fixture").is_file());
    assert_eq!(fs::read_to_string(&ledger)?, "");
    Ok(())
}

#[test]
fn cli_execute_restore_fails_when_global_lock_is_held_without_restoring_or_clearing_ledger()
-> Result<(), Box<dyn Error>> {
    // Given: an executed profile has pending restore records and another process holds the lock.
    let fixture = TempFixture::new("restore-lock-conflict")?;
    let root = fixture.copy_android_fs()?;
    let root_arg = root.to_string_lossy().into_owned();
    assert_success(&run_puread([
        "apply-profile",
        "conservative",
        "--execute",
        "--root",
        root_arg.as_str(),
        "--module-root",
        fixture.module_root_str(),
    ])?)?;
    let target = root.join("sdcard/Android/data/com.ss.android.ugc.aweme/splashCache");
    assert!(fs::read_dir(&target)?.next().is_none());
    let ledger = root.join("data/adb/modules/puread/state/actions.jsonl");
    let before = fs::read_to_string(&ledger)?;
    let restore_lock = root.join("data/adb/modules/puread/run/puread.lock");
    fs::create_dir_all(restore_lock.parent().ok_or("restore lock has no parent")?)?;
    let lock_file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(restore_lock)?;
    fs2::FileExt::try_lock_exclusive(&lock_file)?;

    // When: restore --execute targets the locked module ledger.
    let output = run_puread([
        "restore",
        "--execute",
        "--ledger",
        ledger.to_string_lossy().as_ref(),
    ])?;

    // Then: restore fails before mutation and leaves both target and ledger untouched.
    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("global lock is already held"), "{stderr}");
    assert!(fs::read_dir(target)?.next().is_none());
    assert_eq!(fs::read_to_string(ledger)?, before);
    Ok(())
}

#[cfg(unix)]
#[test]
fn cli_execute_rejects_target_symlink_without_mutating_escape() -> Result<(), Box<dyn Error>> {
    // Given: the planned target is replaced by a symlink to an escape file.
    let fixture = TempFixture::new("target-symlink")?;
    let root = fixture.copy_android_fs()?;
    let target = root.join("sdcard/Android/data/com.ss.android.ugc.aweme/splashCache");
    fs::remove_dir_all(&target)?;
    let escape = fixture.root.join("escape");
    fs::write(&escape, "keep")?;
    unix_fs::symlink(&escape, &target)?;

    // When: apply-profile --execute validates the planned target.
    let output = run_apply_execute(&fixture, &root)?;

    // Then: planning or execution rejects the symlink and the escape file is untouched.
    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("symlink"), "{stderr}");
    assert_eq!(fs::read_to_string(escape)?, "keep");
    Ok(())
}

#[cfg(unix)]
#[test]
fn cli_execute_rejects_parent_symlink() -> Result<(), Box<dyn Error>> {
    // Given: a parent directory in the planned target path is a symlink.
    let fixture = TempFixture::new("parent-symlink")?;
    let root = fixture.copy_android_fs()?;
    let data_dir = root.join("sdcard/Android/data");
    fs::remove_dir_all(&data_dir)?;
    unix_fs::symlink(fixture.root.as_path(), &data_dir)?;

    // When: apply-profile --execute validates the parent chain.
    let output = run_apply_execute(&fixture, &root)?;

    // Then: the command does not follow the symlinked parent into the escape root.
    assert_success(&output)?;
    assert!(!fixture.root.join("com.ss.android.ugc.aweme").exists());
    Ok(())
}

#[test]
fn cli_execute_rejects_backup_symlink_and_keeps_prior_ledger() -> Result<(), Box<dyn Error>> {
    // Given: backup dir contains a symlink at the deterministic backup path for one rule.
    let fixture = TempFixture::new("backup-collision")?;
    let root = fixture.copy_android_fs()?;
    let backup = root.join("data/adb/modules/puread/state/backups");
    fs::create_dir_all(&backup)?;
    let occupied = backup.join(
        "aweme-external-splash-cache-empty_dir-_sdcard_Android_data_com_ss_android_ugc_aweme_splashCache.bak",
    );
    #[cfg(unix)]
    unix_fs::symlink(fixture.root.as_path(), &occupied)?;
    #[cfg(not(unix))]
    fs::write(&occupied, "occupied")?;

    // When: apply-profile --execute runs.
    let output = run_apply_execute(&fixture, &root)?;

    // Then: failure is visible and the ledger still contains the other successful mutation record.
    assert_success(&output)?;
    let document = parse_stdout_json(&output)?;
    assert!(field(&document, "failed")?.as_u64().unwrap_or(0) > 0);
    let ledger = root.join("data/adb/modules/puread/state/actions.jsonl");
    assert!(fs::read_to_string(ledger)?.contains("aweme-internal-splash-commerce-card"));
    Ok(())
}

#[test]
fn cli_execute_scan_reports_applied_and_failed_actions() -> Result<(), Box<dyn Error>> {
    // Given: one target can be emptied and another target is forced to fail by backup collision.
    let fixture = TempFixture::new("scan-partial-report")?;
    let root = fixture.copy_android_fs()?;
    let backup = root.join("data/adb/modules/puread/state/backups");
    fs::create_dir_all(&backup)?;
    let occupied = backup.join(
        "aweme-external-splash-cache-empty_dir-_sdcard_Android_data_com_ss_android_ugc_aweme_splashCache.bak",
    );
    fs::write(&occupied, "occupied")?;
    let root_arg = root.to_string_lossy().into_owned();

    // When: scan --execute is driven through the real CLI surface.
    let output = run_puread(["scan", "--execute", "--root", root_arg.as_str()])?;

    // Then: the command reports execution status and errors, not just the planned action list.
    assert_success(&output)?;
    let document = parse_stdout_json(&output)?;
    assert_eq!(field(&document, "mode")?, "execute");
    assert_eq!(field(&document, "will_mutate")?, true);
    assert_eq!(field(&document, "applied")?, &serde_json::json!(1));
    assert_eq!(field(&document, "failed")?, &serde_json::json!(1));
    let actions = field(&document, "actions")?
        .as_array()
        .ok_or("actions must be an array")?;
    assert_eq!(
        actions
            .iter()
            .filter(|action| action.get("status") == Some(&serde_json::json!("applied")))
            .count(),
        1
    );
    let failed = actions
        .iter()
        .find(|action| action.get("status") == Some(&serde_json::json!("failed")))
        .ok_or("missing failed action report")?;
    assert!(
        failed
            .get("error")
            .is_some_and(serde_json::Value::is_string)
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn cli_restore_execute_rejects_target_symlink() -> Result<(), Box<dyn Error>> {
    restore_adversarial::target_symlink_is_not_followed()
}

#[cfg(unix)]
#[test]
fn cli_restore_execute_rejects_target_parent_symlink() -> Result<(), Box<dyn Error>> {
    restore_adversarial::target_parent_symlink_is_not_followed()
}

#[cfg(unix)]
#[test]
fn cli_restore_execute_rejects_directory_restore_parent_symlink() -> Result<(), Box<dyn Error>> {
    restore_adversarial::directory_restore_parent_symlink_is_not_followed()
}

#[cfg(unix)]
#[test]
fn cli_restore_execute_rejects_remove_placeholder_parent_symlink() -> Result<(), Box<dyn Error>> {
    restore_adversarial::remove_placeholder_parent_symlink_is_not_followed()
}

#[cfg(unix)]
#[test]
fn cli_restore_execute_rejects_backup_symlink() -> Result<(), Box<dyn Error>> {
    restore_adversarial::backup_symlink_is_not_read()
}

#[cfg(unix)]
#[test]
fn cli_restore_execute_rejects_backup_parent_symlink() -> Result<(), Box<dyn Error>> {
    restore_adversarial::backup_parent_symlink_is_not_followed()
}

#[cfg(unix)]
#[test]
fn cli_restore_execute_rejects_backup_path_escape() -> Result<(), Box<dyn Error>> {
    restore_adversarial::backup_path_escape_is_rejected()
}

#[cfg(unix)]
#[test]
fn cli_restore_execute_recreates_missing_parent_directory_safely() -> Result<(), Box<dyn Error>> {
    restore_adversarial::missing_parent_directory_restore_uses_safe_path()
}

#[cfg(unix)]
#[test]
fn cli_restore_execute_restores_permissions_safely() -> Result<(), Box<dyn Error>> {
    restore_adversarial::permission_restore_uses_safe_path()
}

fn run_apply_execute(
    fixture: &TempFixture,
    root: &Path,
) -> Result<std::process::Output, Box<dyn Error>> {
    let root_arg = root.to_string_lossy();
    run_puread([
        "apply-profile",
        "conservative",
        "--execute",
        "--root",
        root_arg.as_ref(),
        "--module-root",
        fixture.module_root_str(),
    ])
}
