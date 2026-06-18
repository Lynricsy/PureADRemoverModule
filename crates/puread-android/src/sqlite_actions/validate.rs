use std::path::Path;

use puread_core::restore_ledger::OriginalFileType;

use crate::secure_fs::ensure_parent_beneath_root_no_symlink;
use crate::sqlite_actions::error::SqliteActionError;
use crate::sqlite_actions::metadata::SqliteTargetMetadata;
use crate::sqlite_actions::target::validate_host_target_path;
use crate::sqlite_actions::types::{SqliteAction, SqliteActionSchedule};

pub(super) fn validate_request(
    path: &Path,
    schedule: SqliteActionSchedule,
) -> Result<(), SqliteActionError> {
    if !schedule.is_allowed() {
        return Err(SqliteActionError::UnsupportedSchedule {
            schedule,
            path: path.to_path_buf(),
        });
    }
    validate_host_target_path(path)
}

pub(super) fn validate_target_for_execution(
    path: &Path,
    filesystem_root: &Path,
) -> Result<(), SqliteActionError> {
    ensure_parent_beneath_root_no_symlink(path, filesystem_root).map_err(|source| {
        SqliteActionError::Io {
            path: path.to_path_buf(),
            source,
        }
    })
}

pub(super) fn require_metadata<'a>(
    path: &Path,
    metadata: Option<&'a SqliteTargetMetadata>,
) -> Result<&'a SqliteTargetMetadata, SqliteActionError> {
    metadata.ok_or_else(|| SqliteActionError::InvalidTarget {
        path: path.to_path_buf(),
        reason: "sqlite target metadata could not be collected",
    })
}

pub(super) fn validate_target_type(
    path: &Path,
    metadata: &SqliteTargetMetadata,
) -> Result<(), SqliteActionError> {
    match metadata.file_type {
        OriginalFileType::File | OriginalFileType::Missing => Ok(()),
        OriginalFileType::Directory | OriginalFileType::Symlink | OriginalFileType::Other => {
            invalid(path, "sqlite target must be a regular file or missing path")
        }
    }
}

pub(super) fn reject_hardlinked_target(
    path: &Path,
    metadata: &SqliteTargetMetadata,
) -> Result<(), SqliteActionError> {
    if metadata.file_type == OriginalFileType::File && metadata.nlink > 1 {
        return invalid(path, "sqlite hardlinked target rejected");
    }
    Ok(())
}

pub(super) fn ensure_parent_for_mutation(
    path: &Path,
    filesystem_root: &Path,
    action: SqliteAction,
    metadata: &SqliteTargetMetadata,
) -> Result<(), SqliteActionError> {
    if metadata.file_type != OriginalFileType::Missing || action == SqliteAction::Delete {
        return Ok(());
    }
    let Some(parent) = path.parent() else {
        return invalid(path, "sqlite target requires a parent directory");
    };
    if parent.is_dir() {
        validate_target_for_execution(path, filesystem_root)
    } else {
        invalid(path, "sqlite target parent directory is missing")
    }
}

fn invalid<T>(path: &Path, reason: &'static str) -> Result<T, SqliteActionError> {
    Err(SqliteActionError::InvalidTarget {
        path: path.to_path_buf(),
        reason,
    })
}
