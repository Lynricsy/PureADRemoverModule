use std::fs;
use std::path::Path;

use crate::file_actions::error::FileActionError;
use crate::secure_fs::{create_dir_all_no_symlink, ensure_parent_beneath_root_no_symlink};

pub(super) fn directory_is_empty(path: &Path) -> Result<bool, FileActionError> {
    let mut entries = fs::read_dir(path).map_err(|source| FileActionError::io(path, source))?;
    match entries.next() {
        Some(Ok(_entry)) => Ok(false),
        Some(Err(source)) => Err(FileActionError::io(path, source)),
        None => Ok(true),
    }
}

pub(super) fn required_backup_path<'a>(
    path: &Path,
    backup_path: Option<&'a Path>,
) -> Result<&'a Path, FileActionError> {
    backup_path.ok_or_else(|| FileActionError::rejected_target(path, "backup path is required"))
}

pub(super) fn ensure_parent(path: &Path) -> Result<(), FileActionError> {
    let Some(parent) = path.parent() else {
        return Err(FileActionError::rejected_target(path, "path has no parent"));
    };
    create_dir_all_no_symlink(parent).map_err(|source| FileActionError::io(parent, source))
}

pub(super) fn ensure_parent_inside_root(
    path: &Path,
    filesystem_root: &Path,
) -> Result<(), FileActionError> {
    ensure_parent_beneath_root_no_symlink(path, filesystem_root)
        .map_err(|source| FileActionError::io(path, source))
}
