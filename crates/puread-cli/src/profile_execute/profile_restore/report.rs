use std::io::Write as _;
use std::path::Path;

use serde::Serialize;

use crate::cli::ReportFormat;
use crate::error::CliError;
use crate::json::{SCHEMA_VERSION, display_path, write_json};
use crate::profile_execute::profile_restore::{final_state, ledger::ProfileLedgerEntry};

#[derive(Debug, Serialize)]
pub struct ProfileRestoreReport {
    schema_version: u8,
    command: &'static str,
    mode: &'static str,
    ledger_path: String,
    record_count: usize,
    pending_restore_count: usize,
    will_mutate: bool,
    restored: usize,
    skipped: usize,
    failed: usize,
    actions: Vec<ProfileRestoreAction>,
}

#[derive(Debug, Serialize)]
pub struct ProfileRestoreAction {
    kind: String,
    status: &'static str,
    error: Option<String>,
    #[serde(skip)]
    entry_indexes: Vec<usize>,
}

impl ProfileRestoreReport {
    pub(super) const fn failed(&self) -> usize {
        self.failed
    }
}

impl ProfileRestoreAction {
    pub(super) fn from_result(
        kind: &str,
        entry_indexes: Vec<usize>,
        result: Result<(), puread_android::profiles::ProfileError>,
    ) -> Self {
        Self {
            kind: kind.to_owned(),
            status: if result.is_ok() { "restored" } else { "failed" },
            error: result.err().map(|error| error.to_string()),
            entry_indexes,
        }
    }

    pub(super) fn failed(&self) -> bool {
        self.status == "failed"
    }

    pub(super) fn restored(&self) -> bool {
        self.status == "restored"
    }

    pub(super) const fn entry_indexes(&self) -> &[usize] {
        self.entry_indexes.as_slice()
    }
}

pub(super) fn planned(path: &Path, entries: &[ProfileLedgerEntry]) -> ProfileRestoreReport {
    let actions = final_state::entries(entries)
        .iter()
        .map(|entry| ProfileRestoreAction {
            kind: entry.ledger_entry().kind().to_owned(),
            status: "planned",
            error: None,
            entry_indexes: Vec::new(),
        })
        .collect();
    document(path, "profile_restore", "dry_run", false, entries, actions)
}

pub(super) fn document(
    path: &Path,
    command: &'static str,
    mode: &'static str,
    will_mutate: bool,
    entries: &[ProfileLedgerEntry],
    actions: Vec<ProfileRestoreAction>,
) -> ProfileRestoreReport {
    let pending_restore_count = final_state::entries(entries).len();
    let restored = actions
        .iter()
        .filter(|action| action.status == "restored")
        .count();
    let skipped = actions
        .iter()
        .filter(|action| action.status == "skipped")
        .count();
    let failed = actions.iter().filter(|action| action.failed()).count();
    ProfileRestoreReport {
        schema_version: SCHEMA_VERSION,
        command,
        mode,
        ledger_path: display_path(path),
        record_count: entries.len(),
        pending_restore_count,
        will_mutate,
        restored,
        skipped,
        failed,
        actions,
    }
}

pub(super) fn write(report: &ProfileRestoreReport, format: ReportFormat) -> Result<(), CliError> {
    match format {
        ReportFormat::Json => write_json(report),
        ReportFormat::Text => {
            let mut stdout = std::io::stdout().lock();
            writeln!(
                stdout,
                "{} {} restored={} failed={} pending={}",
                report.command,
                report.mode,
                report.restored,
                report.failed,
                report.pending_restore_count
            )
            .map_err(|source| CliError::OutputWrite { source })
        }
    }
}
