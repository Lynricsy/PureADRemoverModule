use std::path::{Path, PathBuf};

use puread_core::restore_ledger::{LedgerRecord, RestoreLedger, RestoreStep};

use crate::sqlite_actions::error::SqliteActionError;

pub(super) fn existing_ledger_record(
    ledger: &RestoreLedger,
    record: &LedgerRecord,
) -> Result<Option<LedgerRecord>, SqliteActionError> {
    let key = record.key();
    Ok(ledger
        .read_records()
        .map_err(|source| SqliteActionError::Ledger {
            path: record.original_path.clone().into(),
            source,
        })?
        .into_iter()
        .find(|stored| stored.key() == key))
}

pub(super) fn restore_content_path(record: &LedgerRecord) -> Option<PathBuf> {
    record.restore_steps.iter().find_map(|step| match step {
        RestoreStep::RestoreContent { backup_path } => Some(PathBuf::from(backup_path)),
        RestoreStep::RecreateDirectory
        | RestoreStep::RecreateFile
        | RestoreStep::RemovePlaceholder
        | RestoreStep::SetMode { .. }
        | RestoreStep::SetOwner { .. }
        | RestoreStep::SetSelinuxContext { .. }
        | RestoreStep::SetImmutable { .. } => None,
    })
}

pub(super) fn validate_existing_backup(
    backup_path: Option<&Path>,
) -> Result<(), SqliteActionError> {
    let Some(path) = backup_path else {
        return Ok(());
    };
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.file_type().is_symlink() => return Ok(()),
        Ok(_metadata) => {
            return Err(SqliteActionError::InvalidTarget {
                path: path.to_path_buf(),
                reason: "stored sqlite backup path is symlink",
            });
        }
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(SqliteActionError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
    }
    Err(SqliteActionError::InvalidTarget {
        path: path.to_path_buf(),
        reason: "stored sqlite backup missing before repeated mutation",
    })
}
