mod backup_move;
mod path_ops;

use std::fs;
use std::path::Path;

use puread_core::restore_ledger::OriginalFileType;

use crate::file_actions::error::FileActionError;
use crate::file_actions::mutate::guards::{
    allowed_after_missing, guard_open_file_matches_snapshot, guard_path_for_snapshot,
    open_empty_file_target, open_existing_no_follow, run_after_file_helper_guard_for_tests,
    run_before_file_delete_for_tests, run_before_file_helper_guard_for_tests,
};
use crate::file_actions::plan::FileActionPlan;
use crate::file_actions::request::FileActionKind;
use crate::file_actions::snapshot::TargetSnapshot;

use backup_move::{discard_existing_target, move_target_to_backup};
use path_ops::{
    directory_is_empty, ensure_parent, ensure_parent_inside_root, required_backup_path,
};

pub(in crate::file_actions) fn apply_primary_action(
    plan: &FileActionPlan,
    snapshot: &TargetSnapshot,
    backup_path: Option<&Path>,
    reused_existing_record: bool,
) -> Result<(), FileActionError> {
    match plan.action() {
        FileActionKind::Delete => remove_existing(
            plan.target().host_path(),
            snapshot,
            backup_path,
            reused_existing_record,
        ),
        FileActionKind::EmptyFile => empty_file(
            plan.target().host_path(),
            plan.target().filesystem_root(),
            snapshot,
        ),
        FileActionKind::EmptyDir => empty_dir(
            plan.target().host_path(),
            plan.target().filesystem_root(),
            snapshot,
            backup_path,
            reused_existing_record,
        ),
        FileActionKind::Chmod000 => chmod(plan.target().host_path(), snapshot, 0),
    }
}

fn remove_existing(
    path: &Path,
    snapshot: &TargetSnapshot,
    backup_path: Option<&Path>,
    reused_existing_record: bool,
) -> Result<(), FileActionError> {
    guard_path_for_snapshot(path, snapshot, &[])?;
    match snapshot.original_type {
        OriginalFileType::File | OriginalFileType::Directory => {
            run_before_file_delete_for_tests(path);
            let backup_path = required_backup_path(path, backup_path)?;
            if reused_existing_record && path_entry_exists(backup_path)? {
                return discard_existing_target(path, snapshot, backup_path);
            }
            move_target_to_backup(path, snapshot, backup_path)
        }
        OriginalFileType::Missing => Ok(()),
        OriginalFileType::Symlink | OriginalFileType::Other => Err(
            FileActionError::rejected_target(path, "unsupported file type"),
        ),
    }
}

fn empty_file(
    path: &Path,
    filesystem_root: &Path,
    snapshot: &TargetSnapshot,
) -> Result<(), FileActionError> {
    if snapshot.original_type == OriginalFileType::Directory {
        return Err(FileActionError::rejected_target(
            path,
            "directory cannot be emptied as file",
        ));
    }
    ensure_parent(path)?;
    if snapshot.original_type == OriginalFileType::Missing {
        ensure_parent_inside_root(path, filesystem_root)?;
    }
    run_before_file_helper_guard_for_tests(path);
    guard_path_for_snapshot(
        path,
        snapshot,
        allowed_after_missing(FileActionKind::EmptyFile),
    )?;
    run_after_file_helper_guard_for_tests(path);
    if snapshot.original_type == OriginalFileType::Missing {
        ensure_parent_inside_root(path, filesystem_root)?;
    }
    let file = open_empty_file_target(path, snapshot)?;
    guard_open_file_matches_snapshot(path, snapshot, &file)?;
    file.set_len(0)
        .map_err(|source| FileActionError::io(path, source))
}

fn empty_dir(
    path: &Path,
    filesystem_root: &Path,
    snapshot: &TargetSnapshot,
    backup_path: Option<&Path>,
    reused_existing_record: bool,
) -> Result<(), FileActionError> {
    if snapshot.original_type == OriginalFileType::File {
        return Err(FileActionError::rejected_target(
            path,
            "file cannot be emptied as directory",
        ));
    }
    run_before_file_helper_guard_for_tests(path);
    guard_path_for_snapshot(
        path,
        snapshot,
        allowed_after_missing(FileActionKind::EmptyDir),
    )?;
    if snapshot.original_type == OriginalFileType::Directory {
        let backup_path = required_backup_path(path, backup_path)?;
        if reused_existing_record && path_entry_exists(backup_path)? {
            if directory_is_empty(path)? {
                return Ok(());
            }
            discard_existing_target(path, snapshot, backup_path)?;
            return crate::secure_fs::create_dir_all_no_symlink(path)
                .map_err(|source| FileActionError::io(path, source));
        }
        move_target_to_backup(path, snapshot, backup_path)?;
        return crate::secure_fs::create_dir_all_no_symlink(path)
            .map_err(|source| FileActionError::io(path, source));
    }
    if snapshot.original_type == OriginalFileType::Missing {
        ensure_parent_inside_root(path, filesystem_root)?;
    }
    crate::secure_fs::create_dir_all_no_symlink(path)
        .map_err(|source| FileActionError::io(path, source))
}

#[cfg(unix)]
fn chmod(path: &Path, snapshot: &TargetSnapshot, mode: u32) -> Result<(), FileActionError> {
    use std::os::unix::fs::PermissionsExt;

    run_before_file_helper_guard_for_tests(path);
    guard_path_for_snapshot(path, snapshot, &[])?;
    run_after_file_helper_guard_for_tests(path);
    let file = open_existing_no_follow(path)?;
    guard_open_file_matches_snapshot(path, snapshot, &file)?;
    file.set_permissions(fs::Permissions::from_mode(mode))
        .map_err(|source| FileActionError::io(path, source))
}

#[cfg(not(unix))]
fn chmod(path: &Path, _snapshot: &TargetSnapshot, _mode: u32) -> Result<(), FileActionError> {
    Err(FileActionError::rejected_target(
        path,
        "chmod requires unix filesystem",
    ))
}

fn path_entry_exists(path: &Path) -> Result<bool, FileActionError> {
    match fs::symlink_metadata(path) {
        Ok(_metadata) => Ok(true),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(source) => Err(FileActionError::io(path, source)),
    }
}
