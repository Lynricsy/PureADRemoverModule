use std::path::Path;

use puread_android::file_actions::FileActionExecutor;
use puread_core::restore_ledger::RestoreLedger;

use crate::error::CliError;
use crate::rule_plan::ActionPlan;

pub const LEDGER_RELATIVE_PATH: &str = "state/actions.jsonl";
pub const PROFILE_LEDGER_RELATIVE_PATH: &str = "state/profile-actions.jsonl";
pub const BACKUP_RELATIVE_PATH: &str = "state/backups";

mod android_dispatch;
mod file_actions;
mod profile_restore;
mod profile_runtime;
mod report;

pub use profile_restore::{profile_report, restore_profile};
pub use report::{ApplyActionReport, ExecutionSummary};

pub fn combined_summary(reports: Vec<ApplyActionReport>) -> ExecutionSummary {
    ExecutionSummary::from_reports(reports)
}

pub fn execute_android_profile_surface(
    plan: &ActionPlan,
    module_root: &Path,
    #[cfg(debug_assertions)] test_runner: bool,
    #[cfg(debug_assertions)] runner_log: Option<&Path>,
    #[cfg(debug_assertions)] ledger_fail: bool,
) -> Vec<ApplyActionReport> {
    android_dispatch::execute_android_profile_surface(
        plan,
        module_root,
        #[cfg(debug_assertions)]
        test_runner,
        #[cfg(debug_assertions)]
        runner_log,
        #[cfg(debug_assertions)]
        ledger_fail,
    )
}

pub fn preflight_profile_ledger(module_root: &Path) -> Result<(), CliError> {
    profile_runtime::JsonlProfileLedger::preflight_for_append(module_root)
        .map_err(|source| CliError::AndroidProfile { source })
}

impl ExecutionSummary {
    pub fn dry_run(plan: &ActionPlan) -> Self {
        let reports = plan
            .actions()
            .iter()
            .map(|action| report::file_report(action, "planned", None))
            .collect::<Vec<_>>();
        Self::from_reports(reports)
    }

    pub fn execute(plan: &ActionPlan, root: &Path) -> Self {
        let module_root = root.join("data/adb/modules/puread");
        let ledger = RestoreLedger::at(module_root.join(LEDGER_RELATIVE_PATH));
        let executor = FileActionExecutor::new(ledger, module_root.join(BACKUP_RELATIVE_PATH));
        let reports = plan
            .actions()
            .iter()
            .filter(|action| action.target_kind == "path")
            .map(|action| file_actions::execute_action(action, &executor, root))
            .collect::<Vec<_>>();
        Self::from_reports(reports)
    }
}
