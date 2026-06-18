use puread_android::command_runner::CommandOutput;
use puread_android::profiles::{AndroidProfileExecutor, ComponentProfileRule, PmHidePolicy};

use crate::support::{MemoryLedger, ScriptedRunner};

#[test]
fn component_profile_reports_error_when_confirmed_hide_record_cannot_persist()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: the provisional ledger append succeeds but confirmed append fails after pm hide.
    let runner = ScriptedRunner::with_outputs(vec![
        CommandOutput::success("package:/data/app/base.apk\n", ""),
        CommandOutput::success("", ""),
        CommandOutput::success("", ""),
        CommandOutput::success("", ""),
    ]);
    let ledger = MemoryLedger::failing_after_successes(1);
    let executor = AndroidProfileExecutor::new(&runner, &ledger);
    let rule = ComponentProfileRule::new(
        "mi-market-reverse-ad",
        0,
        "com.xiaomi.market/com.xiaomi.market.reverse_ad.service.ReverseAdScheduleService",
        PmHidePolicy::TryHide,
    )?;

    // When: confirmed durable state cannot be appended after a successful hide mutation.
    let error = executor
        .apply_component(&rule)
        .expect_err("confirmed ledger failure");

    // Then: the operation is not reported as safely applied and disable-user is not attempted.
    assert!(error.to_string().contains("ledger sink failed"));
    let records = ledger.records();
    assert_eq!(records.len(), 1);
    assert!(
        records
            .first()
            .is_some_and(|record| record.contains("\"hide_status\":\"skipped_unavailable\"")),
        "only the conservative provisional record may persist"
    );
    assert_eq!(
        runner.call_lines(),
        [
            "/system/bin/pm path com.xiaomi.market",
            "/system/bin/pm list packages -d com.xiaomi.market",
            "/system/bin/pm list packages --hidden com.xiaomi.market",
            "/system/bin/pm hide com.xiaomi.market",
        ]
    );
    Ok(())
}
