use std::fs;
use std::path::Path;

use crate::sqlite_actions::error::SqliteActionError;
use crate::sqlite_actions::target::path_kind::{
    has_bad_component, has_database_extension, invalid, is_host_test_app_database_path,
};

pub(in crate::sqlite_actions) fn validate_host_target_path(
    path: &Path,
) -> Result<(), SqliteActionError> {
    if !path.is_absolute() || path.as_os_str().is_empty() {
        return invalid(path, "sqlite target must be an absolute path");
    }
    if has_bad_component(path) {
        return invalid(path, "sqlite target must not contain traversal");
    }
    if !has_database_extension(path) {
        return invalid(path, "sqlite target must use a database extension");
    }
    if !is_host_test_app_database_path(path) {
        return invalid(path, "sqlite target must be an app database path");
    }
    reject_symlink(path)
}

pub(super) fn reject_symlink(path: &Path) -> Result<(), SqliteActionError> {
    if fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return invalid(path, "sqlite symlink targets are rejected");
    }
    Ok(())
}
