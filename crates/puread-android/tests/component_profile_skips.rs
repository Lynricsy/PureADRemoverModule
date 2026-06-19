#![doc = "`Component` profile 跳过语义回归测试。"]

#[expect(
    dead_code,
    reason = "shared test fixture exposes helpers used by sibling integration tests"
)]
#[path = "appops_components/support.rs"]
mod support;

use puread_android::command_runner::CommandOutput;
use puread_android::profiles::{
    AndroidProfileExecutor, ComponentProfileRule, PmHidePolicy, ProfileOperationStatus,
};
use support::{MemoryLedger, ScriptedRunner};

#[test]
fn component_profile_skips_when_package_is_not_installed() -> Result<(), Box<dyn std::error::Error>>
{
    // Given: a component rule points at a device-specific package that is absent.
    let runner = ScriptedRunner::with_outputs(vec![CommandOutput::failure(
        1,
        "",
        "Error: unknown package: com.vendor.ads",
    )]);
    let ledger = MemoryLedger::default();
    let executor = AndroidProfileExecutor::new(&runner, &ledger);
    let rule = ComponentProfileRule::new(
        "optional-vendor-component",
        0,
        "com.vendor.ads/com.vendor.ads.ReverseAdService",
        PmHidePolicy::TryHide,
    )?;

    // When: the component profile is applied automatically.
    let apply = executor.apply_component(&rule)?;

    // Then: no component mutation is attempted and the profile reports skipped.
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
    assert_eq!(runner.call_lines(), ["/system/bin/pm path com.vendor.ads"]);
    Ok(())
}

#[test]
fn component_profile_fails_when_package_probe_fails_without_output()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a package probe exits non-zero without enough missing-package evidence.
    let runner = ScriptedRunner::with_outputs(vec![CommandOutput::failure(1, "", "")]);
    let ledger = MemoryLedger::default();
    let executor = AndroidProfileExecutor::new(&runner, &ledger);
    let rule = ComponentProfileRule::new(
        "optional-empty-component",
        0,
        "com.vendor.emptyfail/com.vendor.emptyfail.ReverseAdService",
        PmHidePolicy::TryHide,
    )?;

    // When: the component profile is applied automatically.
    let error = executor
        .apply_component(&rule)
        .expect_err("opaque pm path failures must stay visible");

    // Then: the true command failure is not converted into a skipped record.
    assert!(error.to_string().contains("command failed"));
    assert!(ledger.records().is_empty());
    assert_eq!(
        runner.call_lines(),
        ["/system/bin/pm path com.vendor.emptyfail"]
    );
    Ok(())
}

#[test]
fn component_profile_skips_when_package_probe_returns_empty_path()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a package probe exits successfully but returns no install path.
    let runner = ScriptedRunner::with_outputs(vec![CommandOutput::success("", "")]);
    let ledger = MemoryLedger::default();
    let executor = AndroidProfileExecutor::new(&runner, &ledger);
    let rule = ComponentProfileRule::new(
        "optional-empty-path-component",
        0,
        "com.vendor.empty/com.vendor.empty.ReverseAdService",
        PmHidePolicy::TryHide,
    )?;

    // When: the component profile is applied automatically.
    let apply = executor.apply_component(&rule)?;

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
        ["/system/bin/pm path com.vendor.empty"]
    );
    Ok(())
}
