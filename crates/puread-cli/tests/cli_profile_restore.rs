#![doc = "CLI profile 持久 ledger 恢复与安全边界测试。"]

use std::error::Error;
use std::fs;

#[path = "support/cli_profile_restore.rs"]
pub(crate) mod support;
#[cfg(unix)]
#[path = "support/cli_profile_restore_apply.rs"]
mod support_apply;

use support::{
    TempFixture, assert_success, field, profile_records, run_puread, run_puread_with_profile_test,
};

const COMPONENT_RULES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../rules/components");

#[test]
fn cli_profile_restore_reports_persisted_records_without_mutation() -> Result<(), Box<dyn Error>> {
    // Given: profile records were persisted by a previous Android profile execution.
    let fixture = TempFixture::new("report")?;
    fs::write(fixture.profile_ledger_path(), profile_records(&fixture)?)?;

    // When: profile-report reads the persistent profile ledger.
    let output = run_puread(["profile-report", "--module-root", fixture.module_root_str()])?;

    // Then: JSON report exposes pending records without invoking the runner.
    assert_success(&output)?;
    let document = support::parse_stdout_json(&output)?;
    assert_eq!(field(&document, "command")?, "profile_report");
    assert_eq!(field(&document, "record_count")?, 4);
    assert_eq!(field(&document, "pending_restore_count")?, 4);
    assert!(!fixture.runner_log().exists());
    Ok(())
}

#[test]
fn cli_profile_restore_restores_all_profile_record_kinds() -> Result<(), Box<dyn Error>> {
    // Given: appops, component, ROM setting, and shared_prefs records are in profile ledger.
    let fixture = TempFixture::new("restore")?;
    fs::write(fixture.profile_ledger_path(), profile_records(&fixture)?)?;
    fs::write(
        fixture.shared_prefs_path(),
        "<map><boolean name=\"key_content_promotion\" value=\"false\" /></map>",
    )?;

    // When: profile-restore executes under the module global lock with the test runner seam.
    let output = run_puread_with_profile_test(
        [
            "profile-restore",
            "--execute",
            "--module-root",
            fixture.module_root_str(),
            "--test-profile-runner",
            "--profile-runner-log",
            fixture.runner_log_str(),
        ],
        &fixture,
    )?;

    // Then: every recoverable profile kind is restored and the ledger is marked restored.
    assert_success(&output)?;
    let document = support::parse_stdout_json(&output)?;
    assert_eq!(field(&document, "command")?, "profile_restore");
    assert_eq!(field(&document, "mode")?, "execute");
    assert_eq!(field(&document, "restored")?, 4);
    assert_eq!(field(&document, "failed")?, 0);
    let calls = fs::read_to_string(fixture.runner_log())?;
    assert!(calls.contains("appops set com.luna.music MONITOR_LOCATION default"));
    assert!(calls.contains("pm enable --user 0 com.xiaomi.market/"));
    assert!(calls.contains("settings put global miui_personalized_ad_enabled 1"));
    let ledger = fs::read_to_string(fixture.profile_ledger_path())?;
    assert!(
        ledger.contains("\"restore_status\":\"restored\""),
        "{ledger}"
    );
    assert!(fs::read_to_string(fixture.shared_prefs_path())?.contains("value=\"true\""));
    Ok(())
}

#[test]
fn cli_profile_restore_folds_component_provisional_and_confirmed_records()
-> Result<(), Box<dyn Error>> {
    // Given: apply-profile records provisional and confirmed component entries in JSONL.
    let fixture = TempFixture::new("component-final-state")?;
    let apply = run_puread_with_profile_test(
        [
            "apply-profile",
            "component",
            "--execute",
            "--rules",
            COMPONENT_RULES,
            "--root",
            support::ANDROID_FS_FIXTURE,
            "--module-root",
            fixture.module_root_str(),
            "--test-profile-runner",
            "--profile-runner-log",
            fixture.runner_log_str(),
        ],
        &fixture,
    )?;
    assert_success(&apply)?;
    let ledger = fs::read_to_string(fixture.profile_ledger_path())?;
    assert!(ledger.contains("\"hide_status\":\"skipped_unavailable\""));
    assert!(ledger.contains("\"hide_status\":\"applied\""));
    fs::remove_file(fixture.runner_log())?;

    // When: profile-restore executes from the durable profile-actions JSONL.
    let restore = run_puread_with_profile_test(
        [
            "profile-restore",
            "--execute",
            "--module-root",
            fixture.module_root_str(),
            "--test-profile-runner",
            "--profile-runner-log",
            fixture.runner_log_str(),
        ],
        &fixture,
    )?;

    // Then: each logical component restores once, with confirmed hide state driving unhide.
    assert_success(&restore)?;
    let calls = fs::read_to_string(fixture.runner_log())?;
    assert_eq!(calls.matches("pm unhide com.xiaomi.market").count(), 2);
    assert_eq!(calls.matches("pm enable --user 0").count(), 4);
    Ok(())
}

#[test]
fn cli_profile_restore_keeps_component_skipped_only_record_without_unhide()
-> Result<(), Box<dyn Error>> {
    // Given: the durable ledger only has the conservative skipped TryHide record.
    let fixture = TempFixture::new("component-skipped-only")?;
    fs::write(
        fixture.profile_ledger_path(),
        "{\"kind\":\"component\",\"rule_id\":\"mi-market-reverse-ad-page\",\"user_id\":0,\"package\":\"com.xiaomi.market\",\"component\":\"com.xiaomi.market/com.xiaomi.market.reverse_ad.page.WebReverseAdActivity\",\"original_enabled\":\"enabled\",\"original_hidden\":\"visible\",\"hide_status\":\"skipped_unavailable\"}\n",
    )?;

    // When: profile-restore executes from JSONL.
    let output = run_puread_with_profile_test(
        [
            "profile-restore",
            "--execute",
            "--module-root",
            fixture.module_root_str(),
            "--test-profile-runner",
            "--profile-runner-log",
            fixture.runner_log_str(),
        ],
        &fixture,
    )?;

    // Then: skipped-only records enable the component but do not unhide the package.
    assert_success(&output)?;
    let calls = fs::read_to_string(fixture.runner_log())?;
    assert!(!calls.contains("pm unhide"), "{calls}");
    assert_eq!(calls.matches("pm enable --user 0").count(), 1);
    Ok(())
}

#[test]
fn cli_profile_restore_keeps_component_final_state_per_user_id() -> Result<(), Box<dyn Error>> {
    // Given: two component records differ only by Android user id.
    let fixture = TempFixture::new("component-user-final-state")?;
    fs::write(
        fixture.profile_ledger_path(),
        concat!(
            "{\"kind\":\"component\",\"rule_id\":\"mi-market-reverse-ad-page\",\"user_id\":0,\"package\":\"com.xiaomi.market\",\"component\":\"com.xiaomi.market/com.xiaomi.market.reverse_ad.page.WebReverseAdActivity\",\"original_enabled\":\"enabled\",\"original_hidden\":\"visible\",\"hide_status\":\"not_requested\"}\n",
            "{\"kind\":\"component\",\"rule_id\":\"mi-market-reverse-ad-page\",\"user_id\":10,\"package\":\"com.xiaomi.market\",\"component\":\"com.xiaomi.market/com.xiaomi.market.reverse_ad.page.WebReverseAdActivity\",\"original_enabled\":\"enabled\",\"original_hidden\":\"visible\",\"hide_status\":\"not_requested\"}\n",
        ),
    )?;

    // When: profile-restore executes from the durable profile-actions JSONL.
    let output = run_puread_with_profile_test(
        [
            "profile-restore",
            "--execute",
            "--module-root",
            fixture.module_root_str(),
            "--test-profile-runner",
            "--profile-runner-log",
            fixture.runner_log_str(),
        ],
        &fixture,
    )?;

    // Then: each user-scoped component record is restored and marked independently.
    assert_success(&output)?;
    let document = support::parse_stdout_json(&output)?;
    assert_eq!(field(&document, "restored")?, 2);
    let calls = fs::read_to_string(fixture.runner_log())?;
    assert_eq!(calls.matches("pm enable --user 0").count(), 1, "{calls}");
    assert_eq!(calls.matches("pm enable --user 10").count(), 1, "{calls}");
    let ledger = fs::read_to_string(fixture.profile_ledger_path())?;
    assert_eq!(ledger.matches("\"restore_status\":\"restored\"").count(), 2);
    assert_eq!(
        ledger
            .lines()
            .filter(|line| {
                line.contains("\"user_id\":0") && line.contains("\"restore_status\":\"restored\"")
            })
            .count(),
        1,
        "{ledger}"
    );
    assert_eq!(
        ledger
            .lines()
            .filter(|line| {
                line.contains("\"user_id\":10") && line.contains("\"restore_status\":\"restored\"")
            })
            .count(),
        1,
        "{ledger}"
    );
    Ok(())
}

#[test]
fn cli_profile_restore_default_runner_ignores_env_fake_runner() -> Result<(), Box<dyn Error>> {
    // Given: a profile record needs Android command restore and fake-runner env is set.
    let fixture = TempFixture::new("prod-runner")?;
    fs::write(fixture.profile_ledger_path(), support::appop_record())?;
    let runner_log = fixture.runner_log().to_string_lossy().into_owned();

    // When: production CLI is run without the explicit test seam flag.
    let output = run_puread([
        "profile-restore",
        "--execute",
        "--module-root",
        fixture.module_root_str(),
        "--profile-runner-log",
        runner_log.as_str(),
    ])?;

    // Then: real runner is used, so fake runner log is not created.
    assert!(!output.status.success(), "{output:?}");
    assert!(!fixture.runner_log().exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn cli_profile_restore_rejects_state_symlink_before_mutation() -> Result<(), Box<dyn Error>> {
    support::state_symlink_is_rejected_before_mutation()
}

#[cfg(unix)]
#[test]
fn cli_profile_restore_rejects_profile_ledger_symlink_before_mutation() -> Result<(), Box<dyn Error>>
{
    support::profile_ledger_symlink_is_rejected_before_mutation()
}

#[cfg(unix)]
#[test]
fn cli_profile_restore_rejects_run_symlink_before_mutation() -> Result<(), Box<dyn Error>> {
    support::run_symlink_is_rejected_before_mutation()
}

#[cfg(unix)]
#[test]
fn cli_profile_restore_rejects_lock_symlink_before_mutation() -> Result<(), Box<dyn Error>> {
    support::lock_symlink_is_rejected_before_mutation()
}

#[cfg(unix)]
#[test]
fn cli_profile_restore_execute_lock_contention_prevents_ledger_read() -> Result<(), Box<dyn Error>>
{
    support::execute_lock_contention_prevents_ledger_read()
}

#[cfg(unix)]
#[test]
fn cli_apply_profile_rejects_state_symlink_before_android_mutation() -> Result<(), Box<dyn Error>> {
    support_apply::apply_profile_state_symlink_is_rejected_before_android_mutation()
}
