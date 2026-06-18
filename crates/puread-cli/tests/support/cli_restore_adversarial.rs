#![allow(
    clippy::redundant_pub_crate,
    reason = "integration test support helpers are imported from their parent test module"
)]

use std::error::Error;
use std::fs;
use std::os::unix::fs as unix_fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;

use super::support::{
    TempFixture, assert_restore_failed_without_clearing, assert_success, backup_path, field,
    ledger_path, parse_stdout_json, recreate_directory_step, recreate_file_step,
    remove_placeholder_step, restore_content_step, run_puread, set_mode_step, write_restore_ledger,
};

pub(crate) fn target_symlink_is_not_followed() -> Result<(), Box<dyn Error>> {
    let fixture = TempFixture::new("restore-target-symlink")?;
    let root = fixture.minimal_android_fs()?;
    let ledger = ledger_path(&root);
    let backup = backup_path(&root, "target-symlink.bak");
    fs::write(&backup, "restored")?;
    let target = root.join("data/data/com.demo/cache/ad.bin");
    fs::create_dir_all(parent(&target)?)?;
    let escape = fixture.root.join("escape-target");
    fs::write(&escape, "keep")?;
    unix_fs::symlink(&escape, &target)?;
    let step = restore_content_step(&backup);
    let before = write_restore_ledger(&ledger, "/data/data/com.demo/cache/ad.bin", &[&step])?;

    let output = restore_execute(&ledger)?;

    assert_restore_failed_without_clearing(&output, &ledger, &format!("{before}\n"), "symlink")?;
    assert_eq!(fs::read_to_string(escape)?, "keep");
    Ok(())
}

pub(crate) fn target_parent_symlink_is_not_followed() -> Result<(), Box<dyn Error>> {
    let fixture = TempFixture::new("restore-parent-symlink")?;
    let root = fixture.minimal_android_fs()?;
    let ledger = ledger_path(&root);
    let backup = backup_path(&root, "target-parent-symlink.bak");
    fs::write(&backup, "restored")?;
    let data_dir = root.join("data/data");
    unix_fs::symlink(fixture.root.as_path(), &data_dir)?;
    let step = restore_content_step(&backup);
    let before = write_restore_ledger(&ledger, "/data/data/com.demo/cache/ad.bin", &[&step])?;

    let output = restore_execute(&ledger)?;

    assert_restore_failed_without_clearing(&output, &ledger, &format!("{before}\n"), "symlink")?;
    assert!(!fixture.root.join("com.demo/cache/ad.bin").exists());
    Ok(())
}

pub(crate) fn directory_restore_parent_symlink_is_not_followed() -> Result<(), Box<dyn Error>> {
    let fixture = TempFixture::new("restore-directory-parent-symlink")?;
    let root = fixture.minimal_android_fs()?;
    let ledger = ledger_path(&root);
    let backup = backup_path(&root, "directory-parent-symlink.bak");
    fs::create_dir_all(&backup)?;
    fs::write(backup.join(".fixture"), "restored")?;
    let escape = fixture.root.join("escape-dir");
    fs::create_dir_all(escape.join("com.demo/cache/ad-dir"))?;
    unix_fs::symlink(&escape, root.join("data/data"))?;
    let step = restore_content_step(&backup);
    let before = write_restore_ledger(&ledger, "/data/data/com.demo/cache/ad-dir", &[&step])?;

    let output = restore_execute(&ledger)?;

    assert_restore_failed_without_clearing(&output, &ledger, &format!("{before}\n"), "symlink")?;
    assert!(escape.join("com.demo/cache/ad-dir").is_dir());
    Ok(())
}

pub(crate) fn remove_placeholder_parent_symlink_is_not_followed() -> Result<(), Box<dyn Error>> {
    let fixture = TempFixture::new("restore-remove-parent-symlink")?;
    let root = fixture.minimal_android_fs()?;
    let ledger = ledger_path(&root);
    let escape = fixture.root.join("escape-placeholder");
    fs::create_dir_all(escape.join("com.demo/cache"))?;
    fs::write(escape.join("com.demo/cache/ad.bin"), "keep")?;
    unix_fs::symlink(&escape, root.join("data/data"))?;
    let before = write_restore_ledger(
        &ledger,
        "/data/data/com.demo/cache/ad.bin",
        &[remove_placeholder_step()],
    )?;

    let output = restore_execute(&ledger)?;

    assert_restore_failed_without_clearing(&output, &ledger, &format!("{before}\n"), "symlink")?;
    assert_eq!(
        fs::read_to_string(escape.join("com.demo/cache/ad.bin"))?,
        "keep"
    );
    Ok(())
}

pub(crate) fn backup_symlink_is_not_read() -> Result<(), Box<dyn Error>> {
    let fixture = TempFixture::new("restore-backup-symlink")?;
    let root = fixture.minimal_android_fs()?;
    let ledger = ledger_path(&root);
    let backup = backup_path(&root, "backup-symlink.bak");
    let escape = fixture.root.join("escape-backup");
    fs::write(&escape, "secret")?;
    unix_fs::symlink(&escape, &backup)?;
    let step = restore_content_step(&backup);
    let before = write_restore_ledger(&ledger, "/data/data/com.demo/cache/ad.bin", &[&step])?;

    let output = restore_execute(&ledger)?;

    assert_restore_failed_without_clearing(&output, &ledger, &format!("{before}\n"), "symlink")?;
    assert!(!root.join("data/data/com.demo/cache/ad.bin").exists());
    Ok(())
}

pub(crate) fn backup_parent_symlink_is_not_followed() -> Result<(), Box<dyn Error>> {
    let fixture = TempFixture::new("restore-backup-parent-symlink")?;
    let root = fixture.minimal_android_fs()?;
    let ledger = ledger_path(&root);
    let backup_root = root.join("data/adb/modules/puread/state/backups");
    fs::remove_dir_all(&backup_root)?;
    unix_fs::symlink(fixture.root.as_path(), &backup_root)?;
    let backup = backup_root.join("parent-symlink.bak");
    fs::write(fixture.root.join("parent-symlink.bak"), "secret")?;
    let step = restore_content_step(&backup);
    let before = write_restore_ledger(&ledger, "/data/data/com.demo/cache/ad.bin", &[&step])?;

    let output = restore_execute(&ledger)?;

    assert_restore_failed_without_clearing(&output, &ledger, &format!("{before}\n"), "symlink")?;
    assert!(!root.join("data/data/com.demo/cache/ad.bin").exists());
    Ok(())
}

pub(crate) fn backup_path_escape_is_rejected() -> Result<(), Box<dyn Error>> {
    let fixture = TempFixture::new("restore-backup-escape")?;
    let root = fixture.minimal_android_fs()?;
    let ledger = ledger_path(&root);
    let escaped = root.join("data/adb/modules/puread/state/escape.bak");
    fs::write(&escaped, "secret")?;
    let step = restore_content_step(&escaped);
    let before = write_restore_ledger(&ledger, "/data/data/com.demo/cache/ad.bin", &[&step])?;

    let output = restore_execute(&ledger)?;

    assert_restore_failed_without_clearing(
        &output,
        &ledger,
        &format!("{before}\n"),
        "cannot be mapped",
    )?;
    assert!(!root.join("data/data/com.demo/cache/ad.bin").exists());
    Ok(())
}

pub(crate) fn missing_parent_directory_restore_uses_safe_path() -> Result<(), Box<dyn Error>> {
    let fixture = TempFixture::new("restore-missing-parent")?;
    let root = fixture.minimal_android_fs()?;
    let ledger = ledger_path(&root);
    let before = write_restore_ledger(
        &ledger,
        "/data/data/com.demo/new-cache",
        &[recreate_directory_step()],
    )?;

    let output = restore_execute(&ledger)?;

    assert_success(&output)?;
    let document = parse_stdout_json(&output)?;
    assert_eq!(field(&document, "failed")?, 0);
    assert!(root.join("data/data/com.demo/new-cache").is_dir());
    assert_ne!(fs::read_to_string(&ledger)?, format!("{before}\n"));
    Ok(())
}

pub(crate) fn permission_restore_uses_safe_path() -> Result<(), Box<dyn Error>> {
    let fixture = TempFixture::new("restore-set-mode")?;
    let root = fixture.minimal_android_fs()?;
    let ledger = ledger_path(&root);
    let target = root.join("data/data/com.demo/cache/ad.bin");
    fs::create_dir_all(parent(&target)?)?;
    fs::write(&target, "payload")?;
    let before = write_restore_ledger(
        &ledger,
        "/data/data/com.demo/cache/ad.bin",
        &[recreate_file_step(), set_mode_step(384)],
    )?;

    let output = restore_execute(&ledger)?;

    assert_success(&output)?;
    let document = parse_stdout_json(&output)?;
    assert_eq!(field(&document, "failed")?, 0);
    assert_eq!(fs::metadata(&target)?.permissions().mode() & 0o777, 0o600);
    assert_ne!(fs::read_to_string(&ledger)?, format!("{before}\n"));
    Ok(())
}

fn restore_execute(ledger: &Path) -> Result<std::process::Output, Box<dyn Error>> {
    run_puread([
        "restore",
        "--execute",
        "--ledger",
        ledger.to_string_lossy().as_ref(),
    ])
}

fn parent(path: &Path) -> Result<&Path, Box<dyn Error>> {
    path.parent().ok_or_else(|| "path has no parent".into())
}
