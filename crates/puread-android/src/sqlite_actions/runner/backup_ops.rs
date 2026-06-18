use std::fs::File;
use std::path::{Path, PathBuf};

use crate::secure_fs::remove_file_no_symlink;
use crate::sqlite_actions::backup::copy_open_file_to_backup;
use crate::sqlite_actions::error::SqliteActionError;
use crate::sqlite_actions::metadata::SqliteTargetMetadata;
use crate::sqlite_actions::types::SqliteAction;

use super::restore_content_path;

pub(super) fn copy_backup_from_locked_file(
    action: SqliteAction,
    lock: &mut Option<File>,
    path: &Path,
    record: &puread_core::restore_ledger::LedgerRecord,
) -> Result<Option<PathBuf>, SqliteActionError> {
    if action == SqliteAction::Delete {
        return Ok(None);
    }
    let (Some(file), Some(backup_path)) = (lock.as_mut(), restore_content_path(record)) else {
        return Ok(None);
    };
    if copy_open_file_to_backup(file, path, &backup_path)? {
        Ok(Some(backup_path))
    } else {
        Ok(None)
    }
}

pub(super) fn remove_created_backup(backup_path: Option<&Path>) {
    if let Some(path) = backup_path {
        let _ignored = remove_file_no_symlink(path);
    }
}

pub(super) fn guard_locked_file_matches_original(
    lock: Option<&File>,
    path: &Path,
    metadata: &SqliteTargetMetadata,
) -> Result<(), SqliteActionError> {
    if let Some(file) = lock {
        super::write_ops::guard_file_matches_original(path, file, metadata)?;
    }
    Ok(())
}
