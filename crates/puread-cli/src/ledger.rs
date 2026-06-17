use std::fs;
use std::io;
use std::path::Path;

use puread_core::restore_ledger::{LedgerRecord, RestoreLedger, RestoreStep};
use serde::Serialize;

use crate::cli::{LedgerCommand, LedgerSubcommand, RestoreArgs};
use crate::error::CliError;
use crate::json::{SCHEMA_VERSION, display_path, write_json};

#[derive(Debug, Serialize)]
struct LedgerShowDocument {
    schema_version: u8,
    command: &'static str,
    ledger_path: String,
    record_count: usize,
    records: Vec<LedgerRecord>,
}

#[derive(Debug, Serialize)]
struct RestoreDryRunDocument {
    schema_version: u8,
    mode: &'static str,
    ledger_path: String,
    action_count: usize,
    will_mutate: bool,
    actions: Vec<RestoreDryRunAction>,
}

#[derive(Debug, Serialize)]
struct RestoreDryRunAction {
    original_path: String,
    profile: String,
    source_action: puread_core::restore_ledger::LedgerAction,
    restore_steps: Vec<RestoreStep>,
}

pub fn run_ledger(command: LedgerCommand) -> Result<(), CliError> {
    match command.command {
        LedgerSubcommand::Show(args) => show_ledger(args.ledger.as_path()),
    }
}

pub fn run_restore(args: &RestoreArgs) -> Result<(), CliError> {
    if !args.dry_run {
        return Err(CliError::RealRestoreUnsupported);
    }
    restore_dry_run(args.ledger.as_path())
}

fn show_ledger(path: &Path) -> Result<(), CliError> {
    ensure_ledger_file(path)?;
    let records = RestoreLedger::at(path.to_path_buf()).read_records()?;
    let document = LedgerShowDocument {
        schema_version: SCHEMA_VERSION,
        command: "ledger_show",
        ledger_path: display_path(path),
        record_count: records.len(),
        records,
    };
    write_json(&document)
}

fn restore_dry_run(path: &Path) -> Result<(), CliError> {
    ensure_ledger_file(path)?;
    let records = RestoreLedger::at(path.to_path_buf()).records_for_restore()?;
    let actions = records
        .into_iter()
        .map(|record| RestoreDryRunAction {
            original_path: record.original_path,
            profile: record.profile,
            source_action: record.action,
            restore_steps: record.restore_steps,
        })
        .collect::<Vec<_>>();
    let document = RestoreDryRunDocument {
        schema_version: SCHEMA_VERSION,
        mode: "dry_run",
        ledger_path: display_path(path),
        action_count: actions.len(),
        will_mutate: false,
        actions,
    };
    write_json(&document)
}

fn ensure_ledger_file(path: &Path) -> Result<(), CliError> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(source) if source.kind() == io::ErrorKind::NotFound => {
            return Err(CliError::MissingLedger {
                path: display_path(path),
            });
        }
        Err(source) => {
            return Err(CliError::Filesystem {
                path: display_path(path),
                source,
            });
        }
    };
    if metadata.is_file() {
        return Ok(());
    }
    Err(CliError::LedgerNotFile {
        path: display_path(path),
    })
}
