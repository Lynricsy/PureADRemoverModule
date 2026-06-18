use std::path::{Path, PathBuf};

use puread_core::restore_ledger::{
    LedgerRecord, RestoreAttempt, RestoreLedger, RestoreStatus, RestoreStep,
};
use serde::Serialize;

use crate::error::CliError;
use crate::json::{SCHEMA_VERSION, display_path, write_json};
use crate::lock::{GlobalLock, lock_path};
use crate::restore_fs::{
    GuardedBackup, RestoreRoot, recreate_directory, remove_placeholder, restore_content,
    restore_empty_file, set_mode,
};

#[derive(Debug, Serialize)]
pub struct RestoreDocument {
    schema_version: u8,
    mode: &'static str,
    ledger_path: String,
    action_count: usize,
    will_mutate: bool,
    restored: usize,
    failed: usize,
    actions: Vec<RestoreActionReport>,
}

#[derive(Debug, Serialize)]
struct RestoreActionReport {
    original_path: String,
    host_path: String,
    status: &'static str,
    error: Option<String>,
}

pub fn dry_run(ledger_path: &Path) -> Result<(), CliError> {
    let records = RestoreLedger::at(ledger_path.to_path_buf()).records_for_restore()?;
    let actions = records
        .iter()
        .map(|record| RestoreActionReport {
            original_path: record.original_path.clone(),
            host_path: String::new(),
            status: "planned",
            error: None,
        })
        .collect::<Vec<_>>();
    write_document(ledger_path, "dry_run", false, actions, 0)
}

pub fn execute(ledger_path: &Path) -> Result<(), CliError> {
    let module_root = restore_module_root(ledger_path)?;
    let lock = lock_path(module_root.as_path(), None)?;
    let _lock = GlobalLock::acquire(lock.as_path())?;
    let ledger = RestoreLedger::at(ledger_path.to_path_buf());
    let records = ledger.records_for_restore()?;
    let context = RestoreContext::new(module_root.as_path(), ledger_path)?;
    let mut attempts = Vec::new();
    let actions = records
        .iter()
        .map(|record| restore_record(record, &context, &mut attempts))
        .collect::<Vec<_>>();
    let removed = ledger.apply_restore_attempts(&attempts)?;
    write_document(ledger_path, "execute", true, actions, removed)
}

fn restore_record(
    record: &LedgerRecord,
    context: &RestoreContext,
    attempts: &mut Vec<RestoreAttempt>,
) -> RestoreActionReport {
    let host_path = context.root.map_android_path(&record.original_path);
    let result = apply_steps(host_path.as_path(), &record.restore_steps, context);
    let status = if result.is_ok() {
        RestoreStatus::Succeeded
    } else {
        RestoreStatus::Failed
    };
    attempts.push(RestoreAttempt {
        key: record.key(),
        status,
    });
    RestoreActionReport {
        original_path: record.original_path.clone(),
        host_path: display_path(host_path.as_path()),
        status: if result.is_ok() { "restored" } else { "failed" },
        error: result.err().map(|error| error.to_string()),
    }
}

fn apply_steps(
    path: &Path,
    steps: &[RestoreStep],
    context: &RestoreContext,
) -> Result<(), CliError> {
    for step in steps {
        match step {
            RestoreStep::RestoreContent { backup_path } => {
                let backup = GuardedBackup::new(backup_path, context.backup_root.as_path())?;
                restore_content(path, &backup, &context.root)?;
            }
            RestoreStep::RecreateDirectory => recreate_directory(path, &context.root)?,
            RestoreStep::RecreateFile => restore_empty_file(path, &context.root)?,
            RestoreStep::RemovePlaceholder => remove_placeholder(path, &context.root)?,
            RestoreStep::SetMode { mode } => set_mode(path, *mode, &context.root)?,
            RestoreStep::SetOwner { .. }
            | RestoreStep::SetSelinuxContext { .. }
            | RestoreStep::SetImmutable { .. } => {}
        }
    }
    Ok(())
}

fn restore_module_root(ledger_path: &Path) -> Result<PathBuf, CliError> {
    let Some(state_dir) = ledger_path.parent() else {
        return Err(CliError::RestorePathOutOfRoot {
            path: display_path(ledger_path),
        });
    };
    let Some(module_dir) = state_dir.parent() else {
        return Err(CliError::RestorePathOutOfRoot {
            path: display_path(ledger_path),
        });
    };
    if !module_dir.ends_with(Path::new("data/adb/modules/puread")) {
        return Err(CliError::RestorePathOutOfRoot {
            path: display_path(ledger_path),
        });
    }
    Ok(module_dir.to_path_buf())
}

fn restore_root_from_module(module_dir: &Path, ledger_path: &Path) -> Result<PathBuf, CliError> {
    module_dir
        .ancestors()
        .nth(4)
        .map(Path::to_path_buf)
        .ok_or_else(|| CliError::RestorePathOutOfRoot {
            path: display_path(ledger_path),
        })
}

#[derive(Debug)]
struct RestoreContext {
    root: RestoreRoot,
    backup_root: PathBuf,
}

impl RestoreContext {
    fn new(module_dir: &Path, ledger_path: &Path) -> Result<Self, CliError> {
        let host_root = restore_root_from_module(module_dir, ledger_path)?;
        let root = RestoreRoot::new(host_root)?;
        let state_dir = ledger_path
            .parent()
            .ok_or_else(|| CliError::RestorePathOutOfRoot {
                path: display_path(ledger_path),
            })?;
        root.guard_existing_path(module_dir)?;
        root.guard_existing_path(state_dir)?;
        let backup_root = state_dir.join("backups");
        Ok(Self { root, backup_root })
    }
}

fn write_document(
    ledger_path: &Path,
    mode: &'static str,
    will_mutate: bool,
    actions: Vec<RestoreActionReport>,
    removed: usize,
) -> Result<(), CliError> {
    let restored = actions
        .iter()
        .filter(|action| action.status == "restored")
        .count();
    let failed = actions
        .iter()
        .filter(|action| action.status == "failed")
        .count();
    let document = RestoreDocument {
        schema_version: SCHEMA_VERSION,
        mode,
        ledger_path: display_path(ledger_path),
        action_count: actions.len(),
        will_mutate,
        restored: restored.max(removed),
        failed,
        actions,
    };
    write_json(&document)
}
