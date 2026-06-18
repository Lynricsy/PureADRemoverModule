mod final_state;
mod ledger;
mod report;

use std::path::{Path, PathBuf};

use puread_android::profiles::{AndroidProfileExecutor, ProfileLedgerSink};

use crate::cli::ReportFormat;
use crate::error::CliError;
use crate::lock::{GlobalLock, lock_path};
use crate::profile_execute::profile_runtime::SelectedProfileRunner;

pub fn profile_ledger_path(module_root: &Path) -> PathBuf {
    super::profile_runtime::JsonlProfileLedger::path(module_root)
}

pub fn profile_report(module_root: &Path, format: ReportFormat) -> Result<(), CliError> {
    let path = profile_ledger_path(module_root);
    let entries = ledger::read_entries(module_root, path.as_path())?;
    let document = report::document(
        path.as_path(),
        "profile_report",
        "report",
        false,
        &entries,
        Vec::new(),
    );
    report::write(&document, format)
}

pub fn restore_profile(
    module_root: &Path,
    lock_override: Option<&Path>,
    execute: bool,
    format: ReportFormat,
    #[cfg(debug_assertions)] test_runner: bool,
    #[cfg(debug_assertions)] runner_log: Option<&Path>,
) -> Result<(), CliError> {
    let path = profile_ledger_path(module_root);
    if !execute {
        let entries = ledger::read_entries(module_root, path.as_path())?;
        return report::write(&report::planned(path.as_path(), &entries), format);
    }
    let lock = lock_path(module_root, lock_override)?;
    let _lock = GlobalLock::acquire(lock.as_path())?;
    let entries = ledger::read_entries(module_root, path.as_path())?;
    let actions = restore_entries(
        &entries,
        #[cfg(debug_assertions)]
        test_runner,
        #[cfg(debug_assertions)]
        runner_log,
    );
    ledger::rewrite_restored(path.as_path(), &entries, &actions)?;
    let document = report::document(
        path.as_path(),
        "profile_restore",
        "execute",
        true,
        &entries,
        actions,
    );
    let failed = document.failed();
    report::write(&document, format)?;
    if failed == 0 {
        return Ok(());
    }
    Err(CliError::ProfileRestoreFailed { failed })
}

fn restore_entries(
    entries: &[ledger::ProfileLedgerEntry],
    #[cfg(debug_assertions)] test_runner: bool,
    #[cfg(debug_assertions)] runner_log: Option<&Path>,
) -> Vec<report::ProfileRestoreAction> {
    let entries = final_state::entries(entries);
    let runner = select_runner(
        #[cfg(debug_assertions)]
        test_runner,
        #[cfg(debug_assertions)]
        runner_log,
    );
    let ledger = NoopProfileLedger;
    let executor = AndroidProfileExecutor::new(&runner, &ledger);
    entries
        .iter()
        .map(|entry| restore_entry(entry, &executor))
        .collect()
}

fn restore_entry<R, L>(
    entry: &final_state::FinalRestoreEntry,
    executor: &AndroidProfileExecutor<'_, R, L>,
) -> report::ProfileRestoreAction
where
    R: puread_android::command_runner::AndroidCommandRunner,
    L: ProfileLedgerSink,
{
    let ledger_entry = entry.ledger_entry();
    let result = match ledger_entry.kind() {
        "app_op" => executor.restore_appop(ledger_entry.raw()),
        "component" => executor.restore_component(ledger_entry.raw()),
        "rom_setting" | "shared_prefs_bool" | "rom_skipped" => {
            executor.restore_rom(ledger_entry.raw())
        }
        _ => Ok(()),
    };
    report::ProfileRestoreAction::from_result(
        ledger_entry.kind(),
        entry.restored_indexes().to_vec(),
        result,
    )
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

#[derive(Debug)]
struct NoopProfileLedger;

impl ProfileLedgerSink for NoopProfileLedger {
    fn append(&self, _record: String) -> Result<(), puread_android::profiles::ProfileError> {
        Ok(())
    }
}
