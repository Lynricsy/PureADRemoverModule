#![doc = "`AppOps` 与组件 profile 执行层行为测试。"]

#[path = "appops_components/durable.rs"]
mod durable;
#[path = "appops_components/support.rs"]
mod support;

use puread_android::command_runner::{CommandOutput, CommandRunnerError};
use puread_android::profiles::{
    AndroidProfileExecutor, AppOpProfileRule, ComponentProfileRule, PmHidePolicy,
    ProfileOperationStatus,
};
use support::{MemoryLedger, ScriptedRunner};

#[test]
fn appops_profile_records_original_mode_and_restores_default_when_missing()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: an AppOps rule whose current mode output omits the target op.
    let runner = ScriptedRunner::with_outputs(vec![
        CommandOutput::success("No operations.\n", ""),
        CommandOutput::success("", ""),
        CommandOutput::success("", ""),
    ]);
    let ledger = MemoryLedger::default();
    let executor = AndroidProfileExecutor::new(&runner, &ledger);
    let rule = AppOpProfileRule::new(
        "luna-background-location",
        "com.luna.music",
        "MONITOR_LOCATION",
        "ignore",
    )?;

    // When: the rule is applied and then restored.
    let apply = executor.apply_appop(&rule)?;
    executor.restore_appop(&apply.record)?;

    // Then: runner argv is stable and the ledger records default as the original mode.
    assert_eq!(apply.status, ProfileOperationStatus::Applied);
    assert_eq!(
        runner.call_lines(),
        [
            "/system/bin/cmd appops get com.luna.music MONITOR_LOCATION",
            "/system/bin/cmd appops set com.luna.music MONITOR_LOCATION ignore",
            "/system/bin/cmd appops set com.luna.music MONITOR_LOCATION default",
        ]
    );
    assert_eq!(ledger.records().len(), 1);
    assert!(apply.record.contains("\"original_mode\":\"default\""));
    Ok(())
}

#[test]
fn appops_profile_does_not_mutate_when_ledger_sink_fails() -> Result<(), Box<dyn std::error::Error>>
{
    support::appops_ledger_sink_failure_does_not_mutate()
}

#[test]
fn component_profile_does_not_mutate_when_ledger_sink_fails()
-> Result<(), Box<dyn std::error::Error>> {
    support::component_ledger_sink_failure_does_not_mutate()
}

#[test]
fn component_profile_disables_enables_and_skips_unavailable_pm_hide()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a component rule with pm hide enabled but unsupported on the fake device.
    let runner = ScriptedRunner::with_script(vec![
        Ok(CommandOutput::success("package:/data/app/base.apk\n", "")),
        Ok(CommandOutput::success("", "")),
        Ok(CommandOutput::success("", "")),
        Err(CommandRunnerError::Unavailable {
            detail: "pm hide unsupported".to_owned(),
        }),
        Ok(CommandOutput::success("", "")),
        Ok(CommandOutput::success("", "")),
    ]);
    let ledger = MemoryLedger::default();
    let executor = AndroidProfileExecutor::new(&runner, &ledger);
    let rule = ComponentProfileRule::new(
        "mi-market-reverse-ad",
        0,
        "com.xiaomi.market/com.xiaomi.market.reverse_ad.service.ReverseAdScheduleService",
        PmHidePolicy::TryHide,
    )?;

    // When: the rule is applied and then restored.
    let apply = executor.apply_component(&rule)?;
    executor.restore_component(&apply.record)?;

    // Then: pm hide failure becomes a skip record while disable/enable still run.
    assert_eq!(apply.status, ProfileOperationStatus::Applied);
    assert!(
        apply
            .record
            .contains("\"hide_status\":\"skipped_unavailable\"")
    );
    let records = ledger.records();
    assert_eq!(records.len(), 1);
    assert!(
        records
            .first()
            .is_some_and(|record| record.contains("\"hide_status\":\"skipped_unavailable\"")),
        "persistent ledger must match report when pm hide is unavailable"
    );
    assert_eq!(
        runner.call_lines(),
        [
            "/system/bin/pm path com.xiaomi.market",
            "/system/bin/pm list packages -d com.xiaomi.market",
            "/system/bin/pm list packages --hidden com.xiaomi.market",
            "/system/bin/pm hide com.xiaomi.market",
            "/system/bin/pm disable-user --user 0 com.xiaomi.market/com.xiaomi.market.reverse_ad.service.ReverseAdScheduleService",
            "/system/bin/pm enable --user 0 com.xiaomi.market/com.xiaomi.market.reverse_ad.service.ReverseAdScheduleService",
        ]
    );
    Ok(())
}

#[test]
fn component_profile_skips_pm_hide_when_capability_attempt_fails()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: pm hide exists but reports unsupported through a non-zero command status.
    let runner = ScriptedRunner::with_outputs(vec![
        CommandOutput::success("package:/data/app/base.apk\n", ""),
        CommandOutput::success("", ""),
        CommandOutput::success("", ""),
        CommandOutput::failure(1, "", "Unknown command: hide"),
        CommandOutput::success("", ""),
    ]);
    let ledger = MemoryLedger::default();
    let executor = AndroidProfileExecutor::new(&runner, &ledger);
    let rule = ComponentProfileRule::new(
        "mi-market-reverse-ad",
        0,
        "com.xiaomi.market/com.xiaomi.market.reverse_ad.service.ReverseAdScheduleService",
        PmHidePolicy::TryHide,
    )?;

    // When: the profile applies a component rule with TryHide.
    let apply = executor.apply_component(&rule)?;

    // Then: the runner-backed capability attempt is recorded as unavailable.
    assert_eq!(apply.status, ProfileOperationStatus::Applied);
    assert!(
        apply
            .record
            .contains("\"hide_status\":\"skipped_unavailable\"")
    );
    let records = ledger.records();
    assert_eq!(records.len(), 1);
    assert!(
        records
            .first()
            .is_some_and(|record| record.contains("\"hide_status\":\"skipped_unavailable\"")),
        "persistent ledger must match report when pm hide exits non-zero"
    );
    assert_eq!(
        runner.call_lines(),
        [
            "/system/bin/pm path com.xiaomi.market",
            "/system/bin/pm list packages -d com.xiaomi.market",
            "/system/bin/pm list packages --hidden com.xiaomi.market",
            "/system/bin/pm hide com.xiaomi.market",
            "/system/bin/pm disable-user --user 0 com.xiaomi.market/com.xiaomi.market.reverse_ad.service.ReverseAdScheduleService",
        ]
    );
    Ok(())
}

#[test]
fn component_profile_restore_keeps_enable_when_persisted_hide_was_skipped()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a persisted component record where TryHide was unavailable during apply.
    let record = r#"{"kind":"component","rule_id":"mi-market-reverse-ad","user_id":0,"package":"com.xiaomi.market","component":"com.xiaomi.market/com.xiaomi.market.reverse_ad.service.ReverseAdScheduleService","original_enabled":"enabled","original_hidden":"visible","hide_status":"skipped_unavailable"}"#;
    let runner = ScriptedRunner::with_outputs(vec![CommandOutput::success("", "")]);
    let ledger = MemoryLedger::default();
    let executor = AndroidProfileExecutor::new(&runner, &ledger);

    // When: the persisted record is restored.
    executor.restore_component(record)?;

    // Then: restore does not call pm unhide, but still enables the component.
    assert_eq!(
        runner.call_lines(),
        [
            "/system/bin/pm enable --user 0 com.xiaomi.market/com.xiaomi.market.reverse_ad.service.ReverseAdScheduleService"
        ]
    );
    Ok(())
}

#[test]
fn component_profile_applies_and_restores_available_pm_hide()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a component rule whose package can be hidden by pm hide.
    let runner = ScriptedRunner::with_outputs(vec![
        CommandOutput::success("package:/data/app/base.apk\n", ""),
        CommandOutput::success("", ""),
        CommandOutput::success("", ""),
        CommandOutput::success("", ""),
        CommandOutput::success("", ""),
        CommandOutput::success("", ""),
        CommandOutput::success("", ""),
    ]);
    let ledger = MemoryLedger::default();
    let executor = AndroidProfileExecutor::new(&runner, &ledger);
    let rule = ComponentProfileRule::new(
        "mi-market-reverse-ad",
        0,
        "com.xiaomi.market/com.xiaomi.market.reverse_ad.service.ReverseAdScheduleService",
        PmHidePolicy::TryHide,
    )?;

    // When: the rule is applied and then restored.
    let apply = executor.apply_component(&rule)?;
    executor.restore_component(&apply.record)?;

    // Then: pm hide and pm unhide are both driven through the runner.
    assert_eq!(apply.status, ProfileOperationStatus::Applied);
    assert!(apply.record.contains("\"hide_status\":\"applied\""));
    let records = ledger.records();
    assert_eq!(records.len(), 2);
    assert!(
        records
            .first()
            .is_some_and(|record| record.contains("\"hide_status\":\"skipped_unavailable\"")),
        "ledger-before-mutation record must stay provisional"
    );
    assert!(
        records
            .get(1)
            .is_some_and(|record| record.contains("\"hide_status\":\"applied\"")),
        "durable ledger must include confirmed pm hide success"
    );
    assert_eq!(
        runner.call_lines(),
        [
            "/system/bin/pm path com.xiaomi.market",
            "/system/bin/pm list packages -d com.xiaomi.market",
            "/system/bin/pm list packages --hidden com.xiaomi.market",
            "/system/bin/pm hide com.xiaomi.market",
            "/system/bin/pm disable-user --user 0 com.xiaomi.market/com.xiaomi.market.reverse_ad.service.ReverseAdScheduleService",
            "/system/bin/pm unhide com.xiaomi.market",
            "/system/bin/pm enable --user 0 com.xiaomi.market/com.xiaomi.market.reverse_ad.service.ReverseAdScheduleService",
        ]
    );
    Ok(())
}
