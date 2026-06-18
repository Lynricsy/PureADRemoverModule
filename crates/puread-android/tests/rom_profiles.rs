#![doc = "ROM profile settings 与 `shared_prefs` XML 行为测试。"]

use std::fs;
use std::io;
use std::path::Path;

#[path = "rom_profiles/security.rs"]
mod security;
#[path = "rom_profiles/support.rs"]
pub(crate) mod support;

use puread_android::command_runner::{CommandOutput, SettingsNamespace};
use puread_android::profiles::{
    AndroidProfileExecutor, ProfileOperationStatus, RomMatcher, RomProfileRule, RomSettingsRule,
    SharedPrefsBoolRule,
};
use support::{
    MemoryLedger, ScriptedRunner, extract_backup_path, unique_temp_dir, write_prefs_fixture,
};

#[test]
fn rom_profile_skips_miui_settings_when_getprop_does_not_match()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a MIUI-only ROM settings rule on a non-MIUI fake device.
    let runner = ScriptedRunner::with_outputs(vec![CommandOutput::success("\n", "")]);
    let ledger = MemoryLedger::default();
    let executor = AndroidProfileExecutor::new(&runner, &ledger);
    let rule = RomProfileRule::settings(
        "miui-disable-personalized-ad",
        RomMatcher::miui(),
        RomSettingsRule::new(
            SettingsNamespace::Global,
            "miui_personalized_ad_enabled",
            "0",
        )?,
    )?;

    // When: the ROM profile is applied.
    let outcome = executor.apply_rom(&rule)?;

    // Then: only getprop runs and the mutation is skipped.
    assert_eq!(outcome.status, ProfileOperationStatus::Skipped);
    assert_eq!(
        runner.call_lines(),
        ["/system/bin/getprop ro.miui.ui.version.name"]
    );
    Ok(())
}

#[test]
fn rom_profile_modifies_and_restores_shared_prefs_xml_with_backup_and_hash()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a MIUI XML profile fixture with one enabled ad preference.
    let root = unique_temp_dir();
    let prefs = root.join("data/user/0/com.miui.weather2/shared_prefs/prefs.xml");
    fs::create_dir_all(prefs.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "fixture path has no parent")
    })?)?;
    fs::write(
        &prefs,
        r#"<map><boolean name="key_content_promotion" value="true" /></map>"#,
    )?;
    let backup_dir = root.join("backups");
    let runner = ScriptedRunner::with_outputs(vec![CommandOutput::success("V14\n", "")]);
    let ledger = MemoryLedger::default();
    let executor = AndroidProfileExecutor::new(&runner, &ledger);
    let rule = RomProfileRule::shared_prefs_bool(
        "miui-weather-content-promotion",
        RomMatcher::miui(),
        SharedPrefsBoolRule::new(&prefs, "key_content_promotion", false, &backup_dir)?,
    )?;

    // When: the XML rule is applied and restored.
    let apply = executor.apply_rom(&rule)?;
    executor.restore_rom(&apply.record)?;

    // Then: the file is restored exactly and the record contains hash and backup evidence.
    assert_eq!(apply.status, ProfileOperationStatus::Applied);
    assert_eq!(
        fs::read_to_string(&prefs)?,
        r#"<map><boolean name="key_content_promotion" value="true" /></map>"#
    );
    assert!(apply.record.contains("\"original_sha256\":"));
    assert!(apply.record.contains("\"backup_path\":"));
    assert!(Path::new(extract_backup_path(&apply.record)?).exists());
    Ok(())
}

#[test]
fn rom_settings_does_not_mutate_when_ledger_sink_fails() -> Result<(), Box<dyn std::error::Error>> {
    // Given: a MIUI settings rule whose ledger sink fails after getprop/settings probe.
    let runner = ScriptedRunner::with_outputs(vec![
        CommandOutput::success("V14\n", ""),
        CommandOutput::success("1\n", ""),
    ]);
    let ledger = MemoryLedger::failing();
    let executor = AndroidProfileExecutor::new(&runner, &ledger);
    let rule = RomProfileRule::settings(
        "miui-disable-personalized-ad",
        RomMatcher::miui(),
        RomSettingsRule::new(
            SettingsNamespace::Global,
            "miui_personalized_ad_enabled",
            "0",
        )?,
    )?;

    // When: applying the rule fails at ledger append.
    let error = executor.apply_rom(&rule).expect_err("ledger failure");

    // Then: no settings put mutation was issued.
    assert!(error.to_string().contains("ledger sink failed"));
    assert_eq!(
        runner.call_lines(),
        [
            "/system/bin/getprop ro.miui.ui.version.name",
            "/system/bin/settings get global miui_personalized_ad_enabled",
        ]
    );
    Ok(())
}

#[test]
fn rom_shared_prefs_does_not_mutate_or_backup_when_ledger_sink_fails()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a MIUI shared_prefs rule whose ledger sink fails after read-only planning.
    let root = unique_temp_dir();
    let prefs = write_prefs_fixture(&root)?;
    let backup_dir = root.join("backups");
    let runner = ScriptedRunner::with_outputs(vec![CommandOutput::success("V14\n", "")]);
    let ledger = MemoryLedger::failing();
    let executor = AndroidProfileExecutor::new(&runner, &ledger);
    let rule = RomProfileRule::shared_prefs_bool(
        "miui-weather-content-promotion",
        RomMatcher::miui(),
        SharedPrefsBoolRule::new(&prefs, "key_content_promotion", false, &backup_dir)?,
    )?;

    // When: applying the rule fails at ledger append.
    let error = executor.apply_rom(&rule).expect_err("ledger failure");

    // Then: XML and backup state are unchanged because no mutation ran.
    assert!(error.to_string().contains("ledger sink failed"));
    assert_eq!(
        fs::read_to_string(&prefs)?,
        r#"<map><boolean name="key_content_promotion" value="true" /></map>"#
    );
    assert!(!backup_dir.exists());
    assert!(ledger.records.borrow().is_empty());
    Ok(())
}

#[test]
fn rom_rule_rejects_unsafe_rule_ids_before_backup_path() -> Result<(), Box<dyn std::error::Error>> {
    // Given: a valid shared_prefs rule payload.
    let root = unique_temp_dir();
    let prefs = write_prefs_fixture(&root)?;
    let backup_dir = root.join("backups");

    // When / Then: unsafe ids are rejected before they can become backup filenames.
    for id in ["../escape", "a/b", "", &"a".repeat(97)] {
        assert!(
            RomProfileRule::shared_prefs_bool(
                id,
                RomMatcher::miui(),
                SharedPrefsBoolRule::new(&prefs, "key_content_promotion", false, &backup_dir)?,
            )
            .is_err(),
            "{id:?} should be rejected"
        );
    }
    Ok(())
}
