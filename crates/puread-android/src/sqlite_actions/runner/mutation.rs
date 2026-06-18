use std::fs::File;
use std::io::Write;
use std::path::Path;

use fs2::FileExt;
use puread_core::restore_ledger::OriginalFileType;

use crate::secure_fs::ensure_parent_no_symlink;
use crate::sqlite_actions::error::SqliteActionError;
use crate::sqlite_actions::metadata::{SqliteTargetMetadata, identity_changed};
use crate::sqlite_actions::minimal::minimal_sqlite_image;
use crate::sqlite_actions::types::SqliteAction;

use super::delete;
use super::hooks::{run_after_sqlite_write_for_tests, run_before_sqlite_write_open_for_tests};
use super::write_ops::{
    guard_file_matches_original, guard_file_still_addressed, open_existing_read_no_follow,
    open_write_no_follow, set_mode, validate_written_file,
};

type LockedSqliteFile = File;

pub(super) fn lock_existing(
    path: &Path,
    metadata: &SqliteTargetMetadata,
) -> Result<Option<LockedSqliteFile>, SqliteActionError> {
    if metadata.file_type == OriginalFileType::Missing {
        return Ok(None);
    }
    let file = open_existing_read_no_follow(path)?;
    file.try_lock_exclusive()
        .map_err(|source| SqliteActionError::Locked {
            path: path.to_path_buf(),
            source,
        })?;
    Ok(Some(file))
}

pub(super) fn guard_and_mutate(
    path: &Path,
    action: SqliteAction,
    metadata: &SqliteTargetMetadata,
    backup_path: Option<&Path>,
    reused_existing_record: bool,
) -> Result<(), SqliteActionError> {
    guard_target_unchanged(path, metadata)?;
    mutate(path, action, metadata, backup_path, reused_existing_record)
}

fn mutate(
    path: &Path,
    action: SqliteAction,
    metadata: &SqliteTargetMetadata,
    backup_path: Option<&Path>,
    reused_existing_record: bool,
) -> Result<(), SqliteActionError> {
    match action {
        SqliteAction::Delete => {
            delete::delete_existing(path, metadata, backup_path, reused_existing_record)
        }
        SqliteAction::MinimalSqlite => write_minimal(path, metadata, metadata.mode, false),
        SqliteAction::DenyWrite => write_minimal(path, metadata, 0o444, true),
    }
}

fn write_minimal(
    path: &Path,
    metadata: &SqliteTargetMetadata,
    mode: u32,
    readonly: bool,
) -> Result<(), SqliteActionError> {
    let image = minimal_sqlite_image(path)?;
    run_before_sqlite_write_open_for_tests(path);
    reject_symlink_parent_after_hook(path)?;
    let mut file = open_write_no_follow(path, metadata.file_type == OriginalFileType::Missing)?;
    guard_file_matches_original(path, &file, metadata)?;
    file.write_all(&image)
        .map_err(|source| SqliteActionError::PostMutationIo {
            path: path.to_path_buf(),
            source,
        })?;
    file.set_len(image.len() as u64)
        .map_err(|source| SqliteActionError::PostMutationIo {
            path: path.to_path_buf(),
            source,
        })?;
    run_after_sqlite_write_for_tests(path);
    validate_written_file(path, &mut file).map_err(post_mutation_error)?;
    set_mode(&file, path, if readonly { 0o444 } else { mode }).map_err(post_mutation_error)?;
    guard_file_still_addressed(path, &file).map_err(post_mutation_error)
}

fn reject_symlink_parent_after_hook(path: &Path) -> Result<(), SqliteActionError> {
    ensure_parent_no_symlink(path).map_err(|source| SqliteActionError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn post_mutation_error(error: SqliteActionError) -> SqliteActionError {
    match error {
        SqliteActionError::Io { path, source } => {
            SqliteActionError::PostMutationIo { path, source }
        }
        SqliteActionError::Integrity { path, reason } => {
            SqliteActionError::PostMutationIntegrity { path, reason }
        }
        SqliteActionError::InvalidTarget { path, reason } => {
            SqliteActionError::PostMutationInvalidTarget { path, reason }
        }
        other => other,
    }
}

fn guard_target_unchanged(
    path: &Path,
    metadata: &SqliteTargetMetadata,
) -> Result<(), SqliteActionError> {
    let current = SqliteTargetMetadata::collect(path)?;
    if current.file_type != metadata.file_type {
        return invalid_changed(path);
    }
    if current.file_type == OriginalFileType::Symlink {
        return invalid_changed(path);
    }
    if identity_changed(metadata.identity, current.identity) {
        return invalid_changed(path);
    }
    Ok(())
}

fn invalid_changed(path: &Path) -> Result<(), SqliteActionError> {
    Err(SqliteActionError::InvalidTarget {
        path: path.to_path_buf(),
        reason: "sqlite target changed before mutation",
    })
}
