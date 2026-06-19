use puread_core::model::ProfileKind;
use puread_core::restore_ledger::RestoreLedger;
use serde::Serialize;

use crate::cli::{
    ApplyProfileArgs, DumpReportArgs, JsonFieldIsZeroArgs, ProfileReportArgs, ProfileRestoreArgs,
    StatusArgs,
};
use crate::error::CliError;
use crate::json::{SCHEMA_VERSION, display_path, write_json};
use crate::lock::{GlobalLock, lock_is_held, lock_path};
use crate::profile_execute::{
    ApplyActionReport, ExecutionSummary, combined_summary, execute_android_profile_surface,
    preflight_profile_ledger,
};
use crate::rule_plan::{ActionPlan, ensure_root_dir};

#[derive(Debug, Serialize)]
struct StatusDocument {
    schema_version: u8,
    command: &'static str,
    root_path: String,
    module_root: String,
    lock_path: String,
    lock_held: bool,
}

#[derive(Debug, Serialize)]
struct ApplyProfileDocument {
    schema_version: u8,
    command: &'static str,
    mode: &'static str,
    profile: String,
    root_path: String,
    module_root: String,
    lock_path: String,
    lock_acquired: bool,
    rule_file_count: usize,
    action_count: usize,
    will_mutate: bool,
    applied: usize,
    skipped: usize,
    failed: usize,
    actions: Vec<ApplyActionReport>,
}

#[derive(Debug, Serialize)]
struct DumpReportDocument {
    schema_version: u8,
    command: &'static str,
    ledger_path: String,
    record_count: usize,
    pending_restore_count: usize,
}

pub fn run_status(args: &StatusArgs) -> Result<(), CliError> {
    let lock_path = lock_path(&args.paths.module_root, args.paths.lock_path.as_deref())?;
    let document = StatusDocument {
        schema_version: SCHEMA_VERSION,
        command: "status",
        root_path: display_path(args.paths.root.as_path()),
        module_root: display_path(args.paths.module_root.as_path()),
        lock_path: display_path(lock_path.as_path()),
        lock_held: lock_is_held(lock_path.as_path())?,
    };
    write_json(&document)
}

pub fn run_apply_profile(args: &ApplyProfileArgs) -> Result<(), CliError> {
    if args.dry_run && args.execute {
        return Err(CliError::ConflictingExecutionMode);
    }
    ensure_root_dir(args.paths.root.as_path())?;
    let profile = ProfileKind::parse(&args.profile).map_err(CliError::Model)?;
    let plan = ActionPlan::new(
        args.paths.root.as_path(),
        args.paths.rules.as_path(),
        Some(args.paths.module_root.as_path()),
        Some(profile.as_str()),
    )?;
    let lock_path = lock_path(&args.paths.module_root, args.paths.lock_path.as_deref())?;
    if args.execute {
        let _lock = GlobalLock::acquire(lock_path.as_path())?;
        preflight_profile_ledger_if_needed(&plan, args.paths.module_root.as_path())?;
        let file_summary = ExecutionSummary::execute(
            &plan,
            args.paths.root.as_path(),
            args.paths.module_root.as_path(),
        );
        let mut reports = file_summary.reports();
        reports.extend(execute_android_profile_surface(
            &plan,
            args.paths.module_root.as_path(),
            #[cfg(debug_assertions)]
            args.profile_test.test_profile_runner,
            #[cfg(debug_assertions)]
            args.profile_test.profile_runner_log.as_deref(),
            #[cfg(debug_assertions)]
            args.profile_test.test_profile_ledger_fail,
        ));
        let summary = combined_summary(reports);
        return write_apply_document(args, profile, &plan, lock_path.as_path(), true, summary);
    }
    write_apply_document(
        args,
        profile,
        &plan,
        lock_path.as_path(),
        false,
        ExecutionSummary::dry_run(&plan),
    )
}

pub fn run_profile_report(args: &ProfileReportArgs) -> Result<(), CliError> {
    crate::profile_execute::profile_report(args.paths.module_root.as_path(), args.format)
}

pub fn run_profile_restore(args: &ProfileRestoreArgs) -> Result<(), CliError> {
    if args.dry_run && args.execute {
        return Err(CliError::ConflictingExecutionMode);
    }
    crate::profile_execute::restore_profile(
        args.paths.module_root.as_path(),
        args.paths.lock_path.as_deref(),
        args.execute,
        args.format,
        #[cfg(debug_assertions)]
        args.test_profile_runner,
        #[cfg(debug_assertions)]
        args.profile_runner_log.as_deref(),
    )
}

pub fn run_dump_report(args: &DumpReportArgs) -> Result<(), CliError> {
    super::ledger::ensure_ledger_file(args.ledger.as_path())?;
    let ledger = RestoreLedger::at(args.ledger.clone());
    let records = ledger.read_records()?;
    let pending = ledger.records_for_restore()?;
    let document = DumpReportDocument {
        schema_version: SCHEMA_VERSION,
        command: "dump_report",
        ledger_path: display_path(args.ledger.as_path()),
        record_count: records.len(),
        pending_restore_count: pending.len(),
    };
    write_json(&document)
}

pub fn run_json_field_is_zero(args: &JsonFieldIsZeroArgs) -> Result<(), CliError> {
    let bytes = std::fs::read(args.file.as_path()).map_err(|source| CliError::Filesystem {
        path: display_path(args.file.as_path()),
        source,
    })?;
    let value = serde_json::from_slice::<serde_json::Value>(&bytes).map_err(|source| {
        CliError::ProfileLedgerJson {
            path: display_path(args.file.as_path()),
            source,
        }
    })?;
    if value.get(&args.field).and_then(serde_json::Value::as_u64) == Some(0) {
        return Ok(());
    }
    Err(CliError::JsonFieldNotZero {
        path: display_path(args.file.as_path()),
        field: args.field.clone(),
    })
}

fn preflight_profile_ledger_if_needed(
    plan: &ActionPlan,
    module_root: &std::path::Path,
) -> Result<(), CliError> {
    if plan
        .actions()
        .iter()
        .any(|action| action.target_kind.as_str() != "path")
    {
        preflight_profile_ledger(module_root)?;
    }
    Ok(())
}

fn write_apply_document(
    args: &ApplyProfileArgs,
    profile: ProfileKind,
    plan: &ActionPlan,
    lock_path: &std::path::Path,
    lock_acquired: bool,
    summary: ExecutionSummary,
) -> Result<(), CliError> {
    let mode = if lock_acquired { "execute" } else { "dry_run" };
    let will_mutate = lock_acquired;
    let applied = summary.applied();
    let skipped = summary.skipped();
    let failed = summary.failed();
    let reports = summary.reports();
    let document = ApplyProfileDocument {
        schema_version: SCHEMA_VERSION,
        command: "apply_profile",
        mode,
        profile: profile.as_str().to_owned(),
        root_path: display_path(args.paths.root.as_path()),
        module_root: display_path(args.paths.module_root.as_path()),
        lock_path: display_path(lock_path),
        lock_acquired,
        rule_file_count: plan.rule_file_count(),
        action_count: reports.len(),
        will_mutate,
        applied,
        skipped,
        failed,
        actions: reports,
    };
    write_json(&document)
}
