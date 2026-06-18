use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::secure_fs::{remove_file_no_symlink, rename_new_no_symlink, rename_no_symlink};
use crate::sqlite_actions::error::SqliteActionError;
use crate::sqlite_actions::metadata::{SqliteTargetMetadata, identity_changed};

use super::hooks::{
    run_before_sqlite_delete_for_tests, run_before_sqlite_delete_move_for_tests,
    run_before_sqlite_discard_cleanup_for_tests,
};
use super::write_ops::{guard_file_matches_original, open_existing_for_delete};

static NEXT_SQLITE_DISCARD_ID: AtomicUsize = AtomicUsize::new(0);

pub(super) fn delete_existing(
    path: &Path,
    metadata: &SqliteTargetMetadata,
    backup_path: Option<&Path>,
    reused_existing_record: bool,
) -> Result<(), SqliteActionError> {
    let backup_path = backup_path.ok_or_else(|| SqliteActionError::InvalidTarget {
        path: path.to_path_buf(),
        reason: "sqlite delete requires backup path",
    })?;
    if path_entry_exists(backup_path)? && !reused_existing_record {
        return Err(SqliteActionError::InvalidTarget {
            path: backup_path.to_path_buf(),
            reason: "sqlite delete backup already exists",
        });
    }
    run_before_sqlite_delete_for_tests(path);
    if reused_existing_record && path_entry_exists(backup_path)? {
        return discard_existing_sqlite(path, metadata, backup_path);
    }
    let file = open_existing_for_delete(path)?;
    guard_file_matches_original(path, &file, metadata)?;
    run_before_sqlite_delete_move_for_tests(path);
    rename_new_no_symlink(path, backup_path).map_err(|source| SqliteActionError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    if let Err(error) = guard_moved_sqlite_matches_original(backup_path, metadata) {
        if let Err(source) = rename_no_symlink(backup_path, path) {
            return Err(SqliteActionError::rollback_failed(path, source));
        }
        return Err(error);
    }
    Ok(())
}

fn discard_existing_sqlite(
    path: &Path,
    metadata: &SqliteTargetMetadata,
    backup_path: &Path,
) -> Result<(), SqliteActionError> {
    let discard_path = discard_path_for(backup_path);
    if path_entry_exists(&discard_path)? {
        return Err(SqliteActionError::InvalidTarget {
            path: discard_path,
            reason: "sqlite discard path already exists",
        });
    }
    let file = open_existing_for_delete(path)?;
    guard_file_matches_original(path, &file, metadata)?;
    run_before_sqlite_delete_move_for_tests(path);
    rename_new_no_symlink(path, &discard_path).map_err(|source| SqliteActionError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    if let Err(error) = guard_moved_sqlite_matches_original(&discard_path, metadata) {
        if let Err(source) = rename_no_symlink(&discard_path, path) {
            return Err(SqliteActionError::rollback_failed(path, source));
        }
        return Err(error);
    }
    run_before_sqlite_discard_cleanup_for_tests(&discard_path);
    remove_file_no_symlink(&discard_path).map_err(|source| SqliteActionError::Io {
        path: discard_path,
        source,
    })
}

fn discard_path_for(backup_path: &Path) -> std::path::PathBuf {
    let id = NEXT_SQLITE_DISCARD_ID.fetch_add(1, Ordering::Relaxed);
    let suffix = format!("discard-{}-{id}", std::process::id());
    backup_path.with_extension(suffix)
}

fn guard_moved_sqlite_matches_original(
    backup_path: &Path,
    metadata: &SqliteTargetMetadata,
) -> Result<(), SqliteActionError> {
    let moved = SqliteTargetMetadata::collect(backup_path)?;
    if moved.file_type != metadata.file_type || identity_changed(metadata.identity, moved.identity)
    {
        return Err(SqliteActionError::InvalidTarget {
            path: backup_path.to_path_buf(),
            reason: "moved sqlite target identity does not match snapshot",
        });
    }
    Ok(())
}

fn path_entry_exists(path: &Path) -> Result<bool, SqliteActionError> {
    match std::fs::symlink_metadata(path) {
        Ok(_metadata) => Ok(true),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(source) => Err(SqliteActionError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}
