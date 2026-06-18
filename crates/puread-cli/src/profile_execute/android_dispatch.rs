use std::path::Path;

use puread_android::command_runner::{AndroidCommandRunner, SettingsNamespace};
use puread_android::profiles::{
    AndroidProfileExecutor, AppOpProfileRule, ComponentProfileRule, PmHidePolicy,
    ProfileLedgerSink, ProfileOperationStatus, RomMatcher, RomProfileRule, RomSettingsRule,
    SharedPrefsBoolRule,
};

use super::ApplyActionReport;
use crate::error::CliError;
use crate::profile_execute::profile_runtime::{JsonlProfileLedger, SelectedProfileRunner};
use crate::profile_execute::report::android_report;
use crate::rule_plan::{ActionPlan, PlannedAction};

pub(super) fn execute_android_profile_surface(
    plan: &ActionPlan,
    module_root: &Path,
    #[cfg(debug_assertions)] test_runner: bool,
    #[cfg(debug_assertions)] runner_log: Option<&Path>,
    #[cfg(debug_assertions)] ledger_fail: bool,
) -> Vec<ApplyActionReport> {
    let runner = select_runner(
        #[cfg(debug_assertions)]
        test_runner,
        #[cfg(debug_assertions)]
        runner_log,
    );
    let ledger = select_ledger(
        module_root,
        #[cfg(debug_assertions)]
        ledger_fail,
    );
    execute_android_profile_surface_with(plan, &runner, &ledger)
}

fn select_runner(
    #[cfg(debug_assertions)] test_runner: bool,
    #[cfg(debug_assertions)] runner_log: Option<&Path>,
) -> SelectedProfileRunner {
    #[cfg(debug_assertions)]
    if test_runner {
        return SelectedProfileRunner::scripted(runner_log.map(Path::to_path_buf));
    }
    SelectedProfileRunner::real()
}

fn select_ledger(
    module_root: &Path,
    #[cfg(debug_assertions)] ledger_fail: bool,
) -> JsonlProfileLedger {
    #[cfg(debug_assertions)]
    if ledger_fail {
        return JsonlProfileLedger::failing_for_test(module_root);
    }
    JsonlProfileLedger::new(module_root)
}

fn execute_android_profile_surface_with<R, L>(
    plan: &ActionPlan,
    runner: &R,
    ledger: &L,
) -> Vec<ApplyActionReport>
where
    R: AndroidCommandRunner,
    L: ProfileLedgerSink,
{
    let executor = AndroidProfileExecutor::new(runner, ledger);
    plan.actions()
        .iter()
        .filter(|action| action.target_kind != "path")
        .map(|action| execute_android_action(action, &executor))
        .collect()
}

fn execute_android_action<R, L>(
    action: &PlannedAction,
    executor: &AndroidProfileExecutor<'_, R, L>,
) -> ApplyActionReport
where
    R: AndroidCommandRunner,
    L: ProfileLedgerSink,
{
    match dispatch_android_action(action, executor) {
        Ok((status, record)) => android_report(action, status, Some(record), None),
        Err(error) => android_report(action, "failed", None, Some(error.to_string())),
    }
}

fn dispatch_android_action<R, L>(
    action: &PlannedAction,
    executor: &AndroidProfileExecutor<'_, R, L>,
) -> Result<(&'static str, String), CliError>
where
    R: AndroidCommandRunner,
    L: ProfileLedgerSink,
{
    let operation = match action.target_kind.as_str() {
        "appop" => dispatch_appop(action, executor)?,
        "component" => dispatch_component(action, executor)?,
        "rom" => dispatch_rom(action, executor)?,
        _ => {
            return Err(CliError::InvalidActionTarget {
                path: action.android_path.clone(),
                reason: "unknown profile target kind",
            });
        }
    };
    let status = match operation.status {
        ProfileOperationStatus::Applied => "applied",
        ProfileOperationStatus::Skipped => "skipped",
    };
    Ok((status, operation.record))
}

fn dispatch_appop<R, L>(
    action: &PlannedAction,
    executor: &AndroidProfileExecutor<'_, R, L>,
) -> Result<puread_android::profiles::ProfileOperation, CliError>
where
    R: AndroidCommandRunner,
    L: ProfileLedgerSink,
{
    let op = action
        .appop
        .as_deref()
        .ok_or_else(|| missing_field(action, "appop"))?;
    let mode = action
        .appop_mode
        .as_deref()
        .ok_or_else(|| missing_field(action, "appop_mode"))?;
    let rule = AppOpProfileRule::new(&action.rule_id, &action.package, op, mode)
        .map_err(|source| CliError::AndroidProfile { source })?;
    executor
        .apply_appop(&rule)
        .map_err(|source| CliError::AndroidProfile { source })
}

fn dispatch_component<R, L>(
    action: &PlannedAction,
    executor: &AndroidProfileExecutor<'_, R, L>,
) -> Result<puread_android::profiles::ProfileOperation, CliError>
where
    R: AndroidCommandRunner,
    L: ProfileLedgerSink,
{
    let component = action
        .component
        .as_deref()
        .ok_or_else(|| missing_field(action, "component"))?;
    let rule = ComponentProfileRule::new(
        &action.rule_id,
        0,
        component,
        component_hide_policy(&action.rule_id),
    )
    .map_err(|source| CliError::AndroidProfile { source })?;
    executor
        .apply_component(&rule)
        .map_err(|source| CliError::AndroidProfile { source })
}

fn dispatch_rom<R, L>(
    action: &PlannedAction,
    executor: &AndroidProfileExecutor<'_, R, L>,
) -> Result<puread_android::profiles::ProfileOperation, CliError>
where
    R: AndroidCommandRunner,
    L: ProfileLedgerSink,
{
    let rule = rom_rule_for_action(action)?;
    executor
        .apply_rom(&rule)
        .map_err(|source| CliError::AndroidProfile { source })
}

fn component_hide_policy(rule_id: &str) -> PmHidePolicy {
    if rule_id.starts_with("mi-market-") {
        return PmHidePolicy::TryHide;
    }
    PmHidePolicy::DoNotHide
}

fn rom_rule_for_action(action: &PlannedAction) -> Result<RomProfileRule, CliError> {
    match action.rule_id.as_str() {
        "miui-personalized-ad-enabled" => {
            rom_settings_rule(action, "miui_personalized_ad_enabled", "0")
        }
        "miui-system-ad-solution-switch" => {
            rom_settings_rule(action, "miui_system_ad_solution", "0")
        }
        _ => {
            let rule = SharedPrefsBoolRule::new(
                Path::new(action.android_path.as_str()),
                "key_content_promotion",
                false,
                Path::new("/data/adb/modules/puread/state/profile-backups"),
            )
            .map_err(|source| CliError::AndroidProfile { source })?;
            RomProfileRule::shared_prefs_bool(&action.rule_id, RomMatcher::miui(), rule)
                .map_err(|source| CliError::AndroidProfile { source })
        }
    }
}

fn rom_settings_rule(
    action: &PlannedAction,
    key: &str,
    value: &str,
) -> Result<RomProfileRule, CliError> {
    let settings = RomSettingsRule::new(SettingsNamespace::Global, key, value)
        .map_err(|source| CliError::AndroidCommand { source })?;
    RomProfileRule::settings(&action.rule_id, RomMatcher::miui(), settings)
        .map_err(|source| CliError::AndroidProfile { source })
}

fn missing_field(action: &PlannedAction, field: &'static str) -> CliError {
    CliError::InvalidActionTarget {
        path: action.android_path.clone(),
        reason: field,
    }
}
