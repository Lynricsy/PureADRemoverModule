use serde::Serialize;

use crate::cli::ScanArgs;
use crate::error::CliError;
use crate::json::{SCHEMA_VERSION, display_path, write_json};
use crate::lock::{GlobalLock, lock_path};
use crate::profile_execute::{ApplyActionReport, ExecutionSummary};
use crate::rule_plan::{ActionPlan, PlannedAction, ensure_root_dir};

#[derive(Debug, Serialize)]
struct ScanDryRunDocument {
    schema_version: u8,
    mode: &'static str,
    dry_run: bool,
    root_path: String,
    rule_file_count: usize,
    action_count: usize,
    will_mutate: bool,
    actions: Vec<PlannedAction>,
}

#[derive(Debug, Serialize)]
struct ScanExecuteDocument {
    schema_version: u8,
    mode: &'static str,
    dry_run: bool,
    root_path: String,
    rule_file_count: usize,
    action_count: usize,
    will_mutate: bool,
    applied: usize,
    skipped: usize,
    failed: usize,
    actions: Vec<ApplyActionReport>,
}

pub fn run_scan(args: &ScanArgs) -> Result<(), CliError> {
    if args.dry_run && args.execute {
        return Err(CliError::ConflictingExecutionMode);
    }
    ensure_root_dir(args.root.as_path())?;
    let plan = ActionPlan::new(args.root.as_path(), args.rules.as_path(), None)?;
    if args.execute {
        let module_root = args.root.join("data/adb/modules/puread");
        let lock = lock_path(module_root.as_path(), None)?;
        let _lock = GlobalLock::acquire(lock.as_path())?;
        return write_execute_document(
            args,
            &plan,
            ExecutionSummary::execute(&plan, args.root.as_path()),
        );
    }
    let summary = ExecutionSummary::dry_run(&plan);
    let reports = summary.reports();
    let document = ScanDryRunDocument {
        schema_version: SCHEMA_VERSION,
        mode: if args.execute { "execute" } else { "dry_run" },
        dry_run: !args.execute,
        root_path: display_path(args.root.as_path()),
        rule_file_count: plan.rule_file_count(),
        action_count: reports.len(),
        will_mutate: args.execute,
        actions: plan.actions().to_vec(),
    };
    write_json(&document)
}

fn write_execute_document(
    args: &ScanArgs,
    plan: &ActionPlan,
    summary: ExecutionSummary,
) -> Result<(), CliError> {
    let applied = summary.applied();
    let skipped = summary.skipped();
    let failed = summary.failed();
    let reports = summary.reports();
    let document = ScanExecuteDocument {
        schema_version: SCHEMA_VERSION,
        mode: "execute",
        dry_run: false,
        root_path: display_path(args.root.as_path()),
        rule_file_count: plan.rule_file_count(),
        action_count: reports.len(),
        will_mutate: true,
        applied,
        skipped,
        failed,
        actions: reports,
    };
    write_json(&document)
}
