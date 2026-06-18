use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use puread_core::restore_ledger::OriginalFileType;

use crate::file_actions::error::FileActionError;
use crate::file_actions::mutate::guards::{
    guard_open_file_matches_snapshot, open_existing_read_no_follow,
    run_before_file_discard_cleanup_for_tests, run_before_file_move_to_backup_for_tests,
};
use crate::file_actions::snapshot::TargetSnapshot;
use crate::secure_fs::{remove_file_no_symlink, rename_new_no_symlink, rename_no_symlink};

use super::path_ops::ensure_parent;

static NEXT_DISCARD_ID: AtomicUsize = AtomicUsize::new(0);

pub(super) fn discard_existing_target(
    path: &Path,
    snapshot: &TargetSnapshot,
    backup_path: &Path,
) -> Result<(), FileActionError> {
    if snapshot.original_type == OriginalFileType::Directory {
        return Err(FileActionError::rejected_target(
            path,
            "directory discard cleanup is not supported",
        ));
    }
    let discard_path = discard_path_for(backup_path);
    ensure_parent(&discard_path)?;
    let file = open_existing_read_no_follow(path)?;
    guard_open_file_matches_snapshot(path, snapshot, &file)?;
    run_before_file_move_to_backup_for_tests(path);
    rename_new_no_symlink(path, &discard_path)
        .map_err(|source| FileActionError::io(path, source))?;
    if let Err(error) = guard_moved_target_matches_snapshot(&discard_path, snapshot) {
        if let Err(source) = rename_no_symlink(&discard_path, path) {
            return Err(FileActionError::rollback_failed(path, source));
        }
        return Err(error);
    }
    run_before_file_discard_cleanup_for_tests(&discard_path);
    remove_discarded_target(&discard_path, snapshot)
}

pub(super) fn move_target_to_backup(
    path: &Path,
    snapshot: &TargetSnapshot,
    backup_path: &Path,
) -> Result<(), FileActionError> {
    ensure_parent(backup_path)?;
    let file = open_existing_read_no_follow(path)?;
    guard_open_file_matches_snapshot(path, snapshot, &file)?;
    run_before_file_move_to_backup_for_tests(path);
    rename_new_no_symlink(path, backup_path).map_err(|source| FileActionError::io(path, source))?;
    if let Err(error) = guard_moved_target_matches_snapshot(backup_path, snapshot) {
        if let Err(source) = rename_no_symlink(backup_path, path) {
            return Err(FileActionError::rollback_failed(path, source));
        }
        return Err(error);
    }
    Ok(())
}

fn discard_path_for(backup_path: &Path) -> std::path::PathBuf {
    let id = NEXT_DISCARD_ID.fetch_add(1, Ordering::Relaxed);
    let suffix = format!("discard-{}-{id}", std::process::id());
    backup_path.with_extension(suffix)
}

fn remove_discarded_target(
    discard_path: &Path,
    snapshot: &TargetSnapshot,
) -> Result<(), FileActionError> {
    match snapshot.original_type {
        OriginalFileType::File => remove_file_no_symlink(discard_path)
            .map_err(|source| FileActionError::io(discard_path, source)),
        OriginalFileType::Directory => Err(FileActionError::rejected_target(
            discard_path,
            "directory discard cleanup is not supported",
        )),
        OriginalFileType::Missing | OriginalFileType::Symlink | OriginalFileType::Other => Ok(()),
    }
}

fn guard_moved_target_matches_snapshot(
    backup_path: &Path,
    snapshot: &TargetSnapshot,
) -> Result<(), FileActionError> {
    let moved = TargetSnapshot::collect_host_path(backup_path)?;
    if moved.original_type != snapshot.original_type || moved.identity != snapshot.identity {
        return Err(FileActionError::rejected_target(
            backup_path,
            "moved target identity does not match snapshot",
        ));
    }
    Ok(())
}
