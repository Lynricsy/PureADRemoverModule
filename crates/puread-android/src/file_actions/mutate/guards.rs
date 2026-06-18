use std::fs::{self, File};
use std::path::Path;

use puread_core::restore_ledger::OriginalFileType;

use crate::file_actions::error::FileActionError;
use crate::file_actions::request::FileActionKind;
use crate::file_actions::snapshot::TargetSnapshot;
use crate::secure_fs::{
    ensure_parent_no_symlink, open_create_new_read_write_no_follow, open_read_no_follow,
    open_read_write_no_follow,
};

pub(in crate::file_actions::mutate) fn guard_path_for_snapshot(
    path: &Path,
    snapshot: &TargetSnapshot,
    allowed_after_missing: &[OriginalFileType],
) -> Result<(), FileActionError> {
    ensure_parent_no_symlink(path).map_err(|source| FileActionError::io(path, source))?;
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(source)
            if source.kind() == std::io::ErrorKind::NotFound
                && snapshot.original_type == OriginalFileType::Missing
                && !allowed_after_missing.is_empty() =>
        {
            return Ok(());
        }
        Err(source) => return Err(FileActionError::io(path, source)),
    };
    if metadata.file_type().is_symlink() {
        return Err(FileActionError::rejected_target(
            path,
            "symlink target rejected",
        ));
    }
    let current_type = original_type_from_metadata(&metadata);
    if snapshot.original_type == OriginalFileType::Missing
        && allowed_after_missing.contains(&current_type)
    {
        return Ok(());
    }
    if current_type != snapshot.original_type {
        return Err(FileActionError::rejected_target(
            path,
            "target type changed before mutation",
        ));
    }
    if snapshot.identity_changed_from_metadata(&metadata) {
        return Err(FileActionError::rejected_target(
            path,
            "target identity changed before mutation",
        ));
    }
    Ok(())
}

pub(in crate::file_actions::mutate) const fn allowed_after_missing(
    action: FileActionKind,
) -> &'static [OriginalFileType] {
    match action {
        FileActionKind::EmptyFile => &[OriginalFileType::File],
        FileActionKind::EmptyDir => &[OriginalFileType::Directory],
        FileActionKind::Delete | FileActionKind::Chmod000 => &[],
    }
}

pub(in crate::file_actions::mutate) fn guard_open_file_matches_snapshot(
    path: &Path,
    snapshot: &TargetSnapshot,
    file: &File,
) -> Result<(), FileActionError> {
    if snapshot.original_type == OriginalFileType::Missing {
        return Ok(());
    }
    let current = TargetSnapshot::collect_open_file(path, file)?;
    if current.original_type != snapshot.original_type {
        return Err(FileActionError::rejected_target(
            path,
            "target type changed during mutation",
        ));
    }
    if snapshot.identity_changed_from_metadata(
        &file
            .metadata()
            .map_err(|source| FileActionError::io(path, source))?,
    ) {
        return Err(FileActionError::rejected_target(
            path,
            "target identity changed during mutation",
        ));
    }
    Ok(())
}

pub(in crate::file_actions::mutate) fn open_existing_no_follow(
    path: &Path,
) -> Result<File, FileActionError> {
    open_read_write_no_follow(path).map_err(|source| FileActionError::io(path, source))
}

pub(in crate::file_actions::mutate) fn open_existing_read_no_follow(
    path: &Path,
) -> Result<File, FileActionError> {
    open_read_no_follow(path).map_err(|source| FileActionError::io(path, source))
}

pub(in crate::file_actions::mutate) fn open_empty_file_target(
    path: &Path,
    snapshot: &TargetSnapshot,
) -> Result<File, FileActionError> {
    if snapshot.original_type == OriginalFileType::Missing {
        return open_create_new_read_write_no_follow(path)
            .map_err(|source| FileActionError::io(path, source));
    }
    open_read_write_no_follow(path).map_err(|source| FileActionError::io(path, source))
}

#[cfg(unix)]
pub(in crate::file_actions::mutate) fn fd_path_for_metadata_operation(
    file: &File,
    path: &Path,
) -> Result<std::path::PathBuf, FileActionError> {
    use std::os::fd::AsRawFd;

    let fd_path = std::path::PathBuf::from(format!("/proc/self/fd/{}", file.as_raw_fd()));
    if fd_path.exists() {
        return Ok(fd_path);
    }
    Err(FileActionError::rejected_target(
        path,
        "fd metadata path is unavailable",
    ))
}

#[cfg(test)]
pub(in crate::file_actions::mutate) fn run_before_file_helper_guard_for_tests(path: &Path) {
    super::test_hooks::run_before_file_helper_guard(path);
}

#[cfg(not(test))]
pub(in crate::file_actions::mutate) const fn run_before_file_helper_guard_for_tests(_path: &Path) {}

#[cfg(test)]
pub(in crate::file_actions::mutate) fn run_after_file_helper_guard_for_tests(path: &Path) {
    super::test_hooks::run_after_file_helper_guard(path);
}

#[cfg(not(test))]
pub(in crate::file_actions::mutate) const fn run_after_file_helper_guard_for_tests(_path: &Path) {}

#[cfg(test)]
pub(in crate::file_actions::mutate) fn run_before_file_delete_for_tests(path: &Path) {
    super::test_hooks::run_before_file_delete(path);
}

#[cfg(not(test))]
pub(in crate::file_actions::mutate) const fn run_before_file_delete_for_tests(_path: &Path) {}

#[cfg(test)]
pub(in crate::file_actions::mutate) fn run_before_file_move_to_backup_for_tests(path: &Path) {
    super::test_hooks::run_before_file_move_to_backup(path);
}

#[cfg(not(test))]
pub(in crate::file_actions::mutate) const fn run_before_file_move_to_backup_for_tests(
    _path: &Path,
) {
}

#[cfg(test)]
pub(in crate::file_actions::mutate) fn run_before_file_discard_cleanup_for_tests(path: &Path) {
    super::test_hooks::run_before_file_discard_cleanup(path);
}

#[cfg(not(test))]
pub(in crate::file_actions::mutate) const fn run_before_file_discard_cleanup_for_tests(
    _path: &Path,
) {
}

fn original_type_from_metadata(metadata: &fs::Metadata) -> OriginalFileType {
    if metadata.is_file() {
        OriginalFileType::File
    } else if metadata.is_dir() {
        OriginalFileType::Directory
    } else {
        OriginalFileType::Other
    }
}
