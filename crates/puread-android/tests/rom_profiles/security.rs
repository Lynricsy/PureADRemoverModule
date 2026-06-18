use std::error::Error;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs as unix_fs;
use std::path::Path;

use puread_android::command_runner::CommandOutput;
use puread_android::profiles::{
    AndroidProfileExecutor, RomMatcher, RomProfileRule, SharedPrefsBoolRule,
};

use super::support::{MemoryLedger, ScriptedRunner, unique_temp_dir, write_prefs_fixture};

#[cfg(unix)]
#[test]
fn rom_shared_prefs_rejects_xml_symlink_target() -> Result<(), Box<dyn Error>> {
    // Given: the XML path is a symlink to another file.
    let root = unique_temp_dir();
    let real = write_prefs_fixture(&root)?;
    let link = root.join("data/user/0/com.miui.weather2/shared_prefs/link.xml");
    unix_fs::symlink(&real, &link)?;
    let runner = ScriptedRunner::with_outputs(vec![CommandOutput::success("V14\n", "")]);
    let ledger = MemoryLedger::default();
    let executor = AndroidProfileExecutor::new(&runner, &ledger);
    let rule = shared_prefs_rule(&link, &root.join("backups"))?;

    // When: applying the rule attempts a no-follow XML read.
    let error = executor.apply_rom(&rule).expect_err("symlink rejected");

    // Then: mutation is rejected and no ledger record is written.
    assert!(error.to_string().contains("profile file I/O failed"));
    assert!(ledger.records.borrow().is_empty());
    Ok(())
}

#[test]
fn rom_shared_prefs_rejects_relative_and_traversal_paths() -> Result<(), Box<dyn Error>> {
    // Given: relative and traversal paths are outside the absolute no-follow contract.
    let root = unique_temp_dir();
    let runner = ScriptedRunner::with_outputs(vec![CommandOutput::success("V14\n", "")]);
    let ledger = MemoryLedger::default();
    let executor = AndroidProfileExecutor::new(&runner, &ledger);

    // When / Then: both paths fail before mutation or ledger append.
    for path in [
        Path::new("data/user/0/com.miui.weather2/shared_prefs/prefs.xml"),
        Path::new("/tmp/../tmp/prefs.xml"),
    ] {
        let rule = shared_prefs_rule(path, &root.join("backups"))?;
        let _error = executor.apply_rom(&rule).expect_err("unsafe path rejected");
    }
    assert!(ledger.records.borrow().is_empty());
    Ok(())
}

#[cfg(unix)]
#[test]
fn rom_shared_prefs_rejects_parent_symlink_without_ledger_or_backup() -> Result<(), Box<dyn Error>>
{
    // Given: a parent directory of the XML path is replaced with a symlink.
    let root = unique_temp_dir();
    let prefs = write_prefs_fixture(&root)?;
    let shared_prefs = prefs
        .parent()
        .ok_or("prefs path has no parent")?
        .to_path_buf();
    fs::remove_dir_all(&shared_prefs)?;
    unix_fs::symlink(root.join("escape"), &shared_prefs)?;
    let backup_dir = root.join("backups");
    let runner = ScriptedRunner::with_outputs(vec![CommandOutput::success("V14\n", "")]);
    let ledger = MemoryLedger::default();
    let executor = AndroidProfileExecutor::new(&runner, &ledger);
    let rule = shared_prefs_rule(&prefs, &backup_dir)?;

    // When: applying the rule tries to open through the symlinked parent.
    let error = executor
        .apply_rom(&rule)
        .expect_err("parent symlink rejected");

    // Then: no record or backup is produced.
    assert!(error.to_string().contains("profile file I/O failed"));
    assert!(ledger.records.borrow().is_empty());
    assert!(!backup_dir.exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn rom_shared_prefs_rejects_backup_dir_symlink_without_ledger() -> Result<(), Box<dyn Error>> {
    // Given: the backup directory is a symlink.
    let root = unique_temp_dir();
    let prefs = write_prefs_fixture(&root)?;
    let backup_dir = root.join("backups");
    fs::create_dir_all(root.join("escape"))?;
    unix_fs::symlink(root.join("escape"), &backup_dir)?;
    let runner = ScriptedRunner::with_outputs(vec![CommandOutput::success("V14\n", "")]);
    let ledger = MemoryLedger::default();
    let executor = AndroidProfileExecutor::new(&runner, &ledger);
    let rule = shared_prefs_rule(&prefs, &backup_dir)?;

    // When: committing the mutation validates the backup directory.
    let error = executor
        .apply_rom(&rule)
        .expect_err("backup symlink rejected");

    // Then: ledger remains empty and the XML is unchanged.
    assert!(error.to_string().contains("profile file I/O failed"));
    assert!(ledger.records.borrow().is_empty());
    assert!(fs::read_to_string(prefs)?.contains(r#"value="true""#));
    Ok(())
}

#[test]
fn rom_shared_prefs_rejects_existing_backup_without_ledger() -> Result<(), Box<dyn Error>> {
    // Given: the deterministic backup file already exists.
    let root = unique_temp_dir();
    let prefs = write_prefs_fixture(&root)?;
    let backup_dir = root.join("backups");
    fs::create_dir_all(&backup_dir)?;
    fs::write(
        backup_dir.join("miui-weather-content-promotion.xml.bak"),
        "occupied",
    )?;
    let runner = ScriptedRunner::with_outputs(vec![CommandOutput::success("V14\n", "")]);
    let ledger = MemoryLedger::default();
    let executor = AndroidProfileExecutor::new(&runner, &ledger);
    let rule = shared_prefs_rule(&prefs, &backup_dir)?;

    // When: committing the mutation tries to create a fresh backup.
    let error = executor
        .apply_rom(&rule)
        .expect_err("backup collision rejected");

    // Then: no ledger record is written and the XML remains unchanged.
    assert!(error.to_string().contains("profile file I/O failed"));
    assert!(ledger.records.borrow().is_empty());
    assert!(fs::read_to_string(prefs)?.contains(r#"value="true""#));
    Ok(())
}

#[cfg(unix)]
#[test]
fn rom_shared_prefs_rejects_backup_file_symlink_without_ledger() -> Result<(), Box<dyn Error>> {
    // Given: the deterministic backup path is a symlink.
    let root = unique_temp_dir();
    let prefs = write_prefs_fixture(&root)?;
    let backup_dir = root.join("backups");
    fs::create_dir_all(&backup_dir)?;
    unix_fs::symlink(
        root.join("escape"),
        backup_dir.join("miui-weather-content-promotion.xml.bak"),
    )?;
    let runner = ScriptedRunner::with_outputs(vec![CommandOutput::success("V14\n", "")]);
    let ledger = MemoryLedger::default();
    let executor = AndroidProfileExecutor::new(&runner, &ledger);
    let rule = shared_prefs_rule(&prefs, &backup_dir)?;

    // When: committing the mutation refuses the symlink backup path.
    let error = executor
        .apply_rom(&rule)
        .expect_err("backup symlink rejected");

    // Then: no ledger record is written and the XML remains unchanged.
    assert!(error.to_string().contains("profile file I/O failed"));
    assert!(ledger.records.borrow().is_empty());
    assert!(fs::read_to_string(prefs)?.contains(r#"value="true""#));
    Ok(())
}

fn shared_prefs_rule(
    prefs: &Path,
    backup_dir: &Path,
) -> Result<RomProfileRule, puread_android::profiles::ProfileError> {
    RomProfileRule::shared_prefs_bool(
        "miui-weather-content-promotion",
        RomMatcher::miui(),
        SharedPrefsBoolRule::new(prefs, "key_content_promotion", false, backup_dir)?,
    )
}
