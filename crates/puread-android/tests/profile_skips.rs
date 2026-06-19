#![doc = "`Profile` 跳过语义回归测试。"]

#[expect(
    dead_code,
    reason = "shared test fixture exposes helpers used by sibling integration tests"
)]
#[path = "appops_components/support.rs"]
mod support;

use puread_android::command_runner::{CommandOutput, CommandRunnerError};
use puread_android::profiles::{AndroidProfileExecutor, AppOpProfileRule, ProfileOperationStatus};
use support::{MemoryLedger, ScriptedRunner};

#[test]
fn appops_profile_skips_when_package_is_not_installed() -> Result<(), Box<dyn std::error::Error>> {
    // Given: an AppOps rule points at an optional app that is not installed.
    let runner = ScriptedRunner::with_outputs(vec![CommandOutput::failure(
        1,
        "",
        "Error: package not found",
    )]);
    let ledger = MemoryLedger::default();
    let executor = AndroidProfileExecutor::new(&runner, &ledger);
    let rule = AppOpProfileRule::new(
        "optional-app-location",
        "com.optional.missing",
        "MONITOR_LOCATION",
        "ignore",
    )?;

    // When: the rule is applied as part of an automatic profile.
    let apply = executor.apply_appop(&rule)?;

    // Then: the missing optional target is skipped instead of counted as failed.
    assert_eq!(apply.status, ProfileOperationStatus::Skipped);
    assert!(apply.record.contains("\"kind\":\"app_skipped\""));
    assert!(
        apply
            .record
            .contains("\"reason\":\"package_not_installed\"")
    );
    assert_eq!(
        ledger.records().as_slice(),
        std::slice::from_ref(&apply.record)
    );
    assert_eq!(
        runner.call_lines(),
        ["/system/bin/pm path com.optional.missing"]
    );
    Ok(())
}

#[test]
fn appops_profile_fails_when_package_probe_fails_without_output()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a package probe exits non-zero without enough missing-package evidence.
    let runner = ScriptedRunner::with_outputs(vec![CommandOutput::failure(1, "", "")]);
    let ledger = MemoryLedger::default();
    let executor = AndroidProfileExecutor::new(&runner, &ledger);
    let rule = AppOpProfileRule::new(
        "optional-empty-appops",
        "com.optional.empty",
        "MONITOR_LOCATION",
        "ignore",
    )?;

    // When: the appops profile is applied automatically.
    let error = executor
        .apply_appop(&rule)
        .expect_err("opaque pm path failures must stay visible");

    // Then: the true command failure is not converted into a skipped record.
    assert!(error.to_string().contains("command failed"));
    assert!(ledger.records().is_empty());
    assert_eq!(
        runner.call_lines(),
        ["/system/bin/pm path com.optional.empty"]
    );
    Ok(())
}

#[test]
fn appops_profile_fails_when_package_probe_reports_generic_not_found()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a package probe fails with a generic not-found message.
    let runner =
        ScriptedRunner::with_outputs(vec![CommandOutput::failure(1, "", "service not found")]);
    let ledger = MemoryLedger::default();
    let executor = AndroidProfileExecutor::new(&runner, &ledger);
    let rule = AppOpProfileRule::new(
        "generic-not-found-appops",
        "com.optional.generic",
        "MONITOR_LOCATION",
        "ignore",
    )?;

    // When: the appops profile is applied automatically.
    let error = executor
        .apply_appop(&rule)
        .expect_err("generic not-found errors must stay visible");

    // Then: only explicit missing-package text is downgraded to skipped.
    assert!(error.to_string().contains("command failed"));
    assert!(ledger.records().is_empty());
    assert_eq!(
        runner.call_lines(),
        ["/system/bin/pm path com.optional.generic"]
    );
    Ok(())
}

#[test]
fn appops_profile_skips_when_package_probe_returns_empty_path()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a package probe exits successfully but returns no install path.
    let runner = ScriptedRunner::with_outputs(vec![CommandOutput::success("", "")]);
    let ledger = MemoryLedger::default();
    let executor = AndroidProfileExecutor::new(&runner, &ledger);
    let rule = AppOpProfileRule::new(
        "optional-empty-path-appops",
        "com.optional.empty_path",
        "MONITOR_LOCATION",
        "ignore",
    )?;

    // When: the appops profile is applied automatically.
    let apply = executor.apply_appop(&rule)?;

    // Then: the empty package probe is treated as not applicable, not failed.
    assert_eq!(apply.status, ProfileOperationStatus::Skipped);
    assert!(apply.record.contains("\"kind\":\"app_skipped\""));
    assert!(
        apply
            .record
            .contains("\"reason\":\"package_not_installed\"")
    );
    assert_eq!(
        ledger.records().as_slice(),
        std::slice::from_ref(&apply.record)
    );
    assert_eq!(
        runner.call_lines(),
        ["/system/bin/pm path com.optional.empty_path"]
    );
    Ok(())
}

#[test]
fn appops_profile_fails_when_appops_probe_reports_unknown_operation()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: the package exists but cmd appops rejects the operation.
    let runner = ScriptedRunner::with_outputs(vec![
        CommandOutput::success("package:/data/app/base.apk\n", ""),
        CommandOutput::failure(1, "", "Unknown operation: MONITOR_LOCATION"),
    ]);
    let ledger = MemoryLedger::default();
    let executor = AndroidProfileExecutor::new(&runner, &ledger);
    let rule = AppOpProfileRule::new(
        "unknown-appop",
        "com.optional.present",
        "MONITOR_LOCATION",
        "ignore",
    )?;

    // When: the appops profile is applied automatically.
    let error = executor
        .apply_appop(&rule)
        .expect_err("unknown appops operations must stay visible");

    // Then: unsupported AppOps are real profile failures, not skipped optional targets.
    assert!(error.to_string().contains("command failed"));
    assert!(ledger.records().is_empty());
    assert_eq!(
        runner.call_lines(),
        [
            "/system/bin/pm path com.optional.present",
            "/system/bin/cmd appops get com.optional.present MONITOR_LOCATION",
        ]
    );
    Ok(())
}

#[test]
fn appops_profile_fails_when_appops_runner_is_unavailable() -> Result<(), Box<dyn std::error::Error>>
{
    // Given: the package exists but the command runner cannot launch appops.
    let runner = ScriptedRunner::with_script(vec![
        Ok(CommandOutput::success("package:/data/app/base.apk\n", "")),
        Err(CommandRunnerError::Unavailable {
            detail: "cmd unavailable".to_owned(),
        }),
    ]);
    let ledger = MemoryLedger::default();
    let executor = AndroidProfileExecutor::new(&runner, &ledger);
    let rule = AppOpProfileRule::new(
        "unavailable-appop-runner",
        "com.optional.present",
        "MONITOR_LOCATION",
        "ignore",
    )?;

    // When: the appops profile is applied automatically.
    let error = executor
        .apply_appop(&rule)
        .expect_err("command runner errors must stay visible");

    // Then: command unavailability is not converted into a skipped record.
    assert!(error.to_string().contains("command unavailable"));
    assert!(ledger.records().is_empty());
    assert_eq!(
        runner.call_lines(),
        [
            "/system/bin/pm path com.optional.present",
            "/system/bin/cmd appops get com.optional.present MONITOR_LOCATION",
        ]
    );
    Ok(())
}
